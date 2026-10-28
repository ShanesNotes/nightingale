# RN. Renal

Kidneys and urinary tract. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

Filtration is a Starling balance across a glomerulus whose perfusion is autoregulated and whose nephron number is finite, and everything downstream is saturable transport along a tubule under aldosterone and vasopressin.

## What it owns

Glomerular filtration on the afferent and efferent edges, tubular transport by segment, urine, renin, EPO and calcitriol secretion, tubuloglomerular feedback, dialysis edges.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S1 (afferent and efferent edges, dialysis edges), S4 (excretion, reclamation, acid), S5 (glucose spill, small gluconeogenesis), S7 (renin, EPO, calcitriol), S8 (water excretion), S9 (EPO flux to HC).
- **Reads:** S2 (tissue PO2 for EPO), S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** CV (tubuloglomerular feedback on the afferent edge), EN (renin flux starts RAAS).
- **In:** CV (renal perfusion), EN (aldosterone, vasopressin, natriuretic peptides through receptors), GI (intra-abdominal pressure), MS and HC (myoglobin and free hemoglobin as tubular toxins), devices (Foley, dialysis).

## Place in the contract ownership register

Blood volume: RN owns glomerular filtration edges. Solutes: RN owns tubule membrane transfers. Hormones: RN owns renin and EPO secretion and its share of clearance. Drugs: RN owns excretion (A section 3).

## Injury, failure and demise

Pre-renal, ATN, obstruction, glomerular injury, chronic nephron loss, cardiorenal and hepatorenal states, pigment injury. **Demise:** renal failure kills through what it retains (potassium, acid, water), which reach CV rhythm hazards and RS through BC and FL; RN has no death state of its own.

[pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Myogenic autoregulation in seconds, tubuloglomerular feedback within a minute, hormones over minutes to hours, new creatinine steady state over days, ATN course over weeks.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | `vitals-renal` |
| Pulse reference | `RenalModel.cpp` in Pulse REL_4_3_2; today's crate cites it line by line in its source headers |
| Proposed crate | `ng-renal` (contract candidate A proposes `ng-kidney`) (see the [crate map](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 1 (kidney on the bench), 2 (kidney on a minimal circulation), 7 (full volume loop) (see the [build order](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

Nephron mass is the parameter CKD acts on. Both contract drafts use the kidney as the first bench organ, and the next council pass is the kidney.

## Open

Crate name. Representative nephron per kidney versus cohorts (arena atlas flag; the DAE atlas wants nephron cohorts and cortex and medulla). How much tubular detail the first bench needs.

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card RN at line 516; injury table at line 600; timescales at line 614. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), renal section at line 231.
- Contract pass 1: [synthesis](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
