# IM. Immune and inflammation

Innate and selected adaptive response. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

A reduced innate inflammation model (pathogen, activated phagocytes, pro- and anti-inflammatory mediators, tissue damage) with antibiotic pharmacodynamics on the pathogen term.

## What it owns

Pathogen load per site, phagocyte activation, mediators and cytokines as ligands, antibody states, infection foci, hazards for secondary events.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S6 (fever cost through MT), S7 (cytokines, histamine, mediators), S8 (through FL permeability), S9 (tissue factor, hepcidin, G-CSF, destruction hazards).
- **Reads:** S1, S2, S5, S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** CV (iNOS and myocardial depressant factors), FL (endothelial permeability, glycocalyx), HC (tissue factor, hepcidin), NS (PGE2 set point), MT (mitochondrial efficiency, flagged).
- **In:** RS and GI (infection sources), MS (skin breaks, wounds), devices (line and catheter sources).

## Place in the contract ownership register

Formed elements: IM owns activation reactions. Vessel wall: IM reaches it only through cytokine ligands on endothelial receptor pools (A section 3).

## Injury, failure and demise

Local infection, abscess, SIRS and sepsis, septic shock, immunoparalysis, anaphylaxis, antibody-mediated reactions. **Demise:** septic shock and anaphylaxis act through CV, FL and HC.

[pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Histamine in minutes, cytokine peaks over hours, CRP and culture timing over 1 to 3 days, antibody states over 1 to 2 weeks.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Pulse reference | No matching Pulse system model; parts appear as Pulse actions or inside other models |
| Proposed crate | `ng-immune` (see the [crate map](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md)) |

## Decided

Mediators are ligands on the shared receptor table, never direct parameter writes.

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card IM at line 1306; injury table at line 1377; timescales at line 1390. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), immune section at line 569.
- Contract pass 1: [synthesis](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).
