"""Closed-loop rules: what a run spec's closed-loop line means, and the rule that answers each beat.

The line names the rule's kind, its version and every parameter, so the spec id covers all of them, and a change to
the rule's behavior bumps the version. A rule reads beat readings, the run's end and its events' stamps, and
nothing else, and it never answers what the driver would refuse: a stamp after the end or one an event holds.
These are synthetic rules for an experiment, never care instructions.
"""
from __future__ import annotations

import math
import re
from dataclasses import dataclass, field

from spec import SETTINGS, PostureEvent, RunSpec

KIND = "audit-pressure"
VERSION = 1


class RuleError(Exception):
    pass


@dataclass(frozen=True)
class AuditPressure:
    """Ticket 04's B7 rule, which tests the closed-loop mechanism. After `after_ms`, each beat's arterial pressure
    is compared with the mean of the `baseline_beats` beats that ended last before it: lower sets stand, higher
    sets flat, `delay_ms` after the beat's end, while the rule's own last action is at least `refractory_ms` old."""

    after_ms: int
    baseline_beats: int
    refractory_ms: int
    delay_ms: int

    def text(self) -> str:
        return (
            f"{KIND} {VERSION} after-ms {self.after_ms} baseline-beats {self.baseline_beats} "
            f"refractory-ms {self.refractory_ms} delay-ms {self.delay_ms}"
        )


def parse(text: str) -> AuditPressure:
    shape = rf"{KIND} {VERSION} after-ms (\d+) baseline-beats (\d+) refractory-ms (\d+) delay-ms (\d+)"
    found = re.fullmatch(shape, text)
    if not found:
        raise RuleError(f"no rule reads {text!r}")
    rule = AuditPressure(*(int(group) for group in found.groups()))
    if rule.text() != text or rule.baseline_beats < 1 or rule.delay_ms < 1:
        raise RuleError(f"{text!r} is not the rule's own text")
    return rule


@dataclass
class AuditPressureRule:
    spec: AuditPressure
    end_ms: int
    taken: set[int]
    before: list[float] = field(default_factory=list)
    baseline: float | None = None
    last_ms: int | None = None

    def react(self, beat, earliest_ms: int) -> PostureEvent | None:
        end_ns = round(beat.end_s * 1e9)
        after_ns = self.spec.after_ms * 1_000_000
        if end_ns < after_ns:
            self.before = (self.before + [beat.arterial_pressure])[-self.spec.baseline_beats :]
            return None
        if self.baseline is None:
            if len(self.before) < self.spec.baseline_beats:
                raise RuleError(f"{len(self.before)} beats before {self.spec.after_ms} ms, not {self.spec.baseline_beats}")
            self.baseline = math.fsum(self.before) / len(self.before)
        if end_ns == after_ns:
            return None
        if self.last_ms is not None and end_ns - self.last_ms * 1_000_000 < self.spec.refractory_ms * 1_000_000:
            return None
        pressure = beat.arterial_pressure
        if pressure < self.baseline:
            setting = SETTINGS["stand"]
        elif pressure > self.baseline:
            setting = SETTINGS["flat"]
        else:
            return None
        at = earliest_ms + self.spec.delay_ms
        if at > self.end_ms or at in self.taken:
            return None
        self.last_ms = at
        self.taken.add(at)
        return PostureEvent(at, setting)


def start(run: RunSpec) -> AuditPressureRule:
    """The rule a closed-loop spec names, bound to its end and to the stamps its events hold."""
    if run.closed_loop is None:
        raise RuleError("an open-loop spec has no rule")
    return AuditPressureRule(parse(run.closed_loop), run.end_ms, {event.at_ms for event in run.events})
