"""Compare the generated coupling catalog (src/ng_contract/catalog.rs) with the
"Direct couplings" graph of docs/atlas/coupling-map.md, row by row.

Run from solver-contract-sketch/:  python3 -I evidence/census-check.py
Evidence only; the repository gate the catalog's doc comment names is future work.
"""
import re
import sys

MAP = "../../atlas/coupling-map.md"
CATALOG = "src/ng_contract/catalog.rs"
CARD = {
    "Cardiovascular": "CV", "Fluids": "FL", "Renal": "RN", "Nervous": "NS",
    "Respiratory": "RS", "Metabolism": "MT", "Musculoskeletal": "MS", "Endocrine": "EN",
    "Hepatic": "HP", "Gastrointestinal": "GI", "Immune": "IM", "Hematology": "HC",
    "BloodChemistry": "BC",
}
SLICE = {"CV", "FL", "RN", "NS"}
MAPPED = {"live", "input", "sink", "relation"}

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

source = open(CATALOG, encoding="utf-8").read()
body = source[source.index("coupling_map! {"):]
rows = []
for match in re.finditer(r'(\d+) (\w+) => (\w+), "([^"]+)",\s*(live|input|sink|relation|outside|unresolved)', body):
    payload = re.match(r'\s*\((\w+) \{([^}]*)\}', body[match.end():])
    fields = re.findall(r'(\w+):', payload.group(2)) if payload else []
    rows.append((int(match.group(1)), CARD[match.group(2)], CARD[match.group(3)], match.group(4),
                 match.group(5), payload.group(1) if payload else "-", fields))

failures = []
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
