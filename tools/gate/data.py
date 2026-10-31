"""The research data's rules: what a public record may hold, and that curated values close.

Python 3.11 stdlib only. Run as `python3 -I tools/gate/data.py <command>`; it finds the repo from its own path.

    check    every rule over data/ and every Data: line in .scratch/
    probes   every rule refuses a record built to break it, on a synthetic base held here

Exit codes: 0 pass; 1 violations, one per line as `<code>: <subject>: <what to do>`; 2 an input the check
reads is missing or malformed.
"""

from __future__ import annotations

import ast
import copy
import dataclasses
import json
import math
import pathlib
import re
import sys
from collections.abc import Callable
from fractions import Fraction

ROOT = pathlib.Path(__file__).resolve().parents[2]
DATA = "data"
FILES = ("works", "packet-reference-man", "healthy-range-set", "inferences", "closure")
ID = re.compile(r"^[a-z0-9]+(\.[a-z0-9_]+)+$")
NAME = re.compile(r"[a-z][a-z0-9_]*(?:\.[a-z0-9_]+)+")
WORK_ID = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")
CAPS = {"population": 120, "conditions": 120, "method": 120, "window": 60, "locator": 80, "citation": 240}
PACKET_KINDS = {"anchor", "parameter", "day", "constant", "input"}
SET_KINDS = {"healthy_range", "within_subject_variation"}
TIERS = {"primary_measured", "primary_fitted", "compiled", "textbook"}
CERTAINTY = {"high", "moderate", "low", "very_low"}
READINGS = {"full_text", "abstract", "secondary_reproduction"}
WORK_KINDS = {"primary", "dataset", "review", "guideline", "compilation", "textbook", "standard"}
WORK_IDS = ("doi", "pmid", "pmcid", "isbn", "dataset", "standard")
FORMS = {"fill", "adjust", "flows"}
VALUE_FORMS = (
    {"point"},
    {"mean", "sd"},
    {"mean", "se"},
    {"median", "low", "high", "coverage"},
    {"low", "high", "coverage"},
    {"mean", "ci_low", "ci_high"},
)

Dimension = tuple[int, ...]
DIMENSIONS = ("m", "kg", "s", "mol", "K", "A")
NUMBER: Dimension = (0, 0, 0, 0, 0, 0)


def dim(m: int = 0, kg: int = 0, s: int = 0, mol: int = 0, K: int = 0, A: int = 0) -> Dimension:
    return (m, kg, s, mol, K, A)


VOLUME, PRESSURE, ENERGY, POWER = dim(m=3), dim(kg=1, m=-1, s=-2), dim(kg=1, m=2, s=-2), dim(kg=1, m=2, s=-3)
MMHG = Fraction("133.322387415")
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
    "mmHg": (MMHG, PRESSURE),
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
    pos, factor, power_sum, first = 0, Fraction(1), [0] * len(DIMENSIONS), True
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
        power_sum = [a + s * b for a, b in zip(power_sum, d)]
        pos, first = m.end(), False
    if first:
        raise ValueError("empty unit")
    return factor, tuple(power_sum)


@dataclasses.dataclass(frozen=True)
class Quantity:
    value: float
    dimension: Dimension = NUMBER


def si(value: float, unit: str) -> Quantity:
    factor, dimension = parse_unit(unit)
    return Quantity(value + 273.15 if unit == "degC" else value * float(factor), dimension)


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
    return evaluate(tree, names, env)


def central(value: dict) -> float:
    for k in ("point", "mean", "median"):
        if k in value:
            return value[k]
    return (value["low"] + value["high"]) / 2


def decimals(x: float) -> int:
    text = repr(x)
    return len(text.split(".")[1]) if "." in text and "e" not in text else 0


@dataclasses.dataclass
class Store:
    works: dict[str, dict]
    rows: dict[str, dict]
    packet: list[dict]
    ranges: list[dict]
    inferences: dict[str, dict]
    closures: list[dict]
    tickets: dict[str, list[str]]

    def quantity(self, name: str) -> Quantity:
        if name not in self.rows:
            raise KeyError(name)
        row = self.rows[name]
        return si(central(row["value"]), row["unit"])


def read_jsonl(path: pathlib.Path) -> list[dict]:
    rows = []
    for i, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            rows.append(json.loads(line))
        except json.JSONDecodeError as e:
            raise DataError(f"{path}:{i}: {e}") from e
    return rows


def load(root: pathlib.Path) -> Store:
    tables = {}
    for name in FILES:
        path = root / DATA / f"{name}.jsonl"
        if not path.exists():
            raise DataError(f"{path} is missing")
        tables[name] = read_jsonl(path)
    packet, ranges = tables["packet-reference-man"], tables["healthy-range-set"]
    tickets = {}
    for md in sorted((root / ".scratch").rglob("*.md")):
        lines = md.read_text(encoding="utf-8").splitlines()
        names = [n.strip() for ln in lines if ln.startswith("Data: ") for n in ln[6:].split(",")]
        if names:
            tickets[str(md.relative_to(root))] = names
    return Store(
        {w.get("id", ""): w for w in tables["works"]},
        {r.get("id", ""): r for r in [*packet, *ranges]},
        packet,
        ranges,
        {i.get("id", ""): i for i in tables["inferences"]},
        tables["closure"],
        tickets,
    )


Rule = Callable[[Store], list[Violation]]
RULES: list[tuple[str, Rule, Callable[[Store], None]]] = []


def rule(code: str, probe: Callable[[Store], None]):
    def register(check: Rule) -> Rule:
        RULES.append((code, check, probe))
        return check

    return register


def strings(row: object):
    if isinstance(row, dict):
        for v in row.values():
            yield from strings(v)
    elif isinstance(row, list):
        for v in row:
            yield from strings(v)
    elif isinstance(row, str):
        yield row


def synthetic_store() -> Store:
    def row(rid: str, kind: str, unit: str, value: dict, evidence: dict) -> dict:
        return dict(id=rid, kind=kind, unit=unit, value=value, population="fixture", conditions="fixture",
                    method="fixture", window="fixture", evidence=evidence)

    primary = dict(source="fixture-paper", locator="Table 1", reading="full_text", tier="primary_measured", certainty="low")
    book = dict(source="fixture-book", locator="ch.1 p.1", reading="full_text", tier="textbook", certainty="moderate", upgrade=True)
    packet = [
        row("fx.area", "input", "m^2", {"point": 2.0}, primary),
        row("fx.flow", "anchor", "L/min", {"point": 6.0}, dict(derived="fx.index * fx.area", identity="fixture-paper")),
        row("fx.index", "input", "L/min/m^2", {"point": 3.0}, book),
        row("fx.rate", "anchor", "/min", {"mean": 60.0, "sd": 1.0}, primary),
        row("fx.stroke", "input", "mL", {"low": 90.0, "high": 110.0, "coverage": 0.95}, primary),
    ]
    ranges = [row("fx.span", "healthy_range", "L/min", {"low": 5.0, "high": 7.0, "coverage": 0.95}, primary)]
    works = {
        "fixture-paper": dict(id="fixture-paper", kind="primary", citation="Fixture.", doi="10.0/fixture"),
        "fixture-book": dict(id="fixture-book", kind="textbook", citation="Fixture.", isbn="0", edition="1"),
    }
    inference = dict(id="inf.fixture", target="fx.stroke", form="fill", model="fixture", population="fixture", low=1,
                     high=2, steps=["a step"], premises=["fx.rate"], search=["a search"])
    closures = [dict(id="XC-01", level="packet", claim="flow", lhs="fx.flow", rhs="fx.rate * fx.stroke", tol_rel=0.05)]
    rows = {r["id"]: r for r in [*packet, *ranges]}
    return Store(works, rows, packet, ranges, {"inf.fixture": inference}, closures, {".scratch/fixture.md": ["fx.*"]})


def _field(s: Store) -> None:
    s.rows["fx.rate"]["sentence"] = "a field the language does not have"


def _text(s: Store) -> None:
    s.rows["fx.rate"]["method"] = "x" * 121


def _unit(s: Store) -> None:
    s.rows["fx.rate"]["unit"] = "furlong"


def _work(s: Store) -> None:
    s.rows["fx.rate"]["evidence"]["source"] = "no-such-work"


def _tier(s: Store) -> None:
    s.rows["fx.index"]["evidence"]["certainty"] = "high"


def _locator(s: Store) -> None:
    s.rows["fx.index"]["evidence"]["locator"] = "somewhere"


def _derived(s: Store) -> None:
    s.rows["fx.flow"]["value"] = {"point": 9.0}


def _dimension(s: Store) -> None:
    s.rows["fx.flow"]["unit"] = "kg"


def _anchor(s: Store) -> None:
    s.rows["fx.index"]["evidence"] = {"inferred": "inf.fixture"}


def _inference(s: Store) -> None:
    s.inferences["inf.fixture"]["search"] = []


def _range(s: Store) -> None:
    s.rows["fx.span"]["value"]["low"] = 8.0


def _closure(s: Store) -> None:
    s.rows["fx.rate"]["value"] = {"mean": 180.0, "sd": 1.0}


def _order(s: Store) -> None:
    s.packet.reverse()


def _ticket(s: Store) -> None:
    s.tickets[".scratch/fixture.md"] = ["no.such.record"]


def _method(s: Store) -> None:
    s.rows["fx.rate"]["window"] = ""


@rule("data.schema", _field)
def schema(s: Store) -> list[Violation]:
    code, out = "data.schema", []
    required = {"id", "kind", "unit", "value", "population", "conditions", "method", "window", "evidence"}
    optional = {"n"}
    for row in s.packet + s.ranges:
        subject = str(row.get("id"))
        if not ID.match(subject):
            out.append(Violation(code, subject, "an id is lowercase, dotted, at least two segments"))
        if missing := required - row.keys():
            out.append(Violation(code, subject, f"add {' '.join(sorted(missing))}"))
        if extra := row.keys() - required - optional:
            out.append(Violation(code, subject, f"drop {' '.join(sorted(extra))}: the language has no such field"))
        if row.keys() >= required and set(row["value"]) not in VALUE_FORMS:
            out.append(Violation(code, subject, f"value has fields {sorted(row['value'])}, not one of the forms"))
    for row in s.packet:
        if row.get("kind") not in PACKET_KINDS:
            out.append(Violation(code, str(row.get("id")), f"kind {row.get('kind')} is not a packet kind"))
    for row in s.ranges:
        if row.get("kind") not in SET_KINDS:
            out.append(Violation(code, str(row.get("id")), f"kind {row.get('kind')} is not a set kind"))
    return out


@rule("data.order", _order)
def order(s: Store) -> list[Violation]:
    out = []
    for name, rows in (("packet-reference-man", s.packet), ("healthy-range-set", s.ranges)):
        ids = [r.get("id", "") for r in rows]
        if ids != sorted(set(ids)):
            out.append(Violation("data.order", name, "sort rows by id and keep ids unique, so a rebuild is byte equal"))
    return out


@rule("data.text", _text)
def text(s: Store) -> list[Violation]:
    out = []
    for row in s.packet + s.ranges + list(s.works.values()):
        for field, cap in CAPS.items():
            v = row.get(field) or row.get("evidence", {}).get(field)
            if isinstance(v, str) and len(v) > cap:
                out.append(Violation("data.text", str(row.get("id")), f"{field} is over {cap} characters"))
        if any("—" in t for t in strings(row)):
            out.append(Violation("data.text", str(row.get("id")), "no em dash"))
    return out


@rule("data.unit", _unit)
def unit(s: Store) -> list[Violation]:
    out = []
    for row in s.packet + s.ranges:
        try:
            parse_unit(row.get("unit", ""))
        except ValueError as e:
            out.append(Violation("data.unit", str(row.get("id")), f"{e}; add it to UNITS with its exact factor"))
    return out


@rule("data.work", _work)
def work(s: Store) -> list[Violation]:
    code, out = "data.work", []
    for w in s.works.values():
        subject = str(w.get("id"))
        if not WORK_ID.match(subject):
            out.append(Violation(code, subject, "a work id is lowercase and hyphenated"))
        if w.get("kind") not in WORK_KINDS:
            out.append(Violation(code, subject, f"kind {w.get('kind')} is not a work kind"))
        if not any(w.get(k) for k in WORK_IDS):
            out.append(Violation(code, subject, f"name one of {' '.join(WORK_IDS)}"))
    for row in s.packet + s.ranges:
        e = row.get("evidence", {})
        cited = [e["source"]] if "source" in e else []
        if "derived" in e:
            cited.append(e.get("identity"))
        for c in cited:
            if c not in s.works:
                out.append(Violation(code, str(row.get("id")), f"{c} is not in works.jsonl"))
    return out


@rule("data.tier", _tier)
def tier(s: Store) -> list[Violation]:
    code, out = "data.tier", []
    for row in s.packet + s.ranges:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        if "source" not in e:
            continue
        if e.get("tier") not in TIERS or e.get("reading") not in READINGS or e.get("certainty") not in CERTAINTY:
            out.append(Violation(code, subject, "tier, reading and certainty take the contract's values"))
        capped = e.get("tier") in ("compiled", "textbook")
        if capped and e.get("certainty") == "high":
            out.append(Violation(code, subject, "a compiled or textbook record caps certainty at moderate"))
        if capped != ("upgrade" in e):
            out.append(Violation(code, subject, "upgrade is on a compiled or textbook record and only there"))
    return out


@rule("data.locator", _locator)
def locator(s: Store) -> list[Violation]:
    code, out = "data.locator", []
    for row in s.packet + s.ranges:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        at = e.get("locator", "")
        if "source" in e and not at:
            out.append(Violation(code, subject, "a source record names its table or page"))
        if e.get("tier") == "textbook" and not (
            re.search(r"\bp\.\s?\d+", at) and s.works.get(e.get("source"), {}).get("edition")
        ):
            out.append(Violation(code, subject, "a textbook record names a page, and its work an edition"))
        if e.get("tier") == "compiled" and not re.search(r"\b(table|tab|fig|figure|p|pp|para|section|sec)\b\.?", at, re.I):
            out.append(Violation(code, subject, "a compiled record names its table or page"))
    return out


@rule("data.derived", _derived)
def derived(s: Store) -> list[Violation]:
    code, out = "data.derived", []
    for row in s.packet + s.ranges:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        if "derived" not in e:
            continue
        try:
            got = value_of(e["derived"], s.quantity)
            want = si(central(row["value"]), row["unit"])
            factor = float(parse_unit(row["unit"])[0])
        except (KeyError, ValueError, SyntaxError) as err:
            out.append(Violation(code, subject, f"cannot evaluate: {err}"))
            continue
        if got.dimension != want.dimension:
            continue
        half = 0.5 * 10 ** -decimals(central(row["value"])) * factor
        if abs(got.value - want.value) > half * (1 + 1e-9):
            out.append(Violation(code, subject, f"the identity gives {got.value / factor:.6g} {row['unit']}"))
    return out


@rule("data.dimension", _dimension)
def dimension(s: Store) -> list[Violation]:
    code, out = "data.dimension", []
    for row in s.packet + s.ranges:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        if "derived" not in e:
            continue
        try:
            got = value_of(e["derived"], s.quantity)
            want = si(central(row["value"]), row["unit"])
        except (KeyError, ValueError, SyntaxError):
            continue
        if got.dimension != want.dimension:
            out.append(Violation(code, subject, f"the identity has dimension {got.dimension}, the unit {want.dimension}"))
    return out


def ancestry(s: Store, row: dict) -> set[str]:
    seen, todo = set(), [row]
    while todo:
        e = todo.pop().get("evidence", {})
        if "inferred" in e:
            seen.add(e["inferred"])
        if "derived" in e:
            todo += [s.rows[n] for n in names_in(e["derived"]) if n in s.rows]
    return seen


@rule("data.anchor", _anchor)
def anchor(s: Store) -> list[Violation]:
    out = []
    for row in s.packet:
        if row.get("kind") == "anchor" and ancestry(s, row):
            out.append(Violation("data.anchor", str(row.get("id")), "an anchor never rests on an inference (rule 16)"))
    return out


@rule("data.snapshot", _method)
def snapshot(s: Store) -> list[Violation]:
    out = []
    for row in s.packet:
        if row.get("kind") == "anchor" and not (row.get("method") and row.get("window")):
            out.append(Violation("data.snapshot", str(row.get("id")), "an anchor states its method and window (rule 17)"))
    return out


@rule("data.inference", _inference)
def inference(s: Store) -> list[Violation]:
    code, out = "data.inference", []
    for i in s.inferences.values():
        subject = str(i.get("id"))
        for field in ("model", "population", "steps", "premises", "search"):
            if not i.get(field):
                out.append(Violation(code, subject, f"an inference record names its {field}"))
        if i.get("form") not in FORMS:
            out.append(Violation(code, subject, f"form {i.get('form')} is not one of {sorted(FORMS)}"))
        for p in i.get("premises", []):
            if p not in s.rows:
                out.append(Violation(code, subject, f"premise {p} is not a record"))
    for row in s.packet + s.ranges:
        ref, subject = row.get("evidence", {}).get("inferred"), str(row.get("id"))
        if ref is None:
            continue
        i = s.inferences.get(ref)
        if i is None:
            out.append(Violation(code, subject, f"{ref} is not in inferences.jsonl"))
        elif row.get("kind") in SET_KINDS and i.get("form") == "fill":
            out.append(Violation(code, subject, "a target rests on inference only to adjust a sourced one or from flows"))
    return out


@rule("data.range", _range)
def value_range(s: Store) -> list[Violation]:
    out = []
    for row in s.packet + s.ranges:
        v = row.get("value", {})
        if "low" in v and not v["low"] <= central(v) <= v["high"]:
            out.append(Violation("data.range", str(row.get("id")), "low must not exceed high, nor the centre leave them"))
        if "coverage" in v and not 0 < v["coverage"] <= 1:
            out.append(Violation("data.range", str(row.get("id")), "coverage is a fraction"))
    return out


@rule("data.closure", _closure)
def closure(s: Store) -> list[Violation]:
    code, out = "data.closure", []
    for c in s.closures:
        subject = str(c.get("id"))
        try:
            lhs = value_of(c["lhs"], s.quantity)
            rhs = value_of(c["rhs"], s.quantity)
        except (KeyError, ValueError, SyntaxError) as err:
            out.append(Violation(code, subject, f"cannot evaluate: {err}"))
            continue
        if lhs.dimension != rhs.dimension:
            out.append(Violation(code, subject, f"sides differ in dimension: {lhs.dimension} {rhs.dimension}"))
        elif "tol_rel" in c and abs(lhs.value - rhs.value) > c["tol_rel"] * abs(rhs.value):
            out.append(Violation(code, subject, f"{lhs.value:.6g} against {rhs.value:.6g} is past {c['tol_rel']:.0%}"))
        elif "tol_abs" in c:
            tol = si(c["tol_abs"], c["tol_unit"])
            if tol.dimension != lhs.dimension or abs(lhs.value - rhs.value) > tol.value:
                out.append(Violation(code, subject, f"{lhs.value:.6g} against {rhs.value:.6g} is past {c['tol_abs']}"))
    return out


@rule("data.ticket", _ticket)
def ticket(s: Store) -> list[Violation]:
    out = []
    for path, names in s.tickets.items():
        for name in names:
            stem = name[:-2] if name.endswith(".*") else None
            hit = any(i.startswith(stem + ".") for i in s.rows) if stem else name in s.rows
            if not hit:
                out.append(Violation("data.ticket", path, f"Data: {name} names no record"))
    return out


def run(s: Store) -> list[Violation]:
    return [v for _, check, _ in RULES for v in check(s)]


def probes() -> list[Violation]:
    if base := run(synthetic_store()):
        return [Violation("data.probe", "base", "a probe proves nothing on a base that already fails"), *base]
    out = []
    for code, _, probe in RULES:
        broken = copy.deepcopy(synthetic_store())
        probe(broken)
        if not any(v.code == code for v in run(broken)):
            out.append(Violation("data.probe", code, "the rule did not refuse the record built to break it"))
    return out


def main(argv: list[str]) -> int:
    if len(argv) != 2 or argv[1] not in ("check", "probes"):
        print(__doc__)
        return 2
    if argv[1] == "probes":
        found = probes()
        for v in found:
            print(v)
        print(f"data: {len(RULES)} rules, {len(RULES) - len(found)} probes bite")
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
