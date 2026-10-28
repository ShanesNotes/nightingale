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

### Open questions and risks

- Can the closed instruction set express the first CV, FL, RN and NS laws without opaque whole-organ opcodes, and can compiled evaluation reach 100× with full waveform? The first coupled synthetic loop must measure it.
- Do fixed partial-volume coefficients and the elastic energy potential suffice for this slice, or must the state representation change before clinical physiology? A manufactured coupled pressure and energy example should answer it.
- Who owns ICP, natriuretic secretion, overlapping bed laws and cell-water shifts in the full atlas? The compiler exposes the collisions; this design does not settle them.
- Which publications, locators and reading evidence fill P1–P26, especially P25 and the from-memory masses? Source admission stays `ready-for-human`; the synthetic arena does not wait.
- Can Bernstein segments bound every sliding window's extrema at acceptable memory cost, and which degree does each core's dense output support?
- Is resuming only at a branch head enough, or must the durable checkpoint store refuse a non-head decode on the same branch?
- What product action follows an engine fault under nightingale-pulse#293? The latch and intact physiology are fixed here; the presentation is not.
- Which values do the accuracy tolerances, each core's minimum interval and the certificate's tolerance floor take? They are policy slots with model defaults, set in the validation crate before the arena and reported with every result.
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
