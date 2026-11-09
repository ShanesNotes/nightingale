"""Ticket 04's teacher audit: `make` runs the audit plan through the teacher, and `report` turns the settled runs into
the ticket's results template with every verdict.

    gp.py audit --data DATA --source-commit C --workers 3 --disk /mnt/c    # safe to repeat
    gp.py audit --data DATA --source-commit C --report                     # offline: reads DATA/audit, starts nothing

This is the only module that knows the ticket's rows and criteria. The report finds each planned run by its spec text
(B8's is B7's realized.txt), so it computes no spec id, and its bytes depend only on DATA/audit and this tree.
"""
from __future__ import annotations

import argparse
import fcntl
import hashlib
import math
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tomllib
from dataclasses import dataclass, field
from decimal import Decimal, localcontext
from pathlib import Path
from typing import Callable

import features
import rules
from capture import CaptureError, RunData
from spec import Plan
from teacher import Teacher, TeacherError, TeacherRun

ROOT = Path(__file__).resolve().parents[2]
A0_PACKET = "6d709479f0b103cadc1972febf6437a2dde172e19242eb76908b9dabde5bb752"
A0_SEAL = "58233f790617a4f1bf5122b47fc44efaeface2ebb7d4c3652567bb44ba176ecf"
A0_HEAD_NS = 1_000_079_707
FENCE_NS = 1_000_000_000

TRAFFIC = "algebraic/NS/sympathetic traffic/body"
RATE = "algebraic/CV/rate/body"
LEG_VEINS = "amount/leg veins/Lumen/Water"
LEG_INTERSTITIUM = "amount/leg bed/Interstitium/Water"
ARCH = "algebraic/CV/aortic_arch_stretch/body"
SINUS = "algebraic/CV/carotid_sinus_stretch/body"
PRESSURE = "algebraic/CV/blood pressure/systemic arteries"
SKIN_SUFFIX = "/Water/skin water"
# crates/ng-validate/src/slice.rs:787 sets ms.skin-water; A0's packet pin fails if it moves.
SKIN_BASAL_PER_S = 0.01
STIR = "MS/stir"

TRAFFIC_RATIO = 1.2
LEG_WATER_MARGIN = 10.0
STRETCH_GAP = 0.105
STRETCH_WITHIN = 1e-9
CLOSURE_ULPS = 64
BEAT_GAP = 1e-3
RATE_ERROR_PER_S = 0.02
PRESSURE_ERROR = 2.0
FLOOR_TIMES = 3.0
FEATURES_NEEDED = 2
SLOW_FROM_S = 60
SLOW_TO_S = 1800
TAUS = 161
RESIDUAL_SHARE = 0.05
TAU_SPAN = 2.0
B_OVER_VARIATION = 20.0
CORE_FLAG_SHARE = 0.02
THRESHOLD_AFTER_S = 60.0
FIRST_FREE_BYTES = 40 * 10**9
START_FREE_BYTES = 10 * 10**9
MEMORY_BUDGET = 24 * 10**9
WORKERS_AT_MOST = 4
PLAN_WALL_S = 36 * 3600
# Tickets 06, 09 and 12: 32 tuning runs, 44 validation cases, 104 held-out cases and 160 replays with R's 40;
# 14 training histories each take 2 k schedules.
FIXED_RUNS = 32 + 44 + 104 + 160
TRAINING_HISTORIES = 14
SCHEDULES_AT_MOST = 8
CHANNEL = Path("rust-toolchain.toml")


@dataclass(frozen=True)
class Check:
    measured: str
    verdict: str


@dataclass(frozen=True)
class Extreme:
    value: float
    where: str


@dataclass(frozen=True)
class Fit:
    tau: float
    a: float
    b: float
    rms: float
    span: float
    resolved: bool
    note: str


def short(digest: str) -> str:
    return f"{digest[:8]}..{digest[-4:]}"


def num(value: float | None) -> str:
    if value is None:
        return "none"
    if isinstance(value, float) and not math.isfinite(value):
        return str(value)
    text = f"{value:.6g}"
    return "0" if text == "-0" else text


def ms(ns: int) -> str:
    return f"{ns / 1e6:.6f}"


def verdict(ok: bool) -> str:
    return "pass" if ok else "fail"


def log_of(seals) -> list[tuple[int, int, str]]:
    return [(entry.revision, entry.end_ns, entry.seal) for entry in seals]


def a0_check(run: RunData) -> Check:
    head = next((entry for entry in run.seals if entry.end_ns >= FENCE_NS), None)
    if head is None:
        return Check("no head at or after 1 s", "fail")
    packet = run.summary.packet
    ok = packet == A0_PACKET and head.seal == A0_SEAL and head.end_ns == A0_HEAD_NS
    return Check(f"packet {short(packet)}, seal {short(head.seal)}, head {ms(head.end_ns)} ms", verdict(ok))


def branch_check(run: RunData, at_ms: int) -> Check:
    branch = run.summary.branch
    if branch is None or run.branch_seals is None:
        return Check("no branch", "fail")
    r = branch.from_revision
    log = log_of(run.seals)
    at_ns = at_ms * 1_000_000
    if r >= len(log):
        return Check(f"a branch from revision {r} past the original's end", "fail")
    first_head = log[r][1] >= at_ns and (r == 0 or log[r - 1][1] < at_ns)
    tail = log[r + 1 :]
    problems = []
    if not first_head:
        problems.append(f"revision {r} is not the first head at or after {at_ms} ms")
    if branch.first_seal != log[r][2]:
        problems.append(f"the copy's seal at revision {r} differs")
    if log_of(run.branch_seals) != tail:
        problems.append("the copy's later seals differ")
    if branch.last_seal != run.summary.last_seal or branch.end != run.summary.end:
        problems.append("the record digests at the end differ")
    if run.summary.enqueued != branch.enqueued:
        problems.append("the copy took other events")
    measured = f"equal seals from revision {r} to {log[-1][0]}" if not problems else "; ".join(problems)
    return Check(measured, verdict(not problems))


def sample_mean(run: RunData) -> float:
    total, count = 0.0, 0
    for _, _, value in run.samples(TRAFFIC):
        total += value
        count += 1
    return total / count


def reflex_check(a2: RunData, a3: RunData) -> Check:
    """The reference test's bars (crates/ng-validate/tests/arena_body.rs:523-547): the traffic mean takes one sample
    per accepted step, added in order as the test adds them; the rest are read at the run's last head."""
    flat, stood = sample_mean(a2), sample_mean(a3)
    e2, e3 = a2.edges(), a3.edges()
    last2, last3 = e2.rows - 1, e3.rows - 1
    rate2, rate3 = e2.value(RATE).at(last2), e3.value(RATE).at(last3)
    leg2, leg3 = e2.value(LEG_VEINS).at(last2), e3.value(LEG_VEINS).at(last3)
    gap = e3.value(ARCH).at(last3) - e3.value(SINUS).at(last3)
    ok = stood > TRAFFIC_RATIO * flat and rate3 > rate2 and leg3 > leg2 + LEG_WATER_MARGIN
    ok = ok and abs(gap - STRETCH_GAP) < STRETCH_WITHIN
    measured = f"traffic {num(stood)} vs {num(flat)}; rate {num(rate3)} vs {num(rate2)}; leg-vein water {num(leg3)} vs {num(leg2)}; stretch gap {gap:.12f}"
    return Check(measured, verdict(ok))


def prefix_check(b1: RunData, b2: RunData, divergence_ms: int) -> Check:
    """The logs agree for every revision through the one that ends at exactly the divergence, and differ after it."""
    one, other = log_of(b1.seals), log_of(b2.seals)
    at_ns = divergence_ms * 1_000_000
    cut = next((index for index, entry in enumerate(one) if entry[1] == at_ns), None)
    if cut is None:
        return Check(f"B1 has no revision ending at {divergence_ms} ms", "fail")
    ok = one[: cut + 1] == other[: cut + 1] and len(one) > cut + 1 and len(other) > cut + 1 and one[cut + 1] != other[cut + 1]
    return Check(f"last common revision {one[cut][0]} ends at {ms(one[cut][1])} ms", verdict(ok))


def early_check(b1: RunData, b3: RunData, divergence_ms: int) -> Check:
    """The logs agree through the last revision that ends before the divergence, and B1's next ends at exactly it."""
    one, other = log_of(b1.seals), log_of(b3.seals)
    at_ns = divergence_ms * 1_000_000
    before = max((index for index, entry in enumerate(one) if entry[1] < at_ns), default=None)
    if before is None or before + 1 >= len(one):
        return Check(f"B1 has no revision around {divergence_ms} ms", "fail")
    ok = one[: before + 1] == other[: before + 1] and one[before + 1][1] == at_ns
    return Check(f"agree through revision {one[before][0]} at {ms(one[before][1])} ms; B1's next ends at {ms(one[before + 1][1])} ms", verdict(ok))


def same_log_check(reference: RunData, others: list[RunData]) -> Check:
    expected = log_of(reference.seals)
    differing = []
    for run in others:
        if run.summary.files.get("steps.npy") == reference.summary.files.get("steps.npy"):
            continue
        found = log_of(run.seals)
        if found != expected:
            first = next((i for i, (a, b) in enumerate(zip(expected, found)) if a != b), min(len(expected), len(found)))
            differing.append(f"{run.directory.name[:8]} from revision {first}")
    measured = f"equal end to end, {len(expected) - 1} revisions" if not differing else "differ: " + ", ".join(differing)
    return Check(measured, verdict(not differing))


def closure(run: RunData, edges) -> Extreme:
    """Between consecutive edges, each conserved quantity's inventory change against its inflow less its outflow, over
    64 times 2^-52 of the four magnitudes: both inventories and the interval's inflow and outflow."""
    worst = Extreme(0.0, "none")
    for quantity in edges.ledger():
        inv, inflow, outflow = quantity.inventory, quantity.inflow, quantity.outflow
        for b in range(1, edges.rows):
            a = b - 1
            residual = abs(
                math.fsum((*inv.parts(b), *(-x for x in inv.parts(a)), *(-x for x in inflow.parts(b)), *inflow.parts(a), *outflow.parts(b), *(-x for x in outflow.parts(a))))
            )
            magnitude = abs(inv.at(a)) + abs(inv.at(b)) + abs(inflow.difference(b, a)) + abs(outflow.difference(b, a))
            bound = CLOSURE_ULPS * 2.0**-52 * magnitude
            ratio = residual / bound if bound else (math.inf if residual else 0.0)
            if ratio > worst.value:
                worst = Extreme(ratio, f"{quantity.book} at {quantity.label}, {run.directory.name[:8]} edges {a} to {b}")
    return worst


def beat_gap(run: RunData, edges) -> Extreme:
    worst = Extreme(0.0, "none")
    for j, (mean_rate, beat) in enumerate(zip(edges.beat_means(RATE), run.beats)):
        gap = abs(mean_rate - beat.heart_rate_per_s) / beat.heart_rate_per_s
        if gap > worst.value:
            worst = Extreme(gap, f"{run.directory.name[:8]} beat {j}")
    return worst


def taus() -> list[tuple[float, float]]:
    """The 161 time constants 10 * 10^(k/40) s and each one's one-second decay exp(-1/tau), from `decimal`, so no host
    library reaches a reported digit."""
    found = []
    with localcontext() as context:
        context.prec = 50
        for k in range(TAUS):
            tau = Decimal(10) * Decimal(10) ** (Decimal(k) / Decimal(40))
            found.append((float(tau), float((Decimal(-1) / tau).exp())))
    return found


def slow_fit(y: list[float], variation: float) -> Fit:
    """y on a one-second grid, fitted as a + b exp(-i/tau) for each tau by least squares. Resolved when the best fit's
    root-mean-square residual is at most 5% of abs(b), the taus within twice the best residual span less than a
    factor of 2, abs(b) is at least 20 times the numerical variation, and the best tau is inside the grid: at either
    end the grid, not the data, bounds it."""
    n = len(y)
    mean_y = math.fsum(y) / n
    results = []
    for tau, decay in taus():
        x, value = [], 1.0
        for _ in range(n):
            x.append(value)
            value *= decay
        mean_x = math.fsum(x) / n
        sxx = math.fsum((xi - mean_x) ** 2 for xi in x)
        b = math.fsum((xi - mean_x) * (yi - mean_y) for xi, yi in zip(x, y)) / sxx if sxx else 0.0
        a = mean_y - b * mean_x
        rms = math.sqrt(math.fsum((yi - a - b * xi) ** 2 for xi, yi in zip(x, y)) / n)
        results.append((rms, tau, a, b))
    best_at = min(range(len(results)), key=lambda at: (results[at][0], at))
    rms, tau, a, b = results[best_at]
    near = [found_tau for found_rms, found_tau, _, _ in results if found_rms <= 2 * rms]
    span = max(near) / min(near)
    inside = 0 < best_at < len(results) - 1
    resolved = b != 0 and rms <= RESIDUAL_SHARE * abs(b) and span < TAU_SPAN and abs(b) >= B_OVER_VARIATION * variation and inside
    note = "" if inside else "best tau at the grid's end"
    return Fit(tau, a, b, rms, span, resolved, note)


def feature_floors(c3: features.Features, g1: features.Features, late_counts: list[int]) -> list[float]:
    """Each feature's floor: the larger of its synthetic measurement error over the root of the beats it averages
    (the fewest across C1 to C6 for the late features) and its numerical variation, C3 against G1."""
    counts = [1, 3, min(late_counts), min(late_counts)]
    sigmas = [RATE_ERROR_PER_S, PRESSURE_ERROR, RATE_ERROR_PER_S, PRESSURE_ERROR]
    pairs = zip(c3.items(), g1.items(), counts, sigmas)
    return [max(sigma / math.sqrt(count), abs(one.value - other.value)) for (_, one), (_, other), count, sigma in pairs]


def histories_differ(found: list[features.Features], floors: list[float]) -> tuple[str, bool]:
    ranges = []
    for index in range(4):
        values = [run.items()[index][1].value for run in found]
        ranges.append(max(values) - min(values))
    differing = sum(spread >= FLOOR_TIMES * floor for spread, floor in zip(ranges, floors))
    measured = f"ranges {', '.join(num(r) for r in ranges)} against floors {', '.join(num(f) for f in floors)}"
    return measured, differing >= FEATURES_NEEDED


def encounter_min(fit: Fit) -> int:
    if fit.resolved and fit.tau <= 20 * 60:
        return 10
    return 15


def quintile_cuts(means: list[float]) -> tuple[float, float]:
    cuts = statistics.quantiles(means, n=5, method="inclusive")
    return cuts[0], cuts[3]


def workers_for(logical: int, peak_bytes: float, links_bytes: float, free_bytes: float) -> int:
    limits = [WORKERS_AT_MOST, logical // 2]
    if peak_bytes > 0:
        limits.append(int(MEMORY_BUDGET // peak_bytes))
    if links_bytes > 0:
        limits.append(int(free_bytes // links_bytes))
    return max(0, min(limits))


def plan_size(run_wall_s: float, workers: int) -> int:
    """The most training schedules per family per start history, at most 8, whose runs with every fixed run fit 36 h."""
    if workers < 1:
        return 0
    for k in range(SCHEDULES_AT_MOST, 0, -1):
        if (TRAINING_HISTORIES * 2 * k + FIXED_RUNS) * run_wall_s / workers <= PLAN_WALL_S:
            return k
    return 0


class Missing(Exception):
    def __init__(self, names: list[str]) -> None:
        super().__init__(", ".join(names))
        self.names = names


class Faulted(Exception):
    pass


class Runs:
    """The plan's settled runs in DATA/audit, found by spec text and opened once each."""

    def __init__(self, out: Path, plan: Plan) -> None:
        self.out = out
        self.plan = plan
        self.by_text: dict[str, Path] = {}
        for directory in sorted(out.iterdir()) if out.is_dir() else []:
            name = directory.name.removesuffix(".fault")
            spec = directory / "spec.txt"
            if len(name) == 64 and all(c in "0123456789abcdef" for c in name) and spec.is_file():
                self.by_text.setdefault(spec.read_text(), directory)
        self.opened: dict[str, RunData] = {}

    def directory(self, name: str) -> Path | None:
        run = self.plan[name]
        if run.spec is not None:
            return self.by_text.get(run.spec.text())
        source = self.directory(run.replays)
        realized = source / "realized.txt" if source else None
        return self.by_text.get(realized.read_text()) if realized and realized.is_file() else None

    def has(self, name: str) -> bool:
        return self.directory(name) is not None

    def any(self, name: str) -> RunData:
        if name not in self.opened:
            directory = self.directory(name)
            if directory is None:
                raise Missing([name])
            self.opened[name] = RunData.open(directory)
        return self.opened[name]

    def __getitem__(self, name: str) -> RunData:
        run = self.any(name)
        if run.fault:
            raise Faulted(f"{name} faulted ({run.summary.fault.kind})")
        return run

    def need(self, *names: str) -> None:
        missing = [name for name in names if not self.has(name)]
        if missing:
            raise Missing(missing)


@dataclass
class Row:
    check: str
    runs: str
    measured: str
    criterion: str
    verdict: str
    gate: str


def row(check: str, runs: str, criterion: str, gate: str, compute: Callable[[], Check]) -> Row:
    try:
        found = compute()
    except Missing as missing:
        return Row(check, runs, f"missing {missing}", criterion, "incomplete", gate)
    except Faulted as fault:
        return Row(check, runs, str(fault), criterion, "fail", gate)
    except (features.FeatureError, CaptureError) as error:
        return Row(check, runs, str(error), criterion, "fail", gate)
    return Row(check, runs, found.measured, criterion, found.verdict, gate)


@dataclass
class Evaluation:
    rows: list[Row] = field(default_factory=list)
    table: list[str] = field(default_factory=list)
    derived: str = ""
    technical: str = ""
    informative: str = ""
    histories_failed: bool = False
    retry_needed: bool = False
    incomplete: bool = False


def change_features(run: RunData, change_s: float) -> features.Features:
    return features.response_features(run.beats, change_s, features.BeforeChange(run.beats, change_s))


def held_series(run: RunData, row_name: str, grid_s: list[int]) -> list[float]:
    """One row's value at each grid second, linear between the edges around it."""
    edges = run.edges()
    limbs = edges.value(row_name)
    times = edges.times_ns
    values = [limbs.hi[at] + limbs.lo[at] for at in range(edges.rows)]
    found, at = [], 0
    for second in grid_s:
        t = second * 1_000_000_000
        while at + 1 < edges.rows and times[at + 1] <= t:
            at += 1
        if at + 1 >= edges.rows:
            if times[at] < t:
                raise CaptureError(f"{run.directory.name[:8]} ends before {second} s")
            found.append(values[at])
            continue
        if times[at] > t:
            raise CaptureError(f"{run.directory.name[:8]} starts after {second} s")
        span = times[at + 1] - times[at]
        share = (t - times[at]) / span if span else 0.0
        found.append(values[at] + share * (values[at + 1] - values[at]))
    return found


def stir_s(run: RunData) -> str:
    fired = [at for stream, _, at in run.summary.fired if stream == STIR]
    return ", ".join(f"{at / 1e9:.6f}" for at in fired) if fired else "none"


def skin_range(run: RunData, edges) -> str:
    column = run.catalogue.column_ending(SKIN_SUFFIX)
    net = edges.beat_net_extents(column)
    factors = [abs(extent) / ((edges.times_ns[j + 2] - edges.times_ns[j + 1]) / 1e9) / SKIN_BASAL_PER_S for j, extent in enumerate(net)]
    return f"{num(min(factors))} to {num(max(factors))}" if factors else "no beat"


def simulated_s(run: RunData) -> float:
    total = run.seals[-1].end_ns / 1e9
    branch = run.summary.branch
    if branch is not None and run.branch_seals:
        total += (run.branch_seals[-1].end_ns - run.seals[branch.from_revision].end_ns) / 1e9
    return total


def accepted_steps(run: RunData) -> int:
    branch = run.summary.branch
    return run.summary.number("revisions") + (branch.revisions - branch.from_revision if branch else 0)


def environment(out: Path) -> dict[str, str]:
    path = out / "environment.txt"
    found: dict[str, str] = {}
    for line in path.read_text().splitlines() if path.is_file() else []:
        key, _, value = line.rpartition(" ") if line.startswith("free_bytes ") else line.partition(" ")
        found[key] = value
    return found


def evaluate(out: Path, plan: Plan, commit: str) -> Evaluation:
    runs = Runs(out, plan)
    ev = Evaluation()
    rows = ev.rows
    tech, info, measure = "technical", "informativeness", "measurement"

    rows.append(row("A0 pinned second", "A0", "trace.rs:135-136, 147", tech, lambda: a0_check(runs["A0"])))
    rows.append(row("A1 across checkpoint", "A1", "all equal", tech, lambda: branch_check(runs["A1"], plan["A1"].spec.variant_ms())))
    rows.append(row("A2, A3 reflex", "A2, A3", "the reference test's bars", tech, lambda: reflex_check(runs["A2"], runs["A3"])))

    def late_checkpoint() -> Check:
        found = branch_check(runs["A4"], plan["A4"].spec.variant_ms())
        return Check(f"{found.measured}; resume {num(runs['A4'].summary.branch.restore_s)} s", found.verdict)

    rows.append(row("A4 late checkpoint", "A4", "all equal", tech, late_checkpoint))
    rows.append(row("A5 rewind", "A5", "all equal", tech, lambda: branch_check(runs["A5"], plan["A5"].spec.variant_ms())))

    def prefixes() -> Check:
        runs.need("B1", "B2", "B3")
        b2 = prefix_check(runs["B1"], runs["B2"], plan.divergence_ms("B1", "B2"))
        b3 = early_check(runs["B1"], runs["B3"], plan.divergence_ms("B1", "B3"))
        both = "pass" if b2.verdict == b3.verdict == "pass" else "fail"
        return Check(f"B2: {b2.measured}; B3: {b3.measured}", both)

    rows.append(row("B1/B2, B1/B3 prefixes", "B1, B2, B3", "510 000 / just before", tech, prefixes))

    def variants() -> Check:
        runs.need("B1", "B4", "B5", "B6")
        return same_log_check(runs["B1"], [runs["B4"], runs["B5"], runs["B6"]])

    rows.append(row("B4, B5, B6", "B1, B4, B5, B6", "equal", tech, variants))

    def closed_loop() -> Check:
        runs.need("B7", "B8")
        b7, b8 = runs["B7"], runs["B8"]
        found = same_log_check(b7, [b8])
        replayed = b8.spec_text == b7.realized_text
        ok = found.verdict == "pass" and replayed
        return Check(f"{found.measured}; actions {len(b7.summary.enqueued)}", verdict(ok))

    rows.append(row("B7/B8 closed loop", "B7, B8", "equal", tech, closed_loop))

    names = [run.name for run in plan.runs] + [run.name for run in plan.retry if runs.has(run.name)]

    def identities() -> Check:
        runs.need(*[run.name for run in plan.runs])
        a0 = runs.any("A0").summary
        faults, unshared = [], []
        for name in names:
            found = runs.any(name)
            if found.fault:
                faults.append(f"{name} {found.summary.fault.kind}")
            spec = plan[name].spec or plan[plan[name].replays].spec
            keys = ("model", "packet", "resolution", "run_identity", "first_seal") if spec.cap_ms == 10 else ("model",)
            if any(getattr(found.summary, key) != getattr(a0, key) for key in keys) or found.summary.source_commit != commit:
                unshared.append(name)
        ok = not faults and not unshared
        shared = "shared" if not unshared else "not shared by " + ", ".join(unshared)
        return Check(f"{len(faults)}{': ' + ', '.join(faults) if faults else ''}; {shared}", verdict(ok))

    rows.append(row("Faults, identities", "all", "0; shared", tech, identities))

    def ledgers() -> Check:
        runs.need(*[run.name for run in plan.runs])
        worst, gap = Extreme(0.0, "none"), Extreme(0.0, "none")
        for name in names:
            found = runs.any(name)
            edges = found.edges()
            found_closure, found_gap = closure(found, edges), beat_gap(found, edges)
            worst = max(worst, found_closure, key=lambda extreme: extreme.value)
            gap = max(gap, found_gap, key=lambda extreme: extreme.value)
        ok = worst.value <= 1.0 and gap.value <= BEAT_GAP
        return Check(f"max residual {num(worst.value)} of its bound ({worst.where}); max beat gap {num(gap.value)} ({gap.where})", verdict(ok))

    rows.append(row("Closure, beat identity", "all", "bound; 1e-3", tech, ledgers))
    technical = [r for r in rows if r.gate == tech]
    ev.technical = "pass" if all(r.verdict == "pass" for r in technical) else "fail"

    def histories(suffix: str) -> Check:
        names_c = [f"C{n}{suffix}" for n in range(1, 7)]
        runs.need(*names_c, f"G1{suffix}")
        change_s = plan.shared_change(names_c) / 1000
        found = [change_features(runs[name], change_s) for name in names_c]
        floors = feature_floors(found[2], change_features(runs[f"G1{suffix}"], change_s), [f.late_rate.beats_averaged for f in found])
        measured, ok = histories_differ(found, floors)
        return Check(measured, verdict(ok))

    first = row("Histories differ", "C1-C6", "two of four", info, lambda: histories(""))
    rows.append(first)
    ev.histories_failed = first.verdict == "fail"
    retry_present = all(runs.has(run.name) for run in plan.retry)
    ev.retry_needed = ev.histories_failed and ev.technical == "pass"
    second = None
    if ev.retry_needed or retry_present:
        second = row("Histories differ, 600 s", "C1-600 to C6-600", "two of four", info, lambda: histories("-600"))
        rows.append(second)

    fit_holder: list[Fit] = []

    def slow() -> Check:
        runs.need("D0", "D1", "G2")
        change_s = plan.changes("D1")[0] // 1000
        grid = list(range(change_s + SLOW_FROM_S, SLOW_TO_S + 1))
        d0, d1, g2 = (held_series(runs[name], LEG_INTERSTITIUM, grid) for name in ("D0", "D1", "G2"))
        y = [one - other for one, other in zip(d1, d0)]
        variation = max(abs(one - other) for one, other in zip(d1, g2))
        fit = slow_fit(y, variation)
        fit_holder.append(fit)
        word = "resolved" if fit.resolved else "unresolved within 30 minutes"
        note = f"; {fit.note}" if fit.note else ""
        measured = f"tau {num(fit.tau)} s, b {num(fit.b)}, residual over abs(b) {num(fit.rms / abs(fit.b)) if fit.b else 'none'}, span {num(fit.span)}{note}"
        return Check(measured, word)

    slow_row = row("Slow component", "D0, D1", "resolved or not", info, slow)
    rows.append(slow_row)
    informative = [first if second is None else second, slow_row]
    failed = [r.check for r in informative if r.verdict not in ("pass", "resolved")]
    ev.informative = "pass" if not failed else f"fail ({', '.join(failed)})"

    def tilt() -> Check:
        runs.need("F1", "F2")
        parts = []
        for name in ("F1", "F2"):
            up, back = plan.changes(name)[0] / 1000, plan.changes(name)[1] / 1000
            beats = runs[name].beats
            values = [features.time_to_fraction(beats, field, change) for field in ("stroke", "heart_rate_per_s") for change in (up, back)]
            parts.append(f"{name} stroke {num(values[0])}/{num(values[1])} s, rate {num(values[2])}/{num(values[3])} s")
        return Check("; ".join(parts), "measured")

    rows.append(row("Tilt", "F1, F2", "measured", measure, tilt))

    def cores() -> Check:
        runs.need("G3", "G4")
        g3, g4 = runs["G3"], runs["G4"]
        change_s = plan.changes("G4")[0] / 1000
        worst = {"arterial_pressure": 0.0, "heart_rate_per_s": 0.0}
        for beat in g4.beats:
            for field_name in worst:
                try:
                    other = features.held_mean(g3.beats, field_name, beat.start_s, beat.end_s)
                except features.FeatureError:
                    continue
                worst[field_name] = max(worst[field_name], abs(getattr(beat, field_name) - other))
        rise, fall = features.early_features(g4.beats, change_s, features.BeforeChange(g4.beats, change_s))
        flagged = worst["arterial_pressure"] > CORE_FLAG_SHARE * abs(fall.value) or worst["heart_rate_per_s"] > CORE_FLAG_SHARE * abs(rise.value)
        measured = f"max d pressure {num(worst['arterial_pressure'])}, max d rate {num(worst['heart_rate_per_s'])}; G4's stand: fall {num(fall.value)}, rise {num(rise.value)}"
        return Check(measured, "flagged" if flagged else "not flagged")

    rows.append(row("Cores", "G3, G4", "flag above 2%", measure, cores))

    def variation() -> Check:
        runs.need("C3", "G1", "D1", "G2")
        change_s = plan.shared_change([f"C{n}" for n in range(1, 7)]) / 1000
        c3, g1 = change_features(runs["C3"], change_s), change_features(runs["G1"], change_s)
        per = ", ".join(f"{name} {num(abs(one.value - other.value))}" for (name, one), (_, other) in zip(c3.items(), g1.items()))
        grid = list(range(plan.changes("D1")[0] // 1000 + SLOW_FROM_S, SLOW_TO_S + 1))
        d1, g2 = held_series(runs["D1"], LEG_INTERSTITIUM, grid), held_series(runs["G2"], LEG_INTERSTITIUM, grid)
        leg = max(abs(one - other) for one, other in zip(d1, g2))
        return Check(f"per feature {per}; leg interstitium {num(leg)}", "measured")

    rows.append(row("Numerical variation", "G1, G2", "measured", measure, variation))

    def ripple() -> Check:
        names_c = [f"C{n}" for n in range(1, 7)]
        runs.need(*names_c)
        change_s = plan.shared_change(names_c) / 1000
        squares, falls = [], []
        for name in names_c:
            found = runs[name]
            window_ns = dict(found.catalogue.lens)[PRESSURE]
            for collected_ns, value in found.lens_present(PRESSURE):
                start_s, end_s = (collected_ns - window_ns) / 1e9, collected_ns / 1e9
                try:
                    held = features.held_mean(found.beats, "arterial_pressure", start_s, end_s)
                except features.FeatureError:
                    continue
                squares.append((value - held) ** 2)
            falls.append(change_features(found, change_s).pressure_fall.value)
        rms = math.sqrt(math.fsum(squares) / len(squares)) if squares else None
        return Check(f"RMS {num(rms)} over {len(squares)} windows against the stand response {num(min(falls))} to {num(max(falls))}", "measured")

    rows.append(row("Ripple", "C1-C6", "measured", measure, ripple))

    def accumulated() -> Check:
        runs.need("C7", "C3")
        rate = {name: runs[name].summary.seconds("step_wall_s") / simulated_s(runs[name]) for name in ("C7", "C3")}
        return Check(f"wall per simulated s {num(rate['C7'])} against {num(rate['C3'])}", "measured")

    rows.append(row("Accumulated postures", "C7, C3", "measured", measure, accumulated))

    ev.table = per_run_table(runs, names)
    ev.derived = derive(runs, plan, ev, fit_holder[0] if fit_holder else None, out)
    ev.incomplete = any(r.verdict == "incomplete" for r in rows) or "pending" in ev.derived
    return ev


def per_run_table(runs: Runs, names: list[str]) -> list[str]:
    lines = ["| Run | simulated s | accepted steps | per simulated s | refusals | admission s | wall s | times real time | peak MB | links GB | stir s | skin factor |"]
    for name in names:
        if not runs.has(name):
            lines.append(f"| {name} | missing |")
            continue
        try:
            found = runs.any(name)
        except CaptureError as error:
            lines.append(f"| {name} | unreadable: {error} |")
            continue
        s = found.summary
        simulated, steps = simulated_s(found), accepted_steps(found)
        refusals = s.number("refusals") + (s.branch.refusals if s.branch else 0)
        wall = s.seconds("step_wall_s")
        edges = found.edges()
        cells = [
            name,
            num(simulated),
            str(steps),
            num(steps / simulated) if simulated else "none",
            str(refusals),
            num(s.seconds("admission_s")),
            num(wall),
            num(simulated / wall) if wall else "none",
            num(s.number("peak_rss_bytes") / 1e6),
            num(s.number("links_bytes") / 1e9),
            stir_s(found),
            skin_range(found, edges),
        ]
        lines.append("| " + " | ".join(cells) + " |")
    return lines


def derive(runs: Runs, plan: Plan, ev: Evaluation, fit: Fit | None, out: Path) -> str:
    if ev.technical != "pass":
        return "teaching core none: the technical gate failed and the experiment stops here"
    first = next(r for r in ev.rows if r.check == "Histories differ")
    second = next((r for r in ev.rows if r.check == "Histories differ, 600 s"), None)
    if first.verdict == "pass":
        history, suffix = "300", ""
    elif second is None or second.verdict == "incomplete":
        return "teaching core Core at a 10 ms cap; history pending the 600 s retry (run gp.py audit again)"
    else:
        history = "600" if second.verdict == "pass" else "600 (the across-history order test is removed)"
        suffix = "-600"
    encounter = encounter_min(fit) if fit else None
    tau_note = "" if fit and fit.resolved else " (slow component unresolved)"
    names_c = [f"C{n}{suffix}" for n in range(1, 7)]
    try:
        means = []
        for name in names_c:
            means += [mean for start, mean in features.three_beat_means(runs[name].beats, "arterial_pressure") if start >= THRESHOLD_AFTER_S]
        low, high = quintile_cuts(means)
    except (Missing, Faulted) as error:
        return f"teaching core Core at a 10 ms cap; history {history} s; thresholds incomplete: {error}"
    seconds = int(history.split(" ")[0]) + (encounter or 15) * 60
    env = environment(out)
    c_names = [f"C{n}" for n in range(1, 8)]
    rho = max(accepted_steps(runs[name]) / simulated_s(runs[name]) for name in c_names)
    target = rho * seconds
    scale = {name: max(1.0, target / accepted_steps(runs[name])) for name in c_names}
    peak = max(runs[name].summary.number("peak_rss_bytes") * scale[name] for name in c_names)
    links = max(runs[name].summary.number("links_bytes") * scale[name] for name in c_names)
    logical = int(env.get("logical_processors", "0"))
    free = min((int(value) for key, value in env.items() if key.startswith("free_bytes")), default=0)
    workers = workers_for(logical, peak, links, free)
    per_s = [runs[n].summary.seconds("step_wall_s") / simulated_s(runs[n]) for n in names_c]
    overhead = [runs[n].summary.seconds("run_wall_s") - runs[n].summary.seconds("step_wall_s") for n in names_c]
    run_wall = math.fsum(overhead) / len(overhead) + math.fsum(per_s) / len(per_s) * seconds
    size = plan_size(run_wall, workers)
    size_text = f"{size} training schedules per family per start history" if size else "does not fit 36 h"
    return (
        f"teaching core Core at a 10 ms cap; history {history} s; encounter {encounter} min{tau_note}; workers {workers}; "
        f"rule low {num(low)}, high {num(high)}; plan size {size_text} (a {seconds} s run takes about {num(run_wall)} s)"
    )


def manifest_digest(out: Path) -> str:
    lines = []
    for directory in sorted(out.iterdir()):
        summary = directory / "summary.txt"
        if directory.is_dir() and summary.is_file():
            lines.append(f"{hashlib.sha256(summary.read_bytes()).hexdigest()}  {directory.name}/summary.txt\n")
    return hashlib.sha256("".join(lines).encode()).hexdigest()


def render(ev: Evaluation, out: Path, commit: str) -> str:
    env = environment(out)
    unknown = "unknown"
    lines = [
        f"Input: commit C {commit}; archive sha256, crates tree and engine as designed (against 84744af): the Answer records them from the workstation",
        f"Toolchain: {env.get('rustc', unknown)}, {env.get('cargo', unknown)}, python3 {env.get('python3', unknown)}; rust-src present: {env.get('rust_src', unknown)}",
        f"Machine: the heavy-run machine; logical processors {env.get('logical_processors', unknown)}; memory limit {env.get('memory_bytes', unknown)} bytes; swap limit {env.get('swap_bytes', unknown)} bytes",
        "",
        "| Check | Runs | Measured | Criterion | Verdict |",
    ]
    lines += [f"| {r.check} | {r.runs} | {r.measured} | {r.criterion} | {r.verdict} |" for r in ev.rows]
    lines.append("")
    lines += ev.table
    lines += [
        "",
        f"Technical gate: {ev.technical}. Informativeness gate: {ev.informative}.",
        f"Derived: {ev.derived}.",
        f"Evidence: summaries manifest sha256 {manifest_digest(out)}; report.txt's sha256 is recorded beside it; the data stay on the heavy-run machine.",
    ]
    return "\n".join(lines) + "\n"


def load_plan() -> Plan:
    plan = Plan.read()
    for run in plan.all:
        if run.spec is not None and run.spec.closed_loop is not None:
            rules.parse(run.spec.closed_loop)
    return plan


def report(data: Path, commit: str) -> int:
    out = data / "audit"
    ev = evaluate(out, load_plan(), commit)
    sys.stdout.write(render(ev, out, commit))
    return 2 if ev.incomplete else 0


def toolchain_lines() -> dict[str, str]:
    channel = tomllib.loads((ROOT / CHANNEL).read_text())["toolchain"]["channel"]
    found = {}
    for tool in ("rustc", "cargo"):
        try:
            line = subprocess.run([tool, "--version"], capture_output=True, text=True, timeout=60).stdout.strip()
        except (OSError, subprocess.TimeoutExpired):
            line = ""
        if line.split(" ")[1:2] != [channel]:
            raise TeacherError(f"{tool} reports {line or 'nothing'}, not the pinned {channel}")
        found[tool] = line
    try:
        sysroot = subprocess.run(["rustc", "--print", "sysroot"], capture_output=True, text=True, timeout=60).stdout.strip()
        found["rust_src"] = "yes" if (Path(sysroot) / "lib" / "rustlib" / "src").is_dir() else "no"
    except (OSError, subprocess.TimeoutExpired):
        found["rust_src"] = "unknown"
    return found


def meminfo(key: str) -> int:
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith(key + ":"):
            return int(line.split()[1]) * 1024
    return 0


def preflight(
    out: Path,
    workers: int,
    disks: list[Path],
    logical: int,
    under_wsl: bool,
    free_bytes: Callable[[Path], int],
    tools: Callable[[], dict[str, str]],
) -> None:
    """What the audit refuses before its first long run: the worker count, a WSL run with no Windows drive named, the
    toolchain, and, on the first command only, less than 40 GB free on DATA or any named disk. A later command does
    not repeat the 40 GB check, because a WSL disk image keeps what a deleted links file used."""
    if not 1 <= workers <= min(WORKERS_AT_MOST, logical // 2):
        raise TeacherError(f"--workers {workers}: at most {min(WORKERS_AT_MOST, logical // 2)} here, half the logical processors and no more than 4")
    if under_wsl and not any(str(disk).startswith("/mnt/") for disk in disks):
        raise TeacherError("under WSL the disk image's Windows drive is the real budget; name it with --disk /mnt/c")
    found = tools()
    path = out / "environment.txt"
    if path.exists():
        return
    paths = [out, *disks]
    short_of = [p for p in paths if free_bytes(p) < FIRST_FREE_BYTES]
    if short_of:
        raise TeacherError(f"under 40 GB free on {', '.join(str(p) for p in short_of)}; free space first")
    lines = [f"rustc {found['rustc']}", f"cargo {found['cargo']}", f"python3 {platform.python_version()}", f"rust_src {found['rust_src']}"]
    lines += [f"logical_processors {logical}", f"memory_bytes {meminfo('MemTotal')}", f"swap_bytes {meminfo('SwapTotal')}"]
    lines += [f"free_bytes {p} {free_bytes(p)}" for p in paths]
    temporary = path.with_suffix(".tmp")
    temporary.write_text("\n".join(lines) + "\n")
    os.replace(temporary, path)


def make(data: Path, commit: str, workers: int, disks: list[Path]) -> int:
    out = data / "audit"
    out.mkdir(parents=True, exist_ok=True)
    plan = load_plan()
    lock = open(out / "lock", "a")
    try:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        print(f"gp.py: another audit holds {out}; see {out / 'progress.txt'}", file=sys.stderr)
        return 1
    driver = os.environ.get("GP_DRIVER", "")
    if not (driver and os.access(driver, os.X_OK)):
        print("gp.py: GP_DRIVER names no executable gp-driver", file=sys.stderr)
        return 1
    try:
        under_wsl = "microsoft" in platform.release().lower()
        free = lambda path: shutil.disk_usage(path).free
        preflight(out, workers, disks, os.cpu_count() or 1, under_wsl, free, toolchain_lines)
        teacher = Teacher([driver], out, commit, MEMORY_BUDGET // 10**9 // workers, [data, *disks], START_FREE_BYTES)
        a0 = teacher.settle(TeacherRun("A0", plan["A0"].spec, None))
        pins = a0_check(RunData.open(a0.directory))
        if a0.word == "fault" or pins.verdict != "pass":
            print(f"gp.py: A0 does not reproduce the pinned first second ({pins.measured}); the technical gate fails, so the audit stops", file=sys.stderr)
            return 1
        identity = a0.directory / "summary.txt"
        teacher.ensure([TeacherRun(r.name, r.spec, r.replays) for r in plan.runs if r.name != "A0"], identity, workers)
        ev = evaluate(out, plan, commit)
        if ev.retry_needed:
            teacher.ensure([TeacherRun(r.name, r.spec, r.replays) for r in plan.retry], identity, workers)
    except (TeacherError, CaptureError) as error:
        print(f"gp.py: {error}", file=sys.stderr)
        return 1
    print(f"gp.py: every planned run is settled in {out}; now gp.py audit --report")
    return 0


def command(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(prog="gp.py audit", description="the teacher audit (ticket 04)")
    parser.add_argument("--data", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--workers", type=int, default=3)
    parser.add_argument("--disk", type=Path, action="append", default=[])
    parser.add_argument("--report", action="store_true")
    found = parser.parse_args(argv)
    if found.report:
        return report(found.data, found.source_commit)
    return make(found.data, found.source_commit, found.workers, found.disk)
