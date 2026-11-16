# 65: Blood chemistry reads what it speciates, and its readers reach its fields

Spec: `crates/ng-contract/src/scope/blood_chemistry.rs` and `crates/ng-contract/src/catalog.rs`

Type: task

Status: open

Blocked by: none

**Why.** The gas loop (body 08 to 11) is the longest chain to the first milestone (SRS invariant I19). `BloodChemistryPorts` read nothing (`crates/ng-contract/src/scope/blood_chemistry.rs`), and the catalog's one arrow that names blood chemistry is 94, into it. So no law of blood chemistry can read the amounts it speciates, and the lung's exchange ([body 10](../../body/issues/10-lungs-breathe.md)) and the sensors ([body 11](../../body/issues/11-breathing-is-controlled.md)) cannot read a partial pressure or an acidity another system states. [Body 08](../../body/issues/08-blood-carries-gases.md) has metabolism state each bed's speciation in the meantime, with the code written generic over the system that states it, so the move to blood chemistry is one change in the composition root.

- [ ] A design from a pstack `architect` run, with two runners and a judge on another vendor, settles blood chemistry's reads, how another system's law reaches its fields (new coupling arrows, and then the census's counts and the rule rows that name them, or a read of a site field), and speciation in vessel segments
- [ ] The contract change, with its compile-fail tests
- [ ] Body 08's speciation moves to blood chemistry by one change in the root, and the body's pins move once
