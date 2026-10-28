# MT. Metabolism and thermoregulation

Per-bed energy, oxygen debt and the thermal network. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

Per-bed oxygen supply against demand with a real tissue PO2, then the thermal network, because core temperature by site is a vital sign.

## What it owns

One consumption and production reaction per bed, tissue O2 and oxygen debt, lactate production, fuel uptake, the thermal network and heat effectors, the energy and work ledger.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S2 (consumption), S3 (production), S4 (lactate), S5 (uptake, lipolysis, catabolism), S6 (owns the thermal network), S8 (sweat).
- **Reads:** S1, S7, S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** CV (oxygen debt per bed for metabolic autoregulation), every card (tissue PO2 and O2 debt for injury integrals), NS, HC, HP, BC and CV (temperature per node).
- **In:** CV (flow per bed, myocardial work), RS (work of breathing), NS (thermostat, shiver and sweat drive), IM (fever, mitochondrial efficiency), MS (shivering, activity).

## Place in the contract ownership register

Gases: MT owns one consumption and production reaction per bed. Heat: MT owns heat release, conduction and loss to the environment. Work and ATP: MT charges work where the bed's metabolism pays (A section 3).

## Injury, failure and demise

Type A and B lactic acidosis, hypothermia, hyperthermia and heat stroke, malignant hyperthermia, starvation. **Demise:** MT's oxygen debt is the input to every injury integral, including NS hypoxic-ischemic injury and CV ischemic rhythm hazards. Brain O2 stores last about 10 s.

[pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Tissue PO2 in seconds, O2 stores gone in 1 to 3 min, thermal responses over minutes to hours, basal rate adaptation over days.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | `vitals-substances` (`energy`, `metabolism`), thermal parts of `vitals-core` (`thermal`) |
| Pulse reference | `EnergyModel.cpp` in Pulse REL_4_3_2; today's crate cites it line by line in its source headers |
| Proposed crate | `ng-metabolism` (see the [crate map](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 4 (per-bed energy and O2 debt), 11 (thermal network) (see the [build order](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

Work is charged once per work id. Heat is a species, so energy conservation is checked by the ledger.

## Open

Critical PO2 and anaerobic constants per bed (arena atlas flag). Number of thermal nodes (eight assumed). Whether the thermal network is its own crate (`ng-net-thermal`).

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card MT at line 1553; injury table at line 1630; timescales at line 1643. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), metabolism section at line 673.
- Contract pass 1: [synthesis](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
