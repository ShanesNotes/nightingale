# 07: Contract follow-ups

Type: task
Status: needs-triage
Blocked by: none

Changes to `docs/architecture/solver-contract.md` and its sketch, each a contract version where it changes a type. The split's hygiene pass moved the contract's open questions and its next implementation step here (below).

From the SRS sharpening (conflict ids from the approved SRS's change log):

- **C3, before the first exchange law.** Confirm that one membrane type with two settings, conductance and reflection (SRS section 5), fits the contract. The sketch has no wall type; exchange lives inside site-class laws.
- **C9.** Add the measurement method to anchors, beside the averaging window (`AnchorStatistic`), and match both at admission (SRS rule 17).
- **C15.** The contract's model default for who attests evidence names people per discipline. SRS rule 16 makes attestation a citation check; change that row.
- **C16.** A disease enters as parameter shifts or timed events in the event log (rule 14, I15), but the contract binds parameters once per run and has no parameter-shift payload. Design the boundary payload and the rule for changing a bound value.
- **C18.** "Tier" has no field in the sketch. Define it once and map it to `Certainty`, `Reading` and `Claim`.
- **C19.** Alarm limits are monitor settings a clinician can change (SRS section 6). Decide whether they are their own lens field or a `ReferenceInterval`.
- **Later: C2 and C17.** The SRS names nerve, conduction and gut networks and a longer afferent list than the sketch's `Network` and `Afferent` enums. A contract version when those systems are built.
- **Glossary words (C8 and the rest).** `GLOSSARY.md` settles: `Book` names a conserved quantity, "inventory" is an amount held, "output" is a produced quantity, "finding" is an observation, "evidence record" is a source record, and "organ crate" is a system crate. Rename the contract's prose and, by expand and contract, its code names.
- **Types (old plan table H), if not yet done.** Type the physical `f64`s in public items (absolute tolerances in `ng_validate.rs` and `identity.rs`, `ng_cardiovascular.rs`'s phase, the scales in `split.rs`); put the dimensionless solver vectors on the `f64` allow-list with a reason; add a compile-fail test that passes one dimension where another is required.
- **Game wording.** The rewind gate in section 7 still says "an in-game retry equals a loaded save"; say "a retry from a checkpoint equals the same branch loaded from its saved checkpoint" (SRS I3).
- **FU.** The contract cites `~/lanes/logs/ng-q1-frame-followup.md` by sha256, a host file outside both repos. Decide whether it enters the repo through a reviewed packet or stays a pinned outside source.

## Moved from the contract by the hygiene pass
