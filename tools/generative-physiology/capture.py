from __future__ import annotations

import ast
import hashlib
import math
import struct
import sys
from array import array
from dataclasses import dataclass
from functools import cached_property
from pathlib import Path

READS = 2
ANCHOR_NS = 24_900_000_000_000
PRESSURE = "algebraic/CV/blood pressure/systemic arteries"
AORTIC_WATER = "CV/valves/left ventricle>systemic arteries/Water/inside"
GUARD_ULPS = 4


class CaptureError(Exception):
    pass


def read_table(path: Path, typecode: str) -> tuple[int, int, array]:
    data = path.read_bytes()
    if data[:8] != b"\x93NUMPY\x01\x00" or len(data) < 10:
        raise CaptureError(f"{path.name}: not an NPY 1.0 file")
    (length,) = struct.unpack("<H", data[8:10])
    if length > 4096:
        raise CaptureError(f"{path.name}: a header of {length} bytes")
    try:
        header = ast.literal_eval(data[10 : 10 + length].decode("latin-1").strip())
    except (ValueError, SyntaxError) as error:
        raise CaptureError(f"{path.name}: an unreadable header") from error
    descr = {"Q": "<u8", "d": "<f8"}[typecode]
    shape = header.get("shape") if isinstance(header, dict) else None
    if (
        set(header) != {"descr", "fortran_order", "shape"}
        or header["descr"] != descr
        or header["fortran_order"] is not False
        or not (isinstance(shape, tuple) and len(shape) == 2 and all(isinstance(n, int) for n in shape))
    ):
        raise CaptureError(f"{path.name}: a header other than {descr}, C order and two dimensions")
    rows, columns = shape
    body = data[10 + length :]
    if len(body) != rows * columns * 8:
        raise CaptureError(f"{path.name}: {len(body)} data bytes for a shape of ({rows}, {columns})")
    cells = array(typecode)
    cells.frombytes(body)
    if sys.byteorder == "big":
        cells.byteswap()
    return rows, columns, cells


def as_double(cell: int) -> float:
    return struct.unpack("<d", struct.pack("<Q", cell))[0]


def seconds(ns_after_anchor: int) -> float:
    return ns_after_anchor / 1e9


@dataclass(frozen=True)
class Sealed:
    revision: int
    end_ns: int
    seal: str


@dataclass(frozen=True)
class BeatReading:
    ordinal: int
    start_s: float
    end_s: float
    heart_rate_per_s: float
    arterial_pressure: float
    stroke: float
    filtration: float


@dataclass(frozen=True)
class Quantity:
    book: str
    terms: tuple[tuple[int, int], ...]


@dataclass(frozen=True)
class Catalogue:
    model: str
    rows: tuple[str, ...]
    columns: tuple[str, ...]
    quantities: tuple[Quantity, ...]
    modes: tuple[tuple[str, str], ...]
    lens: tuple[tuple[str, int], ...]
    sampled: tuple[str, ...]

    @staticmethod
    def parse(text: str) -> Catalogue:
        lines = text.split("\n")
        if lines[-1] != "" or lines[0] != "gp-catalogue 1" or not lines[1].startswith("model "):
            raise CaptureError("catalogue.txt: not a gp-catalogue 1 text")
        kinds = ("row", "column", "quantity", "mode", "lens", "sampled")
        found: dict[str, list[str]] = {kind: [] for kind in kinds}
        at = 0
        for line in lines[2:-1]:
            kind, _, rest = line.partition(" ")
            if kind not in kinds or kinds.index(kind) < at or not rest:
                raise CaptureError(f"catalogue.txt: {line!r} out of place")
            at = kinds.index(kind)
            found[kind].append(rest)
        quantities = []
        for line in found["quantity"]:
            book, *terms = line.split(" ")
            pairs = tuple((int(row), int(weight)) for row, weight in (term.split(":") for term in terms))
            quantities.append(Quantity(book, pairs))
        modes = tuple(tuple(line.rsplit(" ", 1)) for line in found["mode"])
        lens = tuple((name, int(window)) for name, window in (line.rsplit(" ", 1) for line in found["lens"]))
        catalogue = Catalogue(
            lines[1][len("model ") :],
            tuple(found["row"]),
            tuple(found["column"]),
            tuple(quantities),
            modes,
            lens,
            tuple(found["sampled"]),
        )
        for names in (catalogue.rows, catalogue.columns):
            if len(set(names)) != len(names):
                raise CaptureError("catalogue.txt: a name listed twice")
        return catalogue

    @property
    def width(self) -> int:
        return 2 + len(self.modes) + 4 * len(self.rows) + 4 * len(self.columns) + 6 * len(self.quantities)

    def row(self, name: str) -> int:
        try:
            return self.rows.index(name)
        except ValueError:
            raise CaptureError(f"the body has no row {name}") from None

    def column(self, name: str) -> int:
        try:
            return self.columns.index(name)
        except ValueError:
            raise CaptureError(f"the body has no column {name}") from None

    def column_ending(self, suffix: str) -> str:
        found = [name for name in self.columns if name.endswith(suffix)]
        if len(found) != 1:
            raise CaptureError(f"{len(found)} columns end in {suffix}")
        return found[0]

    def channel(self, row_name: str) -> int:
        for index, (name, _) in enumerate(self.lens):
            if name == row_name:
                return index
        raise CaptureError(f"no lens channel reads {row_name}")


@dataclass(frozen=True)
class Fault:
    kind: str
    course: str
    at_ns: int
    attempted_ns: int
    step_refusals: int
    detail: str


@dataclass(frozen=True)
class Branch:
    from_revision: int
    enqueued: tuple[tuple[int, int], ...]
    first_seal: str
    revisions: int
    refusals: int
    restore_s: float
    last_seal: str
    end: dict[str, str]


END_KEYS = ("revision", "time_ns", "state", "continuation", "segment", "modes", "streams", "applied", "waiting", "ledger")
FIRST_KEYS = (
    "spec_id",
    "capture_version",
    "source_commit",
    "model",
    "packet",
    "resolution",
    "run_identity",
    "first_seal",
    "last_seal",
    "revisions",
    "refusals",
    "beats",
    "enqueued",
    "fired",
)
COST_KEYS = ("admission_s", "step_wall_s", "run_wall_s", "peak_rss_bytes", "links_bytes")
FAULT_KEYS = ("fault_course", "fault_at_ns", "fault_attempted_ns", "fault_step_refusals", "fault_detail")
BRANCH_KEYS = (
    "branch_from_revision",
    "branch_enqueued",
    "branch_first_seal",
    "branch_revisions",
    "branch_refusals",
    "branch_restore_s",
    "branch_last_seal",
)


def arrivals(text: str) -> tuple[tuple[int, int], ...]:
    if text == "none":
        return ()
    return tuple((int(index), int(revision)) for index, revision in (entry.split("@") for entry in text.split(" ")))


class _Lines:
    def __init__(self, text: str) -> None:
        if not text.endswith("\n"):
            raise CaptureError("summary.txt: no final newline")
        self.lines = [line.partition(" ") for line in text[:-1].split("\n")]
        self.at = 0

    def peek(self) -> str | None:
        return self.lines[self.at][0] if self.at < len(self.lines) else None

    def take(self, key: str) -> str:
        found = self.peek()
        if found != key:
            raise CaptureError(f"summary.txt: {found or 'the end'} where {key} belongs")
        value = self.lines[self.at][2]
        self.at += 1
        return value


@dataclass(frozen=True)
class Summary:
    values: dict[str, str]
    fault: Fault | None
    files: dict[str, str]
    end: dict[str, str]
    branch: Branch | None

    @staticmethod
    def parse(text: str) -> Summary:
        lines = _Lines(text)
        values = {key: lines.take(key) for key in FIRST_KEYS}
        kind = lines.take("fault")
        fault = None
        if kind != "none":
            parts = {key: lines.take(key) for key in FAULT_KEYS}
            fault = Fault(
                kind,
                parts["fault_course"],
                int(parts["fault_at_ns"]) - ANCHOR_NS,
                int(parts["fault_attempted_ns"]),
                int(parts["fault_step_refusals"]),
                parts["fault_detail"],
            )
        values.update({key: lines.take(key) for key in COST_KEYS})
        files = {}
        while lines.peek() == "file":
            name, _, digest = lines.take("file").partition(" ")
            if name in files:
                raise CaptureError(f"summary.txt: file {name} twice")
            files[name] = digest
        values.update({key: lines.take(key) for key in ("memory_cap_gb", "variant")})
        end = {key: lines.take(f"end_{key}") for key in END_KEYS}
        branch = None
        if lines.peek() == "branch_from_revision":
            parts = {key: lines.take(key) for key in BRANCH_KEYS}
            branch = Branch(
                int(parts["branch_from_revision"]),
                arrivals(parts["branch_enqueued"]),
                parts["branch_first_seal"],
                int(parts["branch_revisions"]),
                int(parts["branch_refusals"]),
                float(parts["branch_restore_s"]),
                parts["branch_last_seal"],
                {key: lines.take(f"branch_end_{key}") for key in END_KEYS},
            )
        if lines.peek() is not None:
            raise CaptureError(f"summary.txt: {lines.peek()} after the last key")
        return Summary(values, fault, files, end, branch)

    def __getattr__(self, key: str):
        values = self.__dict__.get("values", {})
        if key in values:
            return values[key]
        raise AttributeError(key)

    def number(self, key: str) -> int:
        return int(self.values[key])

    def seconds(self, key: str) -> float:
        return float(self.values[key])

    @property
    def enqueued(self) -> tuple[tuple[int, int], ...]:
        return arrivals(self.values["enqueued"])

    @property
    def fired(self) -> tuple[tuple[str, int, int], ...]:
        text = self.values["fired"]
        if text == "none":
            return ()
        firings = []
        for entry in text.split(" "):
            stream, revision, at = entry.split("@")
            firings.append((stream, int(revision), int(at) - ANCHOR_NS))
        return tuple(firings)


@dataclass(frozen=True)
class Limbs:
    hi: tuple[float, ...]
    lo: tuple[float, ...]

    def at(self, edge: int) -> float:
        return self.hi[edge]

    def parts(self, edge: int) -> tuple[float, float]:
        return self.hi[edge], self.lo[edge]

    def difference(self, later: int, earlier: int) -> float:
        return math.fsum((self.hi[later], self.lo[later], -self.hi[earlier], -self.lo[earlier]))


@dataclass(frozen=True)
class Ledger:
    book: str
    label: str
    inventory: Limbs
    inflow: Limbs
    outflow: Limbs


class Edges:
    def __init__(self, catalogue: Catalogue, cells: array, rows: int) -> None:
        self.catalogue = catalogue
        self.cells = cells
        self.rows = rows
        self.width = catalogue.width
        self.times_ns = tuple(cells[row * self.width] - ANCHOR_NS for row in range(rows))
        self.revisions = tuple(cells[row * self.width + 1] for row in range(rows))

    def _limbs(self, column: int) -> Limbs:
        width, cells = self.width, self.cells
        hi = tuple(as_double(cells[row * width + column]) for row in range(self.rows))
        lo = tuple(as_double(cells[row * width + column + 1]) for row in range(self.rows))
        return Limbs(hi, lo)

    @property
    def _values(self) -> int:
        return 2 + len(self.catalogue.modes)

    def value(self, row: str) -> Limbs:
        return self._limbs(self._values + 2 * self.catalogue.row(row))

    def integral(self, row: str) -> Limbs:
        return self._limbs(self._values + 2 * len(self.catalogue.rows) + 2 * self.catalogue.row(row))

    def extent(self, column: str) -> tuple[Limbs, Limbs]:
        first = self._values + 4 * len(self.catalogue.rows) + 4 * self.catalogue.column(column)
        return self._limbs(first), self._limbs(first + 2)

    def ledger(self) -> list[Ledger]:
        first = self._values + 4 * len(self.catalogue.rows) + 4 * len(self.catalogue.columns)
        found = []
        for index, quantity in enumerate(self.catalogue.quantities):
            at = first + 6 * index
            label = self.catalogue.rows[quantity.terms[0][0]] if quantity.terms else "none"
            found.append(Ledger(quantity.book, label, self._limbs(at), self._limbs(at + 2), self._limbs(at + 4)))
        return found

    def modes(self, edge: int) -> tuple[int, ...]:
        start = edge * self.width + 2
        return tuple(self.cells[start : start + len(self.catalogue.modes)])

    def beat_rows(self, beat: int) -> tuple[int, int]:
        """The edge rows a beat spans: the start and the beat edges are rows 0 and up, so beat j's are j + 1 and j + 2."""
        return beat + 1, beat + 2

    def beat_ns(self, beat: int) -> int:
        start, end = self.beat_rows(beat)
        return self.times_ns[end] - self.times_ns[start]

    def beat_means(self, row: str) -> list[float]:
        integral = self.integral(row)
        return [integral.difference(*reversed(self.beat_rows(j))) / self.beat_ns(j) for j in range(self.beats)]

    def beat_net_extents(self, column: str) -> list[float]:
        forward, reverse = self.extent(column)
        found = []
        for j in range(self.beats):
            start, end = self.beat_rows(j)
            limbs = (*forward.parts(end), *reverse.parts(start)), (*forward.parts(start), *reverse.parts(end))
            found.append(math.fsum((*limbs[0], *(-x for x in limbs[1]))))
        return found

    @property
    def beats(self) -> int:
        return max(0, self.rows - 3)


def _close(one: float, other: float) -> bool:
    if one == other:
        return True
    if not (math.isfinite(one) and math.isfinite(other)):
        return False
    return abs(one - other) <= GUARD_ULPS * math.ulp(max(abs(one), abs(other)))


class RunData:
    def __init__(self, directory: Path, summary: Summary, catalogue: Catalogue) -> None:
        self.directory = directory
        self.summary = summary
        self.catalogue = catalogue
        self.fault = directory.name.endswith(".fault")
        self.spec_text = (directory / "spec.txt").read_text()
        realized = directory / "realized.txt"
        self.realized_text = realized.read_text() if realized.exists() else None

    @staticmethod
    def open(directory: Path) -> RunData:
        directory = Path(directory)
        try:
            text = (directory / "summary.txt").read_text()
        except OSError as error:
            raise CaptureError(f"{directory.name}: no summary.txt") from error
        if not text.startswith("spec_id ") or "\ncapture_version " not in text:
            raise CaptureError(f"{directory.name}: not a run summary")
        version = text.split("\ncapture_version ", 1)[1].split("\n", 1)[0]
        if version != str(READS):
            raise CaptureError(f"{directory.name}: capture version {version}, and this reader reads {READS}")
        summary = Summary.parse(text)
        for name, digest in summary.files.items():
            path = directory / name
            if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
                raise CaptureError(f"{directory.name}: {name} differs from its summary digest")
        for name in ("spec.txt", "catalogue.txt", "edges.npy", "beats.npy", "steps.npy", "samples.npy"):
            if name not in summary.files:
                raise CaptureError(f"{directory.name}: the summary lists no {name}")
        catalogue = Catalogue.parse((directory / "catalogue.txt").read_text())
        if catalogue.model != summary.model:
            raise CaptureError(f"{directory.name}: the catalogue's model is not the run's model")
        run = RunData(directory, summary, catalogue)
        run.check_layout()
        return run

    def _table(self, name: str, typecode: str, columns: int) -> tuple[int, array]:
        rows, found, cells = read_table(self.directory / name, typecode)
        if found != columns:
            raise CaptureError(f"{self.directory.name}: {name} has {found} columns, not {columns}")
        return rows, cells

    def edges(self) -> Edges:
        rows, cells = self._table("edges.npy", "Q", self.catalogue.width)
        return Edges(self.catalogue, cells, rows)

    def _seal_log(self, name: str, first: Sealed | None) -> tuple[Sealed, ...]:
        rows, cells = self._table(name, "Q", 7)
        log = [first] if first else []
        for row in range(rows):
            at = row * 7
            seal = "".join(f"{word:016x}" for word in cells[at + 3 : at + 7])
            log.append(Sealed(cells[at], cells[at + 1] - ANCHOR_NS, seal))
        return tuple(log)

    @cached_property
    def seals(self) -> tuple[Sealed, ...]:
        return self._seal_log("steps.npy", Sealed(0, 0, self.summary.first_seal))

    @cached_property
    def refusals(self) -> tuple[int, ...]:
        rows, cells = self._table("steps.npy", "Q", 7)
        return tuple(cells[row * 7 + 2] for row in range(rows))

    @cached_property
    def branch_seals(self) -> tuple[Sealed, ...] | None:
        if self.summary.branch is None:
            return None
        return self._seal_log("branch-steps.npy", None)

    @cached_property
    def beats(self) -> tuple[BeatReading, ...]:
        rows, cells = self._table("beats.npy", "d", 7)
        return tuple(BeatReading(int(cells[r * 7]), *cells[r * 7 + 1 : r * 7 + 7]) for r in range(rows))

    def samples(self, row: str) -> list[tuple[int, int, float]]:
        if row not in self.catalogue.sampled:
            raise CaptureError(f"{self.directory.name}: {row} is not sampled")
        width = 2 + 2 * len(self.catalogue.sampled)
        rows, cells = self._table("samples.npy", "Q", width)
        column = 2 + 2 * self.catalogue.sampled.index(row)
        return [(cells[r * width], cells[r * width + 1] - ANCHOR_NS, as_double(cells[r * width + column])) for r in range(rows)]

    def lens_present(self, row: str) -> list[tuple[int, float]]:
        channel = self.catalogue.channel(row)
        rows, cells = self._table("lens.npy", "Q", 6)
        found: dict[int, tuple[int, int]] = {}
        for r in range(rows):
            at = r * 6
            if cells[at] != channel or cells[at + 3] != 0:
                continue
            collected = cells[at + 1] - ANCHOR_NS
            reading = (cells[at + 4], cells[at + 5])
            if found.setdefault(collected, reading) != reading:
                raise CaptureError(f"{self.directory.name}: two readings of channel {channel} at {collected} ns")
        return [(collected, as_double(hi)) for collected, (hi, _) in sorted(found.items())]

    @cached_property
    def applied(self) -> tuple[tuple[int, int], ...]:
        rows, cells = self._table("applied.npy", "Q", 2)
        return tuple((cells[r * 2], cells[r * 2 + 1]) for r in range(rows))

    def check_layout(self) -> None:
        if len(self.seals) != self.summary.number("revisions") + 1:
            raise CaptureError(f"{self.directory.name}: steps.npy holds {len(self.seals) - 1} revisions")
        for row in self.catalogue.sampled:
            self.samples(row)
        for row, _ in self.catalogue.lens:
            self.lens_present(row)
        if self.branch_seals is not None and self.summary.branch is None:
            raise CaptureError(f"{self.directory.name}: a branch table with no branch")
        len(self.applied)
        edges = self.edges()
        if edges.rows == 0 or edges.revisions[0] != 0 or edges.times_ns[0] != 0:
            raise CaptureError(f"{self.directory.name}: edges.npy does not start at admission")
        beats = self.beats
        if len(beats) != self.summary.number("beats") or (beats and edges.rows < len(beats) + 3):
            raise CaptureError(f"{self.directory.name}: {len(beats)} beats against {edges.rows} edges")
        pressure = edges.beat_means(PRESSURE)[: len(beats)]
        stroke = edges.beat_net_extents(AORTIC_WATER)[: len(beats)]
        for j, beat in enumerate(beats):
            spans = tuple(seconds(edges.times_ns[row]) for row in edges.beat_rows(j))
            if spans != (beat.start_s, beat.end_s) or not (_close(pressure[j], beat.arterial_pressure) and _close(stroke[j], beat.stroke)):
                raise CaptureError(f"{self.directory.name}: beat {j} differs from the edges it spans")
