"""Breaks one rule at a time on a commit and reports whether a test notices.

    python3 -I tools/mutate.py TABLE [--rev REV] [--only NAME ...] [--since REV] [--budget SECONDS] [--check]
                                     [--tested RECORD]

Python 3.11 stdlib only. TABLE is TOML with one [[mutant]] per rule, aimed at the test that holds it. `old` must match
exactly once. `test` is what follows `cargo test`, and a row's own `test` replaces the table's. It may not pass
--manifest-path, --target-dir or --config.

    test = "--release -p ng-contract --test map_reads"

    [[mutant]]
    name = "no-supply"
    path = "crates/ng-contract/src/compile.rs"
    old = '''if !anatomy.unsupplied(site) =>'''
    new = '''if false && !anatomy.unsupplied(site) =>'''

A sweep extracts REV, default HEAD, to target/mutate/ in this checkout, so it never edits your files. Each `test` must
pass there before any row runs. Each row prints as it ends, so a killed sweep keeps its lines:

    CAUGHT           a test failed, named on the line, or a test binary crashed
    CAUGHT at build  rustc refused the mutant with a coded error; read it, because a mistyped mutant proves nothing
    SURVIVED         every test passed, so ask what the test could not see
    NOT APPLIED      `old` did not match exactly once, so nothing ran
    UNCLEAR          over budget, killed from outside, or failed with no test or coded error named: neither a catch
                     nor a survivor

Exit 0 only when every row is CAUGHT. --check lists the NOT APPLIED rows and stops before any build. --since keeps the
rows whose path, or any file under their test crate's tests/, changed after REV. Each command runs in its own process
group, and that group dies with the command and with this script, however this script dies.

tools/verify's 40-mutants step runs this script through `gate.py mutants`. Before 10-rust it replays the rows a change
adds or edits, or whose path it changes, and after 10-rust the rest. It stops each sweep at the first row that is not
CAUGHT, so a red run reports only its first failure. A hand run reports every row.

--tested names the record 10-rust writes of the test calls it passed. The rest part of a merge run passes it, and the
sweep then skips each baseline that a passed call covers on this commit with these build settings, and builds its test
instead, so a tree that does not build still stops the sweep. A call covers only when it ran every test, or every test
with the ignored ones. A `--release`, `--doc` or other flagged test, a crate 10-rust did not test, and a record from
another commit or other settings run the baseline. A full run and a hand run run every baseline. The record counts
only while the checkout holds no changed or untracked file and no tracked file marked assume-unchanged or skip-worktree.
A row caught at build runs its skipped baseline, so a filter that names no test still fails. The sweep refuses a Cargo
config in target/.cargo or target/mutate/.cargo, which its cargo reads and 10-rust's does not, and an extract that
lacks a tracked path.
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
import time
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
WORK = ROOT / "target" / "mutate"
TREE = WORK / "tree"
PASSED = re.compile(r"^test .+ \.\.\. ok$", re.M)
FAILED = re.compile(r"^test (.+) \.\.\. FAILED$", re.M)
CODED_ERROR = re.compile(r"^.*\berror\[E\d{4}\].*$", re.M)
DIED = re.compile(r"process didn't exit successfully: .*\((signal: (\d+), .*)\)$", re.M)
FAULTS = {signal.SIGABRT, signal.SIGILL, signal.SIGSEGV, signal.SIGBUS, signal.SIGFPE}
SHOWN = 3
ROW_KEYS = {"name", "path", "old", "new", "test"}
MOVES = {"--manifest-path", "--target-dir", "--config"}
SET_HERE = {"CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR", "CARGO_TERM_COLOR", "CARGO_NET_OFFLINE"}
CARGO_CONFIGS = ("target/.cargo", "target/mutate/.cargo")


def refuse(why):
    sys.exit(f"mutate: {why}; no sweep")


def load_rows(path, only):
    with open(path, "rb") as file:
        table = tomllib.load(file)
    rows = table.get("mutant", [])
    if not rows:
        refuse(f"{path} has no [[mutant]] row")
    for row in rows:
        row.setdefault("test", table.get("test"))
        if set(row) != ROW_KEYS or not row["test"]:
            refuse(f"row {row.get('name')} needs exactly {sorted(ROW_KEYS)}")
        if moved := MOVES & {arg.split("=")[0] for arg in shlex.split(row["test"])}:
            refuse(f"row {row['name']} passes {sorted(moved)}, which can point cargo outside the sweep's tree")
    names = [row["name"] for row in rows]
    if len(set(names)) != len(names):
        refuse("two rows share a name")
    if unknown := set(only) - set(names):
        refuse(f"--only names no row: {sorted(unknown)}")
    return [row for row in rows if not only or row["name"] in only]


def touched(row, changed):
    args = shlex.split(row["test"])
    crates = [b for a, b in zip(args, args[1:]) if a in ("-p", "--package")]
    crates += [a.removeprefix("--package=") for a in args if a.startswith("--package=")]
    crates += [a[2:] for a in args if a.startswith("-p") and len(a) > 2]
    tests = [f"crates/{crate}/tests/" for crate in crates] or ["crates/"]
    return row["path"] in changed or any(path.startswith(t) and "/tests/" in path for t in tests for path in changed)


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


def built(test):
    crates, targets = selection(shlex.split(test), filters=True)
    return " ".join([*(f"-p {crate}" for crate in crates), *(f"--{t}" for t in sorted(targets - {"all"}))])


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
            calls.append(selection(args[: args.index("--")] if "--" in args else args, filters=False))
    return [call for call in calls if call]


def covered(test, calls):
    wanted = selection(shlex.split(test), filters=True)
    if wanted is None or "doc" in wanted[1]:
        return False
    crates, targets = wanted
    return all(
        any(crate in done and ("all" in kinds or target in kinds) for done, kinds in calls)
        for crate in crates
        for target in targets
    )


def unseen(root):
    listed = subprocess.run(["git", "ls-files", "-z", "-v"], cwd=root, capture_output=True, check=True).stdout
    hidden = [entry for entry in listed.decode().split("\0") if entry and not entry.startswith("H ")]
    return [*hidden, *(place for place in CARGO_CONFIGS if os.path.lexists(os.path.join(root, place)))]


def run_tethered(argv, budget=None, **popen):
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
    try:
        output, _ = child.communicate(timeout=budget)
        return child.returncode, output
    except subprocess.TimeoutExpired:
        return None, b""
    finally:
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


def cargo_test(test, env, budget):
    argv = ["cargo", "test", "--manifest-path", TREE / "Cargo.toml", *shlex.split(test)]
    code, output = run_tethered(argv, budget, cwd=TREE, env=env)
    return code, output.decode(errors="replace")


def judge(code, output, budget):
    if code is None:
        return "UNCLEAR", f"over {budget} s"
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
    parser.add_argument("--budget", type=int, default=420, metavar="SECONDS", help="limit on one cargo test")
    parser.add_argument("--since", metavar="REV", help="only rows whose path or test crate's tests/ changed after REV")
    parser.add_argument("--check", action="store_true", help="stop before any build")
    parser.add_argument("--tested", metavar="RECORD", help="skip the baselines 10-rust's passed test calls cover")
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
        code, found = run_tethered(["git", "diff", "--name-only", "-z", found.decode().strip(), sha], cwd=ROOT)
        if code != 0:
            refuse(f"git diff failed:\n{found.decode(errors='replace')}")
        changed = set(found.decode().split("\0")) - {""}
        rows = [row for row in rows if touched(row, changed)]
        if not rows:
            print(f"no row is touched after {args.since}")
            sys.exit(0)
    print(f"sweep of {sha}, {len(rows)} rows, budget {args.budget} s", flush=True)

    tally = collections.Counter()

    def report(verdict, name, seconds, detail):
        tally[verdict] += 1
        print(f"{verdict:<15}  {name}  {seconds:.0f} s  {detail}".rstrip(), flush=True)

    applied = []
    for row in rows:
        count = matches(sha, row)
        if count == 1:
            applied.append(row)
        else:
            report("NOT APPLIED", row["name"], 0, f"{count} matches in {row['path']}")
    if args.check:
        print(f"{len(applied)} of {len(rows)} rows apply")
        sys.exit(0 if len(applied) == len(rows) else 1)

    if configs := [place for place in CARGO_CONFIGS if os.path.lexists(ROOT / place)]:
        refuse(f"cargo in the sweep's tree reads {configs} and the checkout's cargo does not; delete them")
    WORK.mkdir(parents=True, exist_ok=True)
    lock = open(WORK / "lock", "w")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        refuse(f"another sweep is running in {ROOT}")
    extract(sha)
    target = str(WORK / "target")
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

    def baseline(test):
        began = time.monotonic()
        code, output = cargo_test(test, env, args.budget)
        if code != 0 or not PASSED.search(output):
            gave = output[-2000:] or f"nothing in {args.budget} s"
            refuse(f"`cargo test {test}` must pass some test on the unmutated tree, and it gave:\n{gave}\n")
        print(f"baseline  {time.monotonic() - began:.0f} s  cargo test {test}", flush=True)

    skipped = []
    for test in dict.fromkeys(row["test"] for row in applied):
        if covered(test, calls):
            skipped.append(test)
        else:
            baseline(test)
    if skipped:
        print(f"baselines skipped: {len(skipped)}, whose tests 10-rust passed on this commit", flush=True)
    for build in dict.fromkeys(built(test) for test in skipped):
        began = time.monotonic()
        code, output = cargo_test(f"--no-run {build}", env, args.budget)
        if code != 0:
            gave = output[-2000:] or f"nothing in {args.budget} s"
            refuse(f"`cargo test --no-run {build}` must build on the unmutated tree, and it gave:\n{gave}\n")
        print(f"build     {time.monotonic() - began:.0f} s  cargo test --no-run {build}", flush=True)

    for row in applied:
        file = TREE / row["path"]
        original = file.read_bytes()
        file.write_bytes(original.replace(row["old"].encode(), row["new"].encode()))
        began = time.monotonic()
        code, output = cargo_test(row["test"], env, args.budget)
        file.write_bytes(original)
        verdict, detail = judge(code, output, args.budget)
        seconds = time.monotonic() - began
        if verdict == "CAUGHT at build" and row["test"] in skipped:
            skipped.remove(row["test"])
            baseline(row["test"])
        report(verdict, row["name"], seconds, detail)

    counts = ", ".join(f"{count} {verdict}" for verdict, count in tally.items())
    print(f"summary: {counts}, of {len(rows)} rows at {sha}")
    sys.exit(0 if set(tally) <= {"CAUGHT", "CAUGHT at build"} else 1)


if __name__ == "__main__":
    main()
