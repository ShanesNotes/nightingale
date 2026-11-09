"""The four response features, defined once: ticket 04's informativeness gate reads them, and ticket 09's metrics and
ticket 12's order test use them unchanged."""
from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Literal, Protocol, Sequence

from capture import BeatReading

Field = Literal["heart_rate_per_s", "arterial_pressure", "stroke", "filtration"]
EARLY_S = 30.0
LATE_FROM_S = 240.0
LATE_TO_S = 300.0
BASELINE_S = 30.0


class FeatureError(Exception):
    pass


def within(beats: Sequence[BeatReading], start_s: float, end_s: float) -> list[BeatReading]:
    return [beat for beat in beats if beat.start_s >= start_s and beat.end_s <= end_s]


def mean(values: Sequence[float], what: str) -> float:
    if not values:
        raise FeatureError(f"no beat in {what}")
    return math.fsum(values) / len(values)


class Baseline(Protocol):
    def level(self, field: Field, start_s: float, end_s: float) -> float: ...


@dataclass(frozen=True)
class BeforeChange:
    beats: Sequence[BeatReading]
    change_s: float

    def level(self, field: Field, start_s: float, end_s: float) -> float:
        window = within(self.beats, self.change_s - BASELINE_S, self.change_s)
        return mean([getattr(beat, field) for beat in window], f"the {BASELINE_S:g} s before {self.change_s:g} s")


@dataclass(frozen=True)
class Counterfactual:
    beats: Sequence[BeatReading]

    def level(self, field: Field, start_s: float, end_s: float) -> float:
        return held_mean(self.beats, field, start_s, end_s)


def held_mean(beats: Sequence[BeatReading], field: str, start_s: float, end_s: float) -> float:
    total, covered = [], 0.0
    for beat in beats:
        overlap = min(end_s, beat.end_s) - max(start_s, beat.start_s)
        if overlap > 0:
            total.append(overlap * getattr(beat, field))
            covered += overlap
    if not total or not math.isclose(covered, end_s - start_s, rel_tol=1e-9, abs_tol=1e-9):
        raise FeatureError(f"beats cover {covered:g} s of {start_s:g} to {end_s:g} s")
    return math.fsum(total) / (end_s - start_s)


@dataclass(frozen=True)
class Feature:
    value: float
    beats_averaged: int


@dataclass(frozen=True)
class Features:
    rate_rise: Feature
    pressure_fall: Feature
    late_rate: Feature
    late_pressure: Feature

    def items(self) -> list[tuple[str, Feature]]:
        return [
            ("rate rise", self.rate_rise),
            ("pressure fall", self.pressure_fall),
            ("late rate", self.late_rate),
            ("late pressure", self.late_pressure),
        ]


def _change(beat: BeatReading, field: Field, baseline: Baseline) -> float:
    return getattr(beat, field) - baseline.level(field, beat.start_s, beat.end_s)


def early_features(beats: Sequence[BeatReading], change_s: float, baseline: Baseline) -> tuple[Feature, Feature]:
    early = within(beats, change_s, change_s + EARLY_S)
    if len(early) < 3:
        raise FeatureError(f"{len(early)} beats in the {EARLY_S:g} s after {change_s:g} s, fewer than three")
    rise = max(_change(beat, "heart_rate_per_s", baseline) for beat in early)
    falls = [_change(beat, "arterial_pressure", baseline) for beat in early]
    nadir = min(math.fsum(falls[at : at + 3]) / 3 for at in range(len(falls) - 2))
    return Feature(rise, 1), Feature(-nadir, 3)


def response_features(beats: Sequence[BeatReading], change_s: float, baseline: Baseline) -> Features:
    rise, fall = early_features(beats, change_s, baseline)
    late = within(beats, change_s + LATE_FROM_S, change_s + LATE_TO_S)
    late_what = f"{LATE_FROM_S:g} to {LATE_TO_S:g} s after {change_s:g} s"
    return Features(
        rise,
        fall,
        Feature(mean([_change(beat, "heart_rate_per_s", baseline) for beat in late], late_what), len(late)),
        Feature(mean([_change(beat, "arterial_pressure", baseline) for beat in late], late_what), len(late)),
    )


def time_to_fraction(beats: Sequence[BeatReading], field: Field, change_s: float, fraction: float = 0.63) -> float | None:
    before = mean([getattr(b, field) for b in within(beats, change_s - BASELINE_S, change_s)], "the baseline")
    after = mean([getattr(b, field) for b in within(beats, change_s + LATE_FROM_S, change_s + LATE_TO_S)], "the late window")
    if after == before:
        return None
    target = before + fraction * (after - before)
    for beat in within(beats, change_s, change_s + LATE_TO_S):
        value = getattr(beat, field)
        if (value - target) * (after - before) >= 0:
            return beat.end_s - change_s
    return None


def three_beat_means(beats: Sequence[BeatReading], field: Field) -> list[tuple[float, float]]:
    values = [getattr(beat, field) for beat in beats]
    return [(beats[at].start_s, math.fsum(values[at : at + 3]) / 3) for at in range(len(beats) - 2)]
