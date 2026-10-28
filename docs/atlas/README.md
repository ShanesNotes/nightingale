# Atlas: organ cards

The organ system architecture the native engine is built toward: what each system owns, what it reads from and writes to the others, how it fails and dies, and which open design questions wait on it. One card per system, plus the maps that join them.

Everything here is a proposal until a [decision record](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/README.md) accepts it. A card is not a clinical claim. Clinical values arrive only in a reviewed packet (AGENTS.md, **Nonclinical first**).

## Start here

| File | What it is |
|---|---|
| [coupling-map.md](coupling-map.md) | The ten shared substrates, the layers, every direct coupling, and the demise paths, as tables and mermaid diagrams |
| [crate-map.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/crate-map.md) | Where each system and shared layer is proposed to live, and what exists today |
| [build-order.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/build-order.md) | The order the body is built in, each step with the proof that ends it |
| [_template.md](_template.md) | The card template; copy it for a new card |

## Cards

| Id | Card | Scope | Build steps |
|---|---|---|---|
| CV | [Cardiovascular](cardiovascular.md) | Heart, arterial, venous and splanchnic beds | 2 (minimal closed loop for the kidney), 4 (full hemodynamic network), 6 (rhythm modes, arrest and CPR) |
| RS | [Respiratory](respiratory.md) | Lungs, airways, pleura and the respiratory pump | 4 |
| NS | [Nervous](nervous.md) | Central and autonomic nervous system | 5 (receptor layer and reflexes), 6 (hypoxic-ischemic injury for demise) |
| RN | [Renal](renal.md) | Kidneys and urinary tract | 1 (kidney on the bench), 2 (kidney on a minimal circulation), 7 (full volume loop) |
| HP | [Hepatic](hepatic.md) | Liver and biliary tree | 9 (metabolic hub) |
| GI | [Gastrointestinal](gastrointestinal.md) | Gut lumen, wall and secretions | 9 (metabolic hub) |
| EN | [Endocrine](endocrine.md) | Glands and hormone axes | 5 (catecholamine secretion), 7 (RAAS, vasopressin, natriuretic peptides), 9 (glucose axis) |
| BC | [Blood and acid-base chemistry](blood-chemistry.md) | Speciation in every compartment | 3 (transport and chemistry, with the coupled solver) |
| HC | [Hematology and coagulation](hematology.md) | Formed elements, factors and clotting | 10 |
| IM | [Immune and inflammation](immune.md) | Innate and selected adaptive response | 10 |
| FL | [Fluid compartments and the endothelium](fluids-endothelium.md) | Water distribution, Starling exchange, lymph and the vessel wall | 2 (plasma and interstitial water split for the kidney loop), 7 (full volume loop) |
| MT | [Metabolism and thermoregulation](metabolism-thermal.md) | Per-bed energy, oxygen debt and the thermal network | 4 (per-bed energy and O2 debt), 11 (thermal network) |
| MS | [Musculoskeletal and skin](musculoskeletal-skin.md) | Muscle, neuromuscular junction, bone and skin | 11 |

Build step numbers refer to the [build order](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/atlas/build-order.md).

## How to use a card

- Read **What it must get right first** and **Shared substrates** before writing any code for the system. Under both contract drafts a system owns laws, never a pool, so every write is a flux to a substrate in the [coupling map](coupling-map.md#shared-substrates).
- **Where it lives** says what exists today, the Pulse model it diffs against ([pulse 0002](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0002-pulse-as-reference.md)), the proposed crate, the lane and the build step.
- **Decided** and **Open** are the working edge. When a decision record or ADR settles an open item, move it to Decided and cite the record.
- A council pass on one system (the next one is the kidney) updates that card and links its session in **Sources**.

## Sources

The cards merge two atlas documents Shane shared on 2026-10-26, plus core contract pass 1:

- The arena atlas, "Nightingale Organ System Architecture Atlas" (sha256 `4eb9fa7c...`): one card per system with compartments, laws, control, injury, timescales, hooks, parameter sources and validation cases, then the coupling map and build order.
- The DAE atlas, "Version 1.0, Native Rust first" (sha256 `cb4ecaa5...`): the same systems written as a coupled DAE, with an ownership collision checklist and a staged build.
- [Core contract pass 1](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/README.md): two outside drafts of the contract every organ crate depends on.

An earlier note here expected three atlas documents; two arrived. The atlas documents themselves are not in the repo yet; full hashes are in [PROVENANCE.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs), and every line number in the cards refers to those copies. The C3 conditions question ([roadmap.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/vision/roadmap.md#phase-1-condition-modifiers)) is answered in outline by both contract drafts' conditions algebra and stays open until [pulse 0005](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0005-core-contract-pass-1-choices.md).
