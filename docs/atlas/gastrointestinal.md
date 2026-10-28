# GI. Gastrointestinal

Gut lumen, wall and secretions. Not a clinical claim. Numbers and thresholds arrive only in a reviewed packet.

## What it must get right first

Gastric emptying then segmental absorption, the rate limit for every meal, tube feed and oral drug, then the composition of the secretions, because losing them is the commonest cause of an electrolyte and acid-base disorder.

## What it owns

Luminal compartments, emptying and transit, segmental absorption and secretion, gut hormones, splanchnic bed edges, bleeding edges into the lumen.

Under both contract drafts a system owns laws, never a shared pool. The pools (S1 to S10) belong to the body, or to the patient record in contract candidate B.

## Shared substrates

- **Posts flux to:** S1 (splanchnic edges, bleeding into the lumen), S4 (absorption, secretion losses), S5 (absorption), S7 (incretins, gut hormones), S8 (absorption, secretion, losses), S9 (iron, B12, folate, vitamin K).
- **Reads:** S2, S6, S10.

Substrate ids are defined in the [coupling map](coupling-map.md#shared-substrates).

## Direct couplings

- **Out:** CV, RN and RS (intra-abdominal pressure), HP (portal delivery).
- **In:** NS (motility through autonomic release), EN (gut hormones), FL (gut wall water), devices (NG tube, feeding tube, suction).

## Place in the contract ownership register

Blood volume: GI owns portal absorption edges. Solutes: GI owns epithelial transfers. Hormones: GI owns incretin secretion (A section 3).

## Injury, failure and demise

Ileus, gastroparesis, obstruction, GI bleeding, diarrhea, mesenteric ischemia and perforation, pancreatitis. **Demise:** through hemorrhage (CV), sepsis from perforation (IM) and fluid and electrolyte loss.

[pulse 0004](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0004-demise-is-modeled-physiology.md) applies: a numerical failure is an engine fault, never a death.

## Timescales

A swallow or vomit as an event, gastric emptying in 15 min to 4 h, small bowel transit in hours, colon over 1 to 2 days.

Both contract drafts drop Pulse's fixed rate lanes; the coupled solve is the default and a split must be measured first.

## Where it lives

| | |
|---|---|
| Pulse reference | `GastrointestinalModel.cpp` in Pulse REL_4_3_2; today's crate cites it line by line in its source headers |
| Proposed crate | `ng-gastrointestinal` (see the [crate map](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md)) |

## Decided

Oral drug delivery starts in the lumen and reaches the liver through the portal edge, so first-pass is emergent.

## Sources

- Arena atlas ("Nightingale Organ System Architecture Atlas", sha256 `4eb9fa7c`), card GI at line 792; injury table at line 872; timescales at line 885. The card's sections 2 to 7 and 10 to 12 hold compartments, laws, control, energy, hooks, parameter sources and validation cases.
- DAE atlas ("Version 1.0, Native Rust first", sha256 `cb4ecaa5`), gastrointestinal section at line 349.
- Contract pass 1: [synthesis](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/2026-10-26-contract-pass-1-synthesis.md); candidate A ownership table in [DESIGN.md section 3](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract/DESIGN.md).
