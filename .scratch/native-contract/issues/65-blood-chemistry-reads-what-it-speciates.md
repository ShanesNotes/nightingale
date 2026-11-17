# 65: Blood chemistry reads what it speciates, and its readers reach its fields

Spec: `crates/ng-contract/src/scope/blood_chemistry.rs` and `crates/ng-contract/src/catalog.rs`

Type: task

Status: resolved

Blocked by: none

**Why.** The gas loop (body 08 to 11) is the longest chain to the first milestone (SRS invariant I19). `BloodChemistryPorts` read nothing (`crates/ng-contract/src/scope/blood_chemistry.rs`), and the catalog's one arrow that names blood chemistry is 94, into it. So no law of blood chemistry can read the amounts it speciates, and the lung's exchange ([body 10](../../body/issues/10-lungs-breathe.md)) and the sensors ([body 11](../../body/issues/11-breathing-is-controlled.md)) cannot read a partial pressure or an acidity another system states. [Body 08](../../body/issues/08-blood-carries-gases.md) has metabolism state each bed's speciation in the meantime, with the code written generic over the system that states it, so the move to blood chemistry is one change in the composition root.

- [x] A design from a pstack `architect` run, with two runners and a judge on another vendor, settles blood chemistry's reads, how another system's law reaches its fields (new coupling arrows, and then the census's counts and the rule rows that name them, or a read of a site field), and speciation in vessel segments
- [x] The contract change, with its compile-fail tests
- [x] Body 08's speciation moves to blood chemistry by one change in the root, and the body's pins move once

## Answer

Speciation is a shared substrate's derived property, so its readers are a second table beside the 38 arrows, which do not change. Blood chemistry reads the blood at every speciated site (each capillary bed and each segment the map samples) and states four rows and one explicit field, carbon dioxide's partial pressure, from one closed list of five quantities. A card reads them only through its row of the readers' table, wired by a body ticket or held with a reason, and compile refuses a read no row wires (`UnwiredRead`). Compile writes an explicit field into each program that reads it, where the read stood, sharing nothing and naming each written op for the field. The design came from two candidates made apart, a judge from another vendor, and a prototype that built the explicit field and ran its probes. The contract change moves no pin; the move of body 08's speciation is the third box.

The move made blood chemistry state each bed's speciation through its own port, with metabolism reading the two partial pressures through its wired row, and changed no law's form or value: every bed's four rows at the admitted state equal the base's within 1.9e-14 relative, and the body's 325 programs hold 2,334 ops at both. Cost on the beelink, base `5283b85b7a87` against the move: 4,992.5 M a simulated second with 14,510 solves (12,522 ordinary, 1,988 landing) and 20.64 M a solve, against 5,214.5 M with 15,264 solves (12,519 ordinary, 2,745 landing) and 20.50 M a solve. The 222 M is 757 landing solves in the dearer of the two landing states main has shown ([native-contract 64](64-the-landing-reads-a-guard-that-did-not-cross.md)); its cause is not measured. At a fixed landing count the instructions a solve put the move at about -36 M (inferred, at 14,510 solves), but the head's mix holds more landing solves, so that figure is not the move's alone.
