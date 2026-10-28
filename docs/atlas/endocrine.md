# EN. Endocrine

Glands and hormone axes. Status: **proposed card**, drawn from the two atlas documents and contract pass 1. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

The glucose, insulin, glucagon and catecholamine axis, then RAAS and vasopressin, which decide sodium and volume over days.

## What it owns

Secretion of every hormone and its conversions (pancreas, adrenal, pituitary, thyroid, parathyroid), RAAS conversion steps.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S7 (secretion of every hormone; conversions).
- **Reads:** S1, S2, S4 (K, Ca, glucose, osmolality), S5, S8, S10 (adrenal medulla input).

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** Every card through free hormone concentrations at receptors (S7), so no direct wires.
- **In:** NS (CRH, vasopressin and oxytocin drive, circadian phase), RN (renin), CV (wall stress for natriuretic peptides).

## Place in the contract ownership register

Hormones: EN owns secretion from glands and free fractions where binding is fast (A section 3).

## Injury, failure and demise

Type 1 and type 2 diabetes, hypoglycemia unawareness, adrenal insufficiency, Cushing state, SIADH and diabetes insipidus, thyroid disease. **Demise:** through extremes such as DKA, adrenal crisis or thyroid storm, which act on other cards.

Decision [0004](../decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

Under a minute (angiotensin II, epinephrine) to minutes (insulin, vasopressin), hours (aldosterone, cortisol effects), weeks (T4 pool, HPA suppression).

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Crate today | `vitals-substances` (`endocrine`) |
| Pulse reference | `EndocrineModel.cpp` in Pulse REL_4_3_2; today's crate cites it line by line in its source headers |
| Proposed crate | `ng-endocrine` (see the [crate map](crate-map.md)) |
| Lane | 1, organ-system modules and coupling ([engineering-lanes.md](../vision/engineering-lanes.md)). Its conditions, devices, drugs and labs hooks belong to lanes 2 to 5 |
| Build step | 5 (catecholamine secretion), 7 (RAAS, vasopressin, natriuretic peptides), 9 (glucose axis) (see the [build order](build-order.md)) |
| Proof status | Not started on the native contract. Today's crate is the Pulse port, used as the reference to diff against |

## Decided

Hormones are species with receptor tables; nothing reaches a target except through a receptor.

## Open

Portal to peripheral insulin gradient (shared with HP). Which axes are admitted first beyond glucose and RAAS.

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card EN at line 927; injury table at line 1006; timescales at line 1020. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), endocrine section at line 407.
- Contract pass 1: [synthesis](../council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](../council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).

The atlas documents are not in the repo yet; line numbers refer to the copies Shane shared on 2026-10-26, identified by the hashes in [PROVENANCE.md](../council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs).
