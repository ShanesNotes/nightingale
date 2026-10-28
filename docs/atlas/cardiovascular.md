# CV. Cardiovascular

Heart, arterial, venous and splanchnic beds. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

Cardiac output is the intersection of pump function and venous return, so preload, afterload, contractility and rate are real quantities other systems can move, and arterial pressure is an outcome, never an input (Guyton).

## What it owns

The one global hemodynamic solve: chamber elastance, valves, inertance, compliance and resistance on every vascular edge, and the pressure and volume law for blood. Advection of every species and of heat along the solved flows. The cardiac rhythm and its modes.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S1 (owns the solve and all vascular edges), S6 (convection of heat between nodes).
- **Reads:** S2, S3, S4 (K, Ca, Mg and pH for rhythm and tone), S5, S7, S8, S9 (viscosity), S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** NS (baroreceptor and cardiopulmonary afferents), MT (myocardial work, flow per bed), EN (wall stress for natriuretic peptides), FL (capillary and venous pressure per bed), RS (pulmonary capillary flow per unit).
- **In:** RS (pleural and intrathoracic pressure), MT (oxygen debt per bed), RN (tubuloglomerular feedback on the afferent edge), HP (sinusoidal resistance and shunt), GI (intra-abdominal pressure), IM (iNOS and myocardial depressant factors), HC (hemostatic competence at bleeding edges, viscosity, thrombosis events), MS (position and gravity).

## Place in the contract ownership register

Blood volume: CV owns advection on every vascular edge and the one pressure and volume block (A section 3). Heat: CV owns advection of heat with blood. Natriuretic peptide secretion.

## Injury, failure and demise

Ischemic depression, stunning, infarction, tamponade, valve disease, vasoplegia. **Demise:** the arrest rhythms (VF, pulseless VT, PEA, asystole) are CV rhythm modes; in arrest the activation term of elastance is zero and flow comes only from CPR as an external pressure source on the thoracic compartments. The rhythm hazard rises with myocardial O2 debt, potassium and scar.

Decision [0004](../decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

1 to 5 ms (valves, pressure wave) to one beat (0.3 to 1.5 s), reflexes over seconds, hormonal tone over minutes, and stunning recovery and baroreceptor resetting over days.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | `vitals-cardiovascular` (with `arrhythmia`, `cpr`, `ecg`, `hemorrhage` modules) on `vitals-circuit` |
| Pulse reference | `CardiovascularModel.cpp` in Pulse REL_4_3_2; today's crate cites it line by line in its source headers |
| Proposed crate | `ng-cardiovascular` (see the [crate map](crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](../vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 2 (minimal closed loop for the kidney), 4 (full hemodynamic network), 6 (rhythm modes, arrest and CPR) (see the [build order](build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

Pressure is derived from volume and compliance, never stored. One hemodynamic solve for the whole body. Autonomic input arrives as NE and ACh release at receptors, never as a tone scalar.

## Open

How rhythm modes and hazards enter the contract (decision 0005 question 5). Fast step size (arena atlas decision 1: 2 ms native assumed). Whether the hemodynamic network is its own crate (`ng-net-hemo` in the arena atlas) or part of this one.

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card CV at line 83; injury table at line 180; timescales at line 192. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), cardiovascular section at line 75.
- Contract pass 1: [synthesis](../council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](../council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](../council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
