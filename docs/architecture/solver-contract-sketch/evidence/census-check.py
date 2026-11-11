"""Compare the crate's coupling catalog with the "Direct couplings" graph of
docs/atlas/coupling-map.md, row by row, and hold the sketch's copy of the table to the crate's,
byte for byte.

Run from solver-contract-sketch/:  python3 -I evidence/census-check.py
tools/verify (verify.d/20-sketch.sh) runs this as a gate. A change to the crate's catalog or owner
table runs everything (tools/gate/paths.toml), so a change to either file alone reaches this script.
"""
import re
import sys

MAP = "../../atlas/coupling-map.md"
CATALOG = "../../../crates/ng-contract/src/catalog.rs"
SKETCH_CATALOG = "src/ng_contract/catalog.rs"
OWNERS = "../../../crates/ng-contract/src/system.rs"
MAPPED = {"live", "input", "sink", "relation"}
SLICE_KINDS = {"live", "input", "sink"}
SLICE_REASONED = {"relation", "outside", "unresolved"}
SLICE_COUNTS = {"rows": 38, "mapped": 27, "listed unmapped": 11, "incident mapped": 27, "incident unmapped": 1}
BODY_COUNTS = {"rows": 38, "wire a field": 18, "relations": 5, "hold every field": 15, "wire and hold": 6,
               "wired fields": 21, "held fields": 29}
SLICE_LINES = {
    "live": [77, 80, 97],
    "input": [83, 90, 91, 99, 100, 102, 105, 106, 108, 114],
    "sink": [78, 79, 81, 86, 87, 88, 89, 98, 110, 112, 113],
    "relation": [82, 96, 101],
    "outside": [84, 85, 92, 93, 94, 95, 103, 104, 107, 109],
    "unresolved": [111],
}
BODY_LIVE = [77, 78, 79, 80, 81, 83, 84, 85, 86, 87, 88, 89, 90, 91, 95, 97, 98, 114]
BODY_HELD = [92, 93, 94, 100, 102, 103, 105, 106, 107, 108, 109, 110, 111, 112, 113]
RELATIONS = [82, 96, 99, 101, 104]
WIRE_AND_HOLD = [79, 87, 89, 90, 95, 114]

owners = re.findall(r'^\s*(\w+) = "(\w+)", (slice|held);$', open(OWNERS, encoding="utf-8").read(), re.M)
CARD = {name: card for name, card, _ in owners}
SLICE = {card for _, card, place in owners if place == "slice"}

arrows = []
in_graph = False
for map_line, text in enumerate(open(MAP, encoding="utf-8").read().splitlines(), start=1):
    if text.startswith("## Direct couplings"):
        in_graph = True
    elif in_graph and text.startswith("## "):
        break
    match = re.match(r'^\s+([A-Z]{2}) -- "([^"]+)" --> ([A-Z]{2})\s*$', text)
    if in_graph and match:
        arrows.append((map_line, match.group(1), match.group(3), match.group(2)))


def table(path):
    source = open(path, encoding="utf-8").read()
    start = source.index("coupling_map! {\n")
    return source[start:source.index("\n}\n", start) + 3]


TOKEN = re.compile(r'\s+|//[^\n]*|/\*.*?\*/|(?P<tok>"(?:[^"\\]|\\.)*"|\d+|[A-Za-z_]\w*|=>|[,;:(){}\[\]<>])', re.S)


class Unparsed(Exception):
    pass


class Tokens:
    def __init__(self, text):
        self.items, at = [], 0
        while at < len(text):
            match = TOKEN.match(text, at)
            if not match:
                raise Unparsed(f"unexpected {text[at:at + 20]!r}")
            if match.group("tok"):
                self.items.append(match.group("tok"))
            at = match.end()
        self.at = 0

    def peek(self):
        return self.items[self.at] if self.at < len(self.items) else None

    def take(self, want=None):
        token = self.peek()
        if token is None or (want is not None and token != want):
            raise Unparsed(f"expected {want or 'a token'} at {token!r}, token {self.at}")
        self.at += 1
        return token

    def kind(self, pattern, what):
        token = self.take()
        if not re.fullmatch(pattern, token):
            raise Unparsed(f"expected {what} at {token!r}, token {self.at - 1}")
        return token


def ident(tokens):
    return tokens.kind(r"[A-Za-z_]\w*", "a name")


def string(tokens):
    return tokens.kind(r'"(?:[^"\\]|\\.)*"', "a string")[1:-1]


def number(tokens):
    return int(tokens.kind(r"\d+", "a number"))


def unit(tokens):
    text = ident(tokens)
    if tokens.peek() == "<":
        tokens.take("<")
        args = [unit(tokens)]
        while tokens.peek() == ",":
            tokens.take(",")
            args.append(unit(tokens))
        tokens.take(">")
        text += "<" + ", ".join(args) + ">"
    return text


def listed(tokens, close, item):
    items = [item(tokens)]
    while tokens.peek() == ",":
        tokens.take(",")
        if tokens.peek() == close:
            break
        items.append(item(tokens))
    tokens.take(close)
    return items


def wired_field(tokens):
    name = ident(tokens)
    tokens.take(":")
    kind = unit(tokens)
    tokens.take("=>")
    return name, kind, number(tokens)


def held_field(tokens):
    name = ident(tokens)
    tokens.take(":")
    kind = unit(tokens)
    tokens.take("=>")
    holder = ident(tokens)
    if holder not in CARD:
        raise Unparsed(f"holder {holder} is no owner")
    return name, kind, CARD[holder], string(tokens)


def held_list(tokens):
    tokens.take("[")
    return listed(tokens, "]", held_field)


def row(tokens):
    line = number(tokens)
    src = ident(tokens)
    tokens.take("=>")
    dst = ident(tokens)
    tokens.take(",")
    label = string(tokens)
    tokens.take(",")
    kind = ident(tokens)
    if kind in SLICE_REASONED:
        slice_reason = string(tokens)
    elif kind in SLICE_KINDS:
        slice_reason = None
    else:
        raise Unparsed(f"line {line}: no slice disposition {kind}")
    tokens.take(",")
    plan = ident(tokens)
    payload, wired, held, relation = None, [], [], None
    if plan == "relation":
        relation = string(tokens)
    elif plan == "wired":
        tokens.take("(")
        payload = ident(tokens)
        tokens.take("{")
        wired = listed(tokens, "}", wired_field)
        tokens.take(")")
        if tokens.peek() == "held":
            tokens.take("held")
            held = held_list(tokens)
    elif plan == "held":
        held = held_list(tokens)
    else:
        raise Unparsed(f"line {line}: no whole-body plan {plan}")
    tokens.take(";")
    if src not in CARD or dst not in CARD:
        raise Unparsed(f"line {line}: an end is no owner")
    return {"line": line, "from": CARD[src], "to": CARD[dst], "label": label, "slice": kind,
            "slice_reason": slice_reason, "payload": payload, "wired": wired, "held": held, "relation": relation}


body = table(CATALOG)
rows = []
failures = []
try:
    tokens = Tokens(body[body.index("{") + 1:body.rindex("}")])
    while tokens.peek() is not None:
        rows.append(row(tokens))
except Unparsed as e:
    failures.append(f"the table does not parse after {len(rows)} rows: {e}")

if body != table(SKETCH_CATALOG):
    failures.append(f"the tables differ: {CATALOG} and {SKETCH_CATALOG}")
if len(arrows) != len(rows):
    failures.append(f"count differs: map {len(arrows)}, catalog {len(rows)}")
for arrow, r in zip(arrows, rows):
    if arrow != (r["line"], r["from"], r["to"], r["label"]):
        failures.append(f"map {arrow} != catalog {(r['line'], r['from'], r['to'], r['label'])}")
if len({r["line"] for r in rows}) != len(rows):
    failures.append("duplicate catalog line")
for r in rows:
    reasons = [r["slice_reason"], r["relation"], *(reason for *_, reason in r["held"])]
    if "" in reasons:
        failures.append(f"line {r['line']}: a reason is empty")

incident = [r for r in rows if r["from"] in SLICE or r["to"] in SLICE]
slice_census = {
    "rows": len(rows),
    "mapped": sum(r["slice"] in MAPPED for r in rows),
    "listed unmapped": sum(r["slice"] not in MAPPED for r in rows),
    "incident mapped": sum(r["slice"] in MAPPED for r in incident),
    "incident unmapped": sum(r["slice"] not in MAPPED for r in incident),
}
body_census = {
    "rows": len(rows),
    "wire a field": sum(bool(r["wired"]) for r in rows),
    "relations": sum(r["relation"] is not None for r in rows),
    "hold every field": sum(bool(r["held"]) and not r["wired"] for r in rows),
    "wire and hold": sum(bool(r["wired"]) and bool(r["held"]) for r in rows),
    "wired fields": sum(len(r["wired"]) for r in rows),
    "held fields": sum(len(r["held"]) for r in rows),
}
for name, got, want in (("slice", slice_census, SLICE_COUNTS), ("whole body", body_census, BODY_COUNTS)):
    for key, count in want.items():
        if got[key] != count:
            failures.append(f"{name} census: {key} = {got[key]}, not {count}")
for kind, want in SLICE_LINES.items():
    got = [r["line"] for r in rows if r["slice"] == kind]
    if got != want:
        failures.append(f"slice census: {kind} are lines {got}, not {want}")
for name, got, want in (
    ("live", [r["line"] for r in rows if r["wired"]], BODY_LIVE),
    ("held", [r["line"] for r in rows if r["held"] and not r["wired"]], BODY_HELD),
    ("relations", [r["line"] for r in rows if r["relation"] is not None], RELATIONS),
    ("wire and hold", [r["line"] for r in rows if r["wired"] and r["held"]], WIRE_AND_HOLD),
):
    if got != want:
        failures.append(f"whole body census: {name} are lines {got}, not {want}")

print(f"map source: {MAP} (Direct couplings, lines {arrows[0][0]}-{arrows[-1][0]})")
print(f"map edges = {len(arrows)}; catalog rows = {len(rows)}")
print("slice census: " + "; ".join(f"{k} = {v}" for k, v in slice_census.items()))
for kind in SLICE_LINES:
    print(f"  {kind:<10} {[r['line'] for r in rows if r['slice'] == kind]}")
print("whole body census: " + "; ".join(f"{k} = {v}" for k, v in body_census.items()))
print("parse, line, endpoints and label equality, order, uniqueness, reasons and both censuses:",
      "PASS" if not failures else "FAIL")
for failure in failures:
    print("  " + failure)
print()
print("line from->to  slice       | whole body: relation, or payload: wired (ticket); held (holder) | map label")
for r in rows:
    parts = []
    if r["wired"]:
        parts.append(f"{r['payload']}: " + ", ".join(f"{name} ({ticket})" for name, _, ticket in r["wired"]))
    if r["held"]:
        parts.append("held " + ", ".join(f"{name} ({holder})" for name, _, holder, _ in r["held"]))
    print(f"{r['line']} {r['from']}->{r['to']}  {r['slice']:<10}  | {'; '.join(parts) or 'relation'}  | {r['label']}")
sys.exit(1 if failures else 0)
