//! Teacher runs on the arena packet through the release binary. Each is synthetic and ends at most 20 s after
//! the anchor. Run them on the heavy-run machine:
//! `cargo test --release --locked --manifest-path tools/generative-physiology/driver/Cargo.toml -- --ignored --test-threads=1`.

use gp_driver::{RunSpec, SMOKE_SPEC, SpecId, Table, digest_of_words, hex, summary_value};
use ng_validate::anchor_moment;
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

const DRIVER: &str = env!("CARGO_BIN_EXE_gp-driver");

fn scratch(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance")
        .join(name);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

fn written(directory: &Path, name: &str, text: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, text).expect("a spec file");
    path
}

fn id_of(text: &str) -> SpecId {
    RunSpec::parse(text).expect("a canonical spec").id()
}

fn driver(arguments: &[&str]) -> Output {
    Command::new(DRIVER)
        .args(arguments)
        .output()
        .expect("the driver starts")
}

fn path(path: &Path) -> &str {
    path.to_str().expect("a UTF-8 path")
}

fn run(spec: &Path, out: &Path, extra: &[&str]) -> Output {
    let mut arguments = vec!["run", path(spec), "--out", path(out)];
    arguments.extend_from_slice(extra);
    driver(&arguments)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn said(output: &Output) -> String {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    format!("exit {:?}: {text}", output.status.code())
}

/// The first two words of the driver's result line, and its exit code.
fn result(output: &Output) -> (String, String, Option<i32>) {
    let text = stdout(output);
    let mut words = text.lines().next().unwrap_or_default().split(' ');
    let word = words.next().unwrap_or_default().to_string();
    (
        word,
        words.next().unwrap_or_default().to_string(),
        output.status.code(),
    )
}

fn completed(spec: &Path, out: &Path) -> PathBuf {
    let output = run(spec, out, &[]);
    assert_eq!(output.status.code(), Some(0), "{}", said(&output));
    let text = fs::read_to_string(spec).expect("the spec");
    out.join(id_of(&text).to_string())
}

fn summary(directory: &Path) -> String {
    fs::read_to_string(directory.join("summary.txt")).expect("a summary")
}

fn value(summary: &str, key: &str) -> String {
    summary_value(summary, key)
        .unwrap_or_else(|| panic!("no {key} in {summary}"))
        .to_string()
}

/// Each accepted step's revision, end and seal: what two courses must share to be one course.
fn seal_log(file: &Path) -> Vec<[u64; 6]> {
    let steps = Table::<u64>::read(file).expect("a step table");
    steps
        .iter()
        .map(|row| [row[0], row[1], row[3], row[4], row[5], row[6]])
        .collect()
}

fn table(file: &Path) -> Vec<Vec<u64>> {
    Table::<u64>::read(file)
        .expect("a table")
        .iter()
        .map(<[u64]>::to_vec)
        .collect()
}

/// Every entry under a directory with its inode, size and modification time, so a rewrite shows.
fn listing(root: &Path) -> BTreeMap<PathBuf, [i64; 4]> {
    let mut found = BTreeMap::new();
    let mut waiting = vec![root.to_path_buf()];
    while let Some(directory) = waiting.pop() {
        for entry in fs::read_dir(&directory).expect("a directory") {
            let entry = entry.expect("an entry");
            let meta = entry.metadata().expect("metadata");
            let inode = i64::try_from(meta.ino()).expect("an inode");
            let size = i64::try_from(meta.len()).expect("a size");
            found.insert(entry.path(), [inode, size, meta.mtime(), meta.mtime_nsec()]);
            if meta.is_dir() {
                waiting.push(entry.path());
            }
        }
    }
    found
}

fn names(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = (fs::read_dir(directory).expect("a directory"))
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

/// Polls until `ready` holds, a tenth of a second at a time, or panics after `seconds`.
fn wait_for(seconds: u32, what: &str, mut ready: impl FnMut() -> bool) {
    for _ in 0..seconds * 10 {
        if ready() {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    panic!("{what} after {seconds} s");
}

fn spawn(arguments: &[&str]) -> Child {
    (Command::new(DRIVER).args(arguments))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the driver starts")
}

fn smoke() -> &'static Path {
    static SMOKE: OnceLock<PathBuf> = OnceLock::new();
    SMOKE.get_or_init(|| {
        let out = scratch("smoke");
        let output = driver(&["smoke", "--out", path(&out)]);
        assert_eq!(output.status.code(), Some(0), "{}", said(&output));
        assert!(stdout(&output).contains("smoke: passed"), "{}", said(&output));
        out.join(id_of(SMOKE_SPEC).to_string())
    })
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn the_smoke_run_holds_the_first_second_pins() {
    let directory = smoke();
    let summary = summary(directory);
    // crates/ng-validate/tests/trace.rs:135-136 and :147, Core at a 10 ms cap.
    assert_eq!(
        value(&summary, "packet"),
        "6d709479f0b103cadc1972febf6437a2dde172e19242eb76908b9dabde5bb752"
    );
    let anchor = anchor_moment().nanoseconds();
    let steps = Table::<u64>::read(&directory.join("steps.npy")).expect("steps");
    let first = steps
        .iter()
        .find(|row| row[1] >= anchor + 1_000_000_000)
        .expect("a head past 1000 ms");
    assert_eq!(first[1] - anchor, 1_000_079_707);
    let seal = hex(&digest_of_words([first[3], first[4], first[5], first[6]]));
    assert_eq!(
        seal,
        "58233f790617a4f1bf5122b47fc44efaeface2ebb7d4c3652567bb44ba176ecf"
    );
    let beats: u64 = value(&summary, "beats").parse().expect("a count");
    assert!(beats >= 15, "{beats} beats");
    assert_eq!(
        Table::<f64>::read(&directory.join("beats.npy"))
            .expect("beats")
            .rows() as u64,
        beats
    );
    assert_eq!(value(&summary, "fault"), "none");
    let applied = table(&directory.join("applied.npy"));
    assert_eq!(applied.len(), 1, "one stand");
    assert_eq!(applied[0][0], 0, "the spec's first event");
    let stand_at = steps
        .iter()
        .find(|row| row[0] == applied[0][1] - 1)
        .map(|row| row[1]);
    assert_eq!(
        stand_at,
        Some(anchor + 10_000_000_000),
        "the step that applied it starts at 10 s"
    );
    let last = steps.row(steps.rows() - 1);
    assert!(last[1] >= anchor + 20_000_000_000);
    assert!(steps.row(steps.rows() - 2)[1] < anchor + 20_000_000_000);
    assert_eq!(
        value(&summary, "last_seal"),
        hex(&digest_of_words([last[3], last[4], last[5], last[6]]))
    );
    let entries = names(directory);
    let files = [
        "applied.npy",
        "beats.npy",
        "edges.npy",
        "lens.npy",
        "spec.txt",
        "steps.npy",
        "summary.txt",
    ];
    assert_eq!(entries, files, "no link file or lock stays");
    assert_eq!(
        fs::read_to_string(directory.join("spec.txt")).expect("the spec"),
        SMOKE_SPEC
    );
    for name in files.iter().filter(|name| **name != "summary.txt") {
        let digest = hex(&ng_contract::Digest::of_canonical(
            &fs::read(directory.join(name)).expect("a file"),
        ));
        assert_eq!(value(&summary, &format!("file {name}")), digest, "{name}");
    }
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn a_second_run_of_a_present_spec_writes_nothing() {
    let directory = smoke();
    let out = directory.parent().expect("the out directory");
    let spec = written(&scratch("rerun"), "smoke.txt", SMOKE_SPEC);
    let before = listing(out);
    let output = run(&spec, out, &[]);
    let id = id_of(SMOKE_SPEC).to_string();
    assert_eq!(
        result(&output),
        ("present".to_string(), id, Some(0)),
        "{}",
        said(&output)
    );
    assert_eq!(listing(out), before);
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn a_killed_run_leaves_a_partial_that_the_next_run_redoes() {
    let out = scratch("killed");
    let text = "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 20000\nevent at-ms 5000 trunk 1000 legs standing\nvariant \
                replica 11\n";
    let spec = written(&out, "spec.txt", text);
    let id = id_of(text).to_string();
    let mut child = spawn(&["run", path(&spec), "--out", path(&out)]);
    let partial = out.join(format!("{id}.partial-{}", child.id()));
    let steps = partial.join("steps.npy");
    wait_for(120, "no step rows reached the disk", || {
        fs::metadata(&steps).is_ok_and(|meta| meta.len() > 128)
    });
    child.kill().expect("SIGKILL");
    let status = child.wait().expect("reaped");
    assert_eq!(status.code(), None, "killed by a signal");
    assert_eq!(
        names(&out),
        [format!("{id}.partial-{}", child.id()), "spec.txt".to_string()]
    );
    assert!(!partial.join("summary.txt").exists());
    let output = run(&spec, &out, &[]);
    assert_eq!(
        result(&output),
        ("complete".to_string(), id.clone(), Some(0)),
        "{}",
        said(&output)
    );
    assert_eq!(names(&out), [id.clone(), "spec.txt".to_string()]);
    assert_eq!(value(&summary(&out.join(&id)), "fault"), "none");
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn a_run_past_its_memory_cap_is_a_fault_that_stays() {
    let out = scratch("capped");
    let text = "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 3000\nvariant replica 12\n";
    let spec = written(&out, "spec.txt", text);
    let id = id_of(text).to_string();
    let output = run(&spec, &out, &["--memory-cap-gb", "0"]);
    assert_eq!(
        result(&output),
        ("fault".to_string(), id.clone(), Some(3)),
        "{}",
        said(&output)
    );
    let fault = out.join(format!("{id}.fault"));
    let held = summary(&fault);
    assert_eq!(value(&held, "fault"), "MemoryCap");
    assert_eq!(value(&held, "fault_course"), "original");
    assert_eq!(value(&held, "memory_cap_gb"), "0");
    assert!(value(&held, "fault_detail").starts_with("resident_bytes "));
    assert!(fault.join("edges.npy").exists() && fault.join("spec.txt").exists());
    assert!(!fault.join("links").exists() && !fault.join("lock").exists());
    let identity = fault.join("summary.txt");
    let status = driver(&[
        "status",
        "--out",
        path(&out),
        "--identity",
        path(&identity),
        path(&spec),
    ]);
    assert_eq!(
        result(&status),
        ("fault".to_string(), id.clone(), Some(0)),
        "{}",
        said(&status)
    );
    let before = listing(&out);
    let again = run(&spec, &out, &[]);
    assert_eq!(
        result(&again),
        ("fault".to_string(), id, Some(3)),
        "{}",
        said(&again)
    );
    assert_eq!(listing(&out), before);
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn status_reports_every_state_in_one_process() {
    let out = scratch("status");
    let specs = scratch("status-specs");
    let short = |core: &str, variant: &str| {
        format!("gp-run 1\ncore ng-solver cap-ms {core}\nend-ms 2000\nvariant {variant}\n")
    };
    let present = written(&specs, "present.txt", &short("10", "none"));
    let stale = written(&specs, "stale.txt", &short("5", "none"));
    let fault = written(&specs, "fault.txt", &short("10", "replica 21"));
    let partial_text = "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 20000\nvariant replica 22\n";
    let partial = written(&specs, "partial.txt", partial_text);
    let missing = written(&specs, "missing.txt", &short("10", "replica 23"));
    let identity = completed(&present, &out).join("summary.txt");
    completed(&stale, &out);
    assert_eq!(
        run(&fault, &out, &["--memory-cap-gb", "0"]).status.code(),
        Some(3)
    );
    let mut child = spawn(&["run", path(&partial), "--out", path(&out)]);
    let partial_id = id_of(partial_text);
    let started = out
        .join(format!("{partial_id}.partial-{}", child.id()))
        .join("spec.txt");
    wait_for(60, "the partial directory never took its spec", || {
        started.exists()
    });
    let status = |spec: &Path| {
        let output = driver(&[
            "status",
            "--out",
            path(&out),
            "--identity",
            path(&identity),
            path(spec),
        ]);
        assert_eq!(output.status.code(), Some(0), "{}", said(&output));
        stdout(&output)
    };
    assert!(
        status(&partial).trim_end().ends_with(" held"),
        "a live run holds its partial"
    );
    child.kill().expect("SIGKILL");
    child.wait().expect("reaped");
    let every = [&present, &stale, &fault, &partial, &missing];
    let mut arguments = vec!["status", "--out", path(&out), "--identity", path(&identity)];
    arguments.extend(every.iter().map(|spec| path(spec)));
    let output = driver(&arguments);
    assert_eq!(output.status.code(), Some(0), "{}", said(&output));
    let lines: Vec<String> = stdout(&output).lines().map(str::to_string).collect();
    let expected: Vec<String> = (["present", "stale", "fault", "partial", "missing"]
        .iter()
        .zip(every))
    .map(|(word, spec)| {
        let text = fs::read_to_string(spec).expect("a spec");
        format!("{word} {} {}", id_of(&text), path(spec))
    })
    .collect();
    assert_eq!(lines, expected);
}

const VARIANT_BASE: &str = "gp-run 1
core ng-solver cap-ms 10
end-ms 12000
event at-ms 3000 trunk 1000 legs standing
event at-ms 8000 trunk 0 legs horizontal
variant none
";

fn variant_run(variant: &str) -> PathBuf {
    let out = scratch(&format!("variant-{}", variant.replace(' ', "-")));
    let text = VARIANT_BASE.replace("variant none", &format!("variant {variant}"));
    completed(&written(&out, "spec.txt", &text), &out)
}

fn reference() -> &'static Path {
    static REFERENCE: OnceLock<PathBuf> = OnceLock::new();
    REFERENCE.get_or_init(|| variant_run("none"))
}

/// The variant's own course is the reference course, step for step, and so is its applied input.
fn same_course(variant: &Path) {
    let reference = reference();
    let known = seal_log(&reference.join("steps.npy"));
    assert!(known.len() > 1000, "{} steps", known.len());
    assert_eq!(seal_log(&variant.join("steps.npy")), known);
    assert_eq!(
        table(&variant.join("applied.npy")),
        table(&reference.join("applied.npy"))
    );
    assert_eq!(
        table(&reference.join("applied.npy")).len(),
        2,
        "both events applied"
    );
    let (theirs, ours) = (summary(reference), summary(variant));
    for key in [
        "first_seal",
        "last_seal",
        "revisions",
        "beats",
        "packet",
        "run_identity",
        "end_ledger",
    ] {
        assert_eq!(value(&ours, key), value(&theirs, key), "{key}");
    }
}

/// The copy or branch continues the reference course from its checkpoint to the same end.
fn same_branch(variant: &Path) {
    let reference = reference();
    let (theirs, ours) = (summary(reference), summary(variant));
    let from: u64 = value(&ours, "branch_from_revision").parse().expect("a revision");
    let known: Vec<[u64; 6]> = (seal_log(&reference.join("steps.npy")).into_iter())
        .filter(|row| row[0] > from)
        .collect();
    let checkpoint_at = anchor_moment().nanoseconds() + 5_000_000_000;
    let at_checkpoint = seal_log(&reference.join("steps.npy"))
        .into_iter()
        .find(|row| row[0] == from);
    assert!(
        at_checkpoint.is_some_and(|row| row[1] >= checkpoint_at),
        "checkpoint at or after 5 s"
    );
    assert!(!known.is_empty());
    assert_eq!(seal_log(&variant.join("branch-steps.npy")), known);
    assert_eq!(value(&ours, "branch_last_seal"), value(&theirs, "last_seal"));
    for field in [
        "revision",
        "time_ns",
        "state",
        "continuation",
        "segment",
        "modes",
        "streams",
        "applied",
        "waiting",
        "ledger",
    ] {
        let key = format!("end_{field}");
        assert_eq!(
            value(&ours, &format!("branch_{key}")),
            value(&theirs, &key),
            "{key}"
        );
    }
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn a_replica_is_the_same_course() {
    same_course(&variant_run("replica 1"));
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn late_inputs_are_the_same_course() {
    same_course(&variant_run("late lead-ms 500"));
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn an_observer_off_run_is_the_same_course() {
    let variant = variant_run("observer-off");
    same_course(&variant);
    assert_eq!(table(&variant.join("lens.npy")).len(), 0);
    assert!(!table(&reference().join("lens.npy")).is_empty());
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn a_checkpoint_resumed_into_a_fresh_body_and_core_is_the_same_course() {
    let variant = variant_run("checkpoint-resume at-ms 5000");
    same_course(&variant);
    same_branch(&variant);
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn a_rewound_branch_is_the_same_course() {
    let variant = variant_run("rewind at-ms 5000");
    same_course(&variant);
    same_branch(&variant);
}

fn number_after(line: &str, key: &str) -> u64 {
    let start = line.find(key).unwrap_or_else(|| panic!("no {key} in {line}")) + key.len();
    let digits: String = line[start..].chars().take_while(char::is_ascii_digit).collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("no number after {key} in {line}"))
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn a_closed_loop_run_is_the_course_of_its_realized_spec() {
    let out = scratch("interact");
    let text = "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 12000\nclosed-loop acceptance stand after beat 2, lie \
                after beat 6\nvariant none\n";
    let spec = written(&out, "spec.txt", text);
    let mut child = (Command::new(DRIVER).args(["interact", path(&spec), "--out", path(&out)]))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the driver starts");
    let mut answers = child.stdin.take().expect("its stdin");
    let beats = BufReader::new(child.stdout.take().expect("its stdout"));
    let mut asked = 0;
    let mut acted = Vec::new();
    for line in beats.lines() {
        let line = line.expect("a beat line");
        let (beat, earliest) = (
            number_after(&line, "\"beat\":"),
            number_after(&line, "\"earliest_ms\":"),
        );
        assert_eq!(beat, asked, "beats come in order");
        asked += 1;
        let answer = match beat {
            2 => format!("event at-ms {} trunk 1000 legs standing", earliest + 250),
            6 => format!("event at-ms {} trunk 0 legs horizontal", earliest + 250),
            _ => "none".to_string(),
        };
        if answer != "none" {
            acted.push(answer.clone());
        }
        writeln!(answers, "{answer}").expect("an answer");
    }
    drop(answers);
    let output = child.wait_with_output().expect("it ends");
    let told = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(0), "{told}");
    let id = id_of(text).to_string();
    assert!(told.starts_with(&format!("complete {id} ")), "{told}");
    let directory = out.join(&id);
    let held = summary(&directory);
    let beat_rows = Table::<f64>::read(&directory.join("beats.npy")).expect("beats");
    assert_eq!(value(&held, "beats"), beat_rows.rows().to_string());
    let before_end = beat_rows.iter().filter(|row| row[2] < 12.0).count() as u64;
    assert_eq!(
        asked, before_end,
        "one line per complete beat that ends before the end"
    );
    assert!(asked > 6, "{asked} beats");
    let realized = fs::read_to_string(directory.join("realized.txt")).expect("realized.txt");
    let open = RunSpec::parse(&realized).expect("a canonical spec");
    assert_eq!(open.closed_loop(), None);
    let lines: Vec<String> = open.events().iter().map(|event| event.line()).collect();
    assert_eq!(lines, acted);
    assert_eq!(
        table(&directory.join("applied.npy")).len(),
        2,
        "both actions applied"
    );
    let replay = scratch("interact-replay");
    let replayed = completed(&written(&replay, "realized.txt", &realized), &replay);
    assert_eq!(
        seal_log(&replayed.join("steps.npy")),
        seal_log(&directory.join("steps.npy"))
    );
    assert_eq!(
        table(&replayed.join("applied.npy")),
        table(&directory.join("applied.npy"))
    );
    assert_eq!(
        value(&summary(&replayed), "end_waiting"),
        value(&held, "end_waiting")
    );
}

#[test]
#[ignore = "a teacher run of the arena body"]
fn a_closed_loop_answer_before_the_earliest_stamp_is_a_protocol_fault() {
    let out = scratch("protocol");
    let text = "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 6000\nclosed-loop acceptance answers too early\nvariant \
                none\n";
    let spec = written(&out, "spec.txt", text);
    let mut child = (Command::new(DRIVER).args(["interact", path(&spec), "--out", path(&out)]))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the driver starts");
    let mut answers = child.stdin.take().expect("its stdin");
    let mut beats = BufReader::new(child.stdout.take().expect("its stdout"));
    let mut line = String::new();
    beats.read_line(&mut line).expect("the first beat");
    let earliest = number_after(&line, "\"earliest_ms\":");
    writeln!(answers, "event at-ms {} trunk 1000 legs standing", earliest - 1).expect("an answer");
    drop(answers);
    let output = child.wait_with_output().expect("it ends");
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let fault = out.join(format!("{}.fault", id_of(text)));
    let held = summary(&fault);
    assert_eq!(value(&held, "fault"), "Protocol");
    assert!(value(&held, "fault_detail").contains("before the earliest stamp"));
    assert!(fault.join("realized.txt").exists());
}
