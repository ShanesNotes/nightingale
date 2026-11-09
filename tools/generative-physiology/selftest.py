"""The study's tests: synthetic run directories, a stand-in driver and every check, in seconds, with no engine.

`gp.py selftest` runs them; `--real GP_DRIVER` adds a short pass through a built driver. Every value here is
synthetic. A fixture writes the driver's files with its own byte layout, so a reader mistake cannot cancel out.
"""
from __future__ import annotations

import hashlib
import math
import re
import pathlib
import struct
import sys
import tempfile
import unittest
from dataclasses import dataclass, field

ANCHOR_NS = 24_900_000_000_000
SECOND = 1_000_000_000
REAL_DRIVER: pathlib.Path | None = None


def npy(descr: str, rows: list[list], columns: int) -> bytes:
    text = "{'descr': '%s', 'fortran_order': False, 'shape': (%d, %d), }" % (descr, len(rows), columns)
    header = b"\x93NUMPY\x01\x00" + struct.pack("<H", 118) + text.ljust(117).encode() + b"\n"
    code = "Q" if descr == "<u8" else "d"
    cells = [cell for row in rows for cell in row]
    assert all(len(row) == columns for row in rows)
    return header + struct.pack("<%d%s" % (len(cells), code), *cells)


def bits(value: float) -> int:
    return struct.unpack("<Q", struct.pack("<d", value))[0]


def seal(label: str) -> str:
    return hashlib.sha256(label.encode()).hexdigest()


def words(hex_seal: str) -> list[int]:
    raw = bytes.fromhex(hex_seal)
    return [int.from_bytes(raw[at : at + 8], "big") for at in range(0, 32, 8)]


def rewrite(directory: pathlib.Path, name: str, data: bytes) -> None:
    """Replaces one file and its summary digest, so only the planted change differs."""
    (directory / name).write_bytes(data)
    summary = directory / "summary.txt"
    lines = summary.read_text().splitlines()
    digest = hashlib.sha256(data).hexdigest()
    lines = [f"file {name} {digest}" if line.startswith(f"file {name} ") else line for line in lines]
    summary.write_text("\n".join(lines) + "\n")


PRESSURE = "algebraic/CV/blood pressure/systemic arteries"
RATE = "algebraic/CV/rate/body"
TRAFFIC = "algebraic/NS/sympathetic traffic/body"
AORTIC = "CV/valves/left ventricle>systemic arteries/Water/inside"
SKIN = "MS/skin water/leg bed/Water/skin water"

ROWS = [
    "amount/toy basin/Lumen/Water",
    "amount/leg veins/Lumen/Water",
    "amount/leg bed/Interstitium/Water",
    "memory/CV/phase/body",
    RATE,
    PRESSURE,
    TRAFFIC,
    "algebraic/CV/aortic_arch_stretch/body",
    "algebraic/CV/carotid_sinus_stretch/body",
    "algebraic/RN/filtration/glomerular bed>renal tubule",
]
COLUMNS = [AORTIC, SKIN, "FL/toy exchange/toy basin/Water/inside"]
QUANTITIES = [("Water", [(0, 1), (1, 1), (2, 1)]), ("Charge", [(9, 1)])]
MODES = [("mode/CV/rhythm/body", "diastole"), ("mode/MS/stirring/body", "still")]
LENS = [(RATE, SECOND), (PRESSURE, SECOND), (ROWS[9], SECOND)]


def catalogue_text(model: str) -> str:
    lines = ["gp-catalogue 1", f"model {model}"]
    lines += [f"row {name}" for name in ROWS]
    lines += [f"column {name}" for name in COLUMNS]
    lines += [f"quantity {book} " + " ".join(f"{row}:{weight}" for row, weight in terms) for book, terms in QUANTITIES]
    lines += [f"mode {name} {start}" for name, start in MODES]
    lines += [f"lens {name} {window}" for name, window in LENS]
    lines += [f"sampled {TRAFFIC}"]
    return "\n".join(lines) + "\n"


def default_values(t_ns: int) -> dict[str, float]:
    """Synthetic rows at one instant: amounts that drift slowly, and readings held flat."""
    t = t_ns / SECOND
    return {
        ROWS[0]: 1000.0 + 0.25 * t,
        ROWS[1]: 500.0,
        ROWS[2]: 2700.0 - 0.125 * t,
        ROWS[3]: 0.0,
        RATE: 1.0,
        PRESSURE: 100.0,
        TRAFFIC: 0.5,
        ROWS[7]: 0.625,
        ROWS[8]: 0.5,
        ROWS[9]: 2.0,
    }


@dataclass
class Fixture:
    """One synthetic run directory as the driver writes it at capture version 2.

    Beat edges fall every `beat_ns` from `first_edge_ns`; steps every `step_ns`; every reading is held flat unless
    `values` says otherwise, so each integral is its value times time. The aortic column moves `stroke` per beat.
    """

    spec: str = "gp-run 1\ncore ng-solver cap-ms 10\nend-ms 6000\nvariant none\n"
    end_ns: int = 6 * SECOND
    first_edge_ns: int = SECOND
    beat_ns: int = SECOND
    step_ns: int = SECOND // 4
    stroke: float = 70.0
    values: object = default_values
    model: str = seal("synthetic model")
    packet: str = seal("synthetic packet")
    resolution: str = seal("synthetic resolution 10 ms")
    run_identity: str = seal("synthetic run identity")
    label: str = "base"
    source_commit: str = "c" * 40
    fired: str = "none"
    enqueued: str = "none"
    fault: list[str] | None = None
    variant: str = "none"
    branch_from: int | None = None
    branch_label: str | None = None
    realized: str | None = None
    summary_edit: object = None
    edge_edit: object = None
    steps_ns: list[int] | None = None
    seals: dict[int, str] | None = None
    split_ns: int | None = None
    split_label: str = "split"
    directory_name: str | None = None
    faulted: bool = False
    seal_label: str | None = None

    def step_times(self) -> list[int]:
        if self.steps_ns:
            return list(self.steps_ns)
        times = list(range(self.step_ns, self.end_ns + 1, self.step_ns))
        return times if times[-1] >= self.end_ns else times + [self.end_ns]

    def seal_at(self, revision: int, label: str | None = None) -> str:
        if self.seals and revision in self.seals:
            return self.seals[revision]
        steps = self.step_times()
        if self.split_ns is not None and revision > 0 and steps[revision - 1] > self.split_ns:
            return seal(f"{self.split_label} {revision}")
        return seal(f"{label or self.seal_label or self.label} {revision}")

    def edge_times(self) -> list[int]:
        steps = set(self.step_times())
        beats = [t for t in range(self.first_edge_ns, self.end_ns + 1, self.beat_ns) if t in steps]
        return [0] + beats + [self.step_times()[-1]]

    def edge_row(self, t_ns: int, revision: int, integral: dict[str, float]) -> list[int]:
        values = self.values(t_ns)
        row = [ANCHOR_NS + t_ns, revision, 0, 0]
        for name in ROWS:
            row += [bits(values[name]), 0]
        for name in ROWS:
            row += [bits(integral[name]), 0]
        beats_done = max(0, (t_ns - self.first_edge_ns) // self.beat_ns) if t_ns >= self.first_edge_ns else 0
        forward = {AORTIC: self.stroke * beats_done, SKIN: 0.01 * t_ns / SECOND, COLUMNS[2]: 0.0}
        for name in COLUMNS:
            row += [bits(forward[name]), 0, 0, 0]
        water = values[ROWS[0]] + values[ROWS[1]] + values[ROWS[2]]
        start = self.values(0)
        outflow = 0.25 * t_ns / SECOND
        inflow = water - (start[ROWS[0]] + start[ROWS[1]] + start[ROWS[2]]) + outflow
        row += [bits(water), 0, bits(inflow), 0, bits(outflow), 0]
        row += [bits(values[ROWS[9]]), 0, 0, 0, 0, 0]
        return row

    def write(self, out: pathlib.Path) -> pathlib.Path:
        name = self.directory_name or self.label
        directory = out / (name + (".fault" if self.faulted else ""))
        directory.mkdir(parents=True)
        steps = self.step_times()
        revision_at = {t: index + 1 for index, t in enumerate(steps)}
        revision_at[0] = 0
        files: dict[str, bytes] = {"spec.txt": self.spec.encode(), "catalogue.txt": catalogue_text(self.model).encode()}
        edge_rows, integral, before = [], {name: 0.0 for name in ROWS}, 0
        for t in self.edge_times():
            held = self.values(t)
            integral = {name: integral[name] + held[name] * (t - before) for name in ROWS}
            edge_rows.append(self.edge_row(t, revision_at[t], integral))
            before = t
        if self.edge_edit:
            self.edge_edit(edge_rows)
        width = 2 + len(MODES) + 4 * len(ROWS) + 4 * len(COLUMNS) + 6 * len(QUANTITIES)
        files["edges.npy"] = npy("<u8", edge_rows, width)
        beats = []
        beat_edges = self.edge_times()[1:-1]
        for ordinal, (start, end) in enumerate(zip(beat_edges, beat_edges[1:])):
            v = self.values(end)
            beats.append([float(ordinal), start / SECOND, end / SECOND, SECOND / (end - start), v[PRESSURE], self.stroke, v[ROWS[9]]])
        files["beats.npy"] = npy("<f8", beats, 7)
        step_rows = [[r, ANCHOR_NS + t, 0, *words(self.seal_at(r))] for r, t in ((revision_at[t], t) for t in steps)]
        files["steps.npy"] = npy("<u8", step_rows, 7)
        files["samples.npy"] = npy("<u8", [[r, ANCHOR_NS + t, bits(self.values(t)[TRAFFIC]), 0] for r, t in ((revision_at[t], t) for t in steps)], 4)
        files["applied.npy"] = npy("<u8", [], 2)
        lens_rows = []
        for t in range(SECOND, self.end_ns + 1, SECOND):
            for channel, (name, _) in enumerate(LENS):
                lens_rows.append([channel, ANCHOR_NS + t, ANCHOR_NS + t, 0, bits(self.values(t)[name]), 0])
        files["lens.npy"] = npy("<u8", lens_rows, 6)
        if self.branch_from is not None:
            label = self.branch_label or self.seal_label or self.label
            tail = [[r, ANCHOR_NS + t, 0, *words(self.seal_at(r, label))] for r, t in ((revision_at[t], t) for t in steps) if r > self.branch_from]
            files["branch-steps.npy"] = npy("<u8", tail, 7)
        if self.realized is not None:
            files["realized.txt"] = self.realized.encode()
        for name, data in files.items():
            (directory / name).write_bytes(data)
        last = len(steps)
        lines = [
            f"spec_id {seal('spec ' + self.spec)}",
            "capture_version 2",
            f"source_commit {self.source_commit}",
            f"model {self.model}",
            f"packet {self.packet}",
            f"resolution {self.resolution}",
            f"run_identity {self.run_identity}",
            f"first_seal {self.seal_at(0)}",
            f"last_seal {self.seal_at(last)}",
            f"revisions {last}",
            "refusals 0",
            f"beats {len(beats)}",
            f"enqueued {self.enqueued}",
            f"fired {self.fired}",
        ]
        lines += self.fault or ["fault none"]
        lines += ["admission_s 7.000", "step_wall_s 3.000", "run_wall_s 11.000", "peak_rss_bytes 16000000", "links_bytes 4096"]
        for name in ["spec.txt", "catalogue.txt", "edges.npy", "beats.npy", "steps.npy", "samples.npy", "applied.npy", "lens.npy", "branch-steps.npy", "realized.txt"]:
            if name in files:
                lines.append(f"file {name} {hashlib.sha256(files[name]).hexdigest()}")
        lines += ["memory_cap_gb 8", f"variant {self.variant}"]
        end_digests = ["state", "continuation", "segment", "modes", "streams", "applied", "waiting", "ledger"]
        lines += [f"end_revision {last}", f"end_time_ns {ANCHOR_NS + steps[-1]}"]
        lines += [f"end_{key} {seal((self.seal_label or self.label) + ' end ' + key)}" for key in end_digests]
        if self.branch_from is not None:
            label = self.branch_label or self.seal_label or self.label
            lines += [
                f"branch_from_revision {self.branch_from}",
                f"branch_enqueued {self.enqueued}",
                f"branch_first_seal {self.seal_at(self.branch_from)}",
                f"branch_revisions {last}",
                "branch_refusals 0",
                "branch_restore_s 0.250",
                f"branch_last_seal {self.seal_at(last, label)}",
                f"branch_end_revision {last}",
                f"branch_end_time_ns {ANCHOR_NS + steps[-1]}",
            ]
            lines += [f"branch_end_{key} {seal(label + ' end ' + key)}" for key in end_digests]
        if self.summary_edit:
            lines = self.summary_edit(lines)
        (directory / "summary.txt").write_text("\n".join(lines) + "\n")
        return directory


class CaptureReads(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.out = pathlib.Path(self.scratch.name)

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def open(self, fixture: Fixture):
        import capture

        return capture.RunData.open(fixture.write(self.out))

    def refused(self, fixture: Fixture, reason: str) -> None:
        import capture

        with self.assertRaisesRegex(capture.CaptureError, reason):
            capture.RunData.open(fixture.write(self.out))

    def test_a_seal_log_reads_each_word_big_endian_from_revision_zero(self) -> None:
        run = self.open(Fixture())
        log = run.seals
        self.assertEqual(log[0].revision, 0)
        self.assertEqual(log[0].end_ns, 0)
        self.assertEqual(log[0].seal, seal("base 0"))
        self.assertEqual(log[3].seal, seal("base 3"))
        self.assertEqual(log[3].end_ns, 3 * SECOND // 4)
        self.assertEqual(log[-1].seal, run.summary.last_seal)

    def test_a_table_whose_data_disagrees_with_its_header_is_refused(self) -> None:
        directory = Fixture().write(self.out)
        data = (directory / "steps.npy").read_bytes()
        rewrite(directory, "steps.npy", data[:128].replace(b"(24, 7)", b"(0, 7) ") + data[128:])
        import capture

        with self.assertRaisesRegex(capture.CaptureError, "steps.npy"):
            capture.RunData.open(directory)

    def test_a_summary_with_a_missing_repeated_or_unknown_key_is_refused(self) -> None:
        self.refused(Fixture(label="missing", summary_edit=lambda lines: [l for l in lines if not l.startswith("beats ")]), "beats")
        self.refused(Fixture(label="twice", summary_edit=lambda lines: lines + ["beats 5"]), "beats")
        self.refused(Fixture(label="unknown", summary_edit=lambda lines: lines[:5] + ["colour blue"] + lines[5:]), "colour")

    def test_a_file_whose_digest_differs_from_its_line_is_refused(self) -> None:
        fixture = Fixture()
        directory = fixture.write(self.out)
        data = bytearray((directory / "steps.npy").read_bytes())
        data[-1] ^= 1
        (directory / "steps.npy").write_bytes(bytes(data))
        import capture

        with self.assertRaisesRegex(capture.CaptureError, "steps.npy"):
            capture.RunData.open(directory)

    def test_another_capture_version_or_model_is_refused(self) -> None:
        self.refused(Fixture(label="v1", summary_edit=lambda lines: [l.replace("capture_version 2", "capture_version 1") for l in lines]), "capture version")

        def other_model(lines: list[str]) -> list[str]:
            return [f"model {seal('another model')}" if l.startswith("model ") else l for l in lines]

        self.refused(Fixture(label="model", summary_edit=other_model), "model")

    def test_edge_sections_follow_from_the_catalogue(self) -> None:
        run = self.open(Fixture())
        edges = run.edges()
        self.assertEqual(edges.times_ns[0], 0)
        self.assertEqual(edges.times_ns[1], SECOND)
        water = edges.value("amount/leg bed/Interstitium/Water")
        self.assertEqual(water.at(1), 2700.0 - 0.125)
        self.assertEqual(edges.modes(0), (0, 0))
        forward, reverse = edges.extent(AORTIC)
        self.assertEqual(forward.difference(3, 2), 70.0)
        self.assertEqual(reverse.at(3), 0.0)
        ledger = edges.ledger()
        self.assertEqual([quantity.book for quantity in ledger], ["Water", "Charge"])
        self.assertEqual(ledger[0].label, "amount/toy basin/Lumen/Water")

    def test_the_layout_guard_refuses_a_beat_its_edges_do_not_give(self) -> None:
        def stroke_off(rows: list) -> None:
            column = 2 + 2 + 4 * len(ROWS)
            rows[3][column] = bits(struct.unpack("<d", struct.pack("<Q", rows[3][column]))[0] + 1.0)

        self.refused(Fixture(edge_edit=stroke_off), "beat 1")

    def test_beat_means_come_from_the_chord_integral_in_nanoseconds(self) -> None:
        run = self.open(Fixture())
        self.assertEqual(run.edges().beat_means(RATE), [1.0] * len(run.beats))
        self.assertEqual(run.beats[0].heart_rate_per_s, 1.0)

    def test_samples_and_present_lens_readings_are_read_by_row_name(self) -> None:
        run = self.open(Fixture())
        samples = run.samples(TRAFFIC)
        self.assertEqual(len(samples), 24)
        self.assertEqual(samples[0], (1, SECOND // 4, 0.5))
        self.assertEqual(run.lens_present(PRESSURE)[0], (SECOND, 100.0))

    def test_a_branch_log_starts_after_its_checkpoint(self) -> None:
        run = self.open(Fixture(label="ckpt", variant="checkpoint-resume at-ms 1250", branch_from=5))
        self.assertEqual(run.branch_seals[0].revision, 6)
        self.assertEqual(run.summary.branch.first_seal, seal("ckpt 5"))
        self.assertEqual(run.summary.branch.restore_s, 0.25)


A1_TEXT = """gp-run 1
core ng-solver cap-ms 10
end-ms 60000
event at-ms 1300 trunk 1000 legs standing
variant checkpoint-resume at-ms 1250
"""

TINY_PLAN = """gp-audit 1
core ng-solver cap-ms 10

run X1
end-ms 9000
event at-ms 1000 stand
event at-ms 5000 flat

run X2
end-ms 9000
events of X1 except-at-ms 5000
event at-ms 6000 sit
"""


class SpecTexts(unittest.TestCase):
    def test_a_run_spec_renders_the_driver_canonical_text(self) -> None:
        import spec

        a1 = spec.RunSpec("ng-solver cap-ms 10", 60000, (spec.PostureEvent(1300, spec.SETTINGS["stand"]),), None, "checkpoint-resume at-ms 1250")
        self.assertEqual(a1.text(), A1_TEXT)
        head_down = spec.PostureEvent(2000, spec.SETTINGS["head-down"])
        self.assertEqual(head_down.line(), "event at-ms 2000 trunk -200 legs horizontal")

    def test_a_run_spec_refuses_the_fence_equal_stamps_and_an_event_after_the_end(self) -> None:
        import spec

        stand = spec.SETTINGS["stand"]
        for events, end in (((spec.PostureEvent(999, stand),), 5000), ((spec.PostureEvent(2000, stand), spec.PostureEvent(2000, stand)), 5000), ((spec.PostureEvent(6000, stand),), 5000)):
            with self.assertRaises(spec.PlanError):
                spec.RunSpec("ng-solver cap-ms 10", end, events, None, "none")

    def test_the_audit_plan_holds_the_ticket_runs(self) -> None:
        import spec

        plan = spec.Plan.read()
        names = [run.name for run in plan.runs]
        expected = ["A0", "A1", "A2", "A3", "A4", "A5"] + [f"B{n}" for n in range(1, 9)] + [f"C{n}" for n in range(1, 8)]
        expected += ["D0", "D1", "F1", "F2", "G1", "G2", "G3", "G4"]
        self.assertEqual(names, expected)
        self.assertEqual(plan["A1"].spec.text(), A1_TEXT)
        self.assertIn("core reference step-us 1000 floor-us 100 absolute 1 relative 0.001\n", plan["G3"].spec.text())
        ramp = [(e.at_ms, e.setting) for e in plan["B1"].spec.events if 420000 <= e.at_ms <= 430000]
        expected_ramp = [(420000 + 500 * k, spec.Setting(50 * k, spec.Legs.SEATED if k >= 10 else spec.Legs.HORIZONTAL)) for k in range(21)]
        self.assertEqual(ramp, expected_ramp, "crates/ng-validate/tests/arena_body.rs:872-884, moved to 420 s")
        self.assertEqual(len(plan["C7"].spec.events), 123)
        self.assertEqual(len(plan["F2"].spec.events), 10)
        self.assertEqual(plan.divergence_ms("B1", "B2"), 510000)
        self.assertEqual(plan.divergence_ms("B1", "B3"), 510000)
        self.assertEqual(plan.shared_change([f"C{n}" for n in range(1, 7)]), 330000)
        self.assertEqual(plan["B8"].replays, "B7")
        self.assertIsNotNone(plan["B7"].spec.closed_loop)
        self.assertEqual(plan["G1"].spec.events, plan["C3"].spec.events)
        self.assertEqual(plan["G2"].spec.events, plan["D1"].spec.events)
        self.assertEqual(plan.changes("F2"), [300000, 600000])
        self.assertEqual([run.name for run in plan.retry], [f"C{n}-600" for n in range(1, 7)] + ["G1-600"])
        self.assertEqual([e.at_ms for e in plan["C3-600"].spec.events], [1000, 480000, 630000, 930000])
        self.assertEqual(plan.shared_change([f"C{n}-600" for n in range(1, 7)]), 630000)

    def test_the_plan_refuses_what_it_cannot_say_once(self) -> None:
        import spec

        cases = {
            "an unknown setting": TINY_PLAN.replace("event at-ms 6000 sit", "event at-ms 6000 crouch"),
            "a later source": TINY_PLAN.replace("events of X1", "events of X3"),
            "an unmatched except": TINY_PLAN.replace("except-at-ms 5000", "except-at-ms 5001"),
            "a duplicate name": TINY_PLAN.replace("run X2", "run X1"),
            "a ragged ramp": TINY_PLAN.replace("event at-ms 6000 sit", "ramp at-ms 6000 every-ms 100 events 3 trunk 0 to 999 legs horizontal to seated from-event 2"),
            "one spec twice": TINY_PLAN.replace("events of X1 except-at-ms 5000\nevent at-ms 6000 sit", "events of X1"),
        }
        spec.Plan.parse(TINY_PLAN)
        for reason, text in cases.items():
            with self.subTest(reason), self.assertRaises(spec.PlanError):
                spec.Plan.parse(text)


RULE_TEXT = "audit-pressure 1 after-ms 330000 baseline-beats 3 refractory-ms 30000 delay-ms 2000"


def beat(ordinal: int, end_s: float, pressure: float):
    import capture

    return capture.BeatReading(ordinal, end_s - 1.0, end_s, 1.0, pressure, 70.0, 2.0)


def earliest(end_s: float) -> int:
    return math.ceil(round(end_s * 1e9) / 1e6)


class ClosedLoopRule(unittest.TestCase):
    def rule(self, end_ms: int = 900000, taken: set[int] | None = None):
        import rules

        return rules.AuditPressureRule(rules.parse(RULE_TEXT), end_ms, taken or {1000, 180000})

    def answers(self, rule, beats: list) -> list:
        return [rule.react(b, earliest(b.end_s)) for b in beats]

    def test_the_rule_text_reads_back_and_refuses_another_version(self) -> None:
        import rules

        self.assertEqual(rules.parse(RULE_TEXT).text(), RULE_TEXT)
        for text in (RULE_TEXT.replace("pressure 1", "pressure 2"), RULE_TEXT + " ", RULE_TEXT.replace(" delay-ms 2000", ""), "corrects 1"):
            with self.subTest(text), self.assertRaises(rules.RuleError):
                rules.parse(text)

    def test_lower_stands_higher_lies_flat_and_equal_does_nothing(self) -> None:
        import spec

        rule = self.rule()
        before = [beat(n, 326.5 + n, 100.0 - n) for n in range(3)]
        self.assertEqual(self.answers(rule, before), [None, None, None])
        lower, equal, higher = beat(3, 330.25, 98.0), beat(4, 363.0, 99.0), beat(5, 364.0, 99.5)
        self.assertEqual(rule.react(lower, earliest(330.25)), spec.PostureEvent(332250, spec.SETTINGS["stand"]))
        self.assertIsNone(rule.react(equal, earliest(363.0)), "equal to the mean of 100, 99 and 98")
        self.assertEqual(rule.react(higher, earliest(364.0)), spec.PostureEvent(366000, spec.SETTINGS["flat"]))

    def test_no_second_action_within_the_refractory_time_of_the_first_stamp(self) -> None:
        rule = self.rule()
        self.answers(rule, [beat(n, 327.0 + n, 100.0) for n in range(3)])
        first = rule.react(beat(3, 331.0, 90.0), earliest(331.0))
        self.assertEqual(first.at_ms, 333000)
        self.assertIsNone(rule.react(beat(4, 362.999, 90.0), earliest(362.999)), "29.999 s after 333 s")
        self.assertEqual(rule.react(beat(5, 363.0, 90.0), earliest(363.0)).at_ms, 365000, "30 s after it")

    def test_a_beat_ending_at_the_instant_freezes_the_baseline_and_waits(self) -> None:
        rule = self.rule()
        self.answers(rule, [beat(n, 327.0 + n, 100.0) for n in range(3)])
        self.assertIsNone(rule.react(beat(3, 330.0, 50.0), earliest(330.0)))
        self.assertEqual(rule.react(beat(4, 331.0, 99.0), earliest(331.0)).at_ms, 333000)

    def test_no_answer_after_the_end_or_on_a_taken_stamp(self) -> None:
        rule = self.rule(taken={1000, 180000, 335000})
        self.answers(rule, [beat(n, 327.0 + n, 100.0) for n in range(3)])
        self.assertIsNone(rule.react(beat(3, 333.0, 90.0), earliest(333.0)), "335000 is an event's stamp")
        self.assertEqual(rule.react(beat(4, 334.0, 90.0), earliest(334.0)).at_ms, 336000)
        self.assertIsNone(rule.react(beat(5, 898.5, 90.0), earliest(898.5)), "900500 is after the end")

    def test_too_few_baseline_beats_is_refused(self) -> None:
        import rules

        rule = self.rule()
        rule.react(beat(0, 329.0, 100.0), earliest(329.0))
        with self.assertRaises(rules.RuleError):
            rule.react(beat(1, 331.0, 90.0), earliest(331.0))


def stepped_beats(length_s: float = 1.0, offset_s: float = 0.0, end_s: float = 900.0, change_s: float = 330.0):
    """Synthetic beats: rate 1.0 per s and pressure 100 before the change; after it, rate 1.2, and pressure 90 for
    10 s, then 95."""
    import capture

    beats, start, ordinal = [], offset_s, 0
    while start + length_s <= end_s:
        end = start + length_s
        after = start >= change_s
        rate = 1.2 if after else 1.0
        pressure = (90.0 if end <= change_s + 10.0 else 95.0) if after else 100.0
        beats.append(capture.BeatReading(ordinal, start, end, rate, pressure, 70.0, 2.0))
        start, ordinal = end, ordinal + 1
    return beats


class ResponseFeatures(unittest.TestCase):
    def test_the_four_features_of_a_step_response(self) -> None:
        import features

        beats = stepped_beats()
        found = features.response_features(beats, 330.0, features.BeforeChange(beats, 330.0))
        self.assertAlmostEqual(found.rate_rise.value, 0.2, places=12)
        self.assertEqual(found.rate_rise.beats_averaged, 1)
        self.assertAlmostEqual(found.pressure_fall.value, 10.0, places=12)
        self.assertEqual(found.pressure_fall.beats_averaged, 3)
        self.assertAlmostEqual(found.late_rate.value, 0.2, places=12)
        self.assertEqual(found.late_rate.beats_averaged, 60)
        self.assertAlmostEqual(found.late_pressure.value, -5.0, places=12)

    def test_a_counterfactual_run_with_other_beat_edges_gives_the_same_features(self) -> None:
        import capture, features

        treated = stepped_beats()
        control = [capture.BeatReading(b.ordinal, b.start_s, b.end_s, 1.0, 100.0, 70.0, 2.0) for b in stepped_beats(0.75, 0.25)]
        found = features.response_features(treated, 330.0, features.Counterfactual(control))
        self.assertAlmostEqual(found.rate_rise.value, 0.2, places=12)
        self.assertAlmostEqual(found.pressure_fall.value, 10.0, places=12)
        self.assertAlmostEqual(found.late_pressure.value, -5.0, places=12)

    def test_a_window_without_beats_is_refused_never_zero(self) -> None:
        import features

        beats = [b for b in stepped_beats() if not 330.0 <= b.start_s < 360.0]
        with self.assertRaises(features.FeatureError):
            features.response_features(beats, 330.0, features.BeforeChange(beats, 330.0))

    def test_the_time_to_63_percent_of_an_exponential_approach(self) -> None:
        import capture, features

        beats = []
        for n in range(900):
            end = n + 1.0
            value = 1.0 + (0.0 if end <= 300.0 else 0.5 * (1 - math.exp(-(end - 300.0) / 10.0)))
            beats.append(capture.BeatReading(n, float(n), end, value, 100.0, 70.0, 2.0))
        self.assertEqual(features.time_to_fraction(beats, "heart_rate_per_s", 300.0), 10.0)
        flat = [capture.BeatReading(b.ordinal, b.start_s, b.end_s, 1.0, 100.0, 70.0, 2.0) for b in beats]
        self.assertIsNone(features.time_to_fraction(flat, "heart_rate_per_s", 300.0))

    def test_three_beat_means(self) -> None:
        import features

        beats = stepped_beats(end_s=6.0)
        self.assertEqual(features.three_beat_means(beats, "arterial_pressure"), [(0.0, 100.0), (1.0, 100.0), (2.0, 100.0), (3.0, 100.0)])


STAND_IN = r"""
import hashlib, json, os, pathlib, sys, time
args = sys.argv[1:]
control = json.loads(pathlib.Path(os.environ["STAND_IN_CONTROL"]).read_text())
log = pathlib.Path(os.environ["STAND_IN_LOG"])
def note(line):
    with log.open("a") as out:
        out.write(line + "\n")
def flag(name):
    return args[args.index(name) + 1]
def ident(text):
    return hashlib.sha256(b"stand-in\n" + text.encode()).hexdigest()
def label(text):
    return next(line for line in text.splitlines() if line.startswith("end-ms ")).split(" ")[1]
verb = args[0]
if verb == "status":
    out = pathlib.Path(flag("--out"))
    note("status")
    code = 0
    for path in [a for a in args[1:] if not a.startswith("--") and a not in (flag("--out"), flag("--identity"))]:
        text = pathlib.Path(path).read_text()
        if not text.startswith("gp-run 1\n"):
            print(f"invalid {path}: not a spec")
            code = 2
            continue
        i = ident(text)
        if (out / i).is_dir():
            word = "stale" if control.get(label(text), {}).get("stale") else "present"
        elif (out / (i + ".fault")).is_dir():
            word = "fault"
        elif list(out.glob(i + ".partial-*")):
            word = "partial"
        else:
            word = "missing"
        held = " held" if control.get(label(text), {}).get("held") else ""
        print(f"{word} {i} {path}{held}")
    sys.exit(code)
spec_path = pathlib.Path(args[1])
text = spec_path.read_text()
out = pathlib.Path(flag("--out"))
i = ident(text)
name = label(text)
how = control.get(name, {})
if (out / i).is_dir():
    print(f"present {i} {out / i}", file=sys.stderr if verb == "interact" else sys.stdout)
    sys.exit(0)
note(f"start {name} {os.getpid()}")
for old in out.glob(i + ".partial-*"):
    for f in old.iterdir():
        f.unlink()
    old.rmdir()
partial = out / f"{i}.partial-{os.getpid()}"
partial.mkdir(parents=True)
(partial / "spec.txt").write_text(text)
time.sleep(how.get("sleep", 0.0))
if how.get("exit"):
    note(f"end {name}")
    print("gp-driver: a synthetic io error", file=sys.stderr)
    sys.exit(how["exit"])
realized = None
if verb == "interact":
    events = [line for line in text.splitlines() if line.startswith("event ")]
    end_ms = int(name)
    for n, (end_s, pressure) in enumerate(how.get("beats", [])):
        earliest = int(-(-round(end_s * 1e9) // 1_000_000))
        line = json.dumps({"beat": n, "start_s": end_s - 1, "end_s": end_s, "heart_rate_per_s": 1, "arterial_pressure": pressure, "stroke": 70, "filtration": 2, "earliest_ms": earliest}, separators=(",", ":"))
        sys.stdout.write(line + "\n")
        sys.stdout.flush()
        answer = sys.stdin.readline()
        if not answer.endswith("\n"):
            note(f"closed {name}")
            sys.exit(1)
        answer = answer[:-1]
        note(f"answer {name} {answer}")
        if answer != "none":
            at = int(answer.split(" ")[2])
            if at < earliest or at > end_ms:
                note(f"refused {name}")
                sys.exit(3)
            events.append(answer)
    events.sort(key=lambda line: int(line.split(" ")[2]))
    lines = [l for l in text.splitlines() if not l.startswith(("event ", "closed-loop ", "variant "))]
    realized = "\n".join(lines[:3] + events + ["variant none"]) + "\n"
    (partial / "realized.txt").write_text(realized)
(partial / "summary.txt").write_text(f"spec_id {i}\ncapture_version 2\nsource_commit {flag('--source-commit')}\n")
target = out / (i + (".fault" if how.get("fault") else ""))
partial.rename(target)
note(f"end {name}")
word = "fault" if how.get("fault") else "complete"
print(f"{word} {i} {target}", file=sys.stderr if verb == "interact" else sys.stdout)
sys.exit(3 if how.get("fault") else 0)
"""


def open_spec(end_ms: int, closed_loop: str | None = None):
    import spec

    return spec.RunSpec("ng-solver cap-ms 10", end_ms, (spec.PostureEvent(1000, spec.SETTINGS["stand"]),), closed_loop, "none")


class Pool(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.scratch.name)
        self.out = self.root / "audit"
        self.out.mkdir()
        self.log = self.root / "log.txt"
        self.log.write_text("")
        self.control = self.root / "control.json"
        self.control.write_text("{}")
        (self.root / "stand-in.py").write_text(STAND_IN)
        self.identity = self.root / "identity.txt"
        self.identity.write_text("model x\n")
        self.free = 10**15
        import os

        os.environ["STAND_IN_CONTROL"] = str(self.control)
        os.environ["STAND_IN_LOG"] = str(self.log)

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def steer(self, control: dict) -> None:
        import json

        self.control.write_text(json.dumps({str(key): value for key, value in control.items()}))

    def teacher(self, rule_for=None, floor: int = 0):
        import teacher

        return teacher.Teacher(
            [sys.executable, "-I", str(self.root / "stand-in.py")],
            self.out,
            "c" * 40,
            memory_cap_gb=8,
            disks=[self.root],
            floor_bytes=floor,
            free_bytes=lambda path: self.free,
            rule_for=rule_for,
        )

    def runs(self, *specs, replay: str | None = None):
        import teacher

        made = [teacher.TeacherRun(f"R{s.end_ms}", s, None) for s in specs]
        if replay:
            made.append(teacher.TeacherRun("R-replay", None, replay))
        return made

    def notes(self) -> list[str]:
        return self.log.read_text().splitlines()

    def test_the_identity_run_is_found_by_its_spec_text_or_made_first(self) -> None:
        import teacher

        run = teacher.TeacherRun("A0", open_spec(1100), None)
        made = self.teacher().settle(run)
        self.assertEqual(made.word, "complete")
        self.assertEqual(self.notes(), ["start 1100 " + self.notes()[0].split(" ")[2], "end 1100"])
        found = self.teacher().settle(run)
        self.assertEqual(found.directory, made.directory)
        self.assertEqual(len([n for n in self.notes() if n.startswith("start")]), 1, "no second run of A0")

    def test_runs_start_longest_first_and_never_more_than_the_workers(self) -> None:
        self.steer({5000: {"sleep": 0.3}, 6000: {"sleep": 0.3}, 7000: {"sleep": 0.3}, 8000: {"sleep": 0.3}})
        teacher = self.teacher()
        settled = teacher.ensure(self.runs(open_spec(5000), open_spec(8000), open_spec(6000), open_spec(7000)), self.identity, workers=2)
        self.assertEqual(sorted(settled), ["R5000", "R6000", "R7000", "R8000"])
        self.assertEqual(teacher.started, ["R8000", "R7000", "R6000", "R5000"])
        running = peak = 0
        for note in self.notes():
            running += note.startswith("start") - note.startswith("end")
            peak = max(peak, running)
        self.assertEqual(peak, 2)

    def test_settled_runs_never_start_again_and_a_second_call_runs_only_status(self) -> None:
        self.steer({6000: {"fault": True}})
        runs = self.runs(open_spec(5000), open_spec(6000))
        first = self.teacher().ensure(runs, self.identity, workers=2)
        self.assertEqual(first["R6000"].word, "fault")
        self.log.write_text("")
        again = self.teacher().ensure(runs, self.identity, workers=2)
        self.assertEqual(self.notes(), ["status"])
        self.assertEqual({name: s.word for name, s in again.items()}, {"R5000": "present", "R6000": "fault"})

    def test_an_unheld_partial_is_redone(self) -> None:
        runs = self.runs(open_spec(5000))
        self.steer({5000: {"exit": 1}})
        import teacher

        with self.assertRaises(teacher.TeacherError):
            self.teacher().ensure(runs, self.identity, workers=1)
        self.steer({})
        settled = self.teacher().ensure(runs, self.identity, workers=1)
        self.assertEqual(settled["R5000"].word, "complete")

    def test_refusals_come_before_anything_starts(self) -> None:
        import teacher

        self.steer({5000: {"held": True}})
        with self.assertRaisesRegex(teacher.TeacherError, "holds its partial"):
            self.teacher().ensure(self.runs(open_spec(5000), open_spec(6000)), self.identity, workers=2)
        self.assertNotIn("start", " ".join(self.notes()))
        self.steer({})
        self.teacher().ensure(self.runs(open_spec(6000)), self.identity, workers=1)
        other = teacher.Teacher([sys.executable, "-I", str(self.root / "stand-in.py")], self.out, "d" * 40, 8, [self.root], 0, lambda path: self.free)
        with self.assertRaisesRegex(teacher.TeacherError, "source commit"):
            other.ensure(self.runs(open_spec(6000), open_spec(7000)), self.identity, workers=1)

    def test_a_closed_loop_answers_each_beat_and_its_replay_follows_it(self) -> None:
        loop = open_spec(900000, RULE_TEXT)
        beats = [[327.0 + n, 100.0] for n in range(3)] + [[331.0, 90.0], [340.0, 90.0], [364.0, 110.0]]
        self.steer({900000: {"beats": beats}})
        settled = self.teacher().ensure(self.runs(loop, replay="R900000"), self.identity, workers=2)
        answers = [n.split(" ", 2)[2] for n in self.notes() if n.startswith("answer")]
        self.assertEqual(answers, ["none", "none", "none", "event at-ms 333000 trunk 1000 legs standing", "none", "event at-ms 366000 trunk 0 legs horizontal"])
        realized = (settled["R900000"].directory / "realized.txt").read_text()
        self.assertIn("event at-ms 366000 trunk 0 legs horizontal\n", realized)
        self.assertEqual((settled["R-replay"].directory / "spec.txt").read_text(), realized)
        order = [n.split(" ")[1] for n in self.notes() if n.startswith(("start", "end"))]
        self.assertEqual(order[:2], ["900000", "900000"], "the replay starts after its closed loop ends")

    def test_an_answer_the_driver_would_refuse_kills_the_driver_first(self) -> None:
        import spec, teacher

        class Early:
            def react(self, beat, earliest_ms):
                return spec.PostureEvent(earliest_ms - 1, spec.SETTINGS["stand"])

        self.steer({900000: {"beats": [[2.0, 100.0]]}})
        with self.assertRaisesRegex(teacher.TeacherError, "refuse"):
            self.teacher(rule_for=lambda run: Early()).ensure(self.runs(open_spec(900000, RULE_TEXT)), self.identity, workers=1)
        self.assertNotIn("refused 900000", self.notes())
        self.assertEqual(list(self.out.glob("*.fault")), [])

    def test_a_driver_error_stops_new_starts_and_the_others_finish(self) -> None:
        import teacher

        self.steer({8000: {"exit": 1, "sleep": 0.1}, 7000: {"sleep": 0.5}})
        with self.assertRaisesRegex(teacher.TeacherError, "synthetic io error"):
            self.teacher().ensure(self.runs(open_spec(8000), open_spec(7000), open_spec(5000)), self.identity, workers=2)
        starts = [n.split(" ")[1] for n in self.notes() if n.startswith("start")]
        self.assertEqual(starts, ["8000", "7000"])
        self.assertIn("end 7000", self.notes())

    def test_a_start_waits_for_room_on_every_disk(self) -> None:
        import teacher

        self.free = 10**9
        with self.assertRaisesRegex(teacher.TeacherError, "free"):
            self.teacher(floor=10**10).ensure(self.runs(open_spec(5000)), self.identity, workers=1)
        self.assertNotIn("start", " ".join(self.notes()))

    def test_progress_names_every_run_and_its_state(self) -> None:
        self.teacher().ensure(self.runs(open_spec(5000), open_spec(6000)), self.identity, workers=1)
        progress = (self.out / "progress.txt").read_text()
        self.assertTrue(progress.startswith("gp-audit: 2 runs: 2 settled"), progress)
        self.assertIn("complete R5000 ", progress)
        self.assertIn("complete R6000 ", progress)

    def test_the_drivers_die_with_the_teacher(self) -> None:
        import os, signal, subprocess, time

        self.steer({5000: {"sleep": 30.0}})
        program = self.root / "parent.py"
        program.write_text(
            "import pathlib, sys\n"
            f"sys.path.insert(0, {str(pathlib.Path(__file__).resolve().parent)!r})\n"
            "import selftest, teacher\n"
            f"t = teacher.Teacher([sys.executable, '-I', {str(self.root / 'stand-in.py')!r}], pathlib.Path({str(self.out)!r}), 'c' * 40, 8, [], 0, lambda path: 10**15)\n"
            f"t.ensure([teacher.TeacherRun('R5000', selftest.open_spec(5000), None)], pathlib.Path({str(self.identity)!r}), 1)\n"
        )
        parent = subprocess.Popen([sys.executable, "-I", str(program)], env=os.environ.copy())
        for _ in range(100):
            starts = [n for n in self.notes() if n.startswith("start")]
            if starts:
                break
            time.sleep(0.05)
        child = int(starts[0].split(" ")[2])
        parent.send_signal(signal.SIGKILL)
        parent.wait()
        for _ in range(100):
            try:
                os.kill(child, 0)
            except ProcessLookupError:
                break
            time.sleep(0.05)
        else:
            self.fail("the driver outlived the teacher")
        self.assertEqual(len(list(self.out.glob("*.partial-*"))), 1, "a partial for the next command to redo")


def ident(label: str) -> str:
    return seal("directory " + label)


class GateRows(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.out = pathlib.Path(self.scratch.name)

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def made(self, **fields):
        import capture

        return capture.RunData.open(Fixture(**fields).write(self.out))

    def test_a0_s_pins_are_trace_rs_s(self) -> None:
        import audit

        trace = (audit.ROOT / "crates" / "ng-validate" / "tests" / "trace.rs").read_text()
        self.assertIn(f'expect!["{audit.A0_PACKET}"]', trace)
        self.assertIn(f'expect!["{audit.A0_SEAL}"]', trace)
        self.assertIn(f"{audit.A0_HEAD_NS / 1e6:.6f} ms", trace)

    def test_the_a0_row_passes_its_pins_and_fails_one_ns_late(self) -> None:
        import audit

        steps = [250_000_000, 500_000_000, 1_000_079_707, 1_100_000_000]
        good = self.made(label="a0", packet=audit.A0_PACKET, steps_ns=steps, end_ns=1_100_000_000, seals={3: audit.A0_SEAL})
        self.assertEqual(audit.a0_check(good).verdict, "pass")
        late = self.made(label="late", packet=audit.A0_PACKET, steps_ns=[250_000_000, 500_000_000, 1_000_079_708, 1_100_000_000], end_ns=1_100_000_000, seals={3: audit.A0_SEAL})
        self.assertEqual(audit.a0_check(late).verdict, "fail")

    def test_a_branch_row_compares_from_the_checkpoint_inclusive(self) -> None:
        import audit

        good = self.made(label="ckpt", branch_from=5, enqueued="0@5", variant="checkpoint-resume at-ms 1250")
        self.assertEqual(audit.branch_check(good, 1250).verdict, "pass")
        moved = self.made(label="moved", branch_from=6, enqueued="0@6", variant="checkpoint-resume at-ms 1250")
        self.assertEqual(audit.branch_check(moved, 1250).verdict, "fail", "revision 5 is the first head at or after 1.25 s")
        other = self.made(label="other", branch_from=5, enqueued="0@5", branch_label="elsewhere", variant="checkpoint-resume at-ms 1250")
        self.assertEqual(audit.branch_check(other, 1250).verdict, "fail")

    def test_the_reflex_row_takes_the_bars_of_the_reference_test(self) -> None:
        import audit

        def flat(t_ns: int) -> dict:
            return default_values(t_ns)

        def standing(ratio: float, gap: float):
            def values(t_ns: int) -> dict:
                v = default_values(t_ns)
                v.update({TRAFFIC: 0.5 * ratio, RATE: 1.1, ROWS[1]: 512.0, ROWS[7]: 0.5 + gap, ROWS[8]: 0.5})
                return v
            return values

        a2 = self.made(label="a2", values=flat)
        self.assertEqual(audit.reflex_check(a2, self.made(label="a3", values=standing(1.25, 0.105))).verdict, "pass")
        self.assertEqual(audit.reflex_check(a2, self.made(label="low", values=standing(1.19, 0.105))).verdict, "fail")
        self.assertEqual(audit.reflex_check(a2, self.made(label="gap", values=standing(1.25, 0.105 + 2e-9))).verdict, "fail")

    def test_the_prefix_rows_cut_at_the_revision_ending_at_the_divergence(self) -> None:
        import audit

        b1 = self.made(label="b1", seal_label="shared")
        b2 = self.made(label="b2", seal_label="shared", split_ns=3 * SECOND)
        self.assertEqual(audit.prefix_check(b1, b2, 3000).verdict, "pass")
        self.assertEqual(audit.prefix_check(b1, b2, 2000).verdict, "fail", "they still agree after 2 s")
        b3 = self.made(label="b3", seal_label="shared", split_ns=3 * SECOND - SECOND // 4)
        self.assertEqual(audit.early_check(b1, b3, 3000).verdict, "pass")
        self.assertEqual(audit.early_check(b1, b3, 3100).verdict, "fail", "b1's next revision ends at 3.25 s, not 3.1 s")

    def test_equal_seals_with_other_refusals_still_pass_the_same_log_row(self) -> None:
        import audit

        b1 = self.made(label="b1", seal_label="shared")
        b4 = self.made(label="b4", seal_label="shared", variant="replica 1")
        self.assertEqual(audit.same_log_check(b1, [b4]).verdict, "pass")
        b5 = self.made(label="b5", seal_label="shared", split_ns=5 * SECOND)
        self.assertEqual(audit.same_log_check(b1, [b4, b5]).verdict, "fail")

    def test_closure_holds_within_its_bound_and_names_a_planted_leak(self) -> None:
        import audit

        run = self.made(label="closed")
        worst = audit.closure(run, run.edges())
        self.assertLessEqual(worst.value, 1.0)

        def leak(rows: list) -> None:
            column = 2 + 2 + 4 * len(ROWS) + 4 * len(COLUMNS) + 1
            rows[4][column] = bits(1e-9)

        leaking = self.made(label="leak", edge_edit=leak)
        worst = audit.closure(leaking, leaking.edges())
        self.assertGreater(worst.value, 1.0)
        self.assertIn("Water", worst.where)

    def test_the_beat_identity_gap_is_relative_to_one_over_the_length(self) -> None:
        import audit

        run = self.made(label="beats")
        self.assertEqual(audit.beat_gap(run, run.edges()).value, 0.0)

        def faster(t_ns: int) -> dict:
            v = default_values(t_ns)
            v[RATE] = 1.0011
            return v

        off = self.made(label="off", values=faster)
        self.assertAlmostEqual(audit.beat_gap(off, off.edges()).value, 1.1e-3, places=9)


class SlowFit(unittest.TestCase):
    def test_an_exponential_resolves_at_its_time_constant(self) -> None:
        import audit

        tau = 10 * 10 ** (60 / 40)
        y = [3.0 + 40.0 * math.exp(-i / tau) for i in range(1441)]
        fit = audit.slow_fit(y, 0.5)
        self.assertTrue(fit.resolved, fit)
        self.assertAlmostEqual(fit.tau, tau, delta=tau * 1e-9)
        self.assertAlmostEqual(fit.b, 40.0, places=6)

    def test_a_straight_line_resolves_nothing_at_the_grid_s_end(self) -> None:
        import audit

        fit = audit.slow_fit([0.01 * i for i in range(1441)], 0.0)
        self.assertEqual(fit.tau, 100_000.0)
        self.assertLessEqual(fit.rms, 0.05 * abs(fit.b))
        self.assertLess(fit.span, 2.0)
        self.assertFalse(fit.resolved, "a best tau at the grid's end resolves no time constant")

    def test_numerical_variation_above_a_twentieth_of_b_leaves_it_unresolved(self) -> None:
        import audit

        tau = 10 * 10 ** (60 / 40)
        y = [40.0 * math.exp(-i / tau) for i in range(1441)]
        self.assertFalse(audit.slow_fit(y, 2.5).resolved)
        self.assertTrue(audit.slow_fit(y, 1.9).resolved)


class Settings(unittest.TestCase):
    def test_histories_differ_on_two_features_of_three_floors(self) -> None:
        import audit, features

        def made(rise: float, fall: float, late_rate: float, late_pressure: float):
            F = features.Feature
            return features.Features(F(rise, 1), F(fall, 3), F(late_rate, 60), F(late_pressure, 60))

        c3, g1 = made(0.2, 10.0, 0.1, -5.0), made(0.2, 10.0, 0.1, -5.0)
        floors = audit.feature_floors(c3, g1, [60, 61])
        self.assertEqual(floors[0], 0.02)
        self.assertEqual(floors[1], 2.0 / math.sqrt(3))
        self.assertEqual(floors[2:], [0.02 / math.sqrt(60), 2.0 / math.sqrt(60)], "the fewest late beats give the floor")
        varied = made(0.2, 10.0, 0.1, -4.0)
        self.assertEqual(audit.feature_floors(c3, varied, [60])[3], 1.0, "a numerical variation above the error")
        spread = [made(0.2 + 0.06 * n, 10.0 + 3.5 * n, 0.1, -5.0) for n in range(2)]
        self.assertTrue(audit.histories_differ(spread, floors)[1])
        narrow = [made(0.2 + 0.059 * n, 10.0 + 3.5 * n, 0.1, -5.0) for n in range(2)]
        self.assertFalse(audit.histories_differ(narrow, floors)[1])

    def test_the_encounter_follows_the_resolved_time_constant(self) -> None:
        import audit

        self.assertEqual(audit.encounter_min(audit.Fit(1200.0, 0.0, 1.0, 0.0, 1.5, True, "")), 10)
        self.assertEqual(audit.encounter_min(audit.Fit(1201.0, 0.0, 1.0, 0.0, 1.5, True, "")), 15)
        self.assertEqual(audit.encounter_min(audit.Fit(2401.0, 0.0, 1.0, 0.0, 1.5, True, "")), 15)
        self.assertEqual(audit.encounter_min(audit.Fit(100.0, 0.0, 1.0, 0.0, 1.5, False, "")), 15)

    def test_the_rule_thresholds_are_the_inclusive_quintile_cuts(self) -> None:
        import audit

        means = [float(n) for n in range(101)]
        self.assertEqual(audit.quintile_cuts(means), (20.0, 80.0))

    def test_workers_and_plan_size_stay_inside_their_budgets(self) -> None:
        import audit

        self.assertEqual(audit.workers_for(logical=12, peak_bytes=2 * 10**9, links_bytes=5 * 10**9, free_bytes=40 * 10**9), 4)
        self.assertEqual(audit.workers_for(logical=12, peak_bytes=10 * 10**9, links_bytes=5 * 10**9, free_bytes=40 * 10**9), 2)
        self.assertEqual(audit.workers_for(logical=12, peak_bytes=10**9, links_bytes=15 * 10**9, free_bytes=40 * 10**9), 2)
        self.assertEqual(audit.workers_for(logical=4, peak_bytes=10**9, links_bytes=10**9, free_bytes=40 * 10**9), 2)
        self.assertEqual(audit.plan_size(run_wall_s=600.0, workers=4), 8)
        self.assertEqual(audit.plan_size(run_wall_s=1200.0, workers=4), 3)


def corpus(out: pathlib.Path, drop: tuple[str, ...] = ()) -> None:
    """Every planned run as a synthetic directory whose spec is the plan's, built so every technical row passes and
    the start histories differ."""
    import audit, spec

    plan = spec.Plan.read()
    first = seal("first")
    a3_beat = 909_090_909

    def histories(n: int):
        def values(t_ns: int) -> dict:
            v = default_values(t_ns)
            v[PRESSURE] = 100.0 if t_ns <= 330 * SECOND else (100.0 - 2 * n if t_ns <= 340 * SECOND else 100.0 - 0.5 * n)
            return v
        return values

    def slow(extra: float):
        def values(t_ns: int) -> dict:
            v = default_values(t_ns)
            t = t_ns / SECOND
            v[ROWS[2]] += 40.0 * math.exp(-(t - 360.0) / (10 * 10 ** 1.5)) + extra if t >= 300.0 else 0.0
            return v
        return values

    def standing(t_ns: int) -> dict:
        v = default_values(t_ns)
        v.update({TRAFFIC: 0.625, RATE: 1e9 / a3_beat, ROWS[1]: 512.0, ROWS[7]: 0.605, ROWS[8]: 0.5})
        return v

    b7 = plan["B7"].spec
    action = spec.PostureEvent(333000, spec.SETTINGS["stand"])
    realized = spec.RunSpec(b7.core, b7.end_ms, tuple(sorted(b7.events + (action,), key=lambda e: e.at_ms)), None, "none").text()
    special = {
        "A0": dict(steps_ns=[SECOND // 4, SECOND // 2, audit.A0_HEAD_NS, 1_100_000_000], seals={0: first, 3: audit.A0_SEAL}),
        "A1": dict(branch_from=5, enqueued="0@5"),
        "A3": dict(values=standing, beat_ns=a3_beat, first_edge_ns=a3_beat, step_ns=a3_beat),
        "A4": dict(branch_from=290, enqueued="3@290 4@290 5@290"),
        "A5": dict(branch_from=290, enqueued="3@290 4@290 5@290"),
        "B1": dict(seal_label="B1"),
        "B2": dict(seal_label="B1", split_ns=510 * SECOND),
        "B3": dict(seal_label="B1", split_ns=509 * SECOND),
        "B4": dict(seal_label="B1"),
        "B5": dict(seal_label="B1"),
        "B6": dict(seal_label="B1"),
        "B7": dict(seal_label="B7", realized=realized, enqueued="2@330"),
        "B8": dict(seal_label="B7", spec=realized),
        "D1": dict(values=slow(0.0)),
        "G1": dict(values=histories(2)),
        "G2": dict(values=slow(0.001)),
    }
    for n in range(1, 7):
        special[f"C{n}"] = dict(values=histories(n - 1))
    for run in plan.runs:
        if run.name in drop:
            continue
        source = run.spec or plan[run.replays].spec
        fields = dict(label=run.name, spec=run.spec.text() if run.spec else "", end_ns=source.end_ms * 1_000_000)
        fields.update(directory_name=ident(run.name), packet=audit.A0_PACKET, seals={0: first})
        fields["step_ns"] = SECOND if source.end_ms > 60000 else SECOND // 4
        if source.cap_ms != 10:
            fields["resolution"] = seal("synthetic resolution of another core")
        fields.update(special.get(run.name, {}))
        Fixture(**fields).write(out)
    (out / "environment.txt").write_text(
        "rustc rustc 1.98.1 (synthetic)\ncargo cargo 1.98.1 (synthetic)\npython3 3.14.0\nrust_src yes\n"
        "logical_processors 12\nmemory_bytes 40000000000\nswap_bytes 4000000000\nfree_bytes /data 50000000000\n"
    )


class Report(unittest.TestCase):
    commit = "c" * 40

    @classmethod
    def setUpClass(cls) -> None:
        cls.built = tempfile.TemporaryDirectory()
        cls.template = pathlib.Path(cls.built.name) / "audit"
        cls.template.mkdir()
        corpus(cls.template)

    @classmethod
    def tearDownClass(cls) -> None:
        cls.built.cleanup()

    def setUp(self) -> None:
        import shutil

        self.scratch = tempfile.TemporaryDirectory()
        self.data = pathlib.Path(self.scratch.name) / "DATA"
        self.out = self.data / "audit"
        shutil.copytree(self.template, self.out)

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def report(self) -> tuple[int, str]:
        import subprocess

        gp = pathlib.Path(__file__).resolve().parent / "gp.py"
        done = subprocess.run([sys.executable, "-I", str(gp), "audit", "--data", str(self.data), "--source-commit", self.commit, "--report"], capture_output=True, text=True, cwd=self.scratch.name)
        self.assertEqual(done.stderr, "")
        return done.returncode, done.stdout

    def test_a_settled_audit_fills_every_row_the_same_bytes_twice(self) -> None:
        code, text = self.report()
        self.assertEqual(code, 0, text)
        self.assertEqual(self.report(), (code, text), "two processes print the same bytes")
        for check in ("A0 pinned second", "A1 across checkpoint", "A2, A3 reflex", "A4 late checkpoint", "A5 rewind", "B1/B2, B1/B3 prefixes", "B4, B5, B6", "B7/B8 closed loop", "Faults, identities", "Closure, beat identity"):
            self.assertRegex(text, rf"\| {re.escape(check)} \|[^\n]*\| pass \|\n")
        self.assertRegex(text, r"\| Histories differ \|[^\n]*\| pass \|\n")
        self.assertRegex(text, r"\| Slow component \|[^\n]*tau 316\.228 s[^\n]*\| resolved \|\n")
        self.assertIn("Technical gate: pass. Informativeness gate: pass.\n", text)
        self.assertIn("history 300 s; encounter 10 min; workers 4", text)
        self.assertEqual(len(re.findall(r"^\| [A-G][0-9]+ \| [0-9]", text, re.M)), 29, "one cost row per run")
        self.assertNotIn(self.scratch.name, text, "no path reaches the report")

    def test_a_planted_failure_fails_its_row_and_the_gate(self) -> None:
        b5 = self.out / ident("B5")
        steps = bytearray((b5 / "steps.npy").read_bytes())
        steps[-8:] = bytes(8)
        rewrite(b5, "steps.npy", bytes(steps))
        code, text = self.report()
        self.assertEqual(code, 0)
        self.assertRegex(text, r"\| B4, B5, B6 \|[^\n]*differ[^\n]*\| fail \|\n")
        self.assertIn("Technical gate: fail.", text)

    def test_an_unfinished_audit_reports_its_missing_runs_and_exits_2(self) -> None:
        import shutil

        shutil.rmtree(self.out / ident("B5"))
        code, text = self.report()
        self.assertEqual(code, 2)
        self.assertRegex(text, r"\| B4, B5, B6 \|[^\n]*missing B5[^\n]*\| incomplete \|\n")


class Preflight(unittest.TestCase):
    def setUp(self) -> None:
        self.scratch = tempfile.TemporaryDirectory()
        self.out = pathlib.Path(self.scratch.name)

    def tearDown(self) -> None:
        self.scratch.cleanup()

    def check(self, workers=3, disks=(), logical=12, wsl=False, free=50 * 10**9):
        import audit

        tools = lambda: {"rustc": "rustc 1.98.1 (synthetic)", "cargo": "cargo 1.98.1 (synthetic)", "rust_src": "yes"}
        audit.preflight(self.out, workers, list(disks), logical, wsl, lambda path: free, tools)

    def test_the_worker_count_stays_under_half_the_processors_and_four(self) -> None:
        import teacher

        for workers, logical in ((5, 12), (3, 4), (0, 12)):
            with self.subTest(workers=workers, logical=logical), self.assertRaises(teacher.TeacherError):
                self.check(workers=workers, logical=logical)

    def test_under_wsl_the_windows_drive_must_be_named(self) -> None:
        import teacher

        with self.assertRaisesRegex(teacher.TeacherError, "--disk /mnt/c"):
            self.check(wsl=True)
        self.check(wsl=True, disks=[pathlib.Path("/mnt/c")])

    def test_forty_gigabytes_only_before_the_first_long_run(self) -> None:
        import teacher

        with self.assertRaisesRegex(teacher.TeacherError, "40 GB"):
            self.check(free=39 * 10**9)
        self.check(free=41 * 10**9)
        self.assertIn("free_bytes", (self.out / "environment.txt").read_text())
        self.check(free=10**9)


class RealDriver(unittest.TestCase):
    """`selftest --real GP_DRIVER`: three short admissions of the real driver, which settle what no fixture can."""

    @classmethod
    def setUpClass(cls) -> None:
        if REAL_DRIVER is None:
            raise unittest.SkipTest("no --real driver")
        import spec, teacher

        cls.scratch = tempfile.TemporaryDirectory()
        cls.out = pathlib.Path(cls.scratch.name) / "audit"
        cls.teacher = teacher.Teacher([str(REAL_DRIVER)], cls.out, "real-selftest", 8, [], 0)
        cls.plan = spec.Plan.read()
        cls.a0 = cls.teacher.settle(teacher.TeacherRun("A0", cls.plan["A0"].spec, None))

    @classmethod
    def tearDownClass(cls) -> None:
        cls.scratch.cleanup()

    def test_a0_its_names_and_every_planned_spec_hold_on_the_real_driver(self) -> None:
        import audit, capture

        run = capture.RunData.open(self.a0.directory)
        self.assertEqual(audit.a0_check(run).verdict, "pass", audit.a0_check(run).measured)
        for name in (audit.TRAFFIC, audit.RATE, audit.LEG_VEINS, audit.LEG_INTERSTITIUM, audit.ARCH, audit.SINUS, audit.PRESSURE):
            run.catalogue.row(name)
        run.catalogue.column_ending(audit.SKIN_SUFFIX)
        self.assertIn(audit.TRAFFIC, run.catalogue.sampled)
        specs = {r.name: self.teacher._spec_file(r) for r in self.plan.all if r.spec}
        census = self.teacher._status(specs, self.a0.directory / "summary.txt")
        self.assertEqual([name for name, (word, _, _) in census.items() if word == "invalid"], [])

    def test_a_short_closed_loop_and_its_replay_give_one_seal_log(self) -> None:
        import audit, capture, spec, teacher

        rule = "audit-pressure 1 after-ms 3500 baseline-beats 2 refractory-ms 1000 delay-ms 500"
        loop = spec.RunSpec("ng-solver cap-ms 10", 6000, (spec.PostureEvent(1000, spec.SETTINGS["stand"]),), rule, "none")
        settled = self.teacher.ensure([teacher.TeacherRun("loop", loop, None), teacher.TeacherRun("replay", None, "loop")], self.a0.directory / "summary.txt", 2)
        looped, replayed = (capture.RunData.open(settled[name].directory) for name in ("loop", "replay"))
        self.assertGreaterEqual(len(looped.summary.enqueued), 1, "the rule acted")
        self.assertEqual(replayed.spec_text, looped.realized_text)
        self.assertEqual(audit.same_log_check(looped, [replayed]).verdict, "pass")


def run(real: pathlib.Path | None) -> int:
    global REAL_DRIVER
    REAL_DRIVER = real
    suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
    result = unittest.TextTestRunner(stream=sys.stdout, verbosity=1).run(suite)
    return 0 if result.wasSuccessful() else 1
