use gp_driver::{
    BeatReading, CAPTURE_VERSION, Catalogue, MemoryCap, NpyWriter, Options, PostureEvent, RunSpec,
    SMOKE_HEAD_NS, SMOKE_PACKET, SMOKE_SEAL, SpecError, Stamp, Table, TeachError, Teacher, digest_of_hex,
    hex,
};
use ng_contract::{Digest, InputId, Legs, RowKind, SimTime};
use ng_validate::{anchor_moment, arena_body, arena_lens};
use std::fs;
use std::path::PathBuf;

const TICKET_SPEC: &str = "gp-run 1
core ng-solver cap-ms 10
end-ms 900000
event at-ms 1000 trunk 1000 legs standing
event at-ms 180000 trunk 0 legs horizontal
event at-ms 330000 trunk 1000 legs standing
variant none
";

fn with_variant(variant: &str) -> String {
    TICKET_SPEC.replace("variant none", &format!("variant {variant}"))
}

fn scratch(name: &str) -> PathBuf {
    let directory = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("light")
        .join(name);
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).expect("a scratch directory");
    directory
}

#[test]
fn a_canonical_spec_renders_back_to_its_own_text() {
    let texts = [
        TICKET_SPEC.to_string(),
        with_variant("late lead-ms 500"),
        with_variant("checkpoint-resume at-ms 200000"),
        with_variant("rewind at-ms 200000"),
        with_variant("observer-off"),
        with_variant("replica 3"),
        TICKET_SPEC.replace(
            "core ng-solver cap-ms 10",
            "core reference step-us 1000 floor-us 100 absolute 1 relative 0.001",
        ),
        TICKET_SPEC.replace("variant none", "closed-loop a rule with words 3\nvariant none"),
        "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 1000\nvariant none\n".to_string(),
    ];
    for text in texts {
        let spec = RunSpec::parse(&text).unwrap_or_else(|error| panic!("{error}: {text}"));
        assert_eq!(spec.canonical(), text);
        assert_eq!(RunSpec::parse(&spec.canonical()), Ok(spec));
    }
}

#[test]
fn a_spec_reads_back_what_it_says() {
    let spec = RunSpec::parse(TICKET_SPEC).expect("canonical");
    let stamps: Vec<u32> = spec.events().iter().map(|event| event.at.ms()).collect();
    assert_eq!(stamps, [1000, 180000, 330000]);
    assert_eq!(spec.end().ms(), 900000);
    assert_eq!(spec.events()[1].setting.legs, Legs::Horizontal);
    assert_eq!(spec.events()[2].setting.trunk.get(), 1000);
    assert_eq!(spec.closed_loop(), None);
}

#[test]
fn the_spec_id_is_the_digest_of_the_capture_version_and_the_text() {
    let spec = RunSpec::parse(TICKET_SPEC).expect("canonical");
    let named = format!("gp-run-id\n{CAPTURE_VERSION}\n{TICKET_SPEC}");
    assert_eq!(spec.id().digest(), Digest::of_canonical(named.as_bytes()));
    let rendered = spec.id().to_string();
    assert_eq!(rendered.len(), 64);
    assert!(
        rendered
            .bytes()
            .all(|digit| digit.is_ascii_digit() || (b'a'..=b'f').contains(&digit))
    );
    assert_eq!(digest_of_hex(&rendered), Some(spec.id().digest()));
    assert_eq!(hex(&spec.id().digest()), rendered);
}

#[test]
fn specs_that_differ_have_different_ids_and_shared_events_share_ids() {
    let none = RunSpec::parse(TICKET_SPEC).expect("canonical");
    let replica = RunSpec::parse(&with_variant("replica 1")).expect("canonical");
    let other = RunSpec::parse(&with_variant("replica 2")).expect("canonical");
    assert_ne!(none.id(), replica.id());
    assert_ne!(replica.id(), other.id());
    assert_eq!(none.events(), replica.events());
    assert_eq!(none.events()[0].id(), replica.events()[0].id());
    let stand = InputId::new(Digest::of_canonical(
        b"gp-posture\nat-ms 1000 trunk 1000 legs standing",
    ));
    assert_eq!(none.events()[0].id(), stand);
    assert_ne!(none.events()[0].id(), none.events()[2].id());
    let shorter = RunSpec::parse(&TICKET_SPEC.replace("end-ms 900000", "end-ms 400000")).expect("canonical");
    assert_ne!(none.id(), shorter.id());
    assert_eq!(none.events()[2].id(), shorter.events()[2].id());
}

#[test]
fn a_spec_refuses_a_stamp_before_the_fence() {
    let text = TICKET_SPEC.replace("at-ms 1000 trunk", "at-ms 999 trunk");
    assert_eq!(RunSpec::parse(&text), Err(SpecError::BeforeFence { line: 4 }));
    let end = TICKET_SPEC.replace("end-ms 900000", "end-ms 999");
    assert_eq!(RunSpec::parse(&end), Err(SpecError::BeforeFence { line: 3 }));
    let checkpoint = with_variant("checkpoint-resume at-ms 500");
    assert_eq!(
        RunSpec::parse(&checkpoint),
        Err(SpecError::BeforeFence { line: 7 })
    );
}

#[test]
fn a_spec_refuses_equal_and_unsorted_stamps() {
    let equal = TICKET_SPEC.replace("at-ms 330000", "at-ms 180000");
    assert_eq!(RunSpec::parse(&equal), Err(SpecError::EqualStamps { line: 6 }));
    let unsorted = TICKET_SPEC.replace("at-ms 330000", "at-ms 170000");
    assert_eq!(RunSpec::parse(&unsorted), Err(SpecError::Unsorted { line: 6 }));
}

#[test]
fn a_spec_refuses_an_end_before_its_last_event() {
    let text = TICKET_SPEC.replace("end-ms 900000", "end-ms 300000");
    assert_eq!(RunSpec::parse(&text), Err(SpecError::EndBeforeEvent { line: 6 }));
    let rewind = TICKET_SPEC.replace("variant none", "variant rewind at-ms 900001");
    assert_eq!(
        RunSpec::parse(&rewind),
        Err(SpecError::InstantAfterEnd { line: 7 })
    );
}

#[test]
fn a_spec_refuses_text_that_is_not_canonical() {
    let cases = [
        (
            TICKET_SPEC.replace("cap-ms 10", "cap-ms  10"),
            SpecError::NotCanonical { line: 2 },
        ),
        (
            TICKET_SPEC.replace("end-ms 900000", "end-ms 0900000"),
            SpecError::NotCanonical { line: 3 },
        ),
        (
            TICKET_SPEC.replace('\n', "\r\n"),
            SpecError::NotCanonical { line: 1 },
        ),
        (
            TICKET_SPEC.replace(
                "ng-solver cap-ms 10",
                "reference step-us 1000 floor-us 100 absolute 1.0 relative 1e-3",
            ),
            SpecError::NotCanonical { line: 2 },
        ),
        (TICKET_SPEC.trim_end().to_string(), SpecError::NoFinalNewline),
        (
            TICKET_SPEC.replace("cap-ms 10", "cap-ms 0"),
            SpecError::Malformed { line: 2 },
        ),
        (
            TICKET_SPEC.replace("trunk 0", "trunk 1001"),
            SpecError::Malformed { line: 5 },
        ),
        (
            TICKET_SPEC.replace("legs horizontal", "legs kneeling"),
            SpecError::Malformed { line: 5 },
        ),
        (with_variant("late lead-ms 0"), SpecError::Malformed { line: 7 }),
        (
            TICKET_SPEC.replace("gp-run 1", "gp-run 2"),
            SpecError::Malformed { line: 1 },
        ),
        (
            TICKET_SPEC.replace("variant none\n", ""),
            SpecError::Missing { keyword: "variant" },
        ),
        (
            TICKET_SPEC.replace("end-ms 900000\n", "") + "end-ms 900000\n",
            SpecError::OutOfOrder { line: 7 },
        ),
        (
            TICKET_SPEC.replace("variant none", "seed 4"),
            SpecError::UnknownLine { line: 7 },
        ),
        (
            TICKET_SPEC.replace("variant none", "closed-loop rule\nvariant late lead-ms 50"),
            SpecError::ClosedLoopVariant { line: 8 },
        ),
    ];
    for (text, refusal) in cases {
        assert_eq!(RunSpec::parse(&text), Err(refusal), "{text:?}");
    }
}

#[test]
fn an_answer_line_is_an_event_line() {
    let event = PostureEvent::parse_line("event at-ms 1200 trunk -250 legs seated").expect("canonical");
    assert_eq!(event.at.ms(), 1200);
    assert_eq!(event.setting.trunk.get(), -250);
    assert_eq!(event.setting.legs, Legs::SeatedDependent);
    assert_eq!(event.line(), "event at-ms 1200 trunk -250 legs seated");
    assert!(PostureEvent::parse_line("event at-ms 1200  trunk 0 legs seated").is_err());
    assert!(PostureEvent::parse_line("event at-ms 900 trunk 0 legs seated").is_err());
}

#[test]
fn a_closed_loop_realizes_an_open_loop_spec_in_stamp_order() {
    let closed = TICKET_SPEC.replace("variant none", "closed-loop stand when low\nvariant none");
    let spec = RunSpec::parse(&closed).expect("canonical");
    let actions = [
        PostureEvent::parse_line("event at-ms 400000 trunk 0 legs horizontal").expect("canonical"),
        PostureEvent::parse_line("event at-ms 2000 trunk 500 legs seated").expect("canonical"),
    ];
    let realized = spec.realized(&actions).expect("a valid spec");
    let expected = "gp-run 1
core ng-solver cap-ms 10
end-ms 900000
event at-ms 1000 trunk 1000 legs standing
event at-ms 2000 trunk 500 legs seated
event at-ms 180000 trunk 0 legs horizontal
event at-ms 330000 trunk 1000 legs standing
event at-ms 400000 trunk 0 legs horizontal
variant none
";
    assert_eq!(realized.canonical(), expected);
}

#[test]
fn the_earliest_answer_stamp_rounds_up_to_a_whole_ms() {
    let anchor = anchor_moment().nanoseconds();
    let at = |after_ns: u64| Stamp::at_or_after(SimTime::from_ns(anchor + after_ns)).map(Stamp::ms);
    assert_eq!(at(1_000_079_707), Some(1001));
    assert_eq!(at(1_000_000_000), Some(1000));
    assert_eq!(at(999_999_999), Some(1000));
    assert_eq!(at(998_000_000), None);
}

#[test]
fn a_beat_is_one_json_line_for_the_policy() {
    let anchor = anchor_moment().nanoseconds();
    let beat = BeatReading {
        ordinal: 4,
        start: SimTime::from_ns(anchor + 1_500_000_000),
        end: SimTime::from_ns(anchor + 2_500_000_000),
        heart_rate_per_s: 1.0,
        arterial_pressure: 93.25,
        stroke: 70.5,
        filtration: f64::NAN,
    };
    let earliest = Stamp::after_anchor_ms(2501).expect("past the fence");
    assert_eq!(
        beat.json(earliest),
        "{\"beat\":4,\"start_s\":1.5,\"end_s\":2.5,\"heart_rate_per_s\":1,\"arterial_pressure\":93.25,\
         \"stroke\":70.5,\"filtration\":null,\"earliest_ms\":2501}"
    );
    assert_eq!(beat.row()[..3], [4.0, 1.5, 2.5]);
}

#[test]
fn an_npy_file_reads_back_what_was_written() {
    let directory = scratch("npy");
    let path = directory.join("table.npy");
    let rows = [[7u64, u64::MAX], [0, 1], [42, 1 << 63]];
    let mut writer = NpyWriter::<u64>::create(&path, 2).expect("created");
    for row in &rows {
        writer.push(row).expect("written");
    }
    assert!(writer.push(&[1]).is_err(), "a row of another width is refused");
    writer.finish().expect("finished");
    let bytes = fs::read(&path).expect("read");
    assert_eq!(&bytes[..10], b"\x93NUMPY\x01\x00\x76\x00");
    let header = core::str::from_utf8(&bytes[10..128]).expect("text");
    assert!(header.starts_with("{'descr': '<u8', 'fortran_order': False, 'shape': (3, 2), }"));
    assert!(header.ends_with('\n'));
    assert_eq!(bytes.len(), 128 + 3 * 2 * 8);
    let table = Table::<u64>::read(&path).expect("read back");
    assert_eq!((table.rows(), table.columns()), (3, 2));
    assert_eq!(
        table.iter().collect::<Vec<_>>(),
        rows.iter().map(|row| &row[..]).collect::<Vec<_>>()
    );
    assert!(
        Table::<f64>::read(&path).is_err(),
        "another element type is refused"
    );

    let floats = directory.join("floats.npy");
    let values = [-0.0, f64::NAN, 1e-300, f64::INFINITY];
    let mut writer = NpyWriter::<f64>::create(&floats, 4).expect("created");
    writer.push(&values).expect("written");
    writer.finish().expect("finished");
    let table = Table::<f64>::read(&floats).expect("read back");
    let bits = |row: &[f64]| row.iter().map(|value| value.to_bits()).collect::<Vec<_>>();
    assert_eq!(bits(table.row(0)), bits(&values));
    assert!(
        NpyWriter::<f64>::create(&floats, 4).is_err(),
        "a finished file is never overwritten"
    );
}

#[test]
fn the_catalogue_covers_every_column_of_the_compiled_arena_body() {
    let body = arena_body().expect("compiled");
    let catalogue = Catalogue::of(&body).expect("a catalogue");
    assert_eq!(body.columns().len(), 257);
    assert_eq!(catalogue.columns().len(), 257);
    assert_eq!(catalogue.column_ids().len(), 257);
    assert!(
        catalogue
            .column_ids()
            .iter()
            .enumerate()
            .all(|(at, id)| id.index() == at)
    );
    assert_eq!(catalogue.rows().len(), 241);
    let row = |id: ng_contract::RowId| &catalogue.rows()[id.index()];
    assert_eq!(row(catalogue.phase()), "memory/CV/phase/body");
    assert_eq!(row(catalogue.heart_rate()), "algebraic/CV/rate/body");
    assert_eq!(
        row(catalogue.pressure()),
        "algebraic/CV/blood pressure/systemic arteries"
    );
    assert_eq!(
        row(catalogue.filtration()),
        "algebraic/RN/filtration/glomerular bed>renal tubule"
    );
    assert!(matches!(
        body.rows()[catalogue.phase().index()].kind(),
        RowKind::Memory(_)
    ));
    let aortic = &catalogue.columns()[catalogue.aortic_water().index()];
    assert_eq!(aortic, "CV/valves/left ventricle>systemic arteries/Water/inside");
    assert_eq!(
        catalogue.quantities().len(),
        body.conserved_basis().combinations().len()
    );
}

#[test]
fn the_catalogue_text_names_every_table_entry_in_table_order() {
    let body = arena_body().expect("compiled");
    let catalogue = Catalogue::of(&body).expect("a catalogue");
    let lens = arena_lens(&body).expect("the arena lens");
    let text = catalogue.text(&body, &lens, &[]);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0], "gp-catalogue 1");
    assert_eq!(lines[1], format!("model {}", hex(&body.model_hash().digest())));
    let named = |kind: &str| -> Vec<String> {
        (lines.iter())
            .filter_map(|line| line.strip_prefix(&format!("{kind} ")))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(named("row"), catalogue.rows());
    assert_eq!(named("column"), catalogue.columns());
    let quantities = named("quantity");
    assert_eq!(quantities.len(), body.conserved_basis().combinations().len());
    assert!(quantities.iter().any(|line| line.starts_with("Water ")));
    assert_eq!(
        quantities.last().map(|line| line.split(' ').next()),
        Some(Some("Charge"))
    );
    let first = &body.conserved_basis().combinations()[0];
    let (row, weight) = first.terms()[0];
    assert!(quantities[0].contains(&format!(" {}:{weight}", row.index())));
    assert!(named("mode").is_empty(), "no start, so no mode names");
    assert_eq!(
        named("lens"),
        [
            "algebraic/CV/rate/body 1000000000",
            "algebraic/CV/blood pressure/systemic arteries 1000000000",
            "algebraic/RN/filtration/glomerular bed>renal tubule 1000000000",
        ]
    );
    assert_eq!(named("sampled"), ["algebraic/NS/sympathetic traffic/body"]);
    assert!(text.ends_with('\n') && !text.ends_with("\n\n"));
}

#[test]
fn a_late_lead_shorter_than_one_step_is_refused_before_any_directory() {
    let text = "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 3100\nevent at-ms 3000 trunk 1000 legs standing\nvariant \
                late lead-ms 1\n";
    let spec = RunSpec::parse(text).expect("canonical");
    let teacher = Teacher::new(spec.core()).expect("a teacher");
    let options = Options {
        cap: MemoryCap::gb(8),
        source_commit: "unrecorded".to_string(),
    };
    let out = scratch("short-lead");
    let refused = teacher.run(&spec, &options, &out);
    let longest = arena_body().expect("compiled").longest_step_ns();
    assert!(
        matches!(&refused, Err(TeachError::ShortLead { lead_ns: 1_000_000, longest_step_ns })
            if *longest_step_ns == longest),
        "{refused:?}"
    );
    let written = fs::read_dir(&out).expect("the out directory").count();
    assert_eq!(written, 0, "nothing written for {}", spec.id());
}

#[test]
fn the_smoke_pins_are_what_trace_rs_pins_for_the_first_second_on_ng_solver() {
    let trace = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../crates/ng-validate/tests/trace.rs"
    ))
    .expect("crates/ng-validate/tests/trace.rs");
    let pinned = (trace.split_once("fn the_first_second_of_the_slice_on_ng_solver_stays_where_it_was()"))
        .and_then(|(_, rest)| rest.split_once("\n}\n"))
        .expect("trace.rs pins the first second on ng-solver")
        .0;
    let digests: Vec<&str> = (pinned.split("expect![\"").skip(1))
        .filter_map(|rest| rest.split_once('"').map(|(digest, _)| digest))
        .collect();
    assert_eq!(digests, [SMOKE_PACKET, SMOKE_SEAL], "the packet and the seal");
    let (ms, ns) = (pinned.lines())
        .filter_map(|line| line.trim_start().split_once(" ms ")?.0.split_once('.'))
        .next_back()
        .expect("a traced head");
    assert_eq!(ns.len(), 6, "a head to the nanosecond");
    let head_ns = ms.parse::<u64>().expect("whole ms") * 1_000_000 + ns.parse::<u64>().expect("ns");
    assert_eq!(head_ns, SMOKE_HEAD_NS, "the first head at or after 1 s");
}
