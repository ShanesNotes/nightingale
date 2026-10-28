# BC. Blood and acid-base chemistry

Speciation in every compartment. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

The Stewart physicochemical balance per compartment from actual ion sums, with O2 and CO2 speciation that conserves mass exactly.

## What it owns

Speciation only: pH, ionized fractions, PO2, SO2, PCO2 from totals, hemoglobin and temperature. Cell membrane pumps and shifts. Protein binding of drugs.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S2 and S3 (speciation only, mass conserving), S4 (membrane shifts, speciation), S8 (tonicity-driven cell water, with FL).
- **Reads:** S1, S5, S6, S7, S9.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** None as direct wires; everything it computes is read from the substrates.
- **In:** Everything that posts to S2 to S5 and S8.

## Place in the contract ownership register

Solutes: BC owns speciation as a pure derived block that writes no inventory, plus cell membrane transfers. Gases: BC derives PO2, SO2 and PCO2. Drugs: BC owns protein binding (A section 3).

## Injury, failure and demise

BC is a medium, not an organ. Its disturbances (gap and non-gap acidosis, alkalosis, potassium, sodium, calcium, magnesium and phosphate disorders) come from other cards. **Demise:** hyperkalemia and severe acidemia feed CV rhythm and arrest hazards.

[pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Milliseconds (CO2 hydration) inside the fast speciation solve, to hours (intracellular buffering) and days (renal compensation).

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | `vitals-substances` (`blood_chemistry`, `hemoglobin`, `saturation`), with the dynamic SID variant registered in `vitals-machine` |
| Pulse reference | `BloodChemistryModel.cpp` in Pulse REL_4_3_2; today's crate cites it line by line in its source headers |
| Proposed crate | `ng-chemistry` (see the [crate map](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 3 (transport and chemistry, with the coupled solver) (see the [build order](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

Speciation never creates or destroys mass. Strong ion difference comes from ion sums, not a constant.

## Open

Electroneutrality and membrane charge (contract candidate A limit 4: only bulk charge conservation today). Stiff chemistry needs the implicit solver (pulse 0005 question 4).

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card BC at line 1063; injury table at line 1114; timescales at line 1128. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), blood chemistry section at line 459.
- Contract pass 1: [synthesis](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
