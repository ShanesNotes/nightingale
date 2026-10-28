# RS. Respiratory

Lungs, airways, pleura and the respiratory pump. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

PaCO2 is set by alveolar ventilation against CO2 production, and PaO2 by alveolar O2 and the spread of V/Q across units.

## What it owns

Airway and pleural mechanics, alveolar units, alveolar gas exchange, hypoxic pulmonary vasoconstriction, airway heat and water loss.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S1 (pulmonary edges, pleural pressure), S2 (alveolar exchange), S3 (elimination), S6 (airway heat loss), S8 (respiratory water loss).
- **Reads:** S4 (pH reaches drive through NS), S7, S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** CV (pleural and intrathoracic pressure), NS (stretch, irritant and J-receptor afferents), MT and MS (work of breathing, pump load).
- **In:** CV (pulmonary capillary flow per unit), NS (respiratory drive, cough), MS (pump force and fatigue), FL (lung water), GI (intra-abdominal pressure), devices (ventilator and oxygen edges).

## Place in the contract ownership register

Gases: RS owns alveolar exchange transfers (A section 3). BC derives PO2, SO2 and PCO2.

## Injury, failure and demise

Bronchospasm, plugging, atelectasis, pulmonary edema, pneumonia, ARDS, pneumothorax. **Demise:** respiratory failure leads to hypoxemia and hypercapnia that drive CV rhythm hazards and NS hypoxic injury; RS itself has no death state.

[pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

10 to 50 ms (airway flow) and one breath (2 to 6 s), chemoreflex over seconds to minutes, lung water over hours, pneumonia and ARDS over days to weeks.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Pulse reference | `RespiratoryModel.cpp` in Pulse REL_4_3_2; today's crate cites it line by line in its source headers |
| Proposed crate | `ng-respiratory` (see the [crate map](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md)) |

## Decided

Gas amounts are conserved across inspired, alveolar, expired and diffused amounts. Drive comes from NS, muscle from MS, carriage from BC.

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card RS at line 231; injury table at line 313; timescales at line 327. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), respiratory section at line 127.
- Contract pass 1: [synthesis](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).
