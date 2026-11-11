# 04: A scope and ports for each system, and a census for each assembly

Type: task
Status: claimed
Claimed by: 04-scopes-ports-census
Blocked by: 03

**What to build:** Each of the nine systems gets its scope and its ports in the contract, and the census of coupling arrows reads the assembly it is asked about. Today the owner set is sealed with nine systems marked held, and the counts of mapped and unmapped arrows are constants about the slice: moving one system into the slice breaks those constants and the sketch's copy of them. Once the census reads an assembly, the slice keeps its count, the whole body has its own, and no system ticket edits a shared constant.

- [ ] A scope and a ports struct for each of the nine systems, granting only what [ticket 01](01-rung-0-specification.md) says that system owns
- [ ] Each coupling arrow with no payload yet gets one, or is recorded as an anatomy relation, so all 38 are accounted for in the whole body. Arrow 111, intracranial pressure, is held with the nervous system as its owner, and arrows 99 and 104 are anatomy relations, as 82, 96 and 101 are
- [ ] The census has a disposition for a field held with its reason
- [ ] A card with a crate can keep named holds, a card that is not held can own an enclosure's pressure, and the endocrine, hepatic, hematology and gastrointestinal systems get boundary routes for synthesis and loss
- [ ] Each system crate declares one law to a module, so two tickets that add laws to one crate edit different files
- [ ] Every port signal states its time basis: instantaneous, a beat mean or a breath mean
- [ ] The census is computed for an assembly, the slice's census is unchanged, and the check that pins the slice's counts still holds them
- [ ] The rule in `AGENTS.md` that states the census, and its check, name both assemblies
- [ ] The slice's model hash and trace do not move
- [ ] From [ticket 03](03-whole-body-lists.md): a reaction may count its reference species N times, or the Answer says how tickets 14 and 16 write the reactions that have no species at count one

**Non-goals:** any law, and any emitter or receiver for an arrow. Those arrive with the loop that needs them.

This changes the contract's interface, so it runs pstack `architect` with an Opus and a Codex runner and a Codex judge.

From [research 13](../../research/issues/13-law-forms-gut-and-fuels.md) and [research 14](../../research/issues/14-law-forms-volume-and-acid.md): `ReactsAt` grants only metabolism at the capillary beds, so the hepatic card needs a grant at the liver bed for its glycogen, gluconeogenesis, amino acid and ketone reactions and its fixed acid, the musculoskeletal card one at the muscle bed for its glycogen, protein and creatinine reactions, and RN's ammonium one at the peritubular bed.

From [ticket 03](03-whole-body-lists.md)'s Answer:

- The fixed acid is a carried species and states no atoms, so it is no `Reactant` and no reaction makes it. The hepatic card makes it through a synthesis route at research 14's rate `s J_aa`. Its `ReactsAt` grant at the liver bed covers its glycogen, gluconeogenesis, amino acid and ketone reactions, and not the fixed acid.
- A reaction counts its reference species once: `Proportions::using` puts it at -1, and a second term for it is refused as `SpeciesTwice`. A single reaction that stores glucose and burns a share below one half in its own counts (`20 glucose + 6 O2 -> 19 glucosyl + 6 CO2 + 25 H2O`) has no species at count one, so it cannot be written, though a storage reaction beside a separate burn can. Urea from the amino acids and creatinine from body protein compile only with the made species as the reference, to run on a negative rate, and no admission of either has run. Glycolysis run backward would draw its energy from the bed's heat pool, so it does not serve gluconeogenesis. Counting the reference N times, with the rate scaled by 1/N, would let all four be written forward. This ticket opens reactions to three more cards, so it decides that before tickets 14 and 16 write them.
- Four ports exist already and are given: `CardiovascularPorts::heat_carried`, `FluidsPorts::cell_membrane`, `FluidsPorts::pump_work` and `RenalPorts::secretion`. That is one for each route family, and one for the heat route whose far end is a receipt. This ticket adds the rest.
- Each new hormone enters and leaves at sites ticket 01 already names (its lines 602 to 613), and leaves by a first-order clearance, as vasopressin does. This ticket's ports and boundary routes carry each one in and out.
