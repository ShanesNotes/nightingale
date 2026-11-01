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

owners = re.findall(r'^\s*(\w+) = "(\w+)", (slice|held);$', open(OWNERS, encoding="utf-8").read(), re.M)
CARD = {name: card for name, card, _ in owners}
SLICE = {card for _, card, place in owners if place == "slice"}

arrows = []
in_graph = False
for number, text in enumerate(open(MAP, encoding="utf-8").read().splitlines(), start=1):
    if text.startswith("## Direct couplings"):
        in_graph = True
    elif in_graph and text.startswith("## "):
        break
    match = re.match(r'^\s+([A-Z]{2}) -- "([^"]+)" --> ([A-Z]{2})\s*$', text)
    if in_graph and match:
        arrows.append((number, match.group(1), match.group(3), match.group(2)))


def table(path):
    source = open(path, encoding="utf-8").read()
    start = source.index("coupling_map! {\n")
    return source[start:source.index("\n}\n", start) + 3]


body = table(CATALOG)
code = re.sub(r'//[^\n]*|/\*.*?\*/', '', body, flags=re.S)
rows = []
for match in re.finditer(r'(\d+)\s+(\w+)\s*=>\s*(\w+),\s*"([^"]+)",\s*(live|input|sink|relation|outside|unresolved)', code):
    payload = re.match(r'\s*\((\w+) \{([^}]*)\}', code[match.end():])
    fields = re.findall(r'(\w+):', payload.group(2)) if payload else []
    rows.append((int(match.group(1)), CARD[match.group(2)], CARD[match.group(3)], match.group(4),
                 match.group(5), payload.group(1) if payload else "-", fields))

failures = []
if body != table(SKETCH_CATALOG):
    failures.append(f"the tables differ: {CATALOG} and {SKETCH_CATALOG}")
if len(arrows) != len(rows):
    failures.append(f"count differs: map {len(arrows)}, catalog {len(rows)}")
for arrow, row in zip(arrows, rows):
    if arrow != row[:4]:
        failures.append(f"map {arrow} != catalog {row[:4]}")
if len({row[0] for row in rows}) != len(rows):
    failures.append("duplicate catalog line")

mapped = [row for row in rows if row[4] in MAPPED]
incident = [row for row in rows if row[1] in SLICE or row[2] in SLICE]
print(f"map source: {MAP} (Direct couplings, lines {arrows[0][0]}-{arrows[-1][0]})")
print(f"map edges = {len(arrows)}; catalog rows = {len(rows)}")
print(f"mapped = {len(mapped)}; listed unmapped = {len(rows) - len(mapped)}")
print(f"slice-incident = {len(incident)}; mapped = {sum(r[4] in MAPPED for r in incident)}; "
      f"unmapped = {sum(r[4] not in MAPPED for r in incident)}")
for kind in ("live", "input", "sink", "relation", "outside", "unresolved"):
    print(f"  {kind:<10} {[row[0] for row in rows if row[4] == kind]}")
print("line, endpoints and label equality, order and uniqueness:", "PASS" if not failures else "FAIL")
for failure in failures:
    print("  " + failure)
print()
print("line from->to  disposition  payload: fields  | map label")
for line, src, dst, label, kind, payload, fields in rows:
    print(f"{line} {src}->{dst}  {kind:<10}  {payload}: {', '.join(fields) or '-'}  | {label}")
sys.exit(1 if failures else 0)
