"""Making teacher runs: the only module that starts `gp-driver`, for the audit now and for the later tickets.

    teacher = Teacher(driver, data / "audit", commit, memory_cap_gb=8, disks=[data, Path("/mnt/c")], floor_bytes=10**10)
    a0 = teacher.settle(TeacherRun("A0", spec, None))           # found by its spec text, or made alone
    settled = teacher.ensure(runs, a0.directory / "summary.txt", workers=3)

A caller names runs and gets each one's settled directory. The driver's words, exit codes and outcome lines,
`status`, the spec files, the closed loop's beat lines and answers, the order, replays, the disk rule and
progress.txt stay here. It never computes a spec id, never reads a table and never judges a run.

Every driver it starts dies with it (PR_SET_PDEATHSIG), so a dead teacher leaves partial runs that the next call
redoes, never a closed loop that faults because its rule went away. The pool is one thread, so the hook between fork
and exec is safe.
"""
from __future__ import annotations

import ctypes
import json
import math
import os
import selectors
import shutil
import signal
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

import rules
from capture import BeatReading
from spec import PostureEvent, RunSpec

LINK_BYTES_PER_STEP = 18_584
BEAT_KEYS = ("beat", "start_s", "end_s", "heart_rate_per_s", "arterial_pressure", "stroke", "filtration", "earliest_ms")
PR_SET_PDEATHSIG = 1


class TeacherError(Exception):
    pass


@dataclass(frozen=True)
class TeacherRun:
    """A run a caller wants: a spec, or a replay of a closed loop's realized events once that loop has settled."""

    name: str
    spec: RunSpec | None
    replays: str | None


@dataclass(frozen=True)
class Settled:
    name: str
    spec_id: str
    word: str
    directory: Path


def steps_per_s(spec: RunSpec) -> float:
    """Ticket 04's projections, for order and disk only: 212 accepted steps a simulated second on Core at 10 ms, 1.5
    times as many at 5 ms, and one step a millisecond on the reference integrator."""
    if spec.cap_ms is None:
        return 1000.0
    return 212.0 if spec.cap_ms >= 10 else 318.0


def expected_wall_s(spec: RunSpec) -> float:
    simulated = spec.end_ms / 1000 + branch_tail_s(spec)
    per_step = 3.3e-3 if spec.cap_ms is None else 1.4e-3
    return 25.0 + simulated * steps_per_s(spec) * per_step


def branch_tail_s(spec: RunSpec) -> float:
    at = spec.variant_ms()
    if spec.variant.startswith(("checkpoint-resume", "rewind")) and at is not None:
        return (spec.end_ms - at) / 1000
    return 0.0


def expected_link_bytes(spec: RunSpec) -> int:
    return int(spec.end_ms / 1000 * steps_per_s(spec) * LINK_BYTES_PER_STEP)


def _die_with_parent() -> Callable[[], None]:
    parent = os.getpid()
    libc = ctypes.CDLL(None, use_errno=True)

    def hook() -> None:
        libc.prctl(PR_SET_PDEATHSIG, signal.SIGKILL, 0, 0, 0)
        if os.getppid() != parent:
            os._exit(1)

    return hook


def parse_beat(line: str) -> tuple[BeatReading, int]:
    pairs = json.loads(line, object_pairs_hook=list)
    if [key for key, _ in pairs] != list(BEAT_KEYS):
        raise TeacherError(f"a beat line with keys {[key for key, _ in pairs]}")
    values = dict(pairs)

    def number(key: str) -> float:
        value = values[key]
        return math.nan if value is None else float(value)

    beat = BeatReading(int(values["beat"]), *(number(key) for key in BEAT_KEYS[1:7]))
    return beat, int(values["earliest_ms"])


class _Child:
    def __init__(self, run: TeacherRun, spec_path: Path, process: subprocess.Popen, logs: tuple[Path, Path], rule) -> None:
        self.run = run
        self.spec_path = spec_path
        self.process = process
        self.logs = logs
        self.rule = rule
        self.buffer = b""
        self.taken = {event.at_ms for event in run.spec.events} if run.spec else set()
        self.end_ms = run.spec.end_ms if run.spec else 0
        self.started = time.monotonic()
        self.refused: str | None = None


class Teacher:
    def __init__(
        self,
        driver: list[str],
        out: Path,
        source_commit: str,
        memory_cap_gb: int,
        disks: list[Path],
        floor_bytes: int,
        free_bytes: Callable[[Path], int] | None = None,
        rule_for: Callable[[RunSpec], object] | None = None,
    ) -> None:
        self.driver = driver
        self.out = out
        self.source_commit = source_commit
        self.memory_cap_gb = memory_cap_gb
        self.disks = disks
        self.floor_bytes = floor_bytes
        self.free_bytes = free_bytes or (lambda path: shutil.disk_usage(path).free)
        self.rule_for = rule_for or rules.start
        self.ids: dict[str, str] = {}
        self.started: list[str] = []

    def find(self, text: str) -> Path | None:
        """The settled directory whose spec.txt is this text, read by bytes, so no id is computed."""
        if not self.out.is_dir():
            return None
        for directory in sorted(self.out.iterdir()):
            name = directory.name.removesuffix(".fault")
            if len(name) == 64 and all(c in "0123456789abcdef" for c in name) and directory.is_dir():
                spec = directory / "spec.txt"
                if spec.is_file() and spec.read_text() == text:
                    return directory
        return None

    def settle(self, run: TeacherRun) -> Settled:
        """A run found by its spec text, or made alone and waited for. The audit's A0 comes from here, because
        `status` needs an identity summary before it reports anything."""
        found = self.find(run.spec.text())
        if found:
            word = "fault" if found.name.endswith(".fault") else "present"
            return Settled(run.name, found.name.removesuffix(".fault"), word, found)
        settled = self._run_all([run], workers=1)
        return settled[run.name]

    def ensure(self, runs: list[TeacherRun], identity: Path, workers: int) -> dict[str, Settled]:
        """Every run present, stale or faulted, `workers` drivers at a time, the longest chain first. Refuses before
        anything starts: a spec the driver calls invalid, a partial a live driver holds, a settled run of another
        source commit, and a disk without room. A settled run never starts again. A second call with everything
        settled runs one `status` and starts nothing. A driver that fails stops new starts; the others finish."""
        with_specs = [run for run in runs if run.spec]
        paths = {run.name: self._spec_file(run) for run in with_specs}
        census = self._status(paths, identity)
        settled: dict[str, Settled] = {}
        for run in with_specs:
            word, spec_id, held = census[run.name]
            self.ids[run.name] = spec_id
            if word == "invalid":
                raise TeacherError(f"{run.name}: the driver calls its spec invalid: {spec_id}")
            if held:
                raise TeacherError(f"{run.name}: a live driver holds its partial directory")
            if word in ("present", "stale", "fault"):
                directory = self.out / (spec_id + (".fault" if word == "fault" else ""))
                self._check_commit(run.name, directory)
                settled[run.name] = Settled(run.name, spec_id, word, directory)
        pending = [run for run in runs if run.name not in settled]
        made = self._run_all(pending, workers, settled, identity)
        settled.update(made)
        return {run.name: settled[run.name] for run in runs if run.name in settled}

    def _check_commit(self, name: str, directory: Path) -> None:
        for line in (directory / "summary.txt").read_text().splitlines():
            if line.startswith("source_commit "):
                found = line.split(" ", 1)[1]
                if found != self.source_commit:
                    raise TeacherError(f"{name}: a settled run of another source commit, {found}; use another DATA")
                return
        raise TeacherError(f"{name}: a settled run with no source commit")

    def _spec_file(self, run: TeacherRun) -> Path:
        directory = self.out / "specs"
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / f"{run.name}.txt"
        text = run.spec.text()
        if path.exists() and path.read_text() != text:
            raise TeacherError(f"{run.name}: {path} holds another spec; one DATA holds one plan")
        if not path.exists():
            temporary = path.with_suffix(".tmp")
            temporary.write_text(text)
            os.replace(temporary, path)
        return path

    def _status(self, paths: dict[str, Path], identity: Path) -> dict[str, tuple[str, str, bool]]:
        if not paths:
            return {}
        command = [*self.driver, "status", "--out", str(self.out), "--identity", str(identity)]
        done = subprocess.run(command + [str(path) for path in paths.values()], capture_output=True, text=True)
        if done.returncode not in (0, 2):
            raise TeacherError(f"gp-driver status exited {done.returncode}: {done.stderr.strip()}")
        by_path = {str(path): name for name, path in paths.items()}
        found: dict[str, tuple[str, str, bool]] = {}
        for line in done.stdout.splitlines():
            if line.startswith("invalid "):
                path, _, reason = line[len("invalid ") :].partition(": ")
                found[by_path[path]] = ("invalid", reason, False)
                continue
            word, spec_id, rest = line.split(" ", 2)
            held = rest.endswith(" held")
            path = rest.removesuffix(" held")
            found[by_path[path]] = (word, spec_id, held)
        if set(found) != set(paths):
            raise TeacherError("gp-driver status did not report every spec")
        return found

    def _room(self, running: list[_Child], run: TeacherRun) -> str | None:
        need = self.floor_bytes + sum(expected_link_bytes(child.run.spec or run.spec) for child in running)
        need += expected_link_bytes(run.spec) if run.spec else 0
        for path in [*self.disks, self.out]:
            free = self.free_bytes(path)
            if free < need:
                return f"{path} has {free / 1e9:.1f} GB free and the next start needs {need / 1e9:.1f} GB"
        return None

    def _start(self, run: TeacherRun, spec_path: Path, spec: RunSpec, attempt: int) -> _Child:
        logs = self.out / "logs"
        logs.mkdir(parents=True, exist_ok=True)
        stdout_log, stderr_log = logs / f"{run.name}.{attempt}.out", logs / f"{run.name}.{attempt}.err"
        closed = spec.closed_loop is not None
        verb = "interact" if closed else "run"
        command = [*self.driver, verb, str(spec_path), "--out", str(self.out)]
        command += ["--memory-cap-gb", str(self.memory_cap_gb), "--source-commit", self.source_commit]
        with open(stdout_log, "wb") as stdout, open(stderr_log, "wb") as stderr:
            process = subprocess.Popen(
                command,
                stdin=subprocess.PIPE if closed else subprocess.DEVNULL,
                stdout=subprocess.PIPE if closed else stdout,
                stderr=stderr,
                preexec_fn=_die_with_parent(),
                close_fds=True,
            )
        rule = self.rule_for(spec) if closed else None
        return _Child(run, spec_path, process, (stdout_log, stderr_log), rule)

    def _answer(self, child: _Child, line: str) -> str:
        beat, earliest = parse_beat(line)
        event = child.rule.react(beat, earliest)
        if event is None:
            return "none"
        if not earliest <= event.at_ms <= child.end_ms or event.at_ms in child.taken:
            raise TeacherError(f"{child.run.name}: the rule answered {event.line()!r}, which the driver would refuse")
        child.taken.add(event.at_ms)
        return event.line()

    def _converse(self, child: _Child, selector: selectors.BaseSelector) -> None:
        data = os.read(child.process.stdout.fileno(), 65536)
        if not data:
            selector.unregister(child.process.stdout)
            child.process.stdout.close()
            return
        child.buffer += data
        while b"\n" in child.buffer:
            line, child.buffer = child.buffer.split(b"\n", 1)
            try:
                answer = self._answer(child, line.decode())
            except (TeacherError, rules.RuleError, ValueError, KeyError) as error:
                child.refused = str(error)
                child.process.kill()
                selector.unregister(child.process.stdout)
                return
            child.process.stdin.write(answer.encode() + b"\n")
            child.process.stdin.flush()

    def _outcome(self, child: _Child, code: int) -> Settled:
        stdout_log, stderr_log = child.logs
        said = (stderr_log if child.run.spec is None or child.rule else stdout_log).read_text().strip().splitlines()
        if child.refused:
            raise TeacherError(f"{child.run.name}: stopped: {child.refused}")
        if code not in (0, 3) or not said:
            raise TeacherError(f"{child.run.name}: gp-driver exited {code}: {stderr_log.read_text().strip()}")
        word, spec_id, path = said[-1].split(" ", 2)
        if (word, code) not in (("complete", 0), ("present", 0), ("fault", 3)):
            raise TeacherError(f"{child.run.name}: gp-driver said {said[-1]!r} and exited {code}")
        return Settled(child.run.name, spec_id, word, Path(path))

    def _run_all(
        self,
        pending: list[TeacherRun],
        workers: int,
        settled: dict[str, Settled] | None = None,
        identity: Path | None = None,
    ) -> dict[str, Settled]:
        settled = dict(settled or {})
        names = {run.name for run in pending}
        replays_of: dict[str, list[TeacherRun]] = {}
        for run in pending:
            if run.replays:
                replays_of.setdefault(run.replays, []).append(run)

        def chain(run: TeacherRun) -> float:
            own = expected_wall_s(run.spec)
            return own + own * len(replays_of.get(run.name, []))

        ready = sorted((run for run in pending if run.spec), key=chain, reverse=True)
        waiting = [run for run in pending if run.replays]
        for run in list(waiting):
            if run.replays in settled:
                waiting.remove(run)
                ready.append(self._replay(run, settled[run.replays]))
        made: dict[str, Settled] = {}
        running: list[_Child] = []
        selector = selectors.DefaultSelector()
        stop: str | None = None
        attempt = int(time.time())
        self._progress(pending, made, settled, running, ready, waiting)
        while (ready and not stop) or running:
            while ready and not stop and len(running) < workers:
                run = ready[0]
                spec = run.spec
                blocked = self._room(running, run)
                if blocked:
                    stop = f"free disk: {blocked}"
                    break
                ready.pop(0)
                path = self._spec_path(run, settled)
                child = self._start(run, path, spec, attempt)
                self.started.append(run.name)
                running.append(child)
                if child.rule is not None:
                    selector.register(child.process.stdout, selectors.EVENT_READ, child)
            for key, _ in selector.select(timeout=0.2) if selector.get_map() else []:
                self._converse(key.data, selector)
            if not selector.get_map():
                time.sleep(0.02)
            for child in list(running):
                code = child.process.poll()
                if code is None:
                    continue
                running.remove(child)
                if child.process.stdin:
                    child.process.stdin.close()
                try:
                    outcome = self._outcome(child, code)
                except TeacherError as error:
                    stop = stop or str(error)
                    continue
                made[child.run.name] = outcome
                settled[child.run.name] = outcome
                for replay in [r for r in waiting if r.replays == child.run.name]:
                    waiting.remove(replay)
                    ready.insert(0, self._replay(replay, outcome))
            self._progress(pending, made, settled, running, ready, waiting)
        selector.close()
        if stop:
            raise TeacherError(stop)
        return made

    def _replay(self, run: TeacherRun, source: Settled) -> TeacherRun:
        realized = source.directory / "realized.txt"
        if not realized.is_file():
            raise TeacherError(f"{run.name}: {run.replays} settled with no realized.txt")
        from spec import Plan

        text = realized.read_text()
        events = tuple(
            PostureEvent(int(words[2]), _setting(int(words[4]), words[6]))
            for words in (line.split(" ") for line in text.splitlines() if line.startswith("event "))
        )
        lines = text.splitlines()
        core = lines[1][len("core ") :]
        end_ms = int(lines[2].split(" ")[1])
        spec = RunSpec(core, end_ms, events, None, "none")
        if spec.text() != text:
            raise TeacherError(f"{run.name}: {realized} is not a spec this study writes")
        del Plan
        return TeacherRun(run.name, spec, run.replays)

    def _spec_path(self, run: TeacherRun, settled: dict[str, Settled]) -> Path:
        if run.replays:
            return settled[run.replays].directory / "realized.txt"
        return self._spec_file(run)

    def _progress(self, pending, made, settled, running, ready, waiting) -> None:
        total = len(pending) + len(settled) - len(made)
        lines = [
            f"gp-audit: {total} runs: {len(settled)} settled, {len(running)} running, {len(ready)} queued, "
            f"{len(waiting)} waiting (written {time.strftime('%Y-%m-%d %H:%M:%S UTC', time.gmtime())})"
        ]
        now = time.monotonic()
        lines += [f"running {child.run.name} for {now - child.started:.0f} s" for child in running]
        lines += [f"queued {run.name}" for run in ready]
        lines += [f"waiting {run.name} for {run.replays}" for run in waiting]
        lines += [f"{s.word} {s.name} {s.spec_id}" for s in settled.values()]
        self.out.mkdir(parents=True, exist_ok=True)
        temporary = self.out / "progress.txt.tmp"
        temporary.write_text("\n".join(lines) + "\n")
        os.replace(temporary, self.out / "progress.txt")


def _setting(trunk: int, legs: str):
    from spec import Legs, Setting

    return Setting(trunk, Legs(legs))
