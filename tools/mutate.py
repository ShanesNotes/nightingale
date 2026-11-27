"""Breaks one rule at a time on a commit and reports whether a test notices.

    python3 -I tools/mutate.py TABLE [--rev REV] [--only NAME ...] [--since REV] [--budget SECONDS] [--check]
                                     [--tested RECORD] [--jobs N|auto] [--build-jobs N] [--stop-on-fail]

Python 3.11 stdlib only. TABLE is either a TOML file or a directory of `*.toml` files read in sorted filename order,
with one [[mutant]] per rule, aimed at the test that holds it. `old` must match exactly once. `test` is what follows
`cargo test`, and a row's own `test` replaces its file's top-level `test`. It may not pass --manifest-path,
--target-dir or --config. A row may also state an optional positive integer `budget` in seconds as its own minimum
limit (and four times that for its baseline).

    test = "--release -p ng-contract --test map_reads"

    [[mutant]]
    name = "no-supply"
    path = "crates/ng-contract/src/compile.rs"
    old = '''if !anatomy.unsupplied(site) =>'''
    new = '''if false && !anatomy.unsupplied(site) =>'''

A sweep extracts REV, default HEAD, to target/mutate/ in this checkout, so it never edits your files. Each `test` must
pass there before any row runs. Each row prints in table order as it ends, so a killed sweep keeps its lines:

    CAUGHT           a test failed, named on the line, or a test binary crashed
    CAUGHT at build  rustc refused the mutant with a coded error; read it, because a mistyped mutant proves nothing
    SURVIVED         every test passed, so ask what the test could not see
    NOT APPLIED      `old` did not match exactly once, so nothing ran
    UNCLEAR          over its limit, killed from outside, or failed with no test or coded error named: neither a
                     catch nor a survivor

A row's limit is twice its test's baseline time in this sweep, so a loaded worker slows both alike, and never under
--budget (420 s by default) or the row's own `budget`. A row's rebuild rides on its test's time: the slowest measured
took 1.4 times its baseline. A baseline, and the build that stands in for a skipped one, gets four times its budget:
the unmutated test is known to finish, so that limit only stops a hung build. Each verdict line names the row's limit.

When --jobs is `auto` (the default), at least 4 rows run, and the filesystem supports reflinks, the sweep builds once
in lane 0, clones the extract and target into K = min(4, max(1, cpus // 3)) isolated lanes under target/mutate/lanes/
with `cp -a --reflink=auto` and bounded CARGO_BUILD_JOBS per lane, runs unskipped baselines and rows across lanes,
flushes verdicts in exact table order, and removes target/mutate/lanes/ on exit.

Exit 0 only when every row is CAUGHT. --check lists the NOT APPLIED rows and stops before any build. --since keeps the
rows whose path, crate Cargo.toml, or test target's source or test file changed after REV (--test <bin> matches its own
file, imported local helpers and shared helper subdirs under tests/, --lib matches its module or lib.rs and fails closed
to src/, --doc matches src/). Each command runs in its own process group, and that group dies with the command and with
this script, however this script dies.

tools/verify's 40-mutants step runs this script through `gate.py mutants`. Before 10-rust it replays the rows a change
adds or edits, or whose path it changes, and after 10-rust the rest. It passes --stop-on-fail and stops each sweep at
the first row in table order that is not CAUGHT, so a red run reports only its first failure. A hand run reports every
row.

--tested names the record 10-rust writes of the test calls it passed, with the libtest targets cargo ran for each
package. The rest part of a merge run passes it, and the sweep then skips each baseline whose package and target appear
in that record on this commit with these build settings, and builds its test instead, so a tree that does not build
still stops the sweep. A target omitted from a default call (test = false, required-features, autotests = false) or
without libtest (harness = false) is left out of the record and keeps its baseline. A call covers only when it ran
every test, or every test with the ignored ones. A `--release`, `--doc` or other flagged test, a crate 10-rust did not
test, a record lacking recorded targets, and a record from another commit or other settings run the baseline. A full
run and a hand run run every baseline. The record counts only while the checkout holds no changed or untracked file and
no tracked file marked assume-unchanged or skip-worktree.
A row caught at build runs its skipped baseline, so a filter that names no test still fails, and a row over its limit
runs it and then runs again with its limit. The sweep refuses a Cargo config in target/.cargo, target/mutate/.cargo,
target/mutate/lanes/.cargo or any lane under target/mutate/lanes/, which its cargo reads and 10-rust's does not, and an
extract that lacks a tracked path.
"""
import argparse
import collections
import fcntl
import json
import os
import pathlib
import re
import shlex
import shutil
import signal
import subprocess
import sys
import threading
import time
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
WORK = ROOT / "target" / "mutate"
TREE = WORK / "tree"
LANES = WORK / "lanes"
PASSED = re.compile(r"^test .+ \.\.\. ok$", re.M)
FAILED = re.compile(r"^test (.+) \.\.\. FAILED$", re.M)
CODED_ERROR = re.compile(r"^.*\berror\[E\d{4}\].*$", re.M)
DIED = re.compile(r"process didn't exit successfully: .*\((signal: (\d+), .*)\)$", re.M)
FAULTS = {signal.SIGABRT, signal.SIGILL, signal.SIGSEGV, signal.SIGBUS, signal.SIGFPE}
SHOWN = 3
ROW_TIMES = 2
BASELINE_TIMES = 4
MIN_PARALLEL_ROWS = 4
MAX_AUTO_LANES = 4
ROW_KEYS = {"name", "path", "old", "new", "test"}
OPTIONAL_KEYS = {"budget"}
MOVES = {"--manifest-path", "--target-dir", "--config"}
SET_HERE = {"CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR", "CARGO_TERM_COLOR", "CARGO_NET_OFFLINE"}
CARGO_CONFIGS = ("target/.cargo", "target/mutate/.cargo", "target/mutate/lanes/.cargo")


def refuse(why):
    sys.exit(f"mutate: {why}; no sweep")


def table_files(path):
    p = pathlib.Path(path)
    if p.is_dir():
        entries = sorted(p.iterdir(), key=lambda child: child.name)
        bad = [child.name for child in entries if child.is_symlink() or not child.is_file() or child.suffix != ".toml"]
        if bad:
            refuse(f"{path} holds non-TOML entries: {bad[:3]}")
        if not entries:
            refuse(f"{path} has no .toml file")
        return entries
    return [p]


def load_rows(path, only):
    rows = []
    for file_path in table_files(path):
        with open(file_path, "rb") as file:
            table = tomllib.load(file)
        part = table.get("mutant", [])
        if not part:
            refuse(f"{file_path} has no [[mutant]] row")
        for row in part:
            row.setdefault("test", table.get("test"))
            keys = set(row)
            if not ROW_KEYS <= keys or not keys <= (ROW_KEYS | OPTIONAL_KEYS) or not row["test"]:
                refuse(f"row {row.get('name')} needs {sorted(ROW_KEYS)} and optional {sorted(OPTIONAL_KEYS)}")
            if "budget" in row:
                budget = row["budget"]
                if isinstance(budget, bool) or not isinstance(budget, int) or budget < 1:
                    refuse(f"row {row['name']} budget must be a positive integer, got {budget!r}")
            if moved := MOVES & {arg.split("=")[0] for arg in shlex.split(row["test"])}:
                refuse(f"row {row['name']} passes {sorted(moved)}, which can point cargo outside the sweep's tree")
        rows.extend(part)
    names = [row["name"] for row in rows]
    if len(set(names)) != len(names):
        refuse("two rows share a name")
    if unknown := set(only) - set(names):
        refuse(f"--only names no row: {sorted(unknown)}")
    return [row for row in rows if not only or row["name"] in only]


_NO_VAL_FLAGS = frozenset(
    {
        "--release",
        "--no-fail-fast",
        "--no-default-features",
        "--all-features",
        "--offline",
        "--locked",
        "--frozen",
        "-q",
        "--quiet",
        "-v",
        "--verbose",
    }
)
_VAL_FLAGS = frozenset({"--features", "-F", "--profile", "--target", "-j", "--jobs"})


def parse_test_command(test):
    tokens = shlex.split(test)
    if "--" in tokens:
        tokens = tokens[: tokens.index("--")]
    crates, targets, filters, broad = [], set(), [], False
    it = iter(tokens)
    for arg in it:
        if arg in ("-p", "--package"):
            crates.append(next(it, ""))
        elif arg.startswith("--package="):
            crates.append(arg.removeprefix("--package="))
        elif arg.startswith("-p") and not arg.startswith("--"):
            crates.append(arg[2:])
        elif arg == "--lib":
            targets.add(("lib", None))
        elif arg == "--doc":
            targets.add(("doc", None))
        elif arg == "--test":
            targets.add(("test", next(it, "")))
        elif arg.startswith("--test="):
            targets.add(("test", arg.removeprefix("--test=")))
        elif arg in _NO_VAL_FLAGS:
            continue
        elif arg in _VAL_FLAGS:
            next(it, None)
        elif any(arg.startswith(f"{flag}=") for flag in _VAL_FLAGS if flag.startswith("--")):
            continue
        elif arg.startswith("-"):
            broad = True
        else:
            filters.append(arg)
    return crates, targets, filters, broad


_MOD_DECL = re.compile(r"\bmod\s+(?:r#)?([A-Za-z_][A-Za-z0-9_]*)\s*;")
_PATH_ATTR = re.compile(r"#\[\s*path\s*=")
_PATH_STR = re.compile(r'#\[\s*path\s*=\s*"([^"]+)"\s*\]')
_INCLUDE_CALL = re.compile(r"\binclude(?:_str|_bytes)?!\s*\(")
_INCLUDE_STR = re.compile(r'\binclude(?:_str|_bytes)?!\s*\(\s*"([^"]+)"\s*\)')


def _lib_module(crate, filters, root):
    if len(filters) != 1 or "::" not in filters[0]:
        return None
    mod0 = filters[0].split("::", 1)[0]
    if not mod0 or mod0 in ("test", "tests"):
        return None
    src = pathlib.Path(root) / "crates" / crate / "src"
    if (src / f"{mod0}.rs").is_file() or (src / mod0).is_dir():
        return mod0
    return None


def _test_bin_paths(crate, bin_name, root):
    root_path = pathlib.Path(root).resolve()
    tests_dir = root_path / "crates" / crate / "tests"
    entry = tests_dir / f"{bin_name}.rs"
    if not entry.is_file():
        entry = tests_dir / bin_name / "main.rs"
        if not entry.is_file():
            return None
    paths = set()
    seen = set()
    queue = collections.deque([entry])
    while queue:
        current = queue.popleft()
        resolved = current.resolve()
        try:
            rel = resolved.relative_to(root_path).as_posix()
        except ValueError:
            return None
        if resolved in seen:
            continue
        seen.add(resolved)
        paths.add(rel)
        if not resolved.is_file():
            continue
        text = resolved.read_text(encoding="utf-8", errors="replace")
        path_attrs = _PATH_ATTR.findall(text)
        path_strs = _PATH_STR.findall(text)
        if len(path_attrs) != len(path_strs):
            return None
        include_calls = _INCLUDE_CALL.findall(text)
        include_strs = _INCLUDE_STR.findall(text)
        if len(include_calls) != len(include_strs):
            return None
        for rel_target in (*path_strs, *include_strs):
            bases = [resolved.parent]
            if resolved.stem not in ("mod", "main"):
                bases.append(resolved.parent / resolved.stem)
            matched_any = False
            for base in bases:
                cand = (base / rel_target).resolve()
                try:
                    cand_rel = cand.relative_to(root_path).as_posix()
                except ValueError:
                    return None
                if base == resolved.parent or cand.is_file():
                    paths.add(cand_rel)
                    matched_any = True
                    if cand.suffix == ".rs" and cand.is_file() and cand not in seen:
                        queue.append(cand)
            if not matched_any:
                return None
        for ident in _MOD_DECL.findall(text):
            sibling = (resolved.parent / f"{ident}.rs").resolve()
            try:
                paths.add(sibling.relative_to(root_path).as_posix())
            except ValueError:
                return None
            if sibling.is_file() and sibling not in seen:
                queue.append(sibling)
            nested_mod = (resolved.parent / ident / "mod.rs").resolve()
            if nested_mod.is_file() and nested_mod not in seen:
                queue.append(nested_mod)
    return paths


def touched(row, changed, root=ROOT):
    if row["path"] in changed:
        return True
    crates, targets, filters, broad = parse_test_command(row["test"])
    if not crates or any(not crate for crate in crates):
        return any(path.startswith("crates/") for path in changed)
    for crate in crates:
        prefix = f"crates/{crate}/"
        if f"{prefix}Cargo.toml" in changed:
            return True
        if broad or not targets or any(kind == "test" and not name for kind, name in targets):
            if any(path.startswith((f"{prefix}src/", f"{prefix}tests/")) for path in changed):
                return True
            continue
        for kind, name in targets:
            if kind == "test":
                tests_prefix = f"{prefix}tests/"
                exact = f"{tests_prefix}{name}.rs"
                nested = f"{tests_prefix}{name}/"
                bin_paths = _test_bin_paths(crate, name, root)
                if bin_paths is None:
                    if any(path.startswith(tests_prefix) for path in changed):
                        return True
                    continue
                for path in changed:
                    if path == exact or path in bin_paths or path.startswith(nested):
                        return True
                    if path.startswith(tests_prefix) and "/" in path[len(tests_prefix) :]:
                        return True
            elif kind == "lib":
                src_prefix = f"{prefix}src/"
                mod0 = _lib_module(crate, filters, root)
                if mod0 is None:
                    if any(path.startswith(src_prefix) for path in changed):
                        return True
                else:
                    mod_file = f"{src_prefix}{mod0}.rs"
                    mod_dir = f"{src_prefix}{mod0}/"
                    lib_rs = f"{src_prefix}lib.rs"
                    if any(path == mod_file or path.startswith(mod_dir) or path == lib_rs for path in changed):
                        return True
            elif kind == "doc":
                if any(path.startswith(f"{prefix}src/") for path in changed):
                    return True
    return False


def build_settings(env):
    named = {k for k in env if k.startswith(("CARGO", "RUST")) or k == "UPDATE_EXPECT"}
    return {k: env[k] for k in sorted(named - SET_HERE)}


def selection(args, filters):
    crates, targets = [], set()
    args = iter(args)
    for arg in args:
        if arg in ("-p", "--package"):
            crates.append(next(args, ""))
        elif arg.startswith("--package="):
            crates.append(arg.removeprefix("--package="))
        elif arg.startswith("-p") and not arg.startswith("--"):
            crates.append(arg[2:])
        elif arg in ("--lib", "--doc"):
            targets.add(arg[2:])
        elif arg == "--test":
            targets.add("test " + next(args, ""))
        elif arg.startswith("--test="):
            targets.add("test " + arg.removeprefix("--test="))
        elif arg.startswith("-") or not filters:
            return None
    return (crates, targets or {"all"}) if crates else None


def passed_calls(record, sha, settings):
    try:
        lines = pathlib.Path(record).read_text().splitlines()
    except FileNotFoundError:
        return []
    try:
        entries = [json.loads(line) for line in lines]
    except json.JSONDecodeError as e:
        refuse(f"{record} does not read: {e}")
    calls = []
    for entry in entries:
        if entry["commit"] == sha and entry["settings"] == settings:
            args = entry["args"]
            if "--" in args and args[args.index("--") + 1 :] != ["--include-ignored"]:
                continue
            if selection(args[: args.index("--")] if "--" in args else args, filters=False) is None:
                continue
            targets = entry.get("targets")
            if isinstance(targets, dict):
                calls.append({crate: frozenset(kinds) for crate, kinds in targets.items()})
    return calls


def covered(test, calls):
    wanted = selection(shlex.split(test), filters=True)
    if wanted is None or "doc" in wanted[1] or "all" in wanted[1]:
        return False
    crates, targets = wanted
    return all(
        any(target in done.get(crate, ()) for done in calls)
        for crate in crates
        for target in targets
    )


def cargo_configs_in(root):
    root = pathlib.Path(root)
    found = [place for place in CARGO_CONFIGS if os.path.lexists(root / place)]
    lanes = root / "target" / "mutate" / "lanes"
    if lanes.is_dir():
        for child in sorted(lanes.iterdir(), key=lambda p: p.name):
            cfg = child / ".cargo"
            if os.path.lexists(cfg):
                found.append(cfg.relative_to(root).as_posix())
    return found


def unseen(root):
    listed = subprocess.run(["git", "ls-files", "-z", "-v"], cwd=root, capture_output=True, check=True).stdout
    hidden = [entry for entry in listed.decode().split("\0") if entry and not entry.startswith("H ")]
    return [*hidden, *cargo_configs_in(root)]


def run_tethered(argv, limit=None, track=None, **popen):
    read_end, write_end = os.pipe()
    keeper = subprocess.Popen(
        ["sh", "-c", "read line; kill -KILL 0"],
        stdin=read_end,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        process_group=0,
    )
    os.close(read_end)
    child = subprocess.Popen(
        argv,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        process_group=keeper.pid,
        **popen,
    )
    if track is not None:
        track[0](keeper.pid)
    try:
        output, _ = child.communicate(timeout=limit)
        return child.returncode, output
    except subprocess.TimeoutExpired:
        return None, b""
    finally:
        if track is not None:
            track[1](keeper.pid)
        os.close(write_end)
        keeper.wait()
        child.wait()


def extract(sha):
    if TREE.exists():
        shutil.rmtree(TREE)
    TREE.mkdir(parents=True)
    archive = WORK / "tree.tar"
    # -m dates each file now. By its commit date a file can read older than target/'s last build, and cargo then links
    # a stale library.
    for argv in (["git", "archive", "--output", archive, sha], ["tar", "-x", "-m", "-C", TREE, "-f", archive]):
        code, output = run_tethered(argv, cwd=ROOT)
        if code != 0:
            refuse(f"{argv[0]} failed:\n{output.decode(errors='replace')}")
    code, listed = run_tethered(["git", "ls-tree", "-r", "-z", "--name-only", sha], cwd=ROOT)
    if code != 0:
        refuse(f"git ls-tree failed:\n{listed.decode(errors='replace')}")
    if lacking := [path for path in listed.decode().split("\0") if path and not os.path.lexists(TREE / path)]:
        refuse(f"the extract lacks {lacking[:3]}, which git archive left out")


def matches(sha, row):
    code, blob = run_tethered(["git", "cat-file", "blob", f"{sha}:{row['path']}"], cwd=ROOT)
    return blob.count(row["old"].encode()) if code == 0 else 0


def built_batches(tests):
    groups = {}
    for test in tests:
        tokens = shlex.split(test)
        release = "--release" in tokens
        sel = selection([tok for tok in tokens if tok != "--release"], filters=True)
        if sel is None or "doc" in sel[1]:
            continue
        crates, targets = sel
        key = (release, tuple(crates))
        entry = groups.setdefault(key, [set(), []])
        entry[0].update(targets)
        entry[1].append(test)
    batches = []
    for (release, crates), (targets, batch_tests) in groups.items():
        parts = ["--release"] if release else []
        parts.extend(f"-p {crate}" for crate in crates)
        if "all" not in targets:
            parts.extend(f"--{t}" for t in sorted(targets))
        batches.append((" ".join(parts), batch_tests))
    return batches


def reflink_supported(directory):
    src = directory / f".reflink-src-{os.getpid()}"
    dst = directory / f".reflink-dst-{os.getpid()}"
    try:
        src.write_bytes(b"x")
        code, _ = run_tethered(["cp", "--reflink=always", src, dst], cwd=directory)
        return code == 0
    finally:
        src.unlink(missing_ok=True)
        dst.unlink(missing_ok=True)


def parse_jobs(value):
    if value == "auto":
        return None
    try:
        n = int(value)
    except ValueError:
        raise argparse.ArgumentTypeError(f"expected 'auto' or positive integer, got {value!r}") from None
    if n < 1:
        raise argparse.ArgumentTypeError(f"jobs must be >= 1, got {n}")
    return n


def choose_lanes(jobs_arg, row_count, work_dir):
    if row_count <= 1:
        return 1
    if jobs_arg is not None:
        return min(jobs_arg, row_count)
    if row_count < MIN_PARALLEL_ROWS or not reflink_supported(work_dir):
        return 1
    cpus = os.cpu_count() or 1
    return max(1, min(MAX_AUTO_LANES, cpus // 3, row_count))


def clone_lane(lane_idx, seed_tree, seed_target):
    lane_dir = LANES / f"lane-{lane_idx}"
    lane_tree = lane_dir / "tree"
    lane_target = lane_dir / "target"
    lane_dir.mkdir(parents=True, exist_ok=True)
    for src, dst in ((seed_tree, lane_tree), (seed_target, lane_target)):
        if not src.exists():
            continue
        code, output = run_tethered(["cp", "-a", "--reflink=auto", src, dst], cwd=ROOT)
        if code != 0:
            refuse(f"cp failed cloning lane {lane_idx}:\n{output.decode(errors='replace')}")
    return lane_tree, lane_target


def effective_build_jobs(base_env, lanes_n, build_jobs_arg):
    if build_jobs_arg is not None:
        return build_jobs_arg
    cap = max(1, (os.cpu_count() or 1) // lanes_n)
    raw = base_env.get("CARGO_BUILD_JOBS", "").strip()
    try:
        inherited = int(raw)
    except ValueError:
        return cap
    return min(inherited, cap) if inherited >= 1 else cap


def lane_env(base_env, target_dir, build_jobs):
    out = dict(base_env, CARGO_TARGET_DIR=str(target_dir), CARGO_BUILD_BUILD_DIR=str(target_dir))
    if build_jobs is not None:
        out["CARGO_BUILD_JOBS"] = str(build_jobs)
    return out


def cargo_test(test, env, limit, tree=TREE, track=None):
    argv = ["cargo", "test", "--manifest-path", tree / "Cargo.toml", *shlex.split(test)]
    code, output = run_tethered(argv, limit, track=track, cwd=tree, env=env)
    return code, output.decode(errors="replace")


def judge(code, output, limit):
    if code is None:
        return "UNCLEAR", f"over {limit:.0f} s"
    if code == 0:
        return "SURVIVED", ""
    if failed := FAILED.findall(output):
        more = f", {len(failed) - SHOWN} more" if len(failed) > SHOWN else ""
        return "CAUGHT", ", ".join(failed[:SHOWN]) + more
    if coded := CODED_ERROR.search(output):
        return "CAUGHT at build", coded[0]
    if "error: test failed" in output and (died := DIED.search(output)):
        if int(died[2]) in FAULTS:
            return "CAUGHT", f"a test binary crashed, {died[1]}"
        return "UNCLEAR", f"a test binary was killed from outside, {died[1]}"
    return "UNCLEAR", f"exit {code} with no test named"


def main():
    parser = argparse.ArgumentParser(description="Breaks one rule at a time on a commit. This file's header has more.")
    parser.add_argument("table")
    parser.add_argument("--rev", default="HEAD")
    parser.add_argument("--only", nargs="+", default=[], metavar="NAME")
    parser.add_argument("--budget", type=int, default=420, metavar="SECONDS", help="the least limit on a row")
    parser.add_argument("--since", metavar="REV", help="only rows whose path or test crate's tests/ changed after REV")
    parser.add_argument("--check", action="store_true", help="stop before any build")
    parser.add_argument("--tested", metavar="RECORD", help="skip the baselines 10-rust's passed test calls cover")
    parser.add_argument("--jobs", type=parse_jobs, default=None, metavar="N|auto", help="parallel seed-shape lanes")
    parser.add_argument("--build-jobs", type=int, default=None, metavar="N", help="CARGO_BUILD_JOBS per parallel lane")
    parser.add_argument("--stop-on-fail", action="store_true", help="stop dispatching rows past the first non-CAUGHT row")
    args = parser.parse_args()

    rows = load_rows(args.table, args.only)
    code, found = run_tethered(["git", "rev-parse", "--verify", "--quiet", f"{args.rev}^{{commit}}"], cwd=ROOT)
    if code != 0:
        refuse(f"{args.rev} is no commit here")
    sha = found.decode().strip()
    if args.since is not None:
        code, found = run_tethered(["git", "rev-parse", "--verify", "--quiet", f"{args.since}^{{commit}}"], cwd=ROOT)
        if code != 0:
            refuse(f"{args.since} is no commit here")
        code, found = run_tethered(["git", "diff", "--name-only", "--no-renames", "-z", found.decode().strip(), sha], cwd=ROOT)
        if code != 0:
            refuse(f"git diff failed:\n{found.decode(errors='replace')}")
        changed = set(found.decode().split("\0")) - {""}
        rows = [row for row in rows if touched(row, changed)]
        if not rows:
            print(f"no row is touched after {args.since}")
            sys.exit(0)
    held = args.budget * BASELINE_TIMES
    print(
        f"sweep of {sha}, {len(rows)} rows, a row's limit {ROW_TIMES} times its baseline and at least {args.budget} s, "
        f"a baseline's {held} s",
        flush=True,
    )

    tally = collections.Counter()

    def report(verdict, name, seconds, detail, limit=None):
        tally[verdict] += 1
        given = "" if limit is None else f"limit {limit:.0f} s  "
        print(f"{verdict:<15}  {name}  {seconds:.0f} s  {given}{detail}".rstrip(), flush=True)

    applied = []
    for row in rows:
        count = matches(sha, row)
        if count == 1:
            applied.append(row)
        else:
            report("NOT APPLIED", row["name"], 0, f"{count} matches in {row['path']}")
            if args.stop_on_fail and not args.check:
                counts = ", ".join(f"{count} {verdict}" for verdict, count in tally.items())
                print(f"summary: {counts}, of {len(rows)} rows at {sha}")
                sys.exit(1)
    if args.check:
        print(f"{len(applied)} of {len(rows)} rows apply")
        sys.exit(0 if len(applied) == len(rows) else 1)

    if configs := cargo_configs_in(ROOT):
        refuse(f"cargo in the sweep's tree reads {configs} and the checkout's cargo does not; delete them")
    WORK.mkdir(parents=True, exist_ok=True)
    lock = open(WORK / "lock", "w")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        refuse(f"another sweep is running in {ROOT}")
    if LANES.exists():
        shutil.rmtree(LANES)
    extract(sha)
    seed_target = WORK / "target"
    target = str(seed_target)
    # An inherited target or build dir links another checkout's crates.
    env = dict(
        os.environ,
        CARGO_TARGET_DIR=target,
        CARGO_BUILD_BUILD_DIR=target,
        CARGO_TERM_COLOR="never",
        CARGO_NET_OFFLINE="true",
    )
    # UPDATE_EXPECT rewrites a trace where it should fail.
    env.pop("UPDATE_EXPECT", None)

    calls = passed_calls(args.tested, sha, build_settings(env)) if args.tested else []
    code, status = run_tethered(["git", "status", "--porcelain", "-z", "--untracked-files=all"], cwd=ROOT)
    if code != 0:
        refuse(f"git status failed:\n{status.decode(errors='replace')}")
    if calls and (moved := [*filter(None, status.decode().split("\0")), *unseen(ROOT)]):
        print(f"the checkout holds {moved[:3]} outside {sha}, so every baseline runs", flush=True)
        calls = []

    took = {}
    io_lock = threading.Lock()
    test_budget = {}
    for row in applied:
        test_budget[row["test"]] = max(test_budget.get(row["test"], args.budget), row.get("budget", 0))

    def run_baseline(test, tree=TREE, run_env=env, track=None):
        base_limit = max(held, test_budget.get(test, args.budget) * BASELINE_TIMES)
        began = time.monotonic()
        code, output = cargo_test(test, run_env, base_limit, tree=tree, track=track)
        if code != 0 or not PASSED.search(output):
            gave = output[-2000:] or f"nothing in {base_limit} s"
            return f"`cargo test {test}` must pass some test on the unmutated tree, and it gave:\n{gave}\n"
        elapsed = time.monotonic() - began
        with io_lock:
            took[test] = elapsed
            print(f"baseline  {elapsed:.0f} s  cargo test {test}", flush=True)
        return None

    all_tests = list(dict.fromkeys(row["test"] for row in applied))
    skipped = [test for test in all_tests if covered(test, calls)]
    unskipped = [test for test in all_tests if not covered(test, calls)]
    lanes_n = choose_lanes(args.jobs, len(applied), WORK)

    def batch_limit(batch_tests):
        return max(args.budget, *(test_budget.get(test, args.budget) for test in batch_tests)) * BASELINE_TIMES

    def mutated_in(row, tree, run_env, track=None):
        with io_lock:
            base_s = took.get(row["test"], 0)
        limit = max(args.budget, row.get("budget", 0), ROW_TIMES * base_s)
        file = tree / row["path"]
        original = file.read_bytes()
        file.write_bytes(original.replace(row["old"].encode(), row["new"].encode()))
        try:
            began = time.monotonic()
            code, output = cargo_test(row["test"], run_env, limit, tree=tree, track=track)
            seconds = time.monotonic() - began
        finally:
            file.write_bytes(original)
        return code, *judge(code, output, limit), seconds, limit, base_s > 0

    if lanes_n == 1:
        for test in unskipped:
            if err := run_baseline(test):
                refuse(err)
        if skipped:
            print(f"baselines skipped: {len(skipped)}, whose tests 10-rust passed on this commit", flush=True)
        for build, batch_tests in built_batches(skipped):
            limit = batch_limit(batch_tests)
            began = time.monotonic()
            code, output = cargo_test(f"--no-run {build}", env, limit)
            if code != 0:
                gave = output[-2000:] or f"nothing in {limit} s"
                refuse(f"`cargo test --no-run {build}` must build on the unmutated tree, and it gave:\n{gave}\n")
            print(f"build     {time.monotonic() - began:.0f} s  cargo test --no-run {build}", flush=True)
        for row in applied:
            code, verdict, detail, seconds, limit, _ = mutated_in(row, TREE, env)
            if verdict == "CAUGHT at build" and row["test"] in skipped:
                skipped.remove(row["test"])
                if err := run_baseline(row["test"]):
                    refuse(err)
            if code is None and row["test"] in skipped:
                print(f"over      {limit:.0f} s  {row['name']} runs again after its skipped baseline", flush=True)
                skipped.remove(row["test"])
                if err := run_baseline(row["test"]):
                    refuse(err)
                code, verdict, detail, seconds, limit, _ = mutated_in(row, TREE, env)
            report(verdict, row["name"], seconds, detail, limit)
            if args.stop_on_fail and verdict not in ("CAUGHT", "CAUGHT at build"):
                break
    else:
        build_jobs = effective_build_jobs(env, lanes_n, args.build_jobs)
        if skipped:
            print(f"baselines skipped: {len(skipped)}, whose tests 10-rust passed on this commit", flush=True)
        for build, batch_tests in built_batches(all_tests):
            limit = batch_limit(batch_tests)
            began = time.monotonic()
            code, output = cargo_test(f"--no-run {build}", env, limit)
            if code != 0:
                gave = output[-2000:] or f"nothing in {limit} s"
                refuse(f"`cargo test --no-run {build}` must build on the unmutated tree, and it gave:\n{gave}\n")
            print(f"build     {time.monotonic() - began:.0f} s  cargo test --no-run {build}", flush=True)
        print(f"lanes: {lanes_n}, cargo build jobs per lane: {build_jobs}", flush=True)
        try:
            lane_specs = [(TREE, lane_env(env, seed_target, build_jobs))]
            for i in range(1, lanes_n):
                ltree, ltarget = clone_lane(i, TREE, seed_target)
                lane_specs.append((ltree, lane_env(env, ltarget, build_jobs)))

            state_lock = threading.Lock()
            abort_error = None
            stop_at = None
            refuse_at = None
            active_pgs = {}

            def limit_rank():
                ranks = [r for r in (stop_at if args.stop_on_fail else None, refuse_at) if r is not None]
                return min(ranks) if ranks else None

            def cancel_pgs(above=None):
                for pgid, box in list(active_pgs.items()):
                    rank = None if box is None else box[0]
                    if above is None or (rank is not None and rank > above):
                        try:
                            os.killpg(pgid, signal.SIGKILL)
                        except ProcessLookupError:
                            pass

            def track_for_box(box):
                def start(pgid):
                    with state_lock:
                        bound = limit_rank()
                        rank = None if box is None else box[0]
                        if abort_error is not None or (bound is not None and rank is not None and rank > bound):
                            try:
                                os.killpg(pgid, signal.SIGKILL)
                            except ProcessLookupError:
                                pass
                        else:
                            active_pgs[pgid] = box

                def end(pgid):
                    with state_lock:
                        active_pgs.pop(pgid, None)

                return start, end

            def track_for(rank):
                return track_for_box(None if rank is None else [rank])

            def record_exception(exc):
                nonlocal abort_error
                with state_lock:
                    if abort_error is None:
                        abort_error = f"worker failed with {type(exc).__name__}: {exc}"
                        cancel_pgs(None)

            if unskipped:
                base_cursor = 0

                def baseline_worker(ltree, lenv):
                    nonlocal base_cursor, abort_error
                    try:
                        while True:
                            with state_lock:
                                if abort_error is not None or base_cursor >= len(unskipped):
                                    return
                                test = unskipped[base_cursor]
                                base_cursor += 1
                            err = run_baseline(test, tree=ltree, run_env=lenv, track=track_for(None))
                            if err is not None:
                                with state_lock:
                                    if abort_error is None:
                                        abort_error = err
                                        cancel_pgs(None)
                                return
                    except BaseException as exc:
                        record_exception(exc)

                bthreads = [threading.Thread(target=baseline_worker, args=(ltree, lenv)) for ltree, lenv in lane_specs]
                for thread in bthreads:
                    thread.start()
                for thread in bthreads:
                    thread.join()
                if abort_error is not None:
                    refuse(abort_error)

            test_locks = collections.defaultdict(threading.Lock)
            initial_skipped = frozenset(skipped)
            skipped_set = set(skipped)
            recovery_box = {}
            recovery_errors = {}
            cursor = 0
            next_print = 0
            results = {}

            def flush_results():
                nonlocal next_print, abort_error
                while next_print in results:
                    if abort_error is not None:
                        break
                    if args.stop_on_fail and stop_at is not None and next_print > stop_at:
                        break
                    entry = results.pop(next_print)
                    if entry[0] == "REFUSE":
                        abort_error = entry[1]
                        cancel_pgs(None)
                        break
                    v, n, s, d, lim = entry
                    with io_lock:
                        report(v, n, s, d, lim)
                    next_print += 1

            def ensure_baseline(test, tree, run_env, idx):
                with state_lock:
                    bound = limit_rank()
                    if abort_error is not None or (bound is not None and idx > bound):
                        return None, True
                    if test in recovery_errors:
                        return recovery_errors[test], False
                    if test not in skipped_set:
                        return None, False
                    box = recovery_box.setdefault(test, [idx])
                    box[0] = min(box[0], idx)
                with test_locks[test]:
                    with state_lock:
                        bound = limit_rank()
                        if abort_error is not None or (bound is not None and idx > bound):
                            return None, True
                        if test in recovery_errors:
                            return recovery_errors[test], False
                        if test not in skipped_set:
                            return None, False
                        box = recovery_box.setdefault(test, [idx])
                        box[0] = min(box[0], idx)
                    err = run_baseline(test, tree=tree, run_env=run_env, track=track_for_box(box))
                    with state_lock:
                        bound = limit_rank()
                        if abort_error is not None or (bound is not None and box[0] > bound):
                            return None, True
                        if err is not None:
                            recovery_errors[test] = err
                            skipped_set.discard(test)
                            return err, False
                        skipped_set.discard(test)
                    return None, False

            def record_refusal(idx, err):
                nonlocal refuse_at
                with state_lock:
                    bound = limit_rank()
                    if abort_error is not None or (bound is not None and idx > bound):
                        return
                    results[idx] = ("REFUSE", err)
                    refuse_at = idx if refuse_at is None else min(refuse_at, idx)
                    cancel_pgs(limit_rank())
                    flush_results()

            def worker(ltree, lenv):
                nonlocal cursor, stop_at
                try:
                    while True:
                        with state_lock:
                            bound = limit_rank()
                            if abort_error is not None or cursor >= len(applied):
                                return
                            if bound is not None and cursor >= bound:
                                return
                            idx = cursor
                            cursor += 1
                        row = applied[idx]
                        track = track_for(idx)
                        code, verdict, detail, seconds, limit, had_base = mutated_in(row, ltree, lenv, track=track)
                        with state_lock:
                            bound = limit_rank()
                            if abort_error is not None or (bound is not None and idx > bound):
                                return
                        if verdict == "CAUGHT at build":
                            with state_lock:
                                need = row["test"] in skipped_set or row["test"] in recovery_errors
                            if need:
                                err, stopped = ensure_baseline(row["test"], ltree, lenv, idx)
                                if stopped:
                                    return
                                if err is not None:
                                    record_refusal(idx, err)
                                    return
                        if code is None and not had_base and row["test"] in initial_skipped:
                            with io_lock:
                                print(f"over      {limit:.0f} s  {row['name']} runs again after its skipped baseline", flush=True)
                            err, stopped = ensure_baseline(row["test"], ltree, lenv, idx)
                            if stopped:
                                return
                            if err is not None:
                                record_refusal(idx, err)
                                return
                            code, verdict, detail, seconds, limit, _ = mutated_in(row, ltree, lenv, track=track)
                            with state_lock:
                                bound = limit_rank()
                                if abort_error is not None or (bound is not None and idx > bound):
                                    return
                        with state_lock:
                            bound = limit_rank()
                            if abort_error is not None or (bound is not None and idx > bound):
                                return
                            results[idx] = (verdict, row["name"], seconds, detail, limit)
                            if args.stop_on_fail and verdict not in ("CAUGHT", "CAUGHT at build"):
                                stop_at = idx if stop_at is None else min(stop_at, idx)
                                cancel_pgs(limit_rank())
                            flush_results()
                except BaseException as exc:
                    record_exception(exc)

            threads = [threading.Thread(target=worker, args=(ltree, lenv)) for ltree, lenv in lane_specs]
            for thread in threads:
                thread.start()
            for thread in threads:
                thread.join()
            if abort_error is not None:
                refuse(abort_error)
        finally:
            if LANES.exists():
                shutil.rmtree(LANES, ignore_errors=True)

    counts = ", ".join(f"{count} {verdict}" for verdict, count in tally.items())
    print(f"summary: {counts}, of {len(rows)} rows at {sha}")
    sys.exit(0 if sum(tally.values()) == len(rows) and set(tally) <= {"CAUGHT", "CAUGHT at build"} else 1)


if __name__ == "__main__":
    main()
