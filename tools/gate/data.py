"""The research data's rules: what a public record may hold, and that curated values close.

Python 3.11 stdlib only. Run as `python3 -I tools/gate/data.py <command>`; it finds the repo from its own path.

    check    every rule over data/ and every Data: line in .scratch/
    probes   every rule refuses each record built to break it, on a synthetic base held here

Exit codes: 0 pass; 1 violations, one per line as `<code>: <subject>: <what to do>`; 2 an input the check
reads is missing or malformed.
"""

from __future__ import annotations

import ast
import copy
import dataclasses
import decimal
import json
import math
import pathlib
import re
import sys
from collections.abc import Callable, Iterator
from fractions import Fraction

ROOT = pathlib.Path(__file__).resolve().parents[2]
DATA = "data"
FILES = ("works", "packet-reference-man", "healthy-range-set", "inferences", "closure")
ID = re.compile(r"^[a-z][a-z0-9_]*(\.[a-z0-9_]+)+$")
NAME = re.compile(r"[a-z][a-z0-9_]*(?:\.[a-z0-9_]+)+")
PREFIX = re.compile(r"^[a-z][a-z0-9_]*(\.[a-z0-9_]+)*\.\*$")
WORK_ID = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")
DATA_LINE = re.compile(r"^Data:(.*)$")
PAGE = re.compile(r"\bp\.\s?\d+")
TABLE_OR_PAGE = re.compile(r"\b(table|tab|fig|figure|p|pp|para|section|sec)\b\.?", re.I)

CAPS = {"population": 120, "conditions": 120, "method": 120, "window": 60, "locator": 80}
TEXT_LIMIT = 240
ROW_FIELDS = {"id", "kind", "unit", "value", "population", "conditions", "method", "window", "evidence"}
WORK_FIELDS = {"id", "kind", "citation", "doi", "pmid", "pmcid", "isbn", "edition", "dataset", "standard", "query"}
INFERENCE_FIELDS = {"id", "target", "form", "model", "population", "low", "high", "steps", "premises", "search"}
CLOSURE_FIELDS = {"id", "level", "claim", "lhs", "rhs"}
PACKET_KINDS = {"anchor", "parameter", "day", "constant", "input"}
SET_KINDS = {"healthy_range", "within_subject_variation"}
TIERS = {"primary_measured", "primary_fitted", "compiled", "textbook"}
CERTAINTY = {"high", "moderate", "low", "very_low"}
READINGS = {"full_text", "abstract", "secondary_reproduction"}
WORK_TIERS = {
    "primary": {"primary_measured", "primary_fitted"},
    "dataset": {"primary_measured", "primary_fitted"},
    "review": {"compiled"},
    "guideline": {"compiled"},
    "compilation": {"compiled"},
    "standard": {"compiled"},
    "textbook": {"textbook"},
}
WORK_IDS = ("doi", "pmid", "pmcid", "isbn", "dataset", "standard")
FORMS = {"fill", "adjust", "flows"}
LEVELS = {"source", "packet"}
VALUE_FORMS = (
    {"point"},
    {"mean", "sd"},
    {"mean", "se"},
    {"mean", "ci_low", "ci_high"},
    {"low", "high", "coverage"},
    {"median", "low", "high", "coverage"},
)
EVIDENCE_FORMS = {
    "source": ({"source", "locator", "reading", "tier", "certainty"}, {"upgrade"}),
    "derived": ({"derived", "identity"}, set()),
    "inferred": ({"inferred"}, set()),
    "calibrated": ({"calibrated"}, set()),
}

Dimension = tuple[int, ...]
DIMENSIONS = ("m", "kg", "s", "mol", "K", "A")
NUMBER: Dimension = (0, 0, 0, 0, 0, 0)


def dim(m: int = 0, kg: int = 0, s: int = 0, mol: int = 0, K: int = 0, A: int = 0) -> Dimension:
    return (m, kg, s, mol, K, A)


VOLUME, PRESSURE, ENERGY, POWER = dim(m=3), dim(kg=1, m=-1, s=-2), dim(kg=1, m=2, s=-2), dim(kg=1, m=2, s=-3)
UNITS: dict[str, tuple[Fraction, Dimension]] = {
    "1": (Fraction(1), NUMBER),
    "%": (Fraction(1, 100), NUMBER),
    "pH": (Fraction(1), NUMBER),
    "m": (Fraction(1), dim(m=1)),
    "cm": (Fraction(1, 100), dim(m=1)),
    "mm": (Fraction(1, 1000), dim(m=1)),
    "L": (Fraction(1, 1000), VOLUME),
    "dL": (Fraction(1, 10_000), VOLUME),
    "mL": (Fraction(1, 10**6), VOLUME),
    "uL": (Fraction(1, 10**9), VOLUME),
    "fL": (Fraction(1, 10**18), VOLUME),
    "kg": (Fraction(1), dim(kg=1)),
    "g": (Fraction(1, 1000), dim(kg=1)),
    "mg": (Fraction(1, 10**6), dim(kg=1)),
    "ug": (Fraction(1, 10**9), dim(kg=1)),
    "ng": (Fraction(1, 10**12), dim(kg=1)),
    "pg": (Fraction(1, 10**15), dim(kg=1)),
    "s": (Fraction(1), dim(s=1)),
    "ms": (Fraction(1, 1000), dim(s=1)),
    "min": (Fraction(60), dim(s=1)),
    "h": (Fraction(3600), dim(s=1)),
    "d": (Fraction(86400), dim(s=1)),
    "y": (Fraction(31_557_600), dim(s=1)),
    "mol": (Fraction(1), dim(mol=1)),
    "mmol": (Fraction(1, 1000), dim(mol=1)),
    "umol": (Fraction(1, 10**6), dim(mol=1)),
    "nmol": (Fraction(1, 10**9), dim(mol=1)),
    "pmol": (Fraction(1, 10**12), dim(mol=1)),
    "Pa": (Fraction(1), PRESSURE),
    "kPa": (Fraction(1000), PRESSURE),
    "mmHg": (Fraction("133.322387415"), PRESSURE),
    "cmH2O": (Fraction("98.0665"), PRESSURE),
    "dyn": (Fraction(1, 10**5), dim(kg=1, m=1, s=-2)),
    "J": (Fraction(1), ENERGY),
    "kJ": (Fraction(1000), ENERGY),
    "kcal": (Fraction(4184), ENERGY),
    "W": (Fraction(1), POWER),
    "K": (Fraction(1), dim(K=1)),
    "degC": (Fraction(1), dim(K=1)),
}
UNIT_TOKEN = re.compile(r"\s*([*/])?\s*([A-Za-z%][A-Za-z0-9]*|1)(?:\^(-?\d+))?")


class DataError(Exception):
    pass


@dataclasses.dataclass(frozen=True)
class Violation:
    code: str
    subject: str
    fix: str

    def __str__(self) -> str:
        return f"{self.code}: {self.subject}: {self.fix}"


def parse_unit(text: str) -> tuple[Fraction, Dimension]:
    if "degC" in text and text != "degC":
        raise ValueError("degC is an absolute scale and stands alone; write K inside a compound unit")
    pos, factor, powers, first = 0, Fraction(1), [0] * len(DIMENSIONS), True
    while pos < len(text):
        m = UNIT_TOKEN.match(text, pos)
        if not m or (m.group(1) is None and not first):
            raise ValueError(f"cannot read {text!r} at {pos}")
        name = m.group(2)
        if name not in UNITS:
            raise ValueError(f"{name!r} is not in the unit table")
        f, d = UNITS[name]
        s = int(m.group(3) or 1) * (-1 if m.group(1) == "/" else 1)
        factor *= f**s
        powers = [a + s * b for a, b in zip(powers, d)]
        pos, first = m.end(), False
    if first:
        raise ValueError("empty unit")
    return factor, tuple(powers)


@dataclasses.dataclass(frozen=True)
class Quantity:
    value: float
    dimension: Dimension = NUMBER


def si(value: float, unit: str) -> Quantity:
    factor, dimension = parse_unit(unit)
    return Quantity(value + 273.15 if unit == "degC" else value * float(factor), dimension)


def si_difference(value: float, unit: str) -> Quantity:
    factor, dimension = parse_unit(unit)
    return Quantity(value * float(factor), dimension)


def parse_expression(text: str) -> tuple[ast.Expression, dict[str, str]]:
    """A record id can hold a segment no Python name can, such as `bc.constant.pk1.37c`, so ids become placeholders first."""
    names: dict[str, str] = {}

    def hold(m: re.Match[str]) -> str:
        names[f"n{len(names)}"] = m.group(0)
        return f"n{len(names) - 1}"

    return ast.parse(NAME.sub(hold, text), mode="eval"), names


def names_in(text: str) -> set[str]:
    return set(parse_expression(text)[1].values())


def evaluate(node: ast.AST, names: dict[str, str], env: Callable[[str], Quantity]) -> Quantity:
    if isinstance(node, ast.Expression):
        return evaluate(node.body, names, env)
    if isinstance(node, ast.Constant) and isinstance(node.value, (int, float)) and not isinstance(node.value, bool):
        return Quantity(float(node.value))
    if isinstance(node, ast.Name) and node.id in names:
        return env(names[node.id])
    if isinstance(node, ast.UnaryOp) and isinstance(node.op, ast.USub):
        x = evaluate(node.operand, names, env)
        return Quantity(-x.value, x.dimension)
    if isinstance(node, ast.BinOp):
        a, b = evaluate(node.left, names, env), evaluate(node.right, names, env)
        if isinstance(node.op, (ast.Add, ast.Sub)):
            if a.dimension != b.dimension:
                raise ValueError(f"adds {a.dimension} to {b.dimension}")
            return Quantity(a.value + b.value if isinstance(node.op, ast.Add) else a.value - b.value, a.dimension)
        if isinstance(node.op, ast.Mult):
            return Quantity(a.value * b.value, tuple(x + y for x, y in zip(a.dimension, b.dimension)))
        if isinstance(node.op, ast.Div):
            return Quantity(a.value / b.value, tuple(x - y for x, y in zip(a.dimension, b.dimension)))
        if isinstance(node.op, ast.Pow) and b.dimension == NUMBER:
            if a.dimension == NUMBER:
                return Quantity(a.value**b.value)
            if b.value == int(b.value):
                return Quantity(a.value**b.value, tuple(x * int(b.value) for x in a.dimension))
    if isinstance(node, ast.Call) and isinstance(node.func, ast.Name):
        xs = [evaluate(a, names, env) for a in node.args]
        f = node.func.id
        if f in ("log10", "exp") and len(xs) == 1 and xs[0].dimension == NUMBER:
            return Quantity({"log10": math.log10, "exp": math.exp}[f](xs[0].value))
        if f == "sum" and len({x.dimension for x in xs}) == 1:
            return Quantity(sum(x.value for x in xs), xs[0].dimension)
    raise ValueError(f"form not allowed: {ast.dump(node)[:60]}")


def value_of(text: str, env: Callable[[str], Quantity]) -> Quantity:
    tree, names = parse_expression(text)
    q = evaluate(tree, names, env)
    if not math.isfinite(q.value):
        raise ValueError(f"{text} is not finite")
    return q


def central(value: dict) -> float:
    for k in ("point", "mean", "median"):
        if k in value:
            return value[k]
    return (value["low"] + value["high"]) / 2


def last_digit_tolerance(x: float) -> float:
    exponent = decimal.Decimal(repr(x)).as_tuple().exponent
    return 0.5 * 10.0 ** (exponent if isinstance(exponent, int) else 0)


def evidence_form(evidence: object) -> str | None:
    if isinstance(evidence, dict):
        for form, (required, optional) in EVIDENCE_FORMS.items():
            if required <= evidence.keys() <= required | optional:
                return form
    return None


@dataclasses.dataclass
class Store:
    works: list[dict]
    packet: list[dict]
    ranges: list[dict]
    inferences: list[dict]
    closures: list[dict]
    tickets: dict[str, list[str]]

    @property
    def all_rows(self) -> list[dict]:
        return self.packet + self.ranges

    @property
    def rows(self) -> dict[str, dict]:
        return {r.get("id", ""): r for r in self.all_rows}

    @property
    def work_ids(self) -> dict[str, dict]:
        return {w.get("id", ""): w for w in self.works}

    def quantity(self, name: str) -> Quantity:
        row = self.rows[name]
        return si(central(row["value"]), row["unit"])


def refuse_constant(name: str) -> None:
    raise ValueError(f"{name} is not a number")


def read_jsonl(path: pathlib.Path) -> list[dict]:
    rows = []
    for i, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            rows.append(json.loads(line, parse_constant=refuse_constant))
        except ValueError as e:
            raise DataError(f"{path}:{i}: {e}") from e
    return rows


def read_tickets(root: pathlib.Path) -> dict[str, list[str]]:
    tickets = {}
    for md in sorted((root / ".scratch").rglob("*.md")):
        names = []
        for line in md.read_text(encoding="utf-8").splitlines():
            if m := DATA_LINE.match(line):
                names += [n.strip() for n in m.group(1).split(",")]
        if names:
            tickets[str(md.relative_to(root))] = names
    return tickets


def load(root: pathlib.Path) -> Store:
    tables = {}
    for name in FILES:
        path = root / DATA / f"{name}.jsonl"
        if not path.exists():
            raise DataError(f"{path} is missing")
        tables[name] = read_jsonl(path)
    return Store(
        tables["works"],
        tables["packet-reference-man"],
        tables["healthy-range-set"],
        tables["inferences"],
        tables["closure"],
        read_tickets(root),
    )


def rests_on(row: dict) -> set[str]:
    e = row.get("evidence", {})
    try:
        if "derived" in e:
            return names_in(e["derived"])
    except SyntaxError:
        return set()
    return set(e.get("calibrated", []))


def inferred_under(s: Store, row: dict) -> set[str]:
    rows, seen, found, todo = s.rows, set(), set(), [row]
    while todo:
        r = todo.pop()
        if "inferred" in r.get("evidence", {}):
            found.add(r["evidence"]["inferred"])
        for name in rests_on(r) - seen:
            seen.add(name)
            if name in rows:
                todo.append(rows[name])
    return found


def strings(item: object) -> Iterator[str]:
    if isinstance(item, dict):
        for v in item.values():
            yield from strings(v)
    elif isinstance(item, list):
        for v in item:
            yield from strings(v)
    elif isinstance(item, str):
        yield item


def numbers(item: object) -> Iterator[float]:
    if isinstance(item, dict):
        for v in item.values():
            yield from numbers(v)
    elif isinstance(item, (int, float)) and not isinstance(item, bool):
        yield float(item)


Rule = Callable[[Store], list[Violation]]
Probe = Callable[[Store], None]
RULES: list[tuple[str, Rule, tuple[Probe, ...]]] = []


def rule(code: str, *probes: Probe):
    def register(check: Rule) -> Rule:
        RULES.append((code, check, probes))
        return check

    return register


def synthetic_store() -> Store:
    def row(rid: str, kind: str, unit: str, value: dict, evidence: dict) -> dict:
        return dict(id=rid, kind=kind, unit=unit, value=value, population="fixture", conditions="fixture",
                    method="fixture", window="fixture", evidence=evidence)

    def primary() -> dict:
        return dict(source="fixture-paper", locator="Table 1", reading="full_text", tier="primary_measured",
                    certainty="low")

    def compiled() -> dict:
        return dict(source="fixture-review", locator="Table 1", reading="full_text", tier="compiled",
                    certainty="moderate", upgrade=False)

    def book() -> dict:
        return dict(source="fixture-book", locator="ch.1 p.1", reading="full_text", tier="textbook",
                    certainty="moderate", upgrade=True)

    def identity(expr: str) -> dict:
        return dict(derived=expr, identity="fixture-paper")

    packet = [
        row("fx.area", "input", "m^2", {"point": 2.0}, primary()),
        row("fx.base", "input", "L/min/m^2", {"point": 1.5}, book()),
        row("fx.fitted", "parameter", "mL", {"point": 1.0}, dict(calibrated=["fx.rate"])),
        row("fx.flow", "anchor", "L/min", {"point": 6.0}, identity("fx.index * fx.area")),
        row("fx.guess", "parameter", "mL", {"point": 1.5}, dict(inferred="inf.fill")),
        row("fx.half", "input", "L/min", {"point": 3.0}, identity("fx.flow * fx.share")),
        row("fx.index", "input", "L/min/m^2", {"point": 3.0}, identity("fx.base * fx.twice")),
        row("fx.kilojoules", "input", "kJ", {"point": 4.184}, primary()),
        row("fx.kilocalories", "input", "kcal", {"point": 1.0}, primary()),
        row("fx.kilopascals", "input", "kPa", {"point": 100.0}, primary()),
        row("fx.none", "input", "/min", {"point": 0.0}, primary()),
        row("fx.pressure_cmh2o", "input", "cmH2O", {"point": 1019.716}, primary()),
        row("fx.pressure_mmhg", "input", "mmHg", {"point": 750.062}, primary()),
        row("fx.rate", "anchor", "/min", {"mean": 60.0, "sd": 1.0}, primary()),
        row("fx.share", "input", "%", {"point": 50}, compiled()),
        row("fx.stroke", "input", "mL", {"low": 90.0, "high": 110.0, "coverage": 0.95}, primary()),
        row("fx.temp", "input", "degC", {"point": 37.0}, primary()),
        row("fx.temp_kelvin", "input", "K", {"point": 310.15}, primary()),
        row("fx.temp_near", "input", "degC", {"point": 37.2}, primary()),
        row("fx.tiny", "input", "mL", {"point": 2e-06}, identity("fx.stroke * 0.00000002")),
        row("fx.twice", "constant", "1", {"point": 2.0}, primary()),
    ]
    ranges = [
        row("fx.amount", "within_subject_variation", "mL", {"point": 1.0}, dict(inferred="inf.flows")),
        row("fx.narrowed", "healthy_range", "L/min", {"low": 5.5, "high": 6.5, "coverage": 0.95}, dict(inferred="inf.adjust")),
        row("fx.span", "healthy_range", "L/min", {"low": 5.0, "high": 7.0, "coverage": 0.95}, primary()),
    ]
    works = [
        dict(id="fixture-book", kind="textbook", citation="Fixture.", isbn="0", edition="1"),
        dict(id="fixture-paper", kind="primary", citation="Fixture.", doi="10.0/fixture"),
        dict(id="fixture-review", kind="review", citation="Fixture.", doi="10.0/review"),
    ]
    inferences = [
        dict(id="inf.adjust", target="fx.narrowed", form="adjust", model="fixture", population="fixture", low=5.5,
             high=6.5, steps=["a step"], premises=["fx.span", "fx.rate"], search=["a search"]),
        dict(id="inf.fill", target="fx.guess", form="fill", model="fixture", population="fixture", low=1, high=2,
             steps=["a step"], premises=["fx.rate"], search=["a search"]),
        dict(id="inf.flows", target="fx.amount", form="flows", model="fixture", population="fixture", low=0.5, high=1.5,
             steps=["a step"], premises=["fx.rate"], search=["a search"], unmeasurable="no living-person method"),
    ]
    packet.sort(key=lambda r: r["id"])
    closures = [
        dict(id="XC-01", level="packet", claim="flow", lhs="fx.flow", rhs="fx.rate * fx.stroke", tol_rel=0.05),
        dict(id="XC-02", level="source", claim="heat", lhs="fx.temp", rhs="fx.temp_near", tol_abs=0.5, tol_unit="degC"),
        dict(id="XC-03", level="source", claim="mmHg", lhs="fx.pressure_mmhg", rhs="fx.kilopascals", tol_rel=1e-6),
        dict(id="XC-04", level="source", claim="cmH2O", lhs="fx.pressure_cmh2o", rhs="fx.kilopascals", tol_rel=1e-6),
        dict(id="XC-05", level="source", claim="kcal", lhs="fx.kilocalories", rhs="fx.kilojoules", tol_rel=1e-9),
        dict(id="XC-06", level="source", claim="kelvin", lhs="fx.temp", rhs="fx.temp_kelvin", tol_abs=0.01, tol_unit="K"),
    ]
    return Store(works, packet, ranges, inferences, closures, {".scratch/fixture.md": ["fx.*", "fx.rate"]})


def on(s: Store, rid: str) -> dict:
    return s.rows[rid]


def inference(s: Store, iid: str) -> dict:
    return next(i for i in s.inferences if i["id"] == iid)


def _extra_field(s: Store) -> None:
    on(s, "fx.rate")["sentence"] = "a field the language does not have"


def _empty_evidence(s: Store) -> None:
    on(s, "fx.rate")["evidence"] = {}


def _evidence_extra(s: Store) -> None:
    on(s, "fx.rate")["evidence"]["source_text"] = "a passage"


def _calibrated_to_input(s: Store) -> None:
    on(s, "fx.fitted")["evidence"]["calibrated"] = ["fx.stroke"]


def _value_form(s: Store) -> None:
    on(s, "fx.rate")["value"] = {"mean": 60.0}


def _kind(s: Store) -> None:
    on(s, "fx.span")["kind"] = "anchor"


def _work_extra(s: Store) -> None:
    s.works[0]["abstract"] = "a passage"


def _closure_extra(s: Store) -> None:
    s.closures[0]["note"] = "a passage"


def _id_first_digit(s: Store) -> None:
    on(s, "fx.area")["id"] = "1.area"


def _unsorted(s: Store) -> None:
    s.packet.reverse()


def _shared_id(s: Store) -> None:
    s.ranges.append(dict(s.ranges[-1], id="fx.base"))
    s.ranges.sort(key=lambda r: r["id"])


def _long_text(s: Store) -> None:
    on(s, "fx.rate")["method"] = "x" * 121


def _em_dash(s: Store) -> None:
    on(s, "fx.rate")["method"] = "cuff — arm"


def _long_step(s: Store) -> None:
    inference(s, "inf.fill")["steps"] = ["x" * 241]


def _bad_unit(s: Store) -> None:
    on(s, "fx.rate")["unit"] = "furlong"


def _celsius_power(s: Store) -> None:
    on(s, "fx.temp")["unit"] = "degC^1"


def _no_work(s: Store) -> None:
    on(s, "fx.rate")["evidence"]["source"] = "no-such-work"


def _no_identity_work(s: Store) -> None:
    on(s, "fx.flow")["evidence"]["identity"] = "no-such-work"


def _work_without_id(s: Store) -> None:
    del s.works[1]["doi"]


def _capped_high(s: Store) -> None:
    on(s, "fx.base")["evidence"]["certainty"] = "high"


def _tier_relabel(s: Store) -> None:
    e = on(s, "fx.base")["evidence"]
    e["tier"] = "primary_measured"
    del e["upgrade"]


def _no_upgrade(s: Store) -> None:
    del on(s, "fx.share")["evidence"]["upgrade"]


def _bad_tier_word(s: Store) -> None:
    on(s, "fx.rate")["evidence"]["tier"] = "gold"


def _book_no_page(s: Store) -> None:
    on(s, "fx.base")["evidence"]["locator"] = "somewhere"


def _book_no_edition(s: Store) -> None:
    del s.works[0]["edition"]


def _compiled_no_table(s: Store) -> None:
    on(s, "fx.share")["evidence"]["locator"] = "somewhere"


def _infinite(s: Store) -> None:
    on(s, "fx.rate")["value"]["mean"] = float("inf")


def _negative_sd(s: Store) -> None:
    on(s, "fx.rate")["value"]["sd"] = -1.0


def _backward_ci(s: Store) -> None:
    on(s, "fx.rate")["value"] = {"mean": 60.0, "ci_low": 100.0, "ci_high": 1.0}


def _backward_range(s: Store) -> None:
    on(s, "fx.span")["value"]["low"] = 8.0


def _range_point(s: Store) -> None:
    on(s, "fx.span")["value"] = {"point": 6.0}


def _range_half(s: Store) -> None:
    on(s, "fx.span")["value"]["coverage"] = 0.5


def _wrong_derived(s: Store) -> None:
    on(s, "fx.flow")["value"] = {"point": 9.0}


def _wrong_percent(s: Store) -> None:
    on(s, "fx.half")["value"] = {"point": 300.0}


def _wrong_tiny(s: Store) -> None:
    on(s, "fx.tiny")["value"] = {"point": 1e-06}


def _derived_range(s: Store) -> None:
    on(s, "fx.flow")["value"] = {"low": 5.0, "high": 7.0, "coverage": 0.95}


def _reads_the_set(s: Store) -> None:
    e = on(s, "fx.half")["evidence"]
    e["derived"] = "fx.span * fx.share"
    on(s, "fx.half")["value"] = {"point": 3.0}


def _adds_unlike(s: Store) -> None:
    on(s, "fx.flow")["evidence"]["derived"] = "fx.index * fx.area + fx.none"


def _nonfinite_derived(s: Store) -> None:
    on(s, "fx.flow")["evidence"]["derived"] = "(1e309 - 1e309) * fx.index * fx.area"


def _missing_input(s: Store) -> None:
    on(s, "fx.flow")["evidence"]["derived"] = "fx.index * fx.nothing"


def _wrong_dimension(s: Store) -> None:
    on(s, "fx.flow")["unit"] = "kg"


def _cycle(s: Store) -> None:
    on(s, "fx.flow")["evidence"]["derived"] = "fx.half / fx.share"
    on(s, "fx.half")["evidence"]["derived"] = "fx.flow * fx.share"


def _self_cycle(s: Store) -> None:
    on(s, "fx.flow")["evidence"]["derived"] = "fx.flow"


def _fitted_to_itself(s: Store) -> None:
    on(s, "fx.fitted")["evidence"]["calibrated"] = ["fx.flow"]
    on(s, "fx.flow")["evidence"]["derived"] = "fx.fitted / fx.fitted * fx.index * fx.area"


def _inferred_near(s: Store) -> None:
    on(s, "fx.area")["evidence"] = dict(inferred="inf.fill")
    inference(s, "inf.fill")["target"] = "fx.area"


def _inferred_far(s: Store) -> None:
    on(s, "fx.base")["evidence"] = dict(inferred="inf.fill")
    inference(s, "inf.fill")["target"] = "fx.base"


def _inferred_calibrated(s: Store) -> None:
    on(s, "fx.index")["evidence"] = dict(calibrated=["fx.rate"])
    on(s, "fx.rate")["evidence"] = dict(inferred="inf.fill")
    inference(s, "inf.fill")["target"] = "fx.rate"


def _no_window(s: Store) -> None:
    on(s, "fx.rate")["window"] = ""


def _no_method(s: Store) -> None:
    on(s, "fx.rate")["method"] = ""


def _no_search(s: Store) -> None:
    inference(s, "inf.fill")["search"] = []


def _own_premise(s: Store) -> None:
    inference(s, "inf.adjust")["premises"] = ["fx.narrowed", "fx.span"]


def _inferred_premise(s: Store) -> None:
    inference(s, "inf.adjust")["premises"] = ["fx.span", "fx.guess"]


def _fill_on_range(s: Store) -> None:
    inference(s, "inf.adjust")["form"] = "fill"


def _flows_on_range(s: Store) -> None:
    inference(s, "inf.adjust").update(form="flows", unmeasurable="no method")


def _flows_no_reason(s: Store) -> None:
    del inference(s, "inf.flows")["unmeasurable"]


def _one_premise(s: Store) -> None:
    inference(s, "inf.adjust")["premises"] = ["fx.span"]


def _other_target(s: Store) -> None:
    inference(s, "inf.fill")["target"] = "fx.area"


def _no_such_inference(s: Store) -> None:
    on(s, "fx.guess")["evidence"]["inferred"] = "inf.nothing"


def _relative_gap(s: Store) -> None:
    on(s, "fx.rate")["value"] = {"mean": 180.0, "sd": 1.0}


def _absolute_gap(s: Store) -> None:
    on(s, "fx.temp_near")["value"] = {"point": 39.0}


def _no_tolerance(s: Store) -> None:
    del s.closures[0]["tol_rel"]


def _nonfinite_closure(s: Store) -> None:
    s.closures[0]["lhs"] = "(1e309 - 1e309) * fx.flow"


def _unlike_sides(s: Store) -> None:
    s.closures[0]["rhs"] = "fx.rate"


def _bad_level(s: Store) -> None:
    s.closures[0]["level"] = "gut"


def _unknown_name(s: Store) -> None:
    s.tickets[".scratch/fixture.md"] = ["no.such.record"]


def _unknown_prefix(s: Store) -> None:
    s.tickets[".scratch/fixture.md"] = ["no.such.*"]


def _malformed_name(s: Store) -> None:
    s.tickets[".scratch/fixture.md"] = ["not an id"]


@rule(
    "data.schema", _extra_field, _empty_evidence, _evidence_extra, _calibrated_to_input, _value_form, _kind, _work_extra,
    _closure_extra, _id_first_digit,
)
def schema(s: Store) -> list[Violation]:
    code, out = "data.schema", []
    rows = s.rows
    for row in s.all_rows:
        subject = str(row.get("id"))
        if not ID.match(subject):
            out.append(Violation(code, subject, "an id is lowercase, dotted, at least two segments, and starts with a letter"))
        if missing := ROW_FIELDS - row.keys():
            out.append(Violation(code, subject, f"add {' '.join(sorted(missing))}"))
        if extra := row.keys() - ROW_FIELDS - {"n"}:
            out.append(Violation(code, subject, f"drop {' '.join(sorted(extra))}: the language has no such field"))
        value = row.get("value")
        if not isinstance(value, dict) or set(value) not in VALUE_FORMS:
            out.append(Violation(code, subject, f"value {value} is not one of the forms"))
        form = evidence_form(row.get("evidence"))
        if form is None:
            out.append(Violation(code, subject, "evidence is one closed form: source, derived, inferred or calibrated"))
        elif form == "calibrated" and (
            row.get("kind") != "parameter"
            or not row["evidence"]["calibrated"]
            or any(rows.get(a, {}).get("kind") != "anchor" for a in row["evidence"]["calibrated"])
        ):
            out.append(Violation(code, subject, "a calibrated parameter names the anchors it is fitted to"))
    for rows_, kinds, name in ((s.packet, PACKET_KINDS, "packet"), (s.ranges, SET_KINDS, "set")):
        for row in rows_:
            if row.get("kind") not in kinds:
                out.append(Violation(code, str(row.get("id")), f"kind {row.get('kind')} is not a {name} kind"))
    for table, fields, name in ((s.works, WORK_FIELDS, "work"), (s.inferences, INFERENCE_FIELDS | {"unmeasurable"}, "inference")):
        for item in table:
            if extra := item.keys() - fields:
                out.append(Violation(code, str(item.get("id")), f"a {name} has no field {' '.join(sorted(extra))}"))
    for c in s.closures:
        fields = CLOSURE_FIELDS | {"tol_rel", "tol_abs", "tol_unit"}
        if extra := c.keys() - fields:
            out.append(Violation(code, str(c.get("id")), f"a closure has no field {' '.join(sorted(extra))}"))
    return out


@rule("data.order", _unsorted, _shared_id)
def order(s: Store) -> list[Violation]:
    code, out = "data.order", []
    for name, rows in (("packet-reference-man", s.packet), ("healthy-range-set", s.ranges)):
        ids = [r.get("id", "") for r in rows]
        if ids != sorted(set(ids)):
            out.append(Violation(code, name, "sort rows by id and keep ids unique, so a rebuild is byte equal"))
    shared = {r.get("id", "") for r in s.packet} & {r.get("id", "") for r in s.ranges}
    out += [Violation(code, str(i), "one id in both the packet and the set; physiology never reads the set") for i in sorted(shared)]
    return out


@rule("data.text", _long_text, _em_dash, _long_step)
def text(s: Store) -> list[Violation]:
    code, out = "data.text", []
    items = [*s.all_rows, *s.works, *s.inferences, *s.closures]
    for item in items:
        subject = str(item.get("id"))
        for field, cap in CAPS.items():
            v = item.get(field)
            if not isinstance(v, str):
                v = item.get("evidence", {}).get(field)
            if isinstance(v, str) and len(v) > cap:
                out.append(Violation(code, subject, f"{field} is over {cap} characters"))
        for t in strings(item):
            if len(t) > TEXT_LIMIT:
                out.append(Violation(code, subject, f"text is over {TEXT_LIMIT} characters; source text stays local"))
            if "—" in t:
                out.append(Violation(code, subject, "no em dash"))
    return out


@rule("data.unit", _bad_unit, _celsius_power)
def unit(s: Store) -> list[Violation]:
    out = []
    for row in s.all_rows:
        try:
            parse_unit(row.get("unit", ""))
        except ValueError as e:
            out.append(Violation("data.unit", str(row.get("id")), f"{e}; a new unit enters UNITS with its exact factor"))
    return out


@rule("data.work", _no_work, _no_identity_work, _work_without_id)
def work(s: Store) -> list[Violation]:
    code, out = "data.work", []
    for w in s.works:
        subject = str(w.get("id"))
        if not WORK_ID.match(subject):
            out.append(Violation(code, subject, "a work id is lowercase and hyphenated"))
        if w.get("kind") not in WORK_TIERS:
            out.append(Violation(code, subject, f"kind {w.get('kind')} is not a work kind"))
        if not any(w.get(k) for k in WORK_IDS):
            out.append(Violation(code, subject, f"name one of {' '.join(WORK_IDS)}"))
    works = s.work_ids
    for row in s.all_rows:
        e = row.get("evidence", {})
        for field in ("source", "identity"):
            if field in e and e[field] not in works:
                out.append(Violation(code, str(row.get("id")), f"{e[field]} is not in works.jsonl"))
    return out


@rule("data.tier", _capped_high, _tier_relabel, _no_upgrade, _bad_tier_word)
def tier(s: Store) -> list[Violation]:
    code, out = "data.tier", []
    works = s.work_ids
    for row in s.all_rows:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        if evidence_form(e) != "source":
            continue
        if e.get("tier") not in TIERS or e.get("reading") not in READINGS or e.get("certainty") not in CERTAINTY:
            out.append(Violation(code, subject, "tier, reading and certainty take the contract's values"))
            continue
        capped = e["tier"] in ("compiled", "textbook")
        if capped and e["certainty"] == "high":
            out.append(Violation(code, subject, "a compiled or textbook record caps certainty at moderate"))
        if capped != ("upgrade" in e):
            out.append(Violation(code, subject, "upgrade is on a compiled or textbook record and only there"))
        kind = works.get(e.get("source"), {}).get("kind")
        if e["reading"] != "secondary_reproduction" and kind in WORK_TIERS and e["tier"] not in WORK_TIERS[kind]:
            out.append(Violation(code, subject, f"a {kind} work cannot carry tier {e['tier']}"))
    return out


@rule("data.locator", _book_no_page, _book_no_edition, _compiled_no_table)
def locator(s: Store) -> list[Violation]:
    code, out = "data.locator", []
    works = s.work_ids
    for row in s.all_rows:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        if evidence_form(e) != "source":
            continue
        at = e.get("locator", "")
        if not at:
            out.append(Violation(code, subject, "a source record names its table or page"))
        if e.get("tier") == "textbook" and not (PAGE.search(at) and works.get(e.get("source"), {}).get("edition")):
            out.append(Violation(code, subject, "a textbook record names a page, and its work an edition"))
        if e.get("tier") == "compiled" and not TABLE_OR_PAGE.search(at):
            out.append(Violation(code, subject, "a compiled record names its table or page"))
    return out


@rule("data.number", _infinite, _negative_sd, _backward_ci, _backward_range, _range_point, _range_half)
def number(s: Store) -> list[Violation]:
    code, out = "data.number", []
    for row in s.all_rows:
        v, subject = row.get("value", {}), str(row.get("id"))
        if not all(math.isfinite(x) for x in numbers(v)):
            out.append(Violation(code, subject, "every number is finite"))
            continue
        if any(v.get(k, 0) < 0 for k in ("sd", "se")):
            out.append(Violation(code, subject, "a spread is not negative"))
        if "ci_low" in v and not v["ci_low"] <= v["mean"] <= v["ci_high"]:
            out.append(Violation(code, subject, "an interval holds its centre"))
        if "low" in v and not v["low"] <= central(v) <= v["high"]:
            out.append(Violation(code, subject, "an interval holds its centre"))
        if "coverage" in v and not 0 < v["coverage"] <= 1:
            out.append(Violation(code, subject, "coverage is a fraction"))
        if row.get("kind") == "healthy_range" and v.get("coverage") != 0.95:
            out.append(Violation(code, subject, "a healthy range is a central 95% (record 0004)"))
    return out


@rule(
    "data.derived", _wrong_derived, _wrong_percent, _wrong_tiny, _derived_range, _reads_the_set, _adds_unlike,
    _nonfinite_derived, _missing_input,
)
def derived(s: Store) -> list[Violation]:
    code, out = "data.derived", []
    in_packet = {r.get("id") for r in s.packet}
    packet_ids = {id(r) for r in s.packet}
    for row in s.all_rows:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        if evidence_form(e) != "derived":
            continue
        try:
            got = value_of(e["derived"], s.quantity)
            want = si(central(row["value"]), row["unit"])
            factor = float(parse_unit(row["unit"])[0])
            names = names_in(e["derived"])
        except (KeyError, ValueError, SyntaxError) as err:
            out.append(Violation(code, subject, f"cannot evaluate: {err}"))
            continue
        if "point" not in row["value"]:
            out.append(Violation(code, subject, "a derived value is a point"))
            continue
        if id(row) in packet_ids and (read := sorted(names - in_packet)):
            out.append(Violation(code, subject, f"a packet value reads the set through {read[0]} (record 0005)"))
        if got.dimension == want.dimension:
            if abs(got.value - want.value) > last_digit_tolerance(central(row["value"])) * factor * (1 + 1e-9):
                out.append(Violation(code, subject, f"the identity gives {got.value / factor:.6g} {row['unit']}"))
    return out


@rule("data.dimension", _wrong_dimension)
def dimension(s: Store) -> list[Violation]:
    code, out = "data.dimension", []
    for row in s.all_rows:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        if evidence_form(e) != "derived":
            continue
        try:
            got = value_of(e["derived"], s.quantity)
            want = si(central(row["value"]), row["unit"])
        except (KeyError, ValueError, SyntaxError):
            continue
        if got.dimension != want.dimension:
            out.append(Violation(code, subject, f"the identity has dimension {got.dimension}, the unit {want.dimension}"))
    return out


@rule("data.cycle", _cycle, _self_cycle, _fitted_to_itself)
def cycle(s: Store) -> list[Violation]:
    rows = s.rows
    done: set[str] = set()
    out = []

    def walk(name: str, path: tuple[str, ...]) -> None:
        if name in path:
            out.append(Violation("data.cycle", name, f"a record rests on itself through {' '.join(path)}"))
            return
        if name in done or name not in rows:
            return
        for next_name in sorted(rests_on(rows[name])):
            walk(next_name, (*path, name))
        done.add(name)

    for name in sorted(rows):
        walk(name, ())
    return out


@rule("data.anchor", _inferred_near, _inferred_far, _inferred_calibrated)
def anchor(s: Store) -> list[Violation]:
    out = []
    for row in s.packet:
        if row.get("kind") == "anchor" and inferred_under(s, row):
            out.append(Violation("data.anchor", str(row.get("id")), "an anchor never rests on an inference (rule 16)"))
    return out


@rule("data.snapshot", _no_window, _no_method)
def snapshot(s: Store) -> list[Violation]:
    out = []
    for row in s.packet:
        if row.get("kind") == "anchor" and not (row.get("method") and row.get("window")):
            out.append(Violation("data.snapshot", str(row.get("id")), "an anchor states its method and window (rule 17)"))
    return out


@rule(
    "data.inference", _no_search, _own_premise, _inferred_premise, _fill_on_range, _flows_on_range, _flows_no_reason,
    _one_premise, _other_target, _no_such_inference,
)
def inferred(s: Store) -> list[Violation]:
    code, out = "data.inference", []
    rows = s.rows
    by_id = {i.get("id"): i for i in s.inferences}
    for i in s.inferences:
        subject = str(i.get("id"))
        for field in INFERENCE_FIELDS:
            if field not in i or i[field] in ("", [], None):
                out.append(Violation(code, subject, f"an inference record names its {field}"))
        form = i.get("form")
        if form not in FORMS:
            out.append(Violation(code, subject, f"form {form} is not one of {sorted(FORMS)}"))
        premises = i.get("premises", [])
        for p in premises:
            if p not in rows:
                out.append(Violation(code, subject, f"premise {p} is not a record"))
            elif inferred_under(s, rows[p]):
                out.append(Violation(code, subject, f"premise {p} is not sourced"))
        if form == "adjust" and len(premises) < 2:
            out.append(Violation(code, subject, "an adjustment rests on a sourced value and a sourced shift"))
        if form == "flows" and not i.get("unmeasurable"):
            out.append(Violation(code, subject, "state why no method can measure the amount"))
    for row in s.all_rows:
        ref, subject = row.get("evidence", {}).get("inferred"), str(row.get("id"))
        if ref is None:
            continue
        i = by_id.get(ref)
        if i is None:
            out.append(Violation(code, subject, f"{ref} is not in inferences.jsonl"))
            continue
        if i.get("target") != subject:
            out.append(Violation(code, subject, f"{ref} targets {i.get('target')}"))
        form, kind = i.get("form"), str(row.get("kind"))
        allowed = {"parameter": {"fill"}, "day": {"fill"}, "healthy_range": {"adjust"}, "within_subject_variation": {"adjust", "flows"}}
        if form not in allowed.get(kind, set()):
            out.append(Violation(code, subject, f"a {kind} cannot rest on a {form} inference (rule 16)"))
    return out


@rule("data.closure", _relative_gap, _absolute_gap, _no_tolerance, _nonfinite_closure, _unlike_sides, _bad_level)
def closure(s: Store) -> list[Violation]:
    code, out = "data.closure", []
    for c in s.closures:
        subject = str(c.get("id"))
        if c.get("level") not in LEVELS:
            out.append(Violation(code, subject, f"level is one of {sorted(LEVELS)}"))
        rel, abs_ = "tol_rel" in c, "tol_abs" in c and "tol_unit" in c
        if rel == abs_:
            out.append(Violation(code, subject, "state exactly one tolerance: tol_rel, or tol_abs with tol_unit"))
            continue
        try:
            lhs = value_of(c["lhs"], s.quantity)
            rhs = value_of(c["rhs"], s.quantity)
            tol = si_difference(c["tol_abs"], c["tol_unit"]) if abs_ else None
        except (KeyError, ValueError, SyntaxError) as err:
            out.append(Violation(code, subject, f"cannot evaluate: {err}"))
            continue
        if lhs.dimension != rhs.dimension or (tol and tol.dimension != lhs.dimension):
            out.append(Violation(code, subject, "the sides and the tolerance differ in dimension"))
        elif abs(lhs.value - rhs.value) > (tol.value if tol else c["tol_rel"] * abs(rhs.value)):
            out.append(Violation(code, subject, f"{lhs.value:.6g} against {rhs.value:.6g} is past its tolerance"))
    return out


@rule("data.ticket", _unknown_name, _unknown_prefix, _malformed_name)
def ticket(s: Store) -> list[Violation]:
    code, out = "data.ticket", []
    rows = s.rows
    for path, names in s.tickets.items():
        for name in names:
            if PREFIX.match(name):
                if not any(i.startswith(name[:-1]) for i in rows):
                    out.append(Violation(code, path, f"Data: {name} matches no record"))
            elif not ID.match(name):
                out.append(Violation(code, path, f"Data: {name!r} is neither a record id nor a prefix ending in .*"))
            elif name not in rows:
                out.append(Violation(code, path, f"Data: {name} names no record"))
    return out


def run(s: Store) -> list[Violation]:
    return [v for _, check, _ in RULES for v in check(s)]


def probes() -> list[Violation]:
    if base := run(synthetic_store()):
        return [Violation("data.probe", "base", "a probe proves nothing on a base that already fails"), *base]
    out = []
    for code, _, broken_by in RULES:
        for probe in broken_by:
            s = copy.deepcopy(synthetic_store())
            probe(s)
            if not any(v.code == code for v in run(s)):
                out.append(Violation("data.probe", f"{code} {probe.__name__}", "the rule did not refuse this record"))
    return out


def main(argv: list[str]) -> int:
    if len(argv) != 2 or argv[1] not in ("check", "probes"):
        print(__doc__)
        return 2
    if argv[1] == "probes":
        found = probes()
        for v in found:
            print(v)
        total = sum(len(broken_by) for _, _, broken_by in RULES)
        print(f"data: {len(RULES)} rules, {total} probes, {len(found)} did not bite")
        return 1 if found else 0
    try:
        store = load(ROOT)
    except DataError as e:
        print(f"data: {e}")
        return 2
    found = run(store)
    for v in found:
        print(v)
    print(f"data: {len(store.rows)} records, {len(store.works)} works, {len(store.closures)} closures, {len(found)} violations")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
