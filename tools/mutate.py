"""Breaks one rule at a time on a commit and reports whether a test notices.

    python3 -I tools/mutate.py TABLE [--rev REV] [--only NAME ...] [--budget SECONDS] [--check]

Python 3.11 stdlib only. TABLE is TOML with one [[mutant]] per rule, aimed at the test that holds it. `old` must match
exactly once. `test` is what follows `cargo test`, and a row's own `test` replaces the table's. It may not pass
--manifest-path, --target-dir or --config, which can point cargo outside the sweep's tree.

    test = "--release -p ng-contract --test map_reads"

    [[mutant]]
    name = "no-supply"
    path = "crates/ng-contract/src/compile.rs"
    old = '''if !anatomy.unsupplied(site) =>'''
    new = '''if false && !anatomy.unsupplied(site) =>'''

A sweep extracts REV, default HEAD, to target/mutate/ in this checkout, so it never edits your files. Each `test` must
pass there before any row runs. Each row prints as it ends, so a killed sweep keeps its lines:

    CAUGHT           a test failed, named on the line, or a test binary crashed
    CAUGHT at build  the mutant does not compile; read the error, because a mistyped mutant proves nothing
    SURVIVED         every test passed, so ask what the test could not see
    NOT APPLIED      `old` did not match exactly once, so nothing ran
    UNCLEAR          over budget, killed from outside, or failed with no test named: neither a catch nor a survivor

Exit 0 only when every row is CAUGHT. --check lists the NOT APPLIED rows and stops before any build. Each command runs
in its own process group, and that group dies with the command and with this script, however this script dies.
"""
import argparse
import collections
import fcntl
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
# Only an error with a code is the mutant's. A compiler or linker killed from outside, or a full disk, fails the build
# with none.
NO_BUILD = re.compile(r"^.*\berror\[E\d{4}\].*$", re.M)
DIED = re.compile(r"process didn't exit successfully: .*\((signal: (\d+), .*)\)$", re.M)
FAULTS = {signal.SIGABRT, signal.SIGILL, signal.SIGSEGV, signal.SIGBUS, signal.SIGFPE}
SHOWN = 3
ROW_KEYS = {"name", "path", "old", "new", "test"}
MOVES = {"--manifest-path", "--target-dir", "--config"}


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


def run(argv, budget=None, **popen):
    """The keeper leads the group and waits on a pipe only this process holds open. Closing it kills the group, and so
    does this process dying, even by SIGKILL. A group of cargo alone would lose its id when cargo exits, and killing
    cargo alone leaves the test binary running.
    """
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
        code, output = run(argv, cwd=ROOT)
        if code != 0:
            refuse(f"{argv[0]} failed:\n{output.decode(errors='replace')}")


def matches(sha, row):
    code, blob = run(["git", "cat-file", "blob", f"{sha}:{row['path']}"], cwd=ROOT)
    return blob.count(row["old"].encode()) if code == 0 else 0


def cargo_test(test, env, budget):
    argv = ["cargo", "test", "--manifest-path", TREE / "Cargo.toml", *shlex.split(test)]
    code, output = run(argv, budget, cwd=TREE, env=env)
    return code, output.decode(errors="replace")


def judge(code, output, budget):
    if code is None:
        return "UNCLEAR", f"over {budget} s"
    if code == 0:
        return "SURVIVED", ""
    if failed := FAILED.findall(output):
        more = f", {len(failed) - SHOWN} more" if len(failed) > SHOWN else ""
        return "CAUGHT", ", ".join(failed[:SHOWN]) + more
    if coded := NO_BUILD.search(output):
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
    parser.add_argument("--check", action="store_true", help="stop before any build")
    args = parser.parse_args()

    rows = load_rows(args.table, args.only)
    code, found = run(["git", "rev-parse", "--verify", "--quiet", f"{args.rev}^{{commit}}"], cwd=ROOT)
    if code != 0:
        refuse(f"{args.rev} is no commit here")
    sha = found.decode().strip()
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

    WORK.mkdir(parents=True, exist_ok=True)
    lock = open(WORK / "lock", "w")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        refuse(f"another sweep is running in {ROOT}")
    extract(sha)
    target = str(WORK / "target")
    # An inherited target or build dir links another checkout's crates, and UPDATE_EXPECT rewrites a trace where it
    # should fail.
    env = dict(
        os.environ,
        CARGO_TARGET_DIR=target,
        CARGO_BUILD_BUILD_DIR=target,
        CARGO_TERM_COLOR="never",
        CARGO_NET_OFFLINE="true",
    )
    env.pop("UPDATE_EXPECT", None)

    for test in dict.fromkeys(row["test"] for row in applied):
        began = time.monotonic()
        code, output = cargo_test(test, env, args.budget)
        if code != 0 or not PASSED.search(output):
            gave = output[-2000:] or f"nothing in {args.budget} s"
            refuse(f"`cargo test {test}` must pass some test on the unmutated tree, and it gave:\n{gave}\n")
        print(f"baseline  {time.monotonic() - began:.0f} s  cargo test {test}", flush=True)

    for row in applied:
        file = TREE / row["path"]
        original = file.read_bytes()
        file.write_bytes(original.replace(row["old"].encode(), row["new"].encode()))
        began = time.monotonic()
        code, output = cargo_test(row["test"], env, args.budget)
        file.write_bytes(original)
        verdict, detail = judge(code, output, args.budget)
        report(verdict, row["name"], time.monotonic() - began, detail)

    counts = ", ".join(f"{count} {verdict}" for verdict, count in tally.items())
    print(f"summary: {counts}, of {len(rows)} rows at {sha}")
    sys.exit(0 if set(tally) <= {"CAUGHT", "CAUGHT at build"} else 1)


if __name__ == "__main__":
    main()
