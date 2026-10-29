# Atlas: system cards

The coupling map of the body's 13 systems, with one card per system. An agent reads only the card of the system it is building, and the coupling map when its work touches a coupling between systems.

- The atlas is frozen. A card changes through a ticket when its system is built.
- The cards are proposed and nonclinical. Clinical values enter only in a reviewed packet or healthy-range set (SRS rule 12).
- Where any atlas file, the coupling map included, disagrees with `docs/srs.md` or `docs/architecture/solver-contract.md`, those win. Ownership wording such as "CV owns the solve", or an I&O ledger, predates the contract.
- A system owns laws and their internal state, never a conserved pool; the state holds every amount.
- The first milestone is stage 1 of the SRS's scope and its invariant I19; [record 0004](../adr/0004-first-milestone.md) defines it.

| File | What it is |
|---|---|
| [coupling-map.md](coupling-map.md) | The shared substrates, the layers and every direct coupling, as tables and diagrams |
| [_template.md](_template.md) | The card template |

## Cards

| Id | Card | Scope |
|---|---|---|
| CV | [Cardiovascular](cardiovascular.md) | Heart, arterial, venous and splanchnic beds |
| RS | [Respiratory](respiratory.md) | Lungs, airways, pleura and the respiratory pump |
| NS | [Nervous](nervous.md) | Central and autonomic nervous system |
| RN | [Renal](renal.md) | Kidneys and urinary tract |
| HP | [Hepatic](hepatic.md) | Liver and biliary tree |
| GI | [Gastrointestinal](gastrointestinal.md) | Gut lumen, wall and secretions |
| EN | [Endocrine](endocrine.md) | Glands and hormone axes |
| BC | [Blood and acid-base chemistry](blood-chemistry.md) | Speciation in every compartment |
| HC | [Hematology and coagulation](hematology.md) | Formed elements, factors and clotting |
| IM | [Immune and inflammation](immune.md) | Innate and selected adaptive response |
| FL | [Fluid compartments and the endothelium](fluids-endothelium.md) | Water distribution, Starling exchange, lymph and the vessel wall |
| MT | [Metabolism and thermoregulation](metabolism-thermal.md) | Per-bed energy, oxygen debt and the thermal network |
| MS | [Musculoskeletal and skin](musculoskeletal-skin.md) | Muscle, neuromuscular junction, bone and skin |

## Sources

The cards merge two atlas documents and core contract pass 1. Their hashes are in [PROVENANCE.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/PROVENANCE.md#inputs), and every line number in the cards refers to those copies.
