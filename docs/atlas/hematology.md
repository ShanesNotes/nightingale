# HC. Hematology and coagulation

Formed elements, factors and clotting. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

Red cell mass as a conserved inventory with production, destruction, loss and transfusion, then a reduced coagulation model whose outputs are INR, aPTT and bleeding competence.

## What it owns

Production, destruction and consumption of red cells, platelets, leukocytes and factors, clot competence at bleeding edges, products for transfusion, thrombosis events.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S1 (bleeding edge competence, products), S2 (carrier capacity, P50 through DPG), S4 (lysis release, citrate and K in products), S7 (coagulation ligands), S9 (owns production, destruction, counts, factors).
- **Reads:** S3, S5, S6, S8, S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** CV (hemostatic competence at bleeding edges, viscosity, thrombosis events).
- **In:** IM (tissue factor, hepcidin, destruction hazards), HP (factor synthesis), RN (EPO), FL (vessel wall tissue factor exposure), MT (temperature).

## Place in the contract ownership register

Formed elements and factors: HC owns production, destruction and consumption. Blood volume: HC owns red cell production and destruction reactions. Vessel wall: HC reads tissue factor exposure from FL (A section 3); contract candidate B makes the wall a shared typed seam.

## Injury, failure and demise

Anemia, polycythemia, thrombocytopenia, coagulopathy, DIC, thrombosis, HIT. **Demise:** exsanguination through CV and the lethal triad of coagulopathy, acidosis and hypothermia.

[pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Platelet plug in minutes, factor half-lives over days, reticulocyte response in 3 to 5 days, hemoglobin recovery over weeks.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | None as a module; hemorrhage lives in `vitals-cardiovascular` (`hemorrhage`) |
| Pulse reference | No matching Pulse system model; parts appear as Pulse actions or inside other models |
| Proposed crate | `ng-hematology` (see the [crate map](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 10 (see the [build order](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

Hemoglobin concentration is mass over blood volume, so dilution and refill emerge.

## Open

Thrombosis hazard coefficients (arena atlas flag). How much of the cascade runs at runtime (arena atlas: not the full Hockin-Mann cascade yet).

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card HC at line 1167; injury table at line 1249; timescales at line 1263. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), hematology section at line 517.
- Contract pass 1: [synthesis](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
