# 06: Atlas card trims, applied as each system is built

Type: task
Status: open
Blocked by: none

The atlas is frozen at import. When a system's crate is built, its card gets the uniform trim and its own cuts below, and the open items moved here from its card are answered or carried. Line numbers are at the audit's stand-in commit; re-derive them on the card. Never shift lines 1 to 114 of `docs/atlas/coupling-map.md`, and keep lines 77 to 114 byte for byte, because the sketch's census asserts them.

## The uniform trim (U)

All 13 cards share the template's headings on the same lines. L3 drop "and contract pass 1". L9 "What it owns" becomes "Laws it owns", and L13 goes (the atlas README states the rule once: a system owns laws and their internal non-material state, never a conserved pool, and the record holds every amount). L27-29: drop the heading "Place in the contract ownership register" and its candidate-A citations, and fold every process, reaction and derived block L29 lists into "Laws it owns" as proposed laws (they carry assignments the compiled catalog does not, such as RN's filtration and tubule transfers and MT's heat loss). Cut L41 (the same rate-lane note in every card; the README states it once). Replace the L43-52 "Where it lives" table with one line, "Proposed crate `ng-x`; Pulse diff reference `XModel.cpp` (0002)", dropping crate today (`vitals-*`), lane, build step and proof status. L54 "Decided" becomes "Atlas rules (proposed)" (vision L28). Cut L66 (council links) and L68 (PROVENANCE pointer); the README states the source hashes once.

## Per-card cuts

- `docs/atlas/_template.md`: "Mirror the uniform card trim: L9 heading ""What it owns"" -> ""Laws it owns""; L11 -> ""The laws it owns and their internal non-material state; never a conserved pool.""; L23-25: drop the heading 'Place in the contract ownership register' and fold its prompt into 'Laws it owns' (processes, reactions and derived blocks the system proposes to own); replace the L35-44 table with ""Proposed crate (name not decided) and Pulse diff reference (0002), one line.""; L46 heading ""Decided"" -> ""Atlas rules (proposed)"" unless a record is cited; L56 drop ""council outputs"". Kept because 0006 L18 adds a body boundary that may need a card."
- `docs/atlas/blood-chemistry.md`: "Uniform trim U (L3 drop ""and contract pass 1""; L9 -> ""Laws it owns""; cut L13; cut L27-29; cut L41; L43-52 -> ""Proposed crate ng-x; Pulse diff reference X (0002)""; L54 -> ""Atlas rules (proposed)""; cut L66 and L68) plus: L60 replace ""(contract candidate A limit 4: only bulk charge conservation today)"" and ""(decision 0005 question 4)"" with solver contract references (charge compiled L135 and certified L248; core picked by the arena L269-279)."
- `docs/atlas/cardiovascular.md`: "U plus: L11 ""The one global hemodynamic solve"" -> ""The hemodynamic laws""; L17 drop ""owns the solve and"" (keep the all-vascular-edges claim as the open CV/RN overlap); L60 cut ""How rhythm modes and hazards enter the contract (decision 0005 question 5)"" and ""Fast step size (arena atlas decision 1: 2 ms native assumed)"", and add ""Natriuretic peptide secretion: CV or EN (open, solver contract L166)"" (from L29)."
- `docs/atlas/coupling-map.md`: "CONSTRAINT: solver contract L111, L152-166 and the sketch (src/ng_contract/catalog.rs:2, :17; evidence/census.txt:1) key on lines 77-114 and their labels. So lines 1-114 must keep their count (edit one line for one) and L77-114 must stay byte-identical, unless the catalog and contract citations are re-keyed in the same change. In place: L7 -> ""Owned by the body's registry, with every amount in the patient record (0006), never by a system.""; L11 drop ""CV owns the solve"" -> CV owns the assembled hemodynamic laws and each system the laws on its own bed's edges (overlap open, solver contract L166); L18 drop ""and the I&O ledger""; L37 ""CV hemodynamic solve"" -> ""CV hemodynamic laws""; L65-66 route Networks and Organs through a solver proposal that the record validates and publishes, instead of writing REC (two lines for two); L69 ""(ADR-67)"" -> ""(0003; solver contract)""; L121 ""the contract's ownership registry in both drafts"" -> ""the solver contract's compiled catalog"". Below L114 (safe to delete): cut L125 (test 3). Add one boundary line: ""Body boundary: intake, urine exit, evaporative loss, posture geometry, environment, clock and sleep on one timeline; the baseline is 0006's frame (solver contract L164); at rung 0 the L89 circadian phase comes from the forced boundary clock (0006 L28)"". Never relabel L89. Also at L117 (below the census rows, safe): cut 'and oxygen debt to every card for injury integrals'."
- `docs/atlas/endocrine.md`: "U. Nothing else."
- `docs/atlas/fluids-endothelium.md`: "U plus: L11 cut ""the intake and output ledger"" and L17 cut ""and the I&O ledger""; move L60's second sentence into L56 as ""The vessel wall is one typed seam per surface shared by fluid, immune and clotting laws (contract-B piece, 0005 L14, accepted by 0006 L69)"" (from L29)."
- `docs/atlas/gastrointestinal.md`: "U plus: optionally L11 ""Luminal compartments"" -> ""Luminal transit laws""."
- `docs/atlas/hematology.md`: "U plus: L7 cut "", then a reduced coagulation model whose outputs are INR, aPTT and bleeding competence""; L17 ""S9 (owns production, destruction, counts, factors)"" -> ""S9 (production, destruction, factors)""."
- `docs/atlas/hepatic.md`: "U (Pulse line reads ""HepaticModel.cpp, empty"") plus: L7 cut "", then well-stirred drug clearance, so first-pass metabolism, flow-limited clearance and cirrhosis dosing come from one equation""; L56 cut ""Hepatic function is built new; there is nothing to port."" with no replacement."
- `docs/atlas/immune.md`: "U plus: L7 -> ""At rung 0, a quiescent held store with its ports present at zero flux; the reduced inflammation model is a later rung (0006 L18, Q1 section 3b row 14)""."
- `docs/atlas/metabolism-thermal.md`: "U plus: L11 cut ""tissue O2"" (an S2 amount) and ""the energy and work ledger""; L60 check ""Number of thermal nodes (eight assumed)"" against 0006 L57 (Gagge two-node model accepted)."
- `docs/atlas/musculoskeletal-skin.md`: U plus: L7 cut 'with a pressure-injury dose model'; L11 'lean mass' becomes 'lean mass turnover laws' and 'pressure injury dose' goes; L60 cut 'Pressure-injury thresholds in humans (arena atlas flag).'. Not import edits (routing, open items for a ticket): creatinine filed under S9 at L17, receptor density at L17, and the MT input at L25 that the map does not draw.
- `docs/atlas/nervous.md`: "U plus: L11 ""intracranial pressure and volume"" -> ""intracranial pressure laws (ownership open with FL, coupling-map L111)"", and add ""reuptake and clearance reactions"" (from L29)."
- `docs/atlas/renal.md`: "U (proposed crate ng-renal, dropping the ng-kidney aside) plus: L56 cut ""Both contract drafts use the kidney as the first bench organ, and the next council pass is the kidney.""; L60 ""How much tubular detail the first bench needs"" -> ""How much tubular detail rung 0 needs (0006 L97)""."
- `docs/atlas/respiratory.md`: "U. Nothing else."

## Open items moved from the cards by the hygiene pass

### `docs/atlas/_template.md`

What is not settled, and which decision or packet will settle it.

### `docs/atlas/blood-chemistry.md`

Electroneutrality and membrane charge (contract candidate A limit 4: only bulk charge conservation today). Stiff chemistry needs the implicit solver (pulse 0005 question 4).

### `docs/atlas/cardiovascular.md`

How rhythm modes and hazards enter the contract (pulse 0005 question 5). Fast step size (arena atlas decision 1: 2 ms native assumed). Whether the hemodynamic network is its own crate (`ng-net-hemo` in the arena atlas) or part of this one.

### `docs/atlas/endocrine.md`

Portal to peripheral insulin gradient (shared with HP). Which axes are admitted first beyond glucose and RAAS.

### `docs/atlas/fluids-endothelium.md`

Regional filtration coefficients and glycocalyx shedding (arena atlas flag). Whether the vessel wall is FL state (candidate A) or a typed shared seam (candidate B).

### `docs/atlas/gastrointestinal.md`

Enteric nervous control (arena atlas flag: folded into motility modes for now).

### `docs/atlas/hematology.md`

Thrombosis hazard coefficients (arena atlas flag). How much of the cascade runs at runtime (arena atlas: not the full Hockin-Mann cascade yet).

### `docs/atlas/hepatic.md`

Zonation, shunt fraction by stage, regeneration rate (arena atlas flags). Portal to peripheral insulin gradient (shared with EN).

### `docs/atlas/immune.md`

Cytopathic hypoxia size (arena atlas flag). Compensatory anti-inflammatory kinetics (flagged).

### `docs/atlas/metabolism-thermal.md`

Critical PO2 and anaerobic constants per bed (arena atlas flag). Number of thermal nodes (eight assumed). Whether the thermal network is its own crate (`ng-net-thermal`).

### `docs/atlas/musculoskeletal-skin.md`

Pressure-injury thresholds in humans (arena atlas flag). Bone as a reservoir only.

### `docs/atlas/nervous.md`

Consciousness index shape (arena atlas flag: no identified model exists; keep it labeled). Transmitter release and clearance rates (fitted).

### `docs/atlas/renal.md`

Crate name. Representative nephron per kidney versus cohorts (arena atlas flag; the DAE atlas wants nephron cohorts and cortex and medulla). How much tubular detail the first bench needs.

### `docs/atlas/respiratory.md`

Number of alveolar units (arena atlas flag: two per lung, resolver-visible). Whether the airway graph is its own crate (`ng-net-airway`).

## Comments

Triage, 2026-10-29 (developer reviewer's call, delegated): ready. Each card trim lands with the crate of its system (SRS section 2, System), so the atlas matches the code as the 13 systems are built; nothing blocks it.
