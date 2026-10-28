# FL. Fluid compartments and the endothelium

Water distribution, Starling exchange, lymph and the vessel wall. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

Starling exchange with a nonlinear interstitial compliance and lymph return.

## What it owns

Water and osmole distribution between plasma, interstitial, intracellular and transcellular spaces, Starling filtration, lymph return, the intake and output ledger, and the vessel wall state (glycocalyx, tissue factor exposure, NO synthase capacity).

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S1 (Starling filtration, lymph return), S8 (owns distribution and the I&O ledger).
- **Reads:** S4, S6, S7 (endothelial receptors), S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** RS, NS, GI and MS (interstitial volume per region: lung water, brain water and ICP, gut wall, skin edema, compartment pressure).
- **In:** CV (capillary and venous pressure), IM (permeability and glycocalyx), devices (infusion edges, infiltration).

## Place in the contract ownership register

Interstitial water: FL owns Starling filtration and lymph return on every bed. Vessel wall: FL owns its dynamics (A section 3). Contract candidate B: one `VesselWall` per surface shared by fluid, immune and clotting laws.

## Injury, failure and demise

Hypovolemia, hypervolemia, capillary leak, hypoalbuminemia, third spacing, cerebral edema, infiltration and extravasation. **Demise:** through shock (CV) and cerebral edema (NS).

[pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Osmotic water in seconds to minutes, crystalloid distribution in 20 to 30 min, refill after hemorrhage over hours, albumin pool over weeks.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | `vitals-substances` (`tissue`, `tissue_model`, `diffusion`) and `vitals-transport` (compartment graph) |
| Pulse reference | `TissueModel.cpp` in Pulse REL_4_3_2; today's crate cites it line by line in its source headers |
| Proposed crate | `ng-fluids` (see the [crate map](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 2 (plasma and interstitial water split for the kidney loop), 7 (full volume loop) (see the [build order](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

No global edema pool; each organ's interstitium is its own compartment.

## Open

Regional filtration coefficients and glycocalyx shedding (arena atlas flag). Whether the vessel wall is FL state (candidate A) or a typed shared seam (candidate B).

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card FL at line 1433; injury table at line 1498; timescales at line 1512. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), fluid section at line 621.
- Contract pass 1: [synthesis](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
