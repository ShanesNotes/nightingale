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
- **Clinical numbers in the lens (record 0004).** A channel's error model and reference interval carry clinical numbers, and the monitor's alarm limits (C19) may too, but rule 12 lets clinical values in only through a reviewed packet or healthy-range set. Decide their reviewed home, and add the check that holds a channel's error to the device accuracy studies its numbers come from. The milestone's checks read no random error, so that check sits outside the milestone.
- **Later: C2 and C17.** The SRS names nerve, conduction and gut networks and a longer afferent list than the sketch's `Network` and `Afferent` enums. A contract version when those systems are built.
- **Glossary words (C8 and the rest).** `GLOSSARY.md` settles: `Book` names a conserved quantity, "inventory" is an amount held, "output" is a produced quantity, "finding" is an observation, "evidence record" is a source record, and "organ crate" is a system crate. Rename the contract's prose and, by expand and contract, its code names.
- **Types (old plan table H), if not yet done.** Type the physical `f64`s in public items (absolute tolerances in `ng_validate.rs` and `identity.rs`, `ng_cardiovascular.rs`'s phase, the scales in `split.rs`); put the dimensionless solver vectors on the `f64` allow-list with a reason; add a compile-fail test that passes one dimension where another is required.
- **Game wording.** The rewind gate in section 7 still says "an in-game retry equals a loaded save"; say "a retry from a checkpoint equals the same branch loaded from its saved checkpoint" (SRS I3).
- **Refinement against targets (record 0002, I11).** The refinement section and `PortEquivalence` compare a split region with the grouped one within a declared envelope. Replace that reference with the sourced checks: a split keeps regional totals exact (an identity on the `SplitRule`s), keeps every sourced check the grouped region passed, and passes what earned it, while manufactured cases keep testing the numerics. A comparison with the grouped body becomes a report for review, never a pass bar.
- **Grouped regions (record 0002, SRS section 5).** Each grouped region states the structures of the reference anatomy it stands for and the behaviors it cannot show, as data the body model carries, so a scenario that depends on one is refused when it is built and a debrief can list them. `Representation::Reduced` names the region today; add the two lists. Rename the sketch's `_coarse` fields and its "coarse" doc comments in `anatomy.rs` to grouped, then add "coarse region" to the _Avoid_ line of **Grouped region** in `GLOSSARY.md` in the same PR.
- **Inference records (record 0003).** Add an inferred kind to `ValueProvenance` for a parameter or a number of the day, with its inference record (the model, its reasoning, the population, the source records it reasons from and the search that found no source), hashed into the packet. Building a reviewed packet or healthy-range set refuses an inference record that names no search; the packet also refuses an anchor that rests on one, and the set an inferred value that names neither a sourced value and sourced shift nor the sourced flows of an amount no method can measure (I16, records 0004 and 0005). The packet reports how many values are inferred.
- **The day and the healthy-range set (records 0004 and 0005).** Add the day the patient lives (sleep, posture, meals, drinks and room conditions) to the packet, and make the run's boundary events from it. Add the healthy-range set beside the packet and the lens: every healthy range and within-subject variation with its source or inference record, hashed apart from the run's identity, reviewed or synthetic. The certificate, the healthy-range check and the return check run in validation, never inside the engine, and their result names the run and the set it read.
- **FU.** The contract cites `~/lanes/logs/ng-q1-frame-followup.md` by sha256, a host file outside both repos. Decide whether it enters the repo through a reviewed packet or stays a pinned outside source.

## Moved from the contract by the hygiene pass

### Open questions and risks

- Can the closed instruction set express the first CV, FL, RN and NS laws without opaque whole-organ opcodes, and can compiled evaluation reach 100× with full waveform? The first coupled synthetic loop must measure it.
- Do fixed partial-volume coefficients and the elastic energy potential suffice for this slice, or must the state representation change before clinical physiology? A manufactured coupled pressure and energy example should answer it.
- Who owns ICP, natriuretic secretion, overlapping bed laws and cell-water shifts in the full atlas? The compiler exposes the collisions; this design does not settle them.
- Which publications, locators and reading evidence fill P1–P26, especially P25 and the from-memory masses? Source admission stays `ready-for-human`; the synthetic arena does not wait.
- Can Bernstein segments bound every sliding window's extrema at acceptable memory cost, and which degree does each core's dense output support?
- Is resuming only at a branch head enough, or must the durable checkpoint store refuse a non-head decode on the same branch?
- What product action follows an engine fault under nightingale-pulse#293? The latch and intact physiology are fixed here; the presentation is not.
- Which values do the accuracy tolerances, each core's minimum interval and the certificate's closure floor take? They are policy slots with model defaults, set in the validation crate before the arena and reported with every result. The certificate's drift bounds come from the healthy-range set instead, except charge's, which is the closure floor (record 0004).
- Can solver-B pass the full split metric without dt-aware organ updates and incomplete sweep acceptance, or does its speed depend on those shortcuts? Only the arena can answer.
- Is a closed set of six site classes enough for the milestone's 13 systems, or does `TissueSite` need to split (muscle, gut wall, brain) before their first laws?

**Open: model defaults.** Each is written into the contract as the model's recommendation, with one line on the alternative. They are model decisions and stay open until a result or a reviewer moves them.

| Question | Model default (in the contract) | Alternative |
|---|---|---|
| A new attempt that is not a retry from a save: same patient or a new one? | The same authored patient; drawn patients are an opt-in mode once population records exist | Draw a new patient from the population for every new attempt |
| Who attests the evidence for each discipline in the first reviewed packet? | One named reviewer per discipline (critical care nursing and medicine, physiology, pharmacology), each attesting an exact scope; the attestation itself stays `ready-for-human` | One clinical reviewer attests the whole first packet |
| Should the early patients include one who cannot report pain? | Yes, at least one (sedated, ventilated or delirious), so the nociceptive field must drive behavior and signs without the score | Defer until the nervous system deepens |
| Who computes an enclosure's pressure? | The compiler, from the enclosure's `Compliance` data and the volumes inside it | A mechanism owner (FL for body spaces, RS for the pleura) declares the law |
| Where does observer state live? | With the observer, checkpointed beside the record and bound to its seal | Inside the accepted image, sealed apart from physiology |
| How is a forcing interpolated? | A cubic B-spline over seeded knots (continuous rates) | Piecewise linear (cheaper, rates jump at knots) |
| Are site classes a closed set? | Closed: a new class is a contract version | Classes as packet data, with laws checked at body compile |

### Next implementation step

Build the first slice in the spec below, starting with the `ng-contract` anatomy and site-class laws. Then build one manufactured pulsatile circulation, renal, regional-fluid and baroreflex loop from typed fragments. It must prove incidence and energy closure, validate-then-publish refusal with nonempty pending inputs, the generated census gate and the replay gates before either 48 h core runs.
