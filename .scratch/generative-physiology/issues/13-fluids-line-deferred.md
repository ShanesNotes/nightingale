# 13: The fluid-delivery experiment (a deferred line)

Type: research
Status: out-of-scope
Blocked by: 12
Runs on: the workstation; this ticket is a written list until ticket 12 resolves

**What it records:** the second experiment, fluid deliveries through the gut routes, stays a deferred line and not a design. Ticket 12's decision reopens it. These are the prerequisites it must meet first, from ticket 01's section 5.10 and the checks of this pass.

- **A packet whose loop values can vary.** The fixture's value table is private (`crates/ng-validate/src/slice.rs:638`), and a second value for an explicitly bound key is refused (`crates/ng-contract/src/packet.rs:252-258`). A question for the engine's process: does trait variation come from a host copy of the synthetic table or from an engine-side packet builder?
- **Hour-scale runs, or a recorded retune.** Whole-body vasopressin washes out over about 83 minutes by arithmetic on the code (ticket 01, section 3), and history holds 15.6 to 18.5 KB per accepted step (`.scratch/native-contract/issues/11-memory-stays-flat.md:23`). A question for the engine's process: do hour-scale horizons wait for that ticket, or may a host-built synthetic packet record a retuned clearance?
- **A host-computed sodium reading.** No row holds a plasma sodium concentration, and a lens channel reads one row (`crates/ng-contract/src/lens.rs:13`). The host computes the reading, and the measurement-error convention of ticket 06 applies to it.
- **An audit of deliveries enqueued after stepping, on Core.** No test does it. Deliveries go only into runs freshly resumed from a checkpoint (`crates/ng-validate/tests/arena_body.rs:564-625`).
- **Sodium, potassium and chloride totals** in the reduction. The per-species columns already support them (ticket 05).
- **The outside check** is the ordering of cumulative excretion after saline against dextrose, from the infusion study ticket 01 cites, as a direction only, and only if the horizon covers the separation. Basal intake and a delivery share one gut column (`crates/ng-validate/src/slice.rs:290-295`), so a gut route is one intake transfer whose basal part is the extent minus the known delivery's integral.

Not blocking now, for the engine's process: what provenance fitted weights would need under SRS rule 12 if a learned law ever entered the engine.
