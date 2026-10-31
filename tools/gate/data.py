"""The research data's rules: what a public record may hold, and that curated values close.

Python 3.11 stdlib only. Run as `python3 -I tools/gate/data.py <command>`; it finds the repo from its own path.

    check    every rule over data/ and every Data: line in .scratch/
    probes   every rule refuses each record built to break it, on a synthetic base held here

Exit codes: 0 pass; 1 violations, one per line as `<code>: <subject>: <what to do>`; 2 an input the check
reads is missing or malformed.
"""

from __future__ import annotations

import ast
import dataclasses
import decimal
import json
import math
import pathlib
import re
import sys
import tempfile
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
TABLE_OR_PAGE = re.compile(r"\b(table|tab|p|pp)\b\.?\s*\d", re.I)

CAPS = {"population": 120, "conditions": 120, "method": 120, "window": 60, "locator": 80}
TEXT_LIMIT = 240
ROW_FIELDS = {"id", "kind", "unit", "value", "population", "conditions", "method", "window", "evidence"}
WORK_FIELDS = {"id", "kind", "citation", "doi", "pmid", "pmcid", "isbn", "edition", "dataset", "standard", "query"}
INFERENCE_FIELDS = {"id", "target", "form", "model", "population", "low", "high", "steps", "premises", "search"}
CLOSURE_FIELDS = {"id", "level", "claim", "lhs", "rhs"}
PACKET_KINDS = {"anchor", "parameter", "day", "constant", "input"}
SET_KINDS = {"healthy_range", "within_subject_variation", "shift", "spread"}
BASES = {"percentiles", "mean_1_96_sd", "reference_interval"}
ABSENT = {"none", "n/a", "na", "not applicable", "not stated", "unknown"}
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
    "interval": ({"derived_low", "derived_high", "identity"}, set()),
    "inferred": ({"inferred"}, set()),
    "calibrated": ({"calibrated"}, set()),
}
FUNCTIONS = {"log10", "exp", "sum"}
LARGEST_EXACT = 2**53
LARGEST = 1e30
EXPRESSION = re.compile(r"[A-Za-z0-9_.+\-*/(), ]+")
NUMBER_TOKEN = re.compile(r"(?<![\w.])\d+\.?\d*(?:[eE][-+]?\d+)?")
BARE_NAME = re.compile(r"(?<![\w.])[A-Za-z_]\w*")

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
    if not Fraction(1, 10**100) < factor < 10**100:
        raise ValueError(f"the factor of {text!r} leaves double range")
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

    if not EXPRESSION.fullmatch(text):
        raise ValueError("an expression holds record ids, numbers, operators and functions only")
    rest = NAME.sub(" ", text)
    if unknown := set(BARE_NAME.findall(rest)) - FUNCTIONS:
        raise ValueError(f"{sorted(unknown)[0]} is neither a record nor a function")
    for token in NUMBER_TOKEN.findall(rest):
        if decimal.Decimal(repr(float(token))) != decimal.Decimal(token):
            raise ValueError(f"{token} is not exact as a double")
    return ast.parse(NAME.sub(hold, text), mode="eval"), names


def names_in(text: str) -> set[str]:
    return set(parse_expression(text)[1].values())


def evaluate(node: ast.AST, names: dict[str, str], env: Callable[[str], Quantity]) -> Quantity:
    q = step(node, names, env)
    if not math.isfinite(q.value):
        raise ValueError("an intermediate value is not finite")
    return q


def step(node: ast.AST, names: dict[str, str], env: Callable[[str], Quantity]) -> Quantity:
    if isinstance(node, ast.Expression):
        return evaluate(node.body, names, env)
    if isinstance(node, ast.Constant) and is_number(node.value):
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


def last_digit_tolerance(x: float) -> float:
    exponent = decimal.Decimal(repr(x)).as_tuple().exponent
    return 0.5 * 10.0 ** (exponent if isinstance(exponent, int) else 0)


def stated_values(row: dict) -> list[float]:
    ends = [row["value"][k] for k in ("low", "high", "ci_low", "ci_high") if k in row["value"]]
    return [central(row["value"]), *ends]


def well_formed(row: dict) -> bool:
    value = row.get("value")
    return isinstance(value, dict) and set(value) in VALUE_FORMS and all(map(is_number, value.values()))


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
    stray: list[str] = dataclasses.field(default_factory=list)
    not_canonical: list[str] = dataclasses.field(default_factory=list)

    @property
    def all_rows(self) -> list[dict]:
        return self.packet + self.ranges

    @property
    def rows(self) -> dict[str, dict]:
        return {str(r.get("id", "")): r for r in self.all_rows}

    @property
    def work_ids(self) -> dict[str, dict]:
        return {w.get("id", ""): w for w in self.works}

    def quantity(self, name: str) -> Quantity:
        row = self.rows[name]
        if not well_formed(row):
            raise ValueError(f"{name} has no usable value")
        return si(central(row["value"]), row["unit"])


def refuse_constant(name: str) -> None:
    raise ValueError(f"{name} is not a number")


def canonical(item: dict) -> str:
    return json.dumps(item, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def read_jsonl(path: pathlib.Path) -> tuple[list[dict], list[str]]:
    rows, not_canonical = [], []
    for i, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            rows.append(json.loads(line, parse_constant=refuse_constant))
        except ValueError as e:
            raise DataError(f"{path}:{i}: {e}") from e
        if not isinstance(rows[-1], dict):
            raise DataError(f"{path}:{i}: a row is an object")
        if canonical(rows[-1]) != line:
            not_canonical.append(f"{path.name}:{i}")
    return rows, not_canonical


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
    tables, not_canonical = {}, []
    for name in FILES:
        path = root / DATA / f"{name}.jsonl"
        if not path.exists():
            raise DataError(f"{path} is missing")
        tables[name], flagged = read_jsonl(path)
        not_canonical += flagged
    known = {f"{DATA}/{name}.jsonl" for name in FILES}
    found = {p.relative_to(root).as_posix() for p in (root / DATA).rglob("*") if p.is_file()}
    stray = sorted(found - known)
    return Store(
        tables["works"],
        tables["packet-reference-man"],
        tables["healthy-range-set"],
        tables["inferences"],
        tables["closure"],
        read_tickets(root),
        stray,
        not_canonical,
    )


def reads(row: dict) -> set[str]:
    e = row.get("evidence", {})
    try:
        if "derived" in e:
            return names_in(e["derived"])
        if "derived_low" in e:
            return names_in(e["derived_low"]) | names_in(e["derived_high"])
    except (SyntaxError, ValueError):
        pass
    return set()


def rests_on(row: dict) -> set[str]:
    fitted = row.get("evidence", {}).get("calibrated")
    named = {row["range"]} if is_text(row.get("range")) else set()
    return reads(row) | named | (set(fitted) if is_text_list(fitted) else set())


def under(s: Store, row: dict) -> list[dict]:
    rows, seen, found, todo = s.rows, set(), [], [row]
    while todo:
        r = todo.pop()
        found.append(r)
        for name in rests_on(r) - seen:
            seen.add(name)
            if name in rows:
                todo.append(rows[name])
    return found


def inferred_under(s: Store, row: dict) -> set[str]:
    return {r["evidence"]["inferred"] for r in under(s, row) if "inferred" in r.get("evidence", {})}


def strings(item: object) -> Iterator[str]:
    if isinstance(item, dict):
        for k, v in item.items():
            yield str(k)
            yield from strings(v)
    elif isinstance(item, list):
        for v in item:
            yield from strings(v)
    elif isinstance(item, str):
        yield item


def is_number(x: object) -> bool:
    if isinstance(x, bool) or not isinstance(x, (int, float)):
        return False
    return abs(x) <= (LARGEST_EXACT if isinstance(x, int) else LARGEST)


def is_text(x: object) -> bool:
    return isinstance(x, str) and bool(x.strip())


def is_stated(x: object) -> bool:
    return is_text(x) and x.strip().lower() not in ABSENT


def is_text_list(x: object) -> bool:
    return isinstance(x, list) and bool(x) and all(map(is_text, x))


Rule = Callable[[Store], list[Violation]]
RULES: list[tuple[str, Rule]] = []


def rule(code: str):
    def register(check: Rule) -> Rule:
        RULES.append((code, check))
        return check

    return register


@rule("data.schema")
def schema(s: Store) -> list[Violation]:
    code, out = "data.schema", []
    rows = s.rows
    for row in s.all_rows:
        subject = str(row.get("id"))
        if not ID.fullmatch(subject):
            out.append(Violation(code, subject, "an id is lowercase, dotted, at least two segments, and starts with a letter"))
        if missing := ROW_FIELDS - row.keys():
            out.append(Violation(code, subject, f"add {' '.join(sorted(missing))}"))
        if extra := row.keys() - ROW_FIELDS - {"n", "range", "basis"}:
            out.append(Violation(code, subject, f"drop {' '.join(sorted(extra))}: the language has no such field"))
        if not well_formed(row):
            out.append(Violation(code, subject, f"value {row.get('value')} is not one of the forms, in numbers"))
        if not isinstance(row.get("unit"), str):
            out.append(Violation(code, subject, "unit is text"))
        if "range" in row and not is_text(row["range"]):
            out.append(Violation(code, subject, "range names a record"))
        e = row.get("evidence")
        expressions = [e[k] for k in ("derived", "derived_low", "derived_high", "identity") if k in e] if isinstance(e, dict) else []
        if not all(map(is_text, expressions)):
            out.append(Violation(code, subject, "a derivation is text"))
        for field in ("conditions", "method", "window"):
            if not is_text(row.get(field)):
                out.append(Violation(code, subject, f"{field} is text, and stated"))
        if not is_stated(row.get("population")):
            out.append(Violation(code, subject, "state the population measured (rule 11)"))
        if "n" in row and not (type(row["n"]) is int and row["n"] > 0):
            out.append(Violation(code, subject, "n is a positive count"))
        form = evidence_form(row.get("evidence"))
        if form is None:
            out.append(Violation(code, subject, "evidence is one closed form: source, derived, inferred or calibrated"))
        elif form == "calibrated" and (
            row.get("kind") != "parameter"
            or not is_text_list(row["evidence"]["calibrated"])
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
        if not all(is_text(c.get(field)) for field in CLOSURE_FIELDS):
            out.append(Violation(code, str(c.get("id")), f"a closure states {' '.join(sorted(CLOSURE_FIELDS))} as text"))
        if "tol_unit" in c and not isinstance(c["tol_unit"], str):
            out.append(Violation(code, str(c.get("id")), "tol_unit is text"))
    for table in (s.works, s.inferences):
        out += [Violation(code, str(item.get("id")), "an id is text") for item in table if not is_text(item.get("id"))]
    return out


@rule("data.order")
def order(s: Store) -> list[Violation]:
    code, out = "data.order", []
    tables = (
        ("works", s.works),
        ("packet-reference-man", s.packet),
        ("healthy-range-set", s.ranges),
        ("inferences", s.inferences),
        ("closure", s.closures),
    )
    for name, rows in tables:
        ids = [str(r.get("id", "")) for r in rows]
        if ids != sorted(set(ids)):
            out.append(Violation(code, name, "sort rows by id and keep ids unique, so a rebuild is byte equal"))
    shared = {r.get("id", "") for r in s.packet} & {r.get("id", "") for r in s.ranges}
    out += [Violation(code, str(i), "one id in both the packet and the set; physiology never reads the set") for i in sorted(shared)]
    return out


@rule("data.files")
def files(s: Store) -> list[Violation]:
    out = [Violation("data.files", path, "only the five named files hold data; anything else escapes every rule") for path in s.stray]
    out += [Violation("data.files", line, "write each row in canonical form, so its digits say its precision") for line in s.not_canonical]
    return out


@rule("data.text")
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


@rule("data.unit")
def unit(s: Store) -> list[Violation]:
    out = []
    for row in s.all_rows:
        try:
            parse_unit(row.get("unit", ""))
        except ValueError as e:
            out.append(Violation("data.unit", str(row.get("id")), f"{e}; a new unit enters UNITS with its exact factor"))
        if row.get("kind") == "within_subject_variation" and row.get("unit") == "degC":
            out.append(Violation("data.unit", str(row.get("id")), "a variation is a difference: write K, which takes no offset"))
    return out


@rule("data.work")
def work(s: Store) -> list[Violation]:
    code, out = "data.work", []
    for w in s.works:
        subject = str(w.get("id"))
        if not WORK_ID.fullmatch(subject):
            out.append(Violation(code, subject, "a work id is lowercase and hyphenated"))
        if w.get("kind") not in WORK_TIERS:
            out.append(Violation(code, subject, f"kind {w.get('kind')} is not a work kind"))
        if not is_text(w.get("citation")):
            out.append(Violation(code, subject, "a work states its citation"))
        if not all(map(is_text, w.values())):
            out.append(Violation(code, subject, "every field of a work is text"))
        if not any(is_text(w.get(k)) for k in WORK_IDS):
            out.append(Violation(code, subject, f"name one of {' '.join(WORK_IDS)}"))
    works = s.work_ids
    for row in s.all_rows:
        e = row.get("evidence", {})
        for field in ("source", "identity"):
            if field in e and e[field] not in works:
                out.append(Violation(code, str(row.get("id")), f"{e[field]} is not in works.jsonl"))
    return out


@rule("data.tier")
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
        if capped != ("upgrade" in e) or ("upgrade" in e and not isinstance(e["upgrade"], bool)):
            out.append(Violation(code, subject, "upgrade is true or false on a compiled or textbook record, and only there"))
        kind = works.get(e.get("source"), {}).get("kind")
        if kind in WORK_TIERS and e["tier"] not in WORK_TIERS[kind]:
            out.append(Violation(code, subject, f"a {kind} work cannot carry tier {e['tier']}"))
    return out


@rule("data.locator")
def locator(s: Store) -> list[Violation]:
    code, out = "data.locator", []
    works = s.work_ids
    for row in s.all_rows:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        if evidence_form(e) != "source":
            continue
        at = e.get("locator")
        if not is_stated(at):
            out.append(Violation(code, subject, "a source record names its table or page"))
            continue
        if e.get("tier") == "textbook" and not (PAGE.search(at) and works.get(e.get("source"), {}).get("edition")):
            out.append(Violation(code, subject, "a textbook record names a page, and its work an edition"))
        if e.get("tier") == "compiled" and not TABLE_OR_PAGE.search(at):
            out.append(Violation(code, subject, "a compiled record names its table or page"))
    return out


@rule("data.number")
def number(s: Store) -> list[Violation]:
    code, out = "data.number", []
    for row in s.all_rows:
        v, subject = row.get("value", {}), str(row.get("id"))
        if not well_formed(row):
            continue
        variation = row.get("kind") in ("within_subject_variation", "spread")
        lowest = min(central(v), v.get("low", 0), v.get("ci_low", 0))
        if any(v.get(k, 0) < 0 for k in ("sd", "se")) or (variation and lowest < 0):
            out.append(Violation(code, subject, "a spread is not negative"))
        if "ci_low" in v and not v["ci_low"] <= v["mean"] <= v["ci_high"]:
            out.append(Violation(code, subject, "an interval holds its centre"))
        if "low" in v and not v["low"] <= central(v) <= v["high"]:
            out.append(Violation(code, subject, "an interval holds its centre"))
        if "coverage" in v and not 0 < v["coverage"] <= 1:
            out.append(Violation(code, subject, "coverage is a fraction"))
        if row.get("kind") == "healthy_range" and v.get("coverage") != 0.95:
            out.append(Violation(code, subject, "a healthy range is a central 95% (record 0004)"))
        if row.get("basis") == "mean_1_96_sd" and evidence_form(row.get("evidence")) != "interval":
            out.append(Violation(code, subject, "mean plus or minus 1.96 SD is written as a derived interval (record 0004)"))
        if (row.get("kind") == "healthy_range") != (row.get("basis") in BASES) or (row.get("kind") != "healthy_range" and "basis" in row):
            out.append(Violation(code, subject, f"a healthy range, and only one, states its basis, one of {sorted(BASES)}"))
    return out


def same_dimension(a: dict, b: dict) -> bool:
    try:
        return parse_unit(a.get("unit", ""))[1] == parse_unit(b.get("unit", ""))[1]
    except ValueError:
        return False


def is_flow_of(row: dict, amount: dict) -> bool:
    """A flow's dimension is its amount's per unit time, so a urine volume is no flow of a catecholamine."""
    try:
        per_time = list(parse_unit(amount.get("unit", ""))[1])
        per_time[DIMENSIONS.index("s")] -= 1
        return parse_unit(row.get("unit", ""))[1] == tuple(per_time)
    except ValueError:
        return False


def has_spread(row: dict) -> bool:
    v = row.get("value", {})
    return "sd" in v or "coverage" in v or (("se" in v or "ci_low" in v) and "n" in row)


def centred_on_one_mean(s: Store, row: dict) -> bool:
    read = [r for r in map(s.rows.get, reads(row)) if r and well_formed(r)]
    spreads = [r for r in read if r.get("kind") == "spread"]
    means = [r for r in read if r.get("kind") != "spread"]
    if len(spreads) != 1 or len(means) != 1:
        return False
    low, high = (si(row["value"][k], row["unit"]).value for k in ("low", "high"))
    mean, spread = s.quantity(means[0]["id"]).value, s.quantity(spreads[0]["id"]).value
    return math.isclose((low + high) / 2, mean, rel_tol=0.001) and math.isclose(high - low, 3.92 * spread, rel_tol=0.001)


@rule("data.population")
def population(s: Store) -> list[Violation]:
    code, out = "data.population", []
    in_packet = {r.get("id", ""): r for r in s.packet}
    for row in s.packet:
        subject = str(row.get("id"))
        if not well_formed(row):
            continue
        if "range" in row:
            held = in_packet.get(row["range"], {}) if is_text(row["range"]) else {}
            if not (well_formed(held) and "coverage" in held["value"] and held.get("unit") == row.get("unit")):
                out.append(Violation(code, subject, f"range {row['range']} is not a packet interval in the same unit"))
            elif not all(held["value"]["low"] <= x <= held["value"]["high"] for x in stated_values(row)):
                out.append(Violation(code, subject, f"the value leaves {row['range']} (I16)"))
        elif row.get("kind") != "constant" and evidence_form(row.get("evidence")) == "source" and not has_spread(row):
            out.append(Violation(code, subject, "state a spread with its n, or name a sourced range record (rule 11)"))
        elif evidence_form(row.get("evidence")) in ("derived", "interval") and not any(
            r.get("kind") != "constant" and has_spread(r) for r in under(s, row)[1:]
        ):
            out.append(Violation(code, subject, "no record under this derivation carries a population range (I16)"))
    return out


@rule("data.derived")
def derived(s: Store) -> list[Violation]:
    code, out = "data.derived", []
    rows = s.rows
    in_packet = {r.get("id") for r in s.packet}
    packet_ids = {id(r) for r in s.packet}

    def compare(row: dict, expression: str, stated: float) -> None:
        subject = str(row.get("id"))
        got = value_of(expression, s.quantity)
        want = si(stated, row["unit"])
        factor = float(parse_unit(row["unit"])[0])
        if id(row) in packet_ids and (read := sorted(names_in(expression) - in_packet)):
            out.append(Violation(code, subject, f"a packet value reads the set through {read[0]} (record 0005)"))
        if got.dimension == want.dimension and abs(got.value - want.value) > last_digit_tolerance(stated) * factor * (1 + 1e-9):
            out.append(Violation(code, subject, f"the identity gives {got.value / factor:.6g} {row['unit']}"))

    for row in s.all_rows:
        e, subject, value = row.get("evidence", {}), str(row.get("id")), row.get("value", {})
        form = evidence_form(e)
        if form not in ("derived", "interval") or not well_formed(row):
            continue
        if set(value) != ({"point"} if form == "derived" else {"low", "high", "coverage"}):
            out.append(Violation(code, subject, "a derivation states a point, and an interval derivation its two ends"))
            continue
        if not reads(row):
            out.append(Violation(code, subject, "a derivation rests on records; a literal alone cites nothing"))
            continue
        if row.get("kind") == "healthy_range" and any(r.get("kind") == "within_subject_variation" for r in under(s, row)):
            out.append(Violation(code, subject, "a healthy range is not built from within-subject variation (record 0004)"))
        if row.get("kind") == "within_subject_variation" and any(r.get("kind") == "spread" for r in under(s, row)):
            out.append(Violation(code, subject, "variation between people is not variation within one (record 0004)"))
        try:
            if row.get("basis") == "mean_1_96_sd" and not centred_on_one_mean(s, row):
                out.append(Violation(code, subject, "mean plus or minus 1.96 SD is centred on one mean and spans 3.92 of one spread"))
            if form == "derived":
                compare(row, e["derived"], value["point"])
            else:
                compare(row, e["derived_low"], value["low"])
                compare(row, e["derived_high"], value["high"])
        except (KeyError, ValueError, SyntaxError) as err:
            out.append(Violation(code, subject, f"cannot evaluate: {err}"))
    return out


@rule("data.dimension")
def dimension(s: Store) -> list[Violation]:
    code, out = "data.dimension", []
    for row in s.all_rows:
        e, subject = row.get("evidence", {}), str(row.get("id"))
        expressions = [e[k] for k in ("derived", "derived_low", "derived_high") if k in e]
        for expression in expressions:
            try:
                got = value_of(expression, s.quantity)
                want = parse_unit(row["unit"])[1]
            except (KeyError, ValueError, SyntaxError):
                continue
            if got.dimension != want:
                out.append(Violation(code, subject, f"the identity has dimension {got.dimension}, the unit {want}"))
    return out


@rule("data.cycle")
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


@rule("data.anchor")
def anchor(s: Store) -> list[Violation]:
    out = []
    for row in s.packet:
        if row.get("kind") == "anchor" and inferred_under(s, row):
            out.append(Violation("data.anchor", str(row.get("id")), "an anchor never rests on an inference (rule 16)"))
    return out


@rule("data.snapshot")
def snapshot(s: Store) -> list[Violation]:
    out = []
    for row in s.all_rows:
        stated = [row.get("method"), row.get("window")]
        if row.get("kind") in ("anchor", "healthy_range") and not all(map(is_stated, stated)):
            out.append(Violation("data.snapshot", str(row.get("id")), "an anchor or healthy range states its method and window (rule 17, record 0004)"))
    return out


@rule("data.inference")
def inferred(s: Store) -> list[Violation]:
    code, out = "data.inference", []
    rows = s.rows
    by_id = {i.get("id"): i for i in s.inferences}
    in_packet = {r.get("id") for r in s.packet}
    allowed = {
        "parameter": {"fill"},
        "day": {"fill"},
        "healthy_range": {"adjust"},
        "within_subject_variation": {"adjust", "flows"},
    }
    for i in s.inferences:
        subject = str(i.get("id"))
        for field in ("model", "population", "target", "form"):
            if not is_text(i.get(field)):
                out.append(Violation(code, subject, f"an inference record names its {field}"))
        for field in ("steps", "premises", "search"):
            if not is_text_list(i.get(field)):
                out.append(Violation(code, subject, f"an inference record lists its {field}"))
        low, high = i.get("low"), i.get("high")
        if not (is_number(low) and is_number(high) and math.isfinite(low) and math.isfinite(high) and low <= high):
            out.append(Violation(code, subject, "low and high are finite and in order"))
        form = i.get("form")
        if form not in FORMS:
            out.append(Violation(code, subject, f"form {form} is not one of {sorted(FORMS)}"))
        premises = i.get("premises") if is_text_list(i.get("premises")) else []
        for p in premises:
            if p not in rows:
                out.append(Violation(code, subject, f"premise {p} is not a record"))
            elif inferred_under(s, rows[p]):
                out.append(Violation(code, subject, f"premise {p} is not sourced"))
            elif i.get("target") in in_packet and p not in in_packet:
                out.append(Violation(code, subject, f"a packet value reads the set through {p} (record 0005)"))
        kind = rows.get(i.get("target"), {}).get("kind")
        kinds = [rows.get(p, {}).get("kind") for p in set(premises)]
        if form == "adjust" and not (kind in kinds and "shift" in kinds):
            out.append(Violation(code, subject, "an adjustment rests on a sourced value of its kind and a sourced shift"))
        if form == "adjust" and not all(same_dimension(rows.get(p, {}), rows.get(i.get("target"), {})) for p in premises):
            out.append(Violation(code, subject, "an adjustment stays in its target's dimension"))
        if form == "flows" and not all(is_flow_of(rows.get(p, {}), rows.get(i.get("target"), {})) for p in premises):
            out.append(Violation(code, subject, "a flows inference reasons from sourced rates, in and out of the amount"))
        if (form == "flows") != is_text(i.get("unmeasurable")) or (form != "flows" and "unmeasurable" in i):
            out.append(Violation(code, subject, "a flows inference, and only one, states why no method can measure the amount"))
    for row in s.all_rows:
        e, subject, kind = row.get("evidence", {}), str(row.get("id")), str(row.get("kind"))
        if "inferred" in e:
            i = by_id.get(e["inferred"]) if is_text(e["inferred"]) else None
            if i is None:
                out.append(Violation(code, subject, f"{e['inferred']} is not in inferences.jsonl"))
                continue
            if i.get("target") != subject:
                out.append(Violation(code, subject, f"{e['inferred']} targets {i.get('target')}"))
            bounded = is_number(i.get("low")) and is_number(i.get("high")) and well_formed(row)
            if bounded and not all(i["low"] <= x <= i["high"] for x in stated_values(row)):
                out.append(Violation(code, subject, f"the value leaves {e['inferred']}'s range (I16)"))
            if i.get("form") not in allowed.get(kind, set()):
                out.append(Violation(code, subject, f"a {kind} cannot rest on a {i.get('form')} inference (rule 16)"))
    for row in s.ranges:
        for ref in inferred_under(s, row):
            form = by_id.get(ref, {}).get("form")
            if "inferred" not in row.get("evidence", {}) and form not in allowed.get(str(row.get("kind")), set()):
                out.append(Violation(code, str(row.get("id")), f"a derivation carries a {form} inference into the set (rule 16)"))
    return out


@rule("data.closure")
def closure(s: Store) -> list[Violation]:
    code, out = "data.closure", []
    for c in s.closures:
        subject = str(c.get("id"))
        if c.get("level") not in LEVELS:
            out.append(Violation(code, subject, f"level is one of {sorted(LEVELS)}"))
        forms = {"tol_rel", "tol_abs", "tol_unit"} & c.keys()
        rel, abs_ = forms == {"tol_rel"}, forms == {"tol_abs", "tol_unit"}
        tolerance = c.get("tol_rel" if rel else "tol_abs")
        if not (rel or abs_) or not is_number(tolerance):
            out.append(Violation(code, subject, "state exactly one finite tolerance: tol_rel, or tol_abs with tol_unit"))
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
            continue
        gap, allowed = abs(lhs.value - rhs.value), tol.value if tol else c["tol_rel"] * abs(rhs.value)
        if not (math.isfinite(gap) and math.isfinite(allowed)) or gap > allowed:
            out.append(Violation(code, subject, f"{lhs.value:.6g} against {rhs.value:.6g} is past its tolerance"))
    return out


@rule("data.ticket")
def ticket(s: Store) -> list[Violation]:
    code, out = "data.ticket", []
    rows = s.rows
    for path, names in s.tickets.items():
        for name in names:
            if PREFIX.fullmatch(name):
                if not any(i.startswith(name[:-1]) for i in rows):
                    out.append(Violation(code, path, f"Data: {name} matches no record"))
            elif not ID.fullmatch(name):
                out.append(Violation(code, path, f"Data: {name!r} is neither a record id nor a prefix ending in .*"))
            elif name not in rows:
                out.append(Violation(code, path, f"Data: {name} names no record"))
    return out


def synthetic_store() -> Store:
    def row(rid: str, kind: str, unit: str, value: dict, evidence: dict) -> dict:
        return dict(id=rid, kind=kind, unit=unit, value=value, population="fixture", conditions="fixture",
                    method="fixture", window="fixture", evidence=evidence)

    def primary() -> dict:
        return dict(source="fixture-paper", locator="Table 1", reading="full_text", tier="primary_measured", certainty="low")

    def compiled() -> dict:
        return dict(source="fixture-review", locator="Table 1", reading="full_text", tier="compiled", certainty="moderate",
                    upgrade=False)

    def book() -> dict:
        return dict(source="fixture-book", locator="ch.1 p.1", reading="full_text", tier="textbook", certainty="moderate",
                    upgrade=True)

    def identity(expr: str) -> dict:
        return dict(derived=expr, identity="fixture-paper")

    packet = [
        dict(row("fx.area", "input", "m^2", {"point": 2.0}, primary()), range="fx.area_range"),
        row("fx.area_range", "input", "m^2", {"low": 1.5, "high": 2.5, "coverage": 0.95}, primary()),
        dict(row("fx.base", "input", "L/min/m^2", {"mean": 1.5, "se": 0.1}, book()), n=9),
        row("fx.fitted", "parameter", "mL", {"point": 1.0}, dict(calibrated=["fx.rate"])),
        row("fx.flow", "anchor", "L/min", {"point": 6.0}, identity("fx.index * fx.area")),
        row("fx.flow_band", "input", "L/min", {"low": 1.0, "high": 2.0, "coverage": 0.95}, primary()),
        row("fx.guess", "parameter", "mL", {"point": 1.5}, dict(inferred="inf.fill")),
        row("fx.half", "input", "L/min", {"point": 3.0}, identity("fx.flow * fx.share")),
        row("fx.index", "input", "L/min/m^2", {"point": 3.0}, identity("fx.base * fx.twice")),
        row("fx.kilocalories", "constant", "kcal", {"point": 1.0}, primary()),
        row("fx.kilojoules", "constant", "kJ", {"point": 4.184}, primary()),
        row("fx.kilopascals", "constant", "kPa", {"point": 100.0}, primary()),
        row("fx.leaf", "anchor", "L/min", {"point": 8.0}, identity("fx.base * fx.area + fx.base * fx.area + fx.leaf_more")),
        row("fx.leaf_more", "input", "L/min", {"mean": 2.0, "sd": 0.1}, primary()),
        row("fx.median", "input", "mL", {"median": 6.0, "low": 5.0, "high": 7.0, "coverage": 0.5}, primary()),
        row("fx.none", "constant", "/min", {"point": 0.0}, primary()),
        row("fx.pressure_cmh2o", "constant", "cmH2O", {"point": 1019.716}, primary()),
        row("fx.pressure_mmhg", "constant", "mmHg", {"point": 750.062}, primary()),
        row("fx.rate", "anchor", "/min", {"mean": 60.0, "sd": 1.0}, primary()),
        row("fx.share", "input", "%", {"mean": 50, "sd": 5}, compiled()),
        row("fx.stroke", "input", "mL", {"low": 90.0, "high": 110.0, "coverage": 0.95}, primary()),
        row("fx.temp", "constant", "degC", {"point": 37.0}, primary()),
        row("fx.temp_kelvin", "constant", "K", {"point": 310.15}, primary()),
        row("fx.temp_near", "constant", "degC", {"point": 37.2}, primary()),
        row("fx.tiny", "input", "mL", {"point": 2e-06}, identity("fx.stroke * 0.00000002")),
        row("fx.twice", "constant", "1", {"point": 2.0}, primary()),
    ]
    ranges = [
        row("fx.amount", "within_subject_variation", "mL", {"point": 1.0}, dict(inferred="inf.flows")),
        dict(row("fx.area_band", "healthy_range", "m^2", {"low": 1.5, "high": 2.5, "coverage": 0.95}, primary()), basis="percentiles"),
        row("fx.dip", "shift", "L/min", {"mean": -0.5, "sd": 0.1}, primary()),
        row("fx.flux", "shift", "mL/d", {"mean": 10.0, "sd": 1.0}, primary()),
        dict(row("fx.interval", "healthy_range", "/min", {"low": 58.04, "high": 61.96, "coverage": 0.95},
                 dict(derived_low="fx.rate - 1.96 * fx.rate_sd", derived_high="fx.rate + 1.96 * fx.rate_sd", identity="fixture-paper")),
             basis="mean_1_96_sd"),
        dict(row("fx.narrowed", "healthy_range", "L/min", {"low": 5.5, "high": 6.5, "coverage": 0.95}, dict(inferred="inf.adjust")),
             basis="reference_interval"),
        row("fx.none_sd", "within_subject_variation", "/min", {"point": 1.0}, primary()),
        row("fx.rate_sd", "spread", "/min", {"point": 1.0}, primary()),
        dict(row("fx.span", "healthy_range", "L/min", {"low": 5.0, "high": 7.0, "coverage": 0.95}, primary()), basis="percentiles"),
    ]
    works = [
        dict(id="fixture-book", kind="textbook", citation="Fixture.", isbn="0", edition="1"),
        dict(id="fixture-paper", kind="primary", citation="Fixture.", doi="10.0/fixture"),
        dict(id="fixture-review", kind="review", citation="Fixture.", doi="10.0/review"),
    ]
    inferences = [
        dict(id="inf.adjust", target="fx.narrowed", form="adjust", model="fixture", population="fixture", low=5.5, high=6.5,
             steps=["a step"], premises=["fx.dip", "fx.span"], search=["a search"]),
        dict(id="inf.fill", target="fx.guess", form="fill", model="fixture", population="fixture", low=1, high=2,
             steps=["a step"], premises=["fx.rate"], search=["a search"]),
        dict(id="inf.flows", target="fx.amount", form="flows", model="fixture", population="fixture", low=0.5, high=1.5,
             steps=["a step"], premises=["fx.flux"], search=["a search"], unmeasurable="no living-person method"),
    ]
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


def work_of(s: Store, wid: str) -> dict:
    return next(w for w in s.works if w["id"] == wid)


def inf(s: Store, iid: str) -> dict:
    return next(i for i in s.inferences if i["id"] == iid)


def closure_of(s: Store, cid: str) -> dict:
    return next(c for c in s.closures if c["id"] == cid)


def evidence(s: Store, rid: str, **changes: object) -> None:
    on(s, rid)["evidence"].update(changes)


def infer(s: Store, rid: str) -> None:
    on(s, rid)["evidence"] = dict(inferred="inf.fill")
    inf(s, "inf.fill")["target"] = rid


def cut(item: dict, key: str) -> None:
    del item[key]


PROBES: tuple[tuple[str, str, Callable[[Store], object]], ...] = (
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(sentence="a field the language lacks")),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(evidence={})),
    ("data.schema", "fx.rate", lambda s: evidence(s, "fx.rate", source_text="a passage")),
    ("data.schema", "fx.fitted", lambda s: evidence(s, "fx.fitted", calibrated=["fx.stroke"])),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(value={"mean": 60.0})),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(evidence=None)),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(unit=None)),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(value=None)),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(kind=None)),
    ("data.schema", "fx.area", lambda s: on(s, "fx.area").update(range=5)),
    ("data.schema", "fx.flow", lambda s: evidence(s, "fx.flow", derived=123)),
    ("data.schema", "fx.flow", lambda s: evidence(s, "fx.flow", identity=None)),
    ("data.schema", "XC-02", lambda s: closure_of(s, "XC-02").update(tol_unit=None)),
    ("data.schema", "None", lambda s: cut(work_of(s, "fixture-book"), "id")),
    ("data.schema", "fx.area", lambda s: on(s, "fx.area").update(value={"point": True})),
    ("data.schema", "fx.area", lambda s: on(s, "fx.area").update(value={"point": "two"})),
    ("data.schema", "fx.area", lambda s: on(s, "fx.area").update(value={"point": 9007199254740993})),
    ("data.schema", "fx.area", lambda s: on(s, "fx.area").update(value={"point": float("inf")})),
    ("data.schema", "fx.area", lambda s: on(s, "fx.area").update(value={"point": 1e31})),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(population="")),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(population="Not stated")),
    ("data.schema", "fx.area", lambda s: on(s, "fx.area").update(method=123)),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(n=0)),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(n=True)),
    ("data.schema", "fx.fitted", lambda s: evidence(s, "fx.fitted", calibrated={"fx.rate": 1})),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(window=" ")),
    ("data.schema", "fx.rate\n", lambda s: on(s, "fx.rate").update(id="fx.rate\n")),
    ("data.schema", "fx.span", lambda s: on(s, "fx.span").update(kind="anchor")),
    ("data.schema", "fixture-book", lambda s: work_of(s, "fixture-book").update(abstract="a passage")),
    ("data.schema", "inf.fill", lambda s: inf(s, "inf.fill").update(prose="a passage")),
    ("data.schema", "XC-01", lambda s: closure_of(s, "XC-01").update(note="a passage")),
    ("data.schema", "1.area", lambda s: on(s, "fx.area").update(id="1.area")),
    ("data.order", "packet-reference-man", lambda s: s.packet.reverse()),
    ("data.order", "works", lambda s: s.works.append(dict(s.works[-1], doi="10.0/other"))),
    ("data.order", "inferences", lambda s: s.inferences.append(dict(s.inferences[-1], model="another"))),
    ("data.order", "fx.base", lambda s: s.ranges.insert(0, dict(s.ranges[0], id="fx.base"))),
    ("data.text", "fx.rate", lambda s: on(s, "fx.rate").update(method="x" * 121)),
    ("data.text", "fx.rate", lambda s: on(s, "fx.rate").update(method="cuff \u2014 arm")),
    ("data.text", "inf.fill", lambda s: inf(s, "inf.fill").update(steps=["x" * 241])),
    ("data.text", "fixture-paper", lambda s: work_of(s, "fixture-paper").update(query="x" * 241)),
    ("data.text", "fx.share", lambda s: evidence(s, "fx.share", upgrade={"x" * 241: True})),
    ("data.unit", "fx.rate", lambda s: on(s, "fx.rate").update(unit="furlong")),
    ("data.unit", "fx.temp", lambda s: on(s, "fx.temp").update(unit="degC^1")),
    ("data.unit", "fx.area", lambda s: on(s, "fx.area").update(unit="mL^60")),
    ("data.unit", "fx.none_sd", lambda s: on(s, "fx.none_sd").update(unit="degC")),
    ("data.work", "fx.rate", lambda s: evidence(s, "fx.rate", source="no-such-work")),
    ("data.work", "fx.flow", lambda s: evidence(s, "fx.flow", identity="no-such-work")),
    ("data.work", "fixture-paper", lambda s: cut(work_of(s, "fixture-paper"), "doi")),
    ("data.work", "fixture-paper", lambda s: work_of(s, "fixture-paper").update(doi=True)),
    ("data.work", "fixture-paper", lambda s: work_of(s, "fixture-paper").update(citation=False)),
    ("data.work", "fixture-paper", lambda s: cut(work_of(s, "fixture-paper"), "citation")),
    ("data.work", "fixture-paper", lambda s: work_of(s, "fixture-paper").update(kind="pamphlet")),
    ("data.work", "fixture-book", lambda s: work_of(s, "fixture-book").update(edition=True)),
    ("data.work", "fixture-paper\n", lambda s: work_of(s, "fixture-paper").update(id="fixture-paper\n")),
    ("data.tier", "fx.base", lambda s: evidence(s, "fx.base", certainty="high")),
    ("data.tier", "fx.base", lambda s: evidence(s, "fx.base", tier="primary_measured")),
    ("data.tier", "fx.share", lambda s: cut(on(s, "fx.share")["evidence"], "upgrade")),
    ("data.tier", "fx.rate", lambda s: evidence(s, "fx.rate", tier="gold")),
    ("data.tier", "fx.share", lambda s: evidence(s, "fx.share", upgrade={"a passage": True})),
    ("data.tier", "fx.rate", lambda s: evidence(s, "fx.rate", upgrade={"a passage": True})),
    ("data.tier", "fx.base", lambda s: (
        cut(on(s, "fx.base")["evidence"], "upgrade"),
        evidence(s, "fx.base", reading="secondary_reproduction", tier="primary_measured", certainty="low"),
    )),
    ("data.locator", "fx.base", lambda s: evidence(s, "fx.base", locator="somewhere")),
    ("data.locator", "fx.rate", lambda s: evidence(s, "fx.rate", locator=True)),
    ("data.locator", "fx.rate", lambda s: evidence(s, "fx.rate", locator="Not stated")),
    ("data.locator", "fx.base", lambda s: cut(work_of(s, "fixture-book"), "edition")),
    ("data.locator", "fx.share", lambda s: evidence(s, "fx.share", locator="somewhere")),
    ("data.locator", "fx.share", lambda s: evidence(s, "fx.share", locator="Table")),
    ("data.locator", "fx.share", lambda s: evidence(s, "fx.share", locator="Fig 3")),
    ("data.number", "fx.rate", lambda s: on(s, "fx.rate").update(value={"mean": 60.0, "sd": -1.0})),
    ("data.number", "fx.rate", lambda s: on(s, "fx.rate").update(value={"mean": 60.0, "ci_low": 100.0, "ci_high": 1.0})),
    ("data.number", "fx.rate", lambda s: on(s, "fx.rate").update(value={"mean": 60.0, "se": -1.0})),
    ("data.number", "fx.rate", lambda s: on(s, "fx.rate").update(value={"mean": 0.0, "ci_low": 1.0, "ci_high": 2.0})),
    ("data.number", "fx.rate", lambda s: on(s, "fx.rate").update(value={"mean": 3.0, "ci_low": 1.0, "ci_high": 2.0})),
    ("data.number", "fx.none_sd", lambda s: on(s, "fx.none_sd").update(value={"low": -1.0, "high": 3.0, "coverage": 0.95})),
    ("data.number", "fx.none_sd", lambda s: on(s, "fx.none_sd").update(value={"mean": 1.0, "ci_low": -1.0, "ci_high": 3.0})),
    ("data.number", "fx.median", lambda s: on(s, "fx.median")["value"].update(coverage=1.5)),
    ("data.number", "fx.span", lambda s: on(s, "fx.span")["value"].update(low=8.0)),
    ("data.number", "fx.median", lambda s: on(s, "fx.median")["value"].update(median=9.0)),
    ("data.number", "fx.span", lambda s: on(s, "fx.span").update(value={"point": 6.0})),
    ("data.number", "fx.span", lambda s: on(s, "fx.span")["value"].update(coverage=0.5)),
    ("data.number", "fx.span", lambda s: cut(on(s, "fx.span"), "basis")),
    ("data.number", "fx.span", lambda s: on(s, "fx.span").update(basis="confidence_interval")),
    ("data.number", "fx.span", lambda s: on(s, "fx.span").update(basis="mean_1_96_sd")),
    ("data.number", "fx.rate", lambda s: on(s, "fx.rate").update(basis="percentiles")),
    ("data.number", "fx.none_sd", lambda s: on(s, "fx.none_sd").update(value={"point": -1.0})),
    ("data.population", "fx.leaf_more", lambda s: on(s, "fx.leaf_more").update(value={"point": 2.0})),
    ("data.population", "fx.base", lambda s: cut(on(s, "fx.base"), "n")),
    ("data.population", "fx.area", lambda s: cut(on(s, "fx.area"), "range")),
    ("data.population", "fx.area", lambda s: on(s, "fx.area").update(range="fx.rate")),
    ("data.population", "fx.area", lambda s: on(s, "fx.area").update(value={"point": 3.0})),
    ("data.population", "fx.area", lambda s: on(s, "fx.area_range").update(unit="cm^2")),
    ("data.population", "fx.area", lambda s: on(s, "fx.area").update(range="fx.area_band")),
    ("data.population", "fx.flow", lambda s: on(s, "fx.flow").update(range="fx.flow_band")),
    ("data.population", "fx.fitted", lambda s: on(s, "fx.fitted").update(range="fx.area_range", unit="m^2")),
    ("data.population", "fx.area", lambda s: on(s, "fx.area").update(value={"low": 1.0, "high": 3.0, "coverage": 0.95})),
    ("data.population", "fx.bounded", lambda s: (
        s.packet.append(dict(on(s, "fx.kilopascals"), id="fx.bounded", kind="anchor", value={"low": 90.0, "high": 110.0, "coverage": 0.95},
                             evidence=dict(derived_low="fx.kilopascals * 0.9", derived_high="fx.kilopascals * 1.1", identity="fixture-paper"))),
        s.packet.sort(key=lambda r: r["id"]),
    )),
    ("data.population", "fx.reference", lambda s: (
        s.packet.append(dict(on(s, "fx.temp"), id="fx.reference", kind="anchor", evidence=dict(derived="fx.temp_kelvin", identity="fixture-paper"), unit="K", value={"point": 310.15})),
        s.packet.sort(key=lambda r: r["id"]),
    )),
    ("data.derived", "fx.leaf", lambda s: on(s, "fx.leaf").update(value={"point": 9.0})),
    ("data.derived", "fx.half", lambda s: on(s, "fx.half").update(value={"point": 300.0})),
    ("data.derived", "fx.tiny", lambda s: on(s, "fx.tiny").update(value={"point": 1e-06})),
    ("data.derived", "fx.flow", lambda s: on(s, "fx.flow").update(value={"low": 5.0, "high": 7.0, "coverage": 0.95})),
    ("data.derived", "fx.interval", lambda s: on(s, "fx.interval")["value"].update(high=70.0)),
    ("data.derived", "fx.interval", lambda s: on(s, "fx.interval")["value"].update(median=60.0)),
    ("data.derived", "fx.interval", lambda s: (
        on(s, "fx.interval").update(basis="reference_interval"),
        evidence(s, "fx.interval", derived_low="fx.rate - 1.96 * fx.none_sd", derived_high="fx.rate + 1.96 * fx.none_sd"),
    )),
    ("data.derived", "fx.interval", lambda s: on(s, "fx.rate_sd").update(kind="shift")),
    ("data.derived", "fx.none_sd", lambda s: on(s, "fx.none_sd").update(evidence=dict(derived="fx.rate_sd", identity="fixture-paper"))),
    ("data.derived", "fx.interval", lambda s: (
        on(s, "fx.interval").update(value={"low": 60.0, "high": 63.92, "coverage": 0.95}),
        evidence(s, "fx.interval", derived_low="fx.rate", derived_high="fx.rate + 3.92 * fx.rate_sd"),
    )),
    ("data.derived", "fx.interval", lambda s: (
        on(s, "fx.interval").update(value={"low": 58.355, "high": 61.645, "coverage": 0.95}),
        evidence(s, "fx.interval", derived_low="fx.rate - 1.645 * fx.rate_sd", derived_high="fx.rate + 1.645 * fx.rate_sd"),
    )),
    ("data.derived", "fx.interval", lambda s: (
        on(s, "fx.rate_sd").update(evidence=dict(derived="fx.none_sd", identity="fixture-paper")),
        on(s, "fx.none_sd").update(value={"point": 1.0}),
    )),
    ("data.derived", "fx.half", lambda s: evidence(s, "fx.half", derived="fx.flow * fx.share + fx.flow * (9007199254740993 - 9007199254740992)")),
    ("data.derived", "fx.half", lambda s: evidence(s, "fx.half", derived="fx.flow * fx.share + fx.flow * (9007199254740993.0 - 9007199254740992.0)")),
    ("data.derived", "fx.flow", lambda s: evidence(s, "fx.flow", derived="6.0 # fx.index")),
    ("data.derived", "fx.flow", lambda s: evidence(s, "fx.flow", derived="6.0 * 1")),
    ("data.derived", "fx.half", lambda s: evidence(s, "fx.half", derived="sum(fx.flow * fx.share, sum=500)")),
    ("data.derived", "fx.half", lambda s: evidence(s, "fx.half", derived="fx.flow * fx.share + 1 / (fx.twice * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30) * fx.flow")),
    ("data.derived", "fx.half", lambda s: evidence(s, "fx.half", derived="fx.span * fx.share")),
    ("data.derived", "fx.flow", lambda s: evidence(s, "fx.flow", derived="fx.index * fx.area + fx.none")),
    ("data.derived", "fx.flow", lambda s: evidence(s, "fx.flow", derived="(1e309 - 1e309) * fx.index * fx.area")),
    ("data.derived", "fx.flow", lambda s: evidence(s, "fx.flow", derived="fx.index * fx.nothing")),
    ("data.derived", "fx.flow", lambda s: evidence(s, "fx.flow", derived="fx.index * fx.area + n1 * n0 * 0")),
    ("data.dimension", "fx.flow", lambda s: on(s, "fx.flow").update(unit="kg")),
    ("data.cycle", "fx.flow", lambda s: (evidence(s, "fx.flow", derived="fx.half / fx.share"))),
    ("data.cycle", "fx.flow", lambda s: evidence(s, "fx.flow", derived="fx.flow")),
    ("data.cycle", "fx.fitted", lambda s: (
        evidence(s, "fx.fitted", calibrated=["fx.flow"]),
        evidence(s, "fx.flow", derived="fx.fitted / fx.fitted * fx.index * fx.area"),
    )),
    ("data.anchor", "fx.flow", lambda s: infer(s, "fx.area")),
    ("data.anchor", "fx.flow", lambda s: infer(s, "fx.base")),
    ("data.anchor", "fx.flow", lambda s: (
        on(s, "fx.area_range").update(kind="parameter", evidence=dict(inferred="inf.fill")),
        inf(s, "inf.fill").update(target="fx.area_range", high=3),
    )),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(window="")),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(method="")),
    ("data.schema", "fx.rate", lambda s: on(s, "fx.rate").update(method=" ")),
    ("data.snapshot", "fx.rate", lambda s: on(s, "fx.rate").update(window="Not applicable")),
    ("data.snapshot", "fx.rate", lambda s: on(s, "fx.rate").update(method="N/A")),
    ("data.snapshot", "fx.span", lambda s: on(s, "fx.span").update(window="Not stated")),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(search=[])),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(search=False)),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(search=[""])),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(steps=False)),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(model=True)),
    ("data.schema", "None", lambda s: cut(inf(s, "inf.fill"), "id")),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(population=" ")),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(premises=["fx.nothing"])),
    ("data.inference", "inf.adjust", lambda s: inf(s, "inf.adjust").update(premises=["fx.nothing", "fx.dip", "fx.span"])),
    ("data.inference", "inf.adjust", lambda s: inf(s, "inf.adjust").update(premises=["fx.leaf_more", "fx.span"])),
    ("data.inference", "inf.adjust", lambda s: inf(s, "inf.adjust").update(premises=["fx.dip", "fx.flux"])),
    ("data.inference", "inf.flows", lambda s: inf(s, "inf.flows").update(premises=["fx.area"])),
    ("data.inference", "inf.flows", lambda s: inf(s, "inf.flows").update(premises=["fx.rate"])),
    ("data.inference", "inf.adjust", lambda s: on(s, "fx.narrowed").update(unit="mmHg")),
    ("data.inference", "fx.narrowed", lambda s: on(s, "fx.narrowed").update(value={"low": 1.0, "high": 11.0, "coverage": 0.95})),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(high=1e309)),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(low=3, high=2)),
    ("data.inference", "fx.guess", lambda s: on(s, "fx.guess").update(value={"point": 2000.0})),
    ("data.inference", "fx.guess", lambda s: on(s, "fx.guess").update(value={"point": 0.5})),
    ("data.inference", "fx.guess", lambda s: on(s, "fx.guess").update(value={"mean": 1.5, "ci_low": 0.0, "ci_high": 4.0})),
    ("data.inference", "inf.adjust", lambda s: inf(s, "inf.adjust").update(premises=["fx.narrowed", "fx.dip"])),
    ("data.inference", "inf.adjust", lambda s: inf(s, "inf.adjust").update(premises=["fx.guess", "fx.dip", "fx.span"])),
    ("data.inference", "fx.narrowed", lambda s: inf(s, "inf.adjust").update(form="fill")),
    ("data.inference", "fx.narrowed", lambda s: inf(s, "inf.adjust").update(form="flows", unmeasurable="no method")),
    ("data.inference", "inf.flows", lambda s: cut(inf(s, "inf.flows"), "unmeasurable")),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(unmeasurable={"a": [1]})),
    ("data.inference", "inf.fill", lambda s: inf(s, "inf.fill").update(premises=["fx.span"])),
    ("data.inference", "inf.adjust", lambda s: inf(s, "inf.adjust").update(premises=["fx.span"])),
    ("data.inference", "inf.adjust", lambda s: inf(s, "inf.adjust").update(premises=["fx.span", "fx.span"])),
    ("data.inference", "inf.adjust", lambda s: inf(s, "inf.adjust").update(premises=["fx.rate", "fx.area"])),
    ("data.inference", "fx.guess", lambda s: inf(s, "inf.fill").update(target="fx.area")),
    ("data.inference", "fx.guess", lambda s: evidence(s, "fx.guess", inferred="inf.nothing")),
    ("data.inference", "fx.guess", lambda s: evidence(s, "fx.guess", inferred=None)),
    ("data.inference", "fx.guess", lambda s: evidence(s, "fx.guess", inferred=["inf.fill"])),
    ("data.inference", "fx.none_sd", lambda s: on(s, "fx.none_sd").update(evidence=dict(derived="fx.guess / fx.guess * fx.none_sd_base", identity="fixture-paper"), id="fx.none_sd")),
    ("data.closure", "XC-01", lambda s: on(s, "fx.rate").update(value={"mean": 180.0, "sd": 1.0})),
    ("data.closure", "XC-02", lambda s: on(s, "fx.temp_near").update(value={"point": 39.0})),
    ("data.closure", "XC-01", lambda s: cut(closure_of(s, "XC-01"), "tol_rel")),
    ("data.closure", "XC-01", lambda s: closure_of(s, "XC-01").update(tol_rel=float("inf"))),
    ("data.closure", "XC-02", lambda s: closure_of(s, "XC-02").update(tol_abs=float("inf"))),
    ("data.closure", "XC-01", lambda s: closure_of(s, "XC-01").update(tol_abs=-100)),
    ("data.closure", "XC-01", lambda s: closure_of(s, "XC-01").update(tol_rel="a lot")),
    ("data.closure", "XC-01", lambda s: closure_of(s, "XC-01").update(tol_rel=-0.05)),
    ("data.closure", "XC-01", lambda s: closure_of(s, "XC-01").update(lhs="(1e309 - 1e309) * fx.flow")),
    ("data.closure", "XC-01", lambda s: closure_of(s, "XC-01").update(rhs="fx.rate")),
    ("data.closure", "XC-01", lambda s: closure_of(s, "XC-01").update(level="gut")),
    ("data.schema", "None", lambda s: cut(closure_of(s, "XC-01"), "id")),
    ("data.schema", "XC-01", lambda s: closure_of(s, "XC-01").update(claim={"a": [1]})),
    ("data.schema", "True", lambda s: closure_of(s, "XC-01").update(id=True)),
    ("data.closure", "XC-05", lambda s: closure_of(s, "XC-05").update(
        lhs="fx.kilojoules * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 3.6e4",
        rhs="fx.kilojoules * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * 1e30 * -3.6e4",
        tol_rel=2.0)),
)

FILE_PROBES: tuple[tuple[str, str, Callable[[pathlib.Path], object]], ...] = (
    ("data.files", "data/other.jsonl", lambda root: (root / DATA / "other.jsonl").write_text("{}\n")),
    ("data.files", "data/notes.txt", lambda root: (root / DATA / "notes.txt").write_text("a passage\n")),
    ("data.files", "data/archive/works.jsonl", lambda root: (
        (root / DATA / "archive").mkdir(),
        (root / DATA / "archive" / "works.jsonl").write_text("a passage\n"),
    )),
    ("data.files", "closure.jsonl:1", lambda root: spoil_number(root / DATA / "closure.jsonl")),
    ("data.ticket", ".scratch/t.md", lambda root: (root / ".scratch" / "t.md").write_text("Data: no.such.record\n")),
    ("data.ticket", ".scratch/t.md", lambda root: (root / ".scratch" / "t.md").write_text("Data: no.such.*\n")),
    ("data.ticket", ".scratch/t.md", lambda root: (root / ".scratch" / "t.md").write_text("Data: not an id\n")),
    ("data.ticket", ".scratch/t.md", lambda root: (root / ".scratch" / "t.md").write_text("Data:no.such.record\n")),
    ("data.ticket", ".scratch/t.md", lambda root: (root / ".scratch" / "t.md").write_text("Data:\tno.such.record\n")),
)


def spoil_number(path: pathlib.Path) -> None:
    path.write_text(path.read_text(encoding="utf-8").replace('"tol_rel":0.05', '"tol_rel":0.0500'), encoding="utf-8")


def write_store(s: Store, root: pathlib.Path) -> None:
    (root / DATA).mkdir()
    (root / ".scratch").mkdir()
    tables = dict(zip(FILES, (s.works, s.packet, s.ranges, s.inferences, s.closures)))
    for name, rows in tables.items():
        lines = "".join(canonical(r) + "\n" for r in rows)
        (root / DATA / f"{name}.jsonl").write_text(lines, encoding="utf-8")
    for path, names in s.tickets.items():
        (root / path).write_text(f"Data: {', '.join(names)}\n", encoding="utf-8")


def run(s: Store) -> list[Violation]:
    if shapes := RULES[0][1](s):
        return shapes
    return [v for _, check in RULES[1:] for v in check(s)]


def probes() -> list[Violation]:
    if base := run(synthetic_store()):
        return [Violation("data.probe", "base", "a probe proves nothing on a base that already fails"), *base]
    out = []
    for i, (code, subject, breaks) in enumerate(PROBES):
        s = synthetic_store()
        breaks(s)
        if not any(v.code == code and v.subject == subject for v in run(s)):
            out.append(Violation("data.probe", f"{code} {i}", f"{subject} was not refused"))
    for i, (code, subject, breaks) in enumerate(FILE_PROBES):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            write_store(synthetic_store(), root)
            if read_back := run(load(root)):
                return [Violation("data.probe", "files", "the base does not survive being written"), *read_back]
            breaks(root)
            if not any(v.code == code and v.subject == subject for v in run(load(root))):
                out.append(Violation("data.probe", f"{code} file {i}", f"{subject} was not refused"))
    named = {p[0] for p in (*PROBES, *FILE_PROBES)}
    out += [Violation("data.probe", code, "the rule has no probe") for code, _ in RULES if code not in named]
    return out


def main(argv: list[str]) -> int:
    if len(argv) != 2 or argv[1] not in ("check", "probes"):
        print(__doc__)
        return 2
    if argv[1] == "probes":
        found = probes()
        for v in found:
            print(v)
        print(f"data: {len(RULES)} rules, {len(PROBES) + len(FILE_PROBES)} probes, {len(found)} did not bite")
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
