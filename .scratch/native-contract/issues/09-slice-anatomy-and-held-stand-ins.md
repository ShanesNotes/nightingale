# 09: Arena slice anatomy, held stand-ins and the census

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The arena slice's anatomy is declared as data (its beds, five junctions, the pleural and abdominal enclosures, positions), every card outside the slice is held with a declaration of what it supplies, and the coupling census is counted by the compiler.

**Blocked by:** 03

**Status:** ready-for-agent

- [ ] The slice anatomy passes the graph checks; pleural and abdominal pressure reach the enclosed beds as held enclosure pressure
- [ ] Every held stand-in declares its supplies; a held route its declaration omits is refused at compile
- [ ] The census check passes: 38 = 27 mapped + 11 listed unmapped, rows matching the coupling map's lines 77 to 114
