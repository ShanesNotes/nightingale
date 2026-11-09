from __future__ import annotations

import re
from dataclasses import dataclass
from enum import Enum
from pathlib import Path

FENCE_MS = 1000
PLAN = Path(__file__).resolve().parent / "spec" / "audit-v1.txt"
CORE_WORDS = re.compile(r"ng-solver cap-ms [1-9][0-9]*|reference step-us [1-9][0-9]* floor-us [1-9][0-9]* absolute \S+ relative \S+")
VARIANT_WORDS = re.compile(r"none|observer-off|replica [0-9]+|(late lead-ms|checkpoint-resume at-ms|rewind at-ms) [1-9][0-9]*")
NAME = re.compile(r"[A-Z][0-9]+(-[0-9]+)?")


class PlanError(Exception):
    pass


class Legs(Enum):
    HORIZONTAL = "horizontal"
    SEATED = "seated"
    STANDING = "standing"


@dataclass(frozen=True)
class Setting:
    trunk_milli: int
    legs: Legs


SETTINGS = {
    "flat": Setting(0, Legs.HORIZONTAL),
    "sit": Setting(1000, Legs.SEATED),
    "stand": Setting(1000, Legs.STANDING),
    "tilt": Setting(500, Legs.SEATED),
    "head-down": Setting(-200, Legs.HORIZONTAL),
}


@dataclass(frozen=True)
class PostureEvent:
    at_ms: int
    setting: Setting

    def line(self) -> str:
        return f"event at-ms {self.at_ms} trunk {self.setting.trunk_milli} legs {self.setting.legs.value}"


@dataclass(frozen=True)
class RunSpec:
    core: str
    end_ms: int
    events: tuple[PostureEvent, ...]
    closed_loop: str | None
    variant: str

    def __post_init__(self) -> None:
        if not CORE_WORDS.fullmatch(self.core) or not VARIANT_WORDS.fullmatch(self.variant):
            raise PlanError(f"core {self.core!r} or variant {self.variant!r} is not the driver's words")
        stamps = [event.at_ms for event in self.events]
        if any(at < FENCE_MS for at in stamps) or self.end_ms < FENCE_MS:
            raise PlanError(f"a stamp before the fence at {FENCE_MS} ms")
        if stamps != sorted(set(stamps)):
            raise PlanError("event stamps that are not strictly increasing")
        if stamps and stamps[-1] > self.end_ms:
            raise PlanError(f"an event at {stamps[-1]} ms after the end at {self.end_ms} ms")
        if any(not -1000 <= event.setting.trunk_milli <= 1000 for event in self.events):
            raise PlanError("a trunk outside -1000..=1000")

    def text(self) -> str:
        lines = ["gp-run 1", f"core {self.core}", f"end-ms {self.end_ms}"]
        lines += [event.line() for event in self.events]
        if self.closed_loop is not None:
            lines.append(f"closed-loop {self.closed_loop}")
        lines.append(f"variant {self.variant}")
        return "\n".join(lines) + "\n"

    @property
    def cap_ms(self) -> int | None:
        words = self.core.split(" ")
        return int(words[2]) if words[0] == "ng-solver" else None

    def variant_ms(self) -> int | None:
        words = self.variant.split(" ")
        return int(words[2]) if len(words) == 3 else None


@dataclass(frozen=True)
class PlannedRun:
    name: str
    spec: RunSpec | None
    replays: str | None
    changes: tuple[int, ...]
    retry: bool


class _Block:
    def __init__(self, lines: list[str]) -> None:
        self.lines = lines
        self.at = 0

    def take(self, keyword: str) -> list[str] | None:
        if self.at < len(self.lines) and self.lines[self.at].split(" ")[0] == keyword:
            words = self.lines[self.at].split(" ")
            self.at += 1
            return words
        return None

    def take_prefix(self, prefix: str) -> str | None:
        if self.at < len(self.lines) and self.lines[self.at].startswith(prefix):
            rest = self.lines[self.at][len(prefix) :]
            self.at += 1
            return rest
        return None


def _number(word: str) -> int:
    if not re.fullmatch(r"0|-?[1-9][0-9]*", word):
        raise PlanError(f"{word!r} is not a whole number")
    return int(word)


def _setting(word: str) -> Setting:
    if word not in SETTINGS:
        raise PlanError(f"no setting {word!r}; the settings are {', '.join(SETTINGS)}")
    return SETTINGS[word]


def _legs(word: str) -> Legs:
    try:
        return Legs(word)
    except ValueError:
        raise PlanError(f"no legs {word!r}") from None


def _ramp(words: list[str]) -> list[PostureEvent]:
    shape = ["ramp", "at-ms", None, "every-ms", None, "events", None, "trunk", None, "to", None, "legs", None, "to", None, "from-event", None]
    if len(words) != len(shape) or any(want is not None and word != want for word, want in zip(words, shape)):
        raise PlanError(f"a ramp line {' '.join(words)!r}")
    at, every, count, first, last = (_number(words[i]) for i in (2, 4, 6, 8, 10))
    before, after, switch = _legs(words[12]), _legs(words[14]), _number(words[16])
    if count < 2 or every <= 0 or not 1 <= switch <= count:
        raise PlanError(f"a ramp of {count} events every {every} ms switching at {switch}")
    events = []
    for i in range(1, count + 1):
        step, rest = divmod((last - first) * (i - 1), count - 1)
        if rest:
            raise PlanError(f"a ramp from {first} to {last} in {count} events is not whole thousandths")
        events.append(PostureEvent(at + (i - 1) * every, Setting(first + step, before if i < switch else after)))
    return events


def _repeat(words: list[str]) -> list[PostureEvent]:
    if len(words) < 9 or words[1] != "at-ms" or words[3] != "every-ms" or words[5] != "events" or words[7] != "settings":
        raise PlanError(f"a repeat line {' '.join(words)!r}")
    at, every, count = _number(words[2]), _number(words[4]), _number(words[6])
    settings = [_setting(word) for word in words[8:]]
    if count < 1 or every <= 0:
        raise PlanError(f"a repeat of {count} events every {every} ms")
    return [PostureEvent(at + k * every, settings[k % len(settings)]) for k in range(count)]


class Plan:
    def __init__(self, runs: tuple[PlannedRun, ...]) -> None:
        self.all = runs
        self.runs = tuple(run for run in runs if not run.retry)
        self.retry = tuple(run for run in runs if run.retry)
        self.by_name = {run.name: run for run in runs}

    def __getitem__(self, name: str) -> PlannedRun:
        return self.by_name[name]

    @staticmethod
    def read(path: Path = PLAN) -> Plan:
        return Plan.parse(path.read_text())

    @staticmethod
    def parse(text: str) -> Plan:
        if not text.endswith("\n") or "\n\n\n" in text:
            raise PlanError("a plan ends in one newline and has one blank line between blocks")
        blocks = [block.split("\n") for block in text[:-1].split("\n\n")]
        head = blocks[0]
        if len(head) != 2 or head[0] != "gp-audit 1" or not head[1].startswith("core "):
            raise PlanError("a plan opens with gp-audit 1 and a default core")
        default_core = head[1][len("core ") :]
        runs: list[PlannedRun] = []
        retry = False
        for lines in blocks[1:]:
            if lines[0] == "retry if histories-differ fails":
                if retry or len(lines) != 1:
                    raise PlanError("one retry line, alone in its block")
                retry = True
                continue
            runs.append(Plan._run(lines, default_core, runs, retry))
        names = [run.name for run in runs]
        if len(set(names)) != len(names):
            raise PlanError("a run name used twice")
        texts = [run.spec.text() for run in runs if run.spec]
        if len(set(texts)) != len(texts):
            raise PlanError("two runs with one spec text would share one directory")
        return Plan(tuple(runs))

    @staticmethod
    def _run(lines: list[str], default_core: str, earlier: list[PlannedRun], retry: bool) -> PlannedRun:
        block = _Block(lines)
        name_words = block.take("run")
        if not name_words or len(name_words) != 2 or not NAME.fullmatch(name_words[1]):
            raise PlanError(f"a block that does not open with a run name: {lines[0]!r}")
        name = name_words[1]
        known = {run.name: run for run in earlier}
        replays = block.take("replays")
        if replays:
            source = known.get(replays[1]) if len(replays) == 2 else None
            if len(lines) != 2 or source is None or source.spec is None or source.spec.closed_loop is None:
                raise PlanError(f"{name}: a replay names one earlier closed loop and nothing else")
            return PlannedRun(name, None, source.name, (), retry)
        core = block.take_prefix("core ") or default_core
        end = block.take("end-ms")
        if not end or len(end) != 2:
            raise PlanError(f"{name}: no end-ms line")
        events: list[PostureEvent] = []
        changes: list[int] = []
        inherited = block.take("events")
        if inherited:
            events = Plan._inherit(name, inherited, known)
            changes = [event.at_ms for event in events]
        while True:
            words = block.take("event")
            if words:
                if len(words) != 4 or words[1] != "at-ms":
                    raise PlanError(f"{name}: an event line {' '.join(words)!r}")
                event = PostureEvent(_number(words[2]), _setting(words[3]))
                events.append(event)
                changes.append(event.at_ms)
                continue
            words = block.take("ramp") or block.take("repeat")
            if words:
                made = _ramp(words) if words[0] == "ramp" else _repeat(words)
                events += made
                changes.append(made[0].at_ms)
                continue
            break
        closed_loop = block.take_prefix("closed-loop ")
        variant = block.take_prefix("variant ") or "none"
        if block.at != len(lines):
            raise PlanError(f"{name}: {lines[block.at]!r} out of place")
        events.sort(key=lambda event: event.at_ms)
        spec = RunSpec(core, _number(end[1]), tuple(events), closed_loop, variant)
        return PlannedRun(name, spec, None, tuple(sorted(set(changes))), retry)

    @staticmethod
    def _inherit(name: str, words: list[str], known: dict[str, PlannedRun]) -> list[PostureEvent]:
        if len(words) < 3 or words[1] != "of" or words[2] not in known or known[words[2]].spec is None:
            raise PlanError(f"{name}: events of a run the plan has not named yet")
        events = list(known[words[2]].spec.events)
        options = words[3:]
        if len(options) % 2 or any(key not in ("before-ms", "except-at-ms") for key in options[::2]):
            raise PlanError(f"{name}: {' '.join(words)!r}")
        for key, value in zip(options[::2], options[1::2]):
            at = _number(value)
            if key == "before-ms":
                events = [event for event in events if event.at_ms < at]
            else:
                kept = [event for event in events if event.at_ms != at]
                if len(kept) != len(events) - 1:
                    raise PlanError(f"{name}: no event of {words[2]} at {at} ms to leave out")
                events = kept
        return events

    def divergence_ms(self, one: str, other: str) -> int:
        first, second = self[one].spec.events, self[other].spec.events
        for a, b in zip(first, second):
            if a != b:
                return min(a.at_ms, b.at_ms)
        if len(first) == len(second):
            raise PlanError(f"{one} and {other} hold the same events")
        return (first if len(first) > len(second) else second)[min(len(first), len(second))].at_ms

    def shared_change(self, names: list[str]) -> int:
        common = set(self[names[0]].spec.events)
        for name in names[1:]:
            common &= set(self[name].spec.events)
        if not common:
            raise PlanError(f"{', '.join(names)} share no event")
        return min(event.at_ms for event in common)

    def changes(self, name: str) -> list[int]:
        return list(self[name].changes)
