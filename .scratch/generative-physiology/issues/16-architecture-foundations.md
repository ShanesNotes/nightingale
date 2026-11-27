# 16: Architecture foundations for learned models around the engine

Type: research
Status: open
Blocked by: none

**Question:** Beyond the first experiment (`C4a` on the slice, and [ticket 14](14-first-experiment-on-the-whole-body.md)'s meal on the whole body), what architecture guides learned models around the engine? Where does each model meet the engine, what may it claim, and in what order are they built? Which building blocks from working references carry over, which guardrails stay hard, and what does the line need from the engine's process?

This ticket captures the architecture foundations beyond the first experiment. It began as a multi-model exploration (`2333f26`), was fact-checked and rewritten against the engine code and reference repos (`9b6813b`), and then folded in acute-arrival, covariate-scaling and speed-sensitivity refinements after review. The four-seam framing (section 2) and draft rules `L1` to `L14` (section 3) are adopted as the working base for this line's upcoming `pstack` passes; individual forks (section 8) and human gates (section 10) stay open until those passes settle them.

**Read set** ([governance section 5](../../../docs/governance.md#5-what-an-architecture-pass-reads)), at `2333f26`, which is main `b74fad1` plus this ticket (re-checked after merging main `9f5ba6c`, and reconciled against main `ffcd1a7a`):

1. `docs/srs.md`.
2. Records 0002 (a split is earned), 0003 (a model's output is never a source or a target), 0004 (the first milestone), 0005 (a result names the run and the set it read) and 0008 (a run keeps a verifiable chain and a checkpoint each simulated minute).
3. `GLOSSARY.md`.
4. `docs/architecture/solver-contract.md`: section 4, the crate table, and the paragraphs on rewind, identity, replay and the checkpoint (lines 322 to 334). Not the sketch, because this pass edits no types.
5. No atlas card.

It also read this line's map and tickets 01 to 15, the engine tickets cited below and the code cited inline. Each code claim carries ticket 01's labels: implemented, measured, planned or inferred. A cost is measured where it says so, and otherwise it is arithmetic on measured figures.

## Acceptance

- [ ] A `pstack` architecture pass takes section 2's seams and section 3's rules (`L1` to `L14`) from working base to bound specification: each rule kept, changed or dropped, with its test
- [x] Each engine-process question in section 7 is filed where it belongs: [native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md) (what a replay holds of a proposed start), [foundation 07](../../foundation/issues/07-contract.md) item C16 (a scripted shift), [ticket 15](15-learned-emulator-for-parameters.md) (ranking first and provenance of a value calibrated to a response), and the [body map](../../body/map.md) (the ECG's accounting)
- [x] The developer reviewer settles the gate's path rule for `tools/generative-physiology/**` ([ticket 03](03-teacher-driver.md), section 7) (`nightingale#238` added `["10-docs", "10-rust"]` ahead of `tools/**` in `tools/gate/paths.toml:14-16`)
- [ ] Section 4's figures are read again once [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) resolves, and each build's budget is restated from them
- [ ] [Ticket 14](14-first-experiment-on-the-whole-body.md) takes or refuses each arm section 5.4 proposes, and ticket 15 takes or refuses section 5.3's ranking use
- [ ] The map's Decisions so far gains what those passes settle

## In brief

**Destination.** Every question a clinician or an author can put to a simulated patient gets two answers: an exact one from the engine, and a fast one learned from it. The fast answer says how far it can be trusted, and it gives way to the exact one when that arrives. A patient drawn from a population is admitted in a fraction of today's time. Hours of a course are previewed at once and confirmed by the engine behind them. A chart written for a scenario finds the engine patients who fit it. A constant the literature cannot pin is fitted against the responses it can. And the learned side keeps paying the engine back, as its first audit did when it found a fault the engine's gates had missed. The centre of the work is unchanged: no source this line has read shows a learned model carrying a coupled, conserving body through hours of closed-loop use (ticket 01, section 2.3).

- **One judge.** The engine computes the patient, and nothing learned changes that. A learned model meets the engine at one of four seams. It proposes something the engine checks, it reads what the engine accepted, it infers from what an observer read, or it stands in for the engine under a contract of its own. The seams differ in what a wrong answer can do, and the build order follows that.
- **A checked proposal needs no provenance.** Where the engine checks everything a model proposes, the result's validity rests on the engine alone. Where nothing checks a model, its output carries a label and names what taught it. This one rule answers most of the provenance questions the first capture raised.
- **One joint, many conditionals.** Traits, start state, course and observations are one joint distribution that the engine already draws from, slowly and in one direction. Each learned model is a fast copy of one conditional of it. So one library of teacher runs feeds them all, and more engine draws judge them all.
- **The teacher's time is the budget.** On its holds at body 26's commit, the whole body runs near 2.5 times real time, and its cost budget lets it slow to 1.2. A six-hour teacher run takes 2.4 to 4.9 wall hours (section 4). That figure decides the order of work more than any model choice does.
- **The generative physiology model is the centre, not the last stage.** Tickets 01 to 14 already plan it. Chart inference over a course cannot be trained on the engine alone, because at today's speed the engine cannot make enough courses.
- **Most of what the first capture asked of the SRS is already there.** It asked for five changes. Three are answered by the SRS, the contract or the code: a run's identity already names the start it reached and not what proposed it, rule 14 already puts disease in the event log, and a checkpoint is already a legal start away from rest. One is a live question for the engine's process, the provenance of a value fitted to a response (ticket 15). The fifth, a lighter gate for this line's tooling, was the developer reviewer's and no SRS matter, and settled in `nightingale#238` (section 7).
- **Nothing here is built before [body 28](../../body/issues/28-two-synthetic-days.md),** and nothing here changes the engine. What can start now is on paper: the pass on sections 2 and 3, the questions of section 7, and ticket 15's count of the constants only a response pins.

## 1. One joint, drawn by the engine

A run is one draw from a joint distribution that the engine and its observers already define. The SRS names its parts and gives each its own seed.

| Part | How the engine draws it | Seed |
|---|---|---|
| Traits | a packet whose values are drawn from population records (`ParameterPacket::sampled`, `crates/ng-contract/src/packet.rs:311`, implemented; no shipped packet draws one) | patient seed |
| Start state | solved once at admission from the packet's anchors | none: the packet fixes it |
| Course | accepted steps from that start, under the events and seeded chance | run seed |
| Observations | readings through a lens, by a method, over a window, with delay and error | observation seed |
| Chart entries | what a clinician documents (SRS section 2, Chart: defined, not built) | not yet specified |

The engine draws from this joint exactly, but slowly and in one direction: traits, then start, then course, then observations. Every learned model in this ticket is a fast approximation of one conditional of the same joint, taught by engine draws and by nothing else.

| Learned model | The conditional it approximates | Direction |
|---|---|---|
| Start proposal | the start state, given traits | with the engine |
| Emulator | summaries of a response, given traits and events | with the engine |
| Generative physiology model | the course, given start, traits and events | with the engine |
| Observer model | observations, given state | with the engine |
| Chart inference | traits and start, given observations | against it |

Three things follow. One library of teacher runs feeds every model, because each model is a view of the same draws. Each model has a plain meaning of correct: its conditional matches the engine's, and more engine draws can test that. And none of it says anything about a person. The joint is the engine's, so agreement with it is agreement with the engine, and a clinical validity claim stays a separate claim ([SRS section 6](../../../docs/srs.md#6-fidelity-aims)).

## 2. Four seams, in order of the trust each asks

A seam is where a learned model meets the engine. There are four, and a fifth that this line does not open.

| Seam | Where the model sits | What a wrong answer costs | What checks it | What it may claim |
|---|---|---|---|---|
| Proposal | before a check the engine applies to any proposal | wall time, and which valid answer is found, or whether one is | the engine, exactly, every time | nothing: the engine's result stands without it |
| Reading | after an accepted step, read-only | a learner sees something the body did not produce | each output against the state it read | that it is faithful to state, on the features it lists |
| Inference | after observations; it returns a belief, never a state | a wrong belief is shown, or a poor proposal is made | calibration on engine-made charts; anything it starts goes back through the proposal seam | that it is calibrated against the engine, on engine-made charts |
| Standing in | in place of engine steps for a stretch of a run | everything, because no step is checked | its own contract: a frozen study, a support signal, and the engine replaying the same event log | that it is faithful to a named teacher inside a stated envelope |
| Inside the engine | a law or a solver core whose kernel is fitted | the engine's own guarantees | not designed | out of scope until its weights have a provenance decision (ticket 01, section 1) |

The engine already keeps the first seam's rule at its own seams. A solver core can only propose, and the record replays and checks what it proposed ([foundation map](../../foundation/map.md), Decisions so far). Admission does the same: `InitializationSystem::admit` rechecks "every row of the system again, on this workspace alone: no core, no solver and nothing the initializer said" (`crates/ng-contract/src/admission.rs:1464-1467`, implemented). The proposal seam adds no new kind of trust.

Two rules follow, and section 7 leans on both.

1. **A checked proposal earns no provenance and needs none.** The result's validity rests on the engine's own check, and the proposer could be a table, a fitted model or a guess. A proposer can still choose among answers the check accepts, such as one of two rests. So what it proposed travels with the run wherever a replay needs it (section 7).
2. **An unchecked output carries a label and names what taught it.** The label is ticket 01's: validated, approximate or untested, at a stated level of its section 6.

A model that chooses inputs, a scripted or learned clinician, is none of the five. It acts on the body as any clinician does, through the boundary port (rule 13), and no ticket plans one ([map](../map.md), Notes).

## 3. Draft rules, each to become a test

These are the line's rules in the SRS's form. Most restate what tickets 01 to 09 already hold, gathered so that a later specification can bind them. A rule's last column names the ticket that holds it today, or says it is new.

| # | Rule | How it is tested | Rests on | Held by |
|---|---|---|---|---|
| L1 | The engine is the only judge of physiology. No learned output becomes engine state except as a proposal that a check of the engine accepts or refuses whole | A proposal built to be wrong is refused and changes nothing. No path in the driver sets a state: `PatientRecord` has "no public constructor and no setter" (`crates/ng-record/src/record.rs:25`, implemented) | Rules 2 and 13, I4, I18 | new |
| L2 | A learned model that carries state moves an amount only through a transfer or a reaction derived from the engine's columns, each with its ends | P S = B A in whole numbers, and balance to rounding under random weights | Rule 3, I10 | 05, 08 |
| L3 | Readings never feed a learned model's dynamics | A change of cadence, lens or observation seed leaves the learned state's bits unchanged | I7, I9 | 08 |
| L4 | Variation comes only from the three seeds. A learned model draws nothing of its own. A draw depends on its seed, its stream and the physiological event it has reached, and it changes a rate or fires an event, never an amount | The same seeds give the same bits, and a new stream moves no other draw | Rule 4, I8, I10 | new |
| L5 | A learned run repeats bit for bit from its start, events and seeds, at any pace and however it is split into calls, within its pinned arithmetic | The piecewise-input test, and a checkpoint with a resume | Rule 4, I2 | 08 |
| L6 | The learned object is a rate per simulated second, never an increment per step | Step halving shows the scheme's order | Ticket 01, section 3 | 08 |
| L7 | A model that claims something about physiology is never judged by its teacher alone. Its gate holds one check whose reference is neither the engine nor the training data. A model at the proposal seam claims nothing and needs none | Each gate in section 5 names its outside check | Rule 16 | 01, 09 |
| L8 | The study is frozen before any model is tuned, and held-out cases are read once | A tree whose digest differs from FROZEN is refused | Ticket 01, section 4 | 09 |
| L9 | A teacher run that faults or is refused is kept, counted and reported. No dataset drops one | The dataset's manifest lists every spec id with how its run ended, and a planted fault shows in the count | none: a dropped fault biases what the model sees | 03 for runs; new for datasets |
| L10 | A learned artifact names what taught it: the engine's commit, the model hash, the packet hashes, the spec ids, the reduction's digest and the frozen study. It refuses to run against another model hash | A load against another digest is refused | Record 0005 | 08 for the reduction; new for the rest |
| L11 | A model that stands in for the engine says where it is supported. Outside that it still publishes and records the excursion, and it never clamps. Every metric is reported inside and outside support | Planted cases outside support are flagged | Ticket 01, section 3 | new |
| L12 | No model here reads a real person's data, and none gives an output about a real person | A dataset's manifest lists teacher runs only | SRS section 1 | map |
| L13 | Whatever an observer model shows is produced by the body, or is the instrument's behaviour drawn from the observation seed, or is a stated convention. The observer lists which feature is which | Section 5.5's checks | SRS section 6, record 0002 | new |
| L14 | Each build starts from the cheapest model that could pass its gate. A richer one stays only if it beats that baseline under the frozen study | The gate's table carries the baseline's row | Ticket 02, decision 1 | 02, 11 |

## 4. The teacher's time is the budget

| Quantity | Slice | Whole body, on its holds | Label and where |
|---|---|---|---|
| Rows and columns | 241 and 257 | 1,465 and 1,463, with 318 pools and 84 route columns held | pins (`crates/ng-validate/tests/body.rs:928`; [body 26](../../body/issues/26-cost-budget.md)) |
| Cost a simulated second | 887.8 M instructions | 5,960.1 M instructions, 6.7 times the slice's (at body 26's commit) | measured on one machine at body 26's commit; `nightingale#235`, `nightingale#236`, `nightingale#239` and `nightingale#240` landed ahead of [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) |
| Wall time a simulated second | 131.5 ms, 7.6 times real time | 396.9 to 422.4 ms quiet at body 26's commit (408 ms representative, near 2.5 times real time). The ceiling after pass 1 is 816 ms, 1.2 times real time | measured on two machines ([foundation map](../../foundation/map.md), item 6; body 26's commit, ahead of `nightingale#235`, `nightingale#236`, `nightingale#239`, `nightingale#240` and body 30) |
| Admission | about 14 s | 177.5 to 190.6 s quiet at body 26's commit, and about 240 s on main by native-contract 86's count. One variant of the body admitted in 55 to 58 s from a state solved under another form, and never from the packet's guess | measured (ticket 04's Comments; body 26; [native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md)) |
| History a step | 18,584 bytes | 131,920 bytes raw and about 65,960 compacted, so about 46 GB compacted a simulated hour | measured ([native-contract 95](../../native-contract/issues/95-the-record-trims-its-history.md)) |
| Distance from the speed aim | 1.3 times short of 10× (13.1 times short of earlier 100×) | near 4 times short of 10× (4.1 times at body 26's 408 ms; 41 times short of earlier 100×) | [SRS section 6](../../../docs/srs.md#6-fidelity-aims) (`1×` live, at least `10×` unbounded, `nightingale#234`); [body map](../../body/map.md); body 26 |

By arithmetic on those figures, all estimates:

- A six-hour run takes 2.4 to 4.9 wall hours on one worker. It writes about 276 GB of compacted links unless its store trims them, and ticket 03's driver keeps every link until its run completes.
- A study of 500 six-hour runs is 3,000 simulated hours: 1,200 to 2,450 worker-hours. That cost returns each time the body's model hash moves, and until body 28 it moves often.
- Ten thousand six-hour runs, the scale an amortized estimator over courses usually asks for, is 24,000 to 49,000 worker-hours. That is out of reach at today's speed.
- Ten thousand admissions is 490 to 670 worker-hours from the packet's guess, and about 160 if a solved neighbour gives what that one variant got. That is in reach.
- Every course-scale figure above scales linearly with the engine's speed. [SRS section 6](../../../docs/srs.md#6-fidelity-aims) now sets the single-core aim at `1×` live and at least `10×` unbounded (`nightingale#234`, leaving ultra-fast encounters to separate learned models outside the engine). At body 26's commit (near `2.5×`, near `4×` short of `10×`), closing that gap to `10×` drops a six-hour run from 2.4 to 4.9 hours to about 36 minutes, while 10,000 six-hour runs still take 6,000 worker-hours. Because `nightingale#235`, `nightingale#236`, `nightingale#239` and `nightingale#240` landed ahead of [body 30](../../body/issues/30-step-plans-on-the-whole-body.md), these budgets are restated once body 30 measures the current body.

Six consequences shape section 5.

1. **Variety is scarce, not beats.** One six-hour run holds tens of thousands of beats but one packet and one day. A design is judged by the start states, traits and inputs it covers for each simulated hour.
2. **A paired branch should not pay for its prefix twice.** Ticket 02 chose fresh runs of one prefix when the prefix was a third of a 15-minute run. On hour-scale runs that choice is reopened (section 8, fork 2).
3. **Anything that needs many courses draws them from the generative physiology model,** with the engine running a sample of them as the check. That is why chart inference over a course comes after it (unless engine speed rises sharply after body 30).
4. **Anything that needs many patients at rest can use the engine itself,** because one draw is one admission. That is where a start proposal pays.
5. **Runs are spent in rounds.** Each round goes where the last one left the model unsure, as history matching does in waves (section 5.3).
6. **Models stay small and cheap to refit,** because the teacher keeps moving.

## 5. The build order

What the engine's process must give first. None of it is this line's work, and each row is already a ticket there.

| Need | Why | Where it is asked |
|---|---|---|
| The whole body lives two synthetic days | The line is parked on it | [body 28](../../body/issues/28-two-synthetic-days.md) |
| A teacher whose step hides no oscillation, at a measured speed | `nightingale#241` added `SelfSustainedOscillation` to the accuracy gate, refusing damped oscillations; the step plans remain unmeasured on the body | [native-contract 91](../../native-contract/issues/91-the-accuracy-gate-holds-no-oscillation.md) (resolved), [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) (open) |
| A history a teacher run can afford | Section 4's 276 GB a run | native-contract 95 (open; unblocked by `nightingale#240`'s `CertificateFold`), and ticket 14's need 3 |
| Varied packets, with sourced covariate scaling | Every model that conditions on traits needs them. `sampled` draws each key alone and uniformly (`packet.rs:311`, implemented), the body's value table is private, and a second value for a bound key is refused. Independent draws across dozens of traits risk anatomically incompatible combinations that waste admission runs; clinical population sampling needs sourced allometric and covariate links (GLOSSARY: Population record, "the parameters it varies with"), never unreviewed empirical patient fits | tickets 13 and 15 |
| What a replay holds of a proposed start | Section 7 | native-contract 86, second box |
| What replay promises across revisions of the engine | A learned artifact names its teacher by commit and model hash (L10) | [native-contract 81](../../native-contract/issues/81-a-runs-identity-names-no-revision-of-the-engines-numerics.md) |
| The parameter-shift payload | Until it exists no teacher run carries a disease | foundation 07, item C16 |
| The ECG's home | Whether a law or an observer makes the waveform is open | body map, Not yet specified |

Ticket 14's need 1 is met at this commit: `ng_validate` exports `whole_body`, `body_packet`, `body_lens` and `body_start_guess` (`crates/ng-validate/src/lib.rs:411-417`, implemented).

```text
teacher library (5.1)
  +--> start proposals (5.2) --------------------> chart inference at admission (5.6)
  +--> emulator (5.3) --> which traits matter --+
  +--> generative physiology model (5.4) <------+
         +--> chart inference over a course (5.6) --> the patient beside the engine (5.7)
observer models (5.5) --> both builds of chart inference (5.6)
```

| Build | Seam | Cheapest baseline first | What it gives the engine |
|---|---|---|---|
| 5.1 The teacher library | none: it is the data | ticket 03's driver, as built | a long-run test that no engine gate runs |
| 5.2 Start proposals | proposal | the solved start of the nearest admitted packet | whether a packet has one rest, and a faster admission |
| 5.3 The emulator | proposal | one Gaussian process for each summary | which constants to source first, and candidates for the ones only a response pins |
| 5.4 The generative physiology model | standing in, offline | C1 linear | nothing until a product asks for previews |
| 5.5 Observer models | reading | rules | an account of what the monitor may show |
| 5.6 Chart inference | inference, then proposal | keep the population draws whose readings match | none directly |
| 5.7 The patient beside the engine | standing in | the engine alone | none: it is a product decision |

### 5.1 The teacher library

Every later model reads teacher runs from one store, addressed by spec id, as ticket 03 built for the slice. The whole body adds four things to it:

- a run spec that carries the day's events (ticket 14's table)
- paired branches that share a prefix (section 8, fork 2)
- a manifest that lists every spec id with how its run ended (L9)
- the starts admission reached, kept by packet hash, as the pool section 5.2 draws from

**Gate.** The whole body's teacher audit passes both of ticket 04's gates on the chosen experiment's runs (ticket 14's fourth box). A six-hour run stays inside a stated disk bound. A planted fault shows in the manifest's count.

**What it gives the engine.** The audit runs the body longer, and across more inputs, than any engine gate does. On the slice its first run found that the leg veins empty head-down, a fault the engine's own gates had missed ([native-contract 33](../../native-contract/issues/33-leg-veins-empty-head-down.md)). That is this line's first result, and it came before any model.

### 5.2 Start proposals

A start proposal is a guess for admission's unknowns, for a packet not yet admitted. `admit` judges it as it judges any guess.

**Why it pays.** A guess does more than save time. One variant of the breathing body admits in under a minute from a state solved under another form, and never admits from the packet's own guess (native-contract 86). A proposal pays most where one draw is one admission: a library of patients, a calibration loop and section 5.6's first build. A six-hour run does not notice four minutes.

**The ladder.** Each step is tried only if the one before leaves a stated share of the time on the table (L14).

1. The packet's own guess, as today.
2. The solved unknowns of the nearest admitted packet in the library.
3. A continuation from that packet in the values that differ. [Native-contract 82](../../native-contract/issues/82-what-a-moved-rest-does-not-admit.md)'s continuation admitted where the plain solve stalled.
4. A fitted map from packet values to solved unknowns, linear first.

**Gate.** On held-out packets, every start comes from `admit`. Where the solve from the packet's own guess and the solve from a proposal both admit, the two starts agree within admission's tolerance on every row, or the difference is reported as a second rest. The median admission time falls by a factor stated before it is measured.

**What it gives the engine.** A measurement nobody has: whether a packet has one rest. Several guesses for one packet either land within tolerance of each other or show two rests. Native-contract 86's fourth box asks a form of this.

### 5.3 The emulator

Ticket 15 holds the question and its model default: one Gaussian process for each response summary, searched by history matching, with every proposal run on the engine. This pass adds an order of use.

1. **Ranking first.** Which constants move which readings and responses, and by how much, over their population ranges. The output is an order for [research 05](../../research/issues/05-parameters.md)'s sourcing work and never a value, so it needs no provenance decision. It also names the traits that matter, which section 5.4's trait conditioning starts from.
2. **Fitting second,** for the constants only a response pins, once ticket 15's record exists.

A neural estimator replaces the Gaussian process only if the count of fitted constants or of summaries breaks it. Ticket 15's first box counts that group. [Research 16](../../research/issues/16-constants-and-what-binds-them.md) bounds it loosely: of 558 constants, 57 are model defaults and 20 have no route yet.

**Gate.** Held-out engine runs fall inside the emulator's stated intervals at their stated rate. A planted value is recovered inside the set not ruled out, on responses the engine made at a known value. A pair of constants built to be inseparable is reported as unpinned and is not fitted.

**Outside check,** for a fit: a sourced response held out of it, as ticket 15 already requires.

### 5.4 The generative physiology model

Its question, its discipline and its first experiment are tickets 01 to 14's and do not change here. This pass proposes seven arms and choices for ticket 14 to take or refuse.

- **The reduction starts grouped, and each split is earned.** Containment is a tree, so an organ's totals are the sums over its sites (SRS section 2), and a split of the map keeps every regional total exact (I11). So the named totals can be a cut of that tree for each species. P S = B A holds at any cut. C4a sits at a coarse cut and R at the finest. A named total is split only when the sufficiency screen or a failed out-of-policy check earns it, as record 0002 earns a split of the map. A cut also outlives a split of the engine's map below it, because the totals keep their meaning. Whether the fitted model outlives it is a hypothesis.
- **Kernels shared by law and site class.** Laws are declared once per site class (rule 10), and ticket 08 already shares one network per transfer origin, with a learned embedding for each transfer. On the whole body the embedding gives way to the site's own packet values. The hypothesis is that data cost then grows with the number of law origins and not with the number of sites, and that trait conditioning arrives by the same door.
- **A structural mask as an arm, not a given.** `CompiledBody::sparsity()` gives the unknowns each row reads, and `blocks()` gives the coupled parts in dependency order (`crates/ng-contract/src/body.rs:127-133`, implemented). One column's own reads are not public (`Instance::inputs`, `body.rs:599`, implemented), so a host derives a mask for each total and not for each transfer. The body has 415 blocks and its largest holds 669 of 1,465 rows, so once algebraic rows are closed the circulation will stay dense, and a mask will prune mostly the slower chemistry (inferred, not run). The first step is to measure the mask's density at the chosen cut. The second compares a masked C4a with a dense one out of policy. An accessor for a column's reads would be a question for the engine's process.
- **A contrast term on paired branches.** A day check already reads a meal's response as a difference from a counterfactual run (GLOSSARY: Day check). A loss on the same difference weights the response over the day's common drift. Branches from one parent stay in one partition (ticket 01, section 6).
- **Chance the engine's way.** Once the teacher carries a hazard that matters, the model carries a learned hazard rate, integrates it and fires it against a draw from its own run seed's stream, as the contract does (GLOSSARY: Hazard). It samples no latent and adds no noise per step (L4). A model that predicts the mean of two courses is wrong about both, so it is judged by time to event across seeds.
- **Scale by natural size.** A row's natural size already standardizes a small ligand and a large water pool (GLOSSARY: Natural size). A logarithmic transform goes only on the inputs of rate networks and reading decoders, for a total that spans decades within one run, and never on the ledger.
- **Small models, pinned arithmetic.** Ticket 08 caps a model at 100,000 parameters, so speed is not the constraint and reduced-precision weights buy nothing. Determinism is the constraint, and section 8's fork 7 holds it.

The order after the first experiment is ticket 01's, section 5.4: trait draws, then the planted-discrepancy experiment, then learning from readings. Learning past the teacher is the line's own next step once a first experiment earns it ([map](../map.md)). It is not parked.

### 5.5 Observer models

Three kinds, and they share only the seam.

| Kind | What it draws | What it rests on | Seed |
|---|---|---|---|
| Instrument | a reading or a waveform sample from state, as one tool at one site gives it: its filter, delay, error and artifacts | device accuracy studies (record 0004 asks for this check) | observation seed |
| Qualitative observation | an ordinal draw, such as an edema grade, from the produced quantity under it | diagnostic-accuracy and observer-agreement studies (ticket 01, section 1) | observation seed |
| Documentation | which entries are charted, when, and what is left out | not yet specified; it is clinical content under rule 16 | not yet specified |

**The accounting (L13).** Each feature of what an observer shows is one of three things. It is produced: computed from state, and checked against it. It is the instrument's: a tool's behaviour, sourced, and drawn from the observation seed. Or it is a convention: a shape with no state behind it.

**The ECG is the test case.** The body holds the rhythm's phase, its two modes and an activation memory, and no electrical model (`crates/ng-cardiovascular/src/rhythm.rs:15-40`, implemented; body map). So beat timing is produced. Lead noise and motion would be the instrument's. The shape of each wave is a convention. A template locked to the rhythm's phase is honest only if it says so, as a grouped region states what it cannot show (record 0002), and a scenario that turns on wave shape is refused when it is built. The contract's own design is that a waveform is a lens and an observer (`docs/architecture/solver-contract.md:684`, planned). Whether a law or an observer makes the ECG stays the engine's question, and this ticket supplies only the accounting.

**Checks.** For each produced feature, the value recovered from the output equals the state's within the method's stated error. The beats counted on a strip equal the beat edges in its window, and a pressure wave's mean over a beat equals the beat reading. An artifact sample carries a mark that a check can read, and the unmarked rest still passes. A convention does not move when state does.

**Outside check.** The device accuracy or observer-agreement study that each number comes from.

A learned part belongs only in the instrument kind, for an artifact that a rule misses, and it passes the same checks. Nothing here is this line's crate: an observer lives in `ng-observe` or in the workspace, as the engine's process decides.

### 5.6 Chart inference

From observations and chart entries it returns a distribution over packets whose engine runs would give that chart, and later over the events before the encounter.

**Scope.** A chart is engine-made, or a person authored it for a scenario. No real person's chart is ever an input (L12). The two uses the first capture named, starting a patient from a chart and a debrief on what could have been known, are product features, which this line leaves out ([map](../map.md), Decisions so far). The line owns the model and the proof that it is calibrated.

**Its only way into the engine is the proposal seam.** A draw is a packet. The engine admits it, an observer reads it, and the reading is compared with the chart within the chart's own stated error. No patient starts from an inferred state.

**At admission first (healthy and chronic steady states).** One engine draw is one admission, so ten thousand are in reach (section 4), including chronic conditions whose traits sit in the packet and admit directly at rest. The ladder: keep the population draws whose readings match the chart within its stated error, which is exact to that error and slow; then the nearest patients in the library; then an amortized estimator, only if matching is too slow at the tolerance a use needs.

**Over a course and acute non-equilibrium arrivals second.** The generative physiology model makes the courses, or the emulator does where the chart's readings are among its summaries. The engine runs a sample of the posterior's draws as the check. For a patient arriving hours into an acute evolving state (such as hemorrhage or worsening sepsis), the body is no longer on a returning periodic cycle (`NetBoundaryFlow`), and simulating many hours from rest per proposal costs 5 to 10 wall hours at today's speed. Fork 9 weighs searching a pre-run engine checkpoint library over acute trajectories against asking scope stage 2 for a non-periodic admission judge.

**Gate.** On engine-made charts whose packets are known, simulation-based calibration and the TARP coverage test hold at stated levels. Posterior draws pass the check through the engine at a stated rate. A chart from another revision of the body, or one authored to be impossible, is flagged.

**Outside check.** A chart that reviewed literature says cannot tell two causes apart yields a posterior that holds both. It is a test of direction only, as ticket 01's table of knowledge sources allows.

**A question for the engine's process.** An anchor is a target that rests on the literature. A value an author charts for a scenario's patient is not one. How an authored chart binds a packet belongs to the scope stage that owns scenarios.

### 5.7 The patient beside the engine

This is a product decision and not a model. It needs requirements of its own (ticket 01, section 7.5). There are three ways to hold the record.

| Way | Who the learner's patient is | What the learned model does | What it costs |
|---|---|---|---|
| Preview | the engine, always | answers what-if and fast-forward questions, shown as previews | the engine must keep pace with the encounter |
| Shadow | the learned model now, the engine after | answers at once, while the engine replays the same event log and its course is the one kept | the engine's lag, and a learner who acted on readings the kept course did not show |
| Learned only | the learned model | everything | no check at all |

In all three the handover is the event log and never a state, because the engine has no door for a state it did not compute (L1). Ticket 01's section 5.7 already separates a model's error from the effect of different actions the same way, by replaying each event log through the teacher.

**Gate, before any of the three.** Ticket 12's verdict on the whole body. The support signal flags planted cases outside support (L11). A statechart of the handover is checked in `docs/statecharts/`: no event is dropped, none applies twice, and none is stamped before the fence (I5).

## 6. Reference building blocks, as checked

Each source was read at the commit named, from a clone or a release, and nothing in it was run. A paper is cited from its repo's own text or from a search, and none was read in full. The files and symbols the first capture named exist. What it said some of them do was wrong in seven places, and Comments list them.

| Source, as read | What it holds | What carries over | What does not |
|---|---|---|---|
| `bobjz/H2NCM` at `7d2e119` (Zou and colleagues, ICML 2024) | In its MNODE model, one small network for each state reads only the states and inputs a hand-written list gives it (`MNODE_model.py:10-29`). In its LPSC model, a mechanistic model is trained and frozen, and small closure networks are then trained through it (`LPSC_reduced_model.py:25-30`, `LPSC_reduced.py:67-69`). Beside its prediction loss, a second loss runs three hypothetical inputs from one shared history and asks that the one a domain rule says should end highest does (`utils.py:153-159`). That supervises the order of effects and never their size, and no outcome of a hypothetical input is simulated | The read list, as section 5.4's mask arm, derived from `sparsity()` and not written by hand. Freeze, then close: a closure term is fitted only on a frozen named model, only as a rate on a declared transfer (L2), and only to fix a test the named model failed. The order loss, as the form a published direction would take if the planted-discrepancy experiment trains on one. On teacher data this line can do more, because the teacher gives every branch's exact outcome, so section 5.4's contrast term supervises the size of a difference | Its closure terms add to one state with no other end, and line 105 of `LPSC_reduced_model.py` reuses one of them for a second state. Its step is one Euler increment per sample of its data's grid (L6). Its channels are standardized, so it keeps no ledger |
| `DrunkJin/dreamer-from-scratch` at `9218432`, a small teaching copy of DreamerV3 on toy control tasks | A step that takes an observation, apart from a step that does not (`dreamer/rssm.py:118, 143`). A sampled categorical latent mixed with 1% uniform (`rssm.py:35, 106`). `symlog`, `symexp` and a two-hot code (`dreamer/utils.py:45-113`). A fixed tenfold weight on terminal transitions in its continue loss (`dreamer/networks.py:133-134`) | The two steps are L3 in code: the generative physiology model has only the step without an observation, and chart inference is the other. `symlog` as an input transform under section 5.4's scaling rule | A sampled latent (L4), a recurrent state with no ledger (L2), fixed ticks (L6), and a reward with an actor. Its tenfold weight names a real fault, a rare event learned as never. Here an event fires from a hazard rate against a seeded draw and is judged by time to event, so no loss weight stands in for it |
| `sbi-dev/sbi`, release 0.27.0 and main at `db2d84f` | `NPE_C` is posterior estimation over several rounds on flows or mixture density networks (Greenberg and colleagues, ICML 2019). `FMPE` is flow matching in one round only: "Multi-round ... is not yet implemented" (`sbi/inference/trainers/vfpe/base_vf_inference.py:144`). So the package holds no sequential flow matching. `NLE_A` learns the density of outputs given parameters, which is the nearest thing it has to an emulator, and it has no Gaussian process. `run_sbc` and `run_tarp` test coverage. `calc_misspecification_mmd` compares an observation with prior-predictive draws by a kernel distance, in an embedding if asked. Its set embedding takes NaN only as padding for whole missing trials of exchangeable data: main refuses any other NaN (`sbi/neural_nets/embedding_nets/permutation_invariant.py:114`), and the release fills one with zero | Simulation-based calibration and TARP, as section 5.6's gate. The kernel test, as one candidate for flagging a chart no engine patient makes. `NPE_C` to refine on one chart over rounds, or `FMPE` for an amortized estimator, and not both in one | The set embedding as a chart's encoder. A chart is ordered, stamped and partly observed within each row. None of that is supported, so a chart needs an encoder of its own. The kernel test as L11's support signal: it judges an observation and says nothing about a learned state that has left its support. sbi's own tutorial also warns that its embedding is not regularized as the test's paper asks |
| `beards-lab/PhysicalAI` at `77eaf14`; its README names no paper | A variational autoencoder fitted to 146 parameter sets, each fitted to a patient, draws 30,000 new sets from the fitted mean and covariance of its latent. A mechanistic model runs each set and labels it by solver success and fixed output bounds (`src/phys_informed_gen_ai/simulate_from_python.m:82-98`). Every row is kept with its label: 24,531 pass and 5,469 fail. Four classifiers then train on labels from the model's runs | Draw, run the mechanistic model, keep the label. Here a drawn packet goes to `admit`, and a refusal is kept and counted (L9) | Its joint structure, which comes from 146 patients' fits. No real person's data enters here (L12), and a correlation between two traits is a clinical value that needs a source (rule 16). `sampled` draws each key alone. The joint structure of produced quantities comes from the body at no cost |

| Other reference | What it is | Where it bears |
|---|---|---|
| NeuralGCM, now `neuralgcm/neuralgcm` (Kochkov and colleagues, Nature 2024) | a numerical core with learned tendencies | the seam inside the engine, which stays out of scope |
| ECGSYN (McSharry and colleagues, IEEE Trans. Biomed. Eng. 2003) | three coupled equations with hand-set wave shapes | a convention in section 5.5's sense, to be declared as one |
| SSSD-ECG (Alcaraz and Strodthoff, Comput. Biol. Med. 2023) | a diffusion model that generates 12-lead ECGs conditioned on disease labels | nothing yet: no state in the body drives a wave's shape, so a generated shape would be a convention that looks like clinical content |
| SepsisAgent (Wu and colleagues, arXiv:2605.14723) | a language-model policy with a learned world model, both trained on retrospective patient records | nothing: it is a model that chooses inputs, and ticket 01's section 2.5 keeps such data out of training |
| MeanFlow (Geng and colleagues, NeurIPS 2025) | a one-step generator trained from scratch, with no distillation | section 9's one-step sampler |
| Parareal (Gander and Vandewalle, SIAM J. Sci. Comput. 2007; Ruprecht, Comput. Vis. Sci. 2018) | with a standard coarse propagator it is unstable, or too slow to pay, where the spectrum lies on or near the imaginary axis | section 9 |
| History matching in cardiac models (Coveney and Clayton, Prog. Biophys. Mol. Biol. 2018; Longobardi and colleagues, Phil. Trans. R. Soc. A 2020; Rodero and colleagues, Ann. Biomed. Eng. 2023) | Gaussian-process emulators, an implausibility measure, and waves that rule values out and return a set, not one best value | ticket 15's model default has working precedent |
| Simformer (Gloeckler and colleagues, ICML 2024, arXiv:2404.09636) | one model trained on the joint of parameters and data, with attention masks for known dependencies, that samples any conditional | fork 8 |

## 7. What the first capture asked of the SRS, and what is left

| Asked | What the repo already says | What is left, and whose it is |
|---|---|---|
| Keep a proposal's provenance out of a run's identity | It is out. The identity names the start a run reached and not what proposed it: the packet hash closes over "the start, modes and fits it reached ... so one identity has one start whatever made it" (`crates/ng-contract/src/packet.rs:55-57`, implemented; GLOSSARY: Run identity) | What a replay holds. The engine keeps no start image: the first link holds opening totals and digests (`crates/ng-record/src/history.rs:591-604`), a checkpoint holds its head's image and the start's digest (`crates/ng-record/src/checkpoint.rs:257-266`), and replay admits again (`crates/ng-validate/src/replay.rs:22-26`). So a start reached from a proposal is rebuilt only if the proposal travels with the run. Native-contract 86's second box owns this, and fork 1 lists the routes |
| A provenance tier for a value fitted through an emulator | Value provenance has six kinds, and "fitted to a response" is none of them (ticket 15) | Ticket 15's decision record. This pass's input is rule 1 of section 2: the engine runs every proposal, so the emulator is no part of what the value rests on. It rests on the engine's run against a sourced response, stated with its method and window as an anchor is (rule 17), and on the rule that picked it among the values not ruled out |
| Provenance tiers for a distilled model and a learned observer | Rule 12 binds values in a packet or a healthy-range set. A model beside the engine holds none | No SRS change. L10 and section 2's rule 2 cover it. A fitted law inside the engine is still the map's open question |
| Events for disease that progresses in a run | Rule 14, I15 and the Event entry already put disease in the event log as parameter shifts or timed events | The contract "binds parameters once per run and has no parameter-shift payload" (foundation 07, C16). The capture's idea is an input to C16 and to scope stage 2: a scripted shift stands for a mechanism the body lacks, says so, and retires when a law lands. It needs no learned model |
| A judged entry away from rest | A checkpoint of a run that reached the condition is already a legal start, and a branch continues from it (GLOSSARY: Branch). A run keeps one each simulated minute (record 0008). Chronic resting conditions admit directly from chronic packet traits | Acute non-equilibrium arrivals (such as hours into hemorrhage or sepsis) are not periodic (`InitializationFailure::NetBoundaryFlow`, "no state returns") and cost hours to forward-run from rest per proposal at today's speed. Fork 9 weighs a pre-run checkpoint library (no SRS change) against raising a non-periodic admission judge in scope stage 2 |
| Reduced-precision weights beside an exact ledger | Any rates balance by construction (ticket 08) | Nothing. Such weights could not break balance, and they could break L5. At ticket 08's model size they buy nothing |
| A lighter gate for this line's tooling | Settled by `nightingale#238`: `tools/generative-physiology/**` sits ahead of `tools/**` (`tools/gate/paths.toml:18-24`) and runs `["10-docs", "10-rust"]` (`tools/gate/paths.toml:14-16`), covering this line's tests inside `10-rust` (`tools/verify.d/10-rust.sh:50-65`) | Resolved in `nightingale#238`; no SRS change |

Two citations in the first capture pointed at the wrong invariant. I4 says a refused step changes nothing, and I7 says physiology's bits do not change with observers attached. The ban on writing state from outside a step, which both were cited for, is I18, with I6 for vitals.

## 8. Forks, and the evidence that settles each

| # | Fork | Options | Evidence that settles it |
|---|---|---|---|
| 1 | How a start proposal reaches `admit` | As the packet's start entries, which the packet hash already seals (`packet.rs:462-463`); as a workspace a host fills and `admit` judges, every item of which is public today (`admission.rs:831, 1663`); or inside the validation crate's own solve, which is private | Native-contract 86's prototypes: each route's admission time, and what a replay must hold under each |
| 2 | Paired branches on long runs | Fresh runs of one prefix, as ticket 02 chose; or branches from one checkpoint. A branch can add an input after its checkpoint. It cannot take back one that already waits, because waiting inputs carry into a rewind (`solver-contract.md:322`) | The prefix's share of a run's cost on the body; the audit's checkpoint and rewind rows on the body; ticket 14's need 2 |
| 3 | Where the cut sits | Coarse totals for each region, or finer | Ticket 07's screen, and R against C4a |
| 4 | Dense or masked transfers | Every total feeds every transfer, or section 5.4's mask | The mask's density at the cut, then out-of-policy error |
| 5 | The emulator's form | A Gaussian process with history matching, or a neural estimator | The count of constants only a response pins, and the number of summaries |
| 6 | Who holds the record beside a learned model | Preview, shadow or learned only (section 5.7) | Body 30's speed, the statechart, and section 10's third call |
| 7 | The runtime of a deployed model | Pinned Python, which repeats only inside its environment (ticket 08); or Rust under the contract's arithmetic profile, which is a second forward pass of the kind ticket 02 refused for the first build | Asked only if fork 6 leaves preview. Then a parity gate between the two passes, and its cost |
| 8 | One model for each conditional, or one model of the joint | Section 1's five small models, or one model that answers any conditional | Two separate models sharing a failure that a joint model would fix |
| 9 | How an acute non-equilibrium chart enters the engine | Search a pre-run checkpoint library of acute trajectories (`(packet, event_history, checkpoint_time)`), keeping every start a legal engine checkpoint; or propose a non-periodic admission judge in scope stage 2 | Engine speed after body 30, library coverage over acute scenarios, and whether scope stage 2 needs non-periodic admission |

## 9. Parked

- **One model of the joint,** which fork 8 would earn.
- **Parallel in time with a learned coarse propagator.** Its result matches a serial run to a tolerance and not bit for bit, so it would be another declared resolution with the fitted weights inside the run's identity. It also converges poorly on oscillating systems (section 6), and the beat is one.
- **A fitted law inside the engine,** which waits on a provenance decision (ticket 01, section 1).
- **A one-step sampler,** for a waveform decoder that proves too slow. Nothing here is slow yet.

## 10. Human gates

No model makes these calls. Each is exact in scope, and each becomes a `ready-for-human` ticket when its build starts. The first, second, fourth and fifth are clinical content, which a licensed clinician reviews (rule 16).

1. Which features of a waveform or an exam a learner reads as clinical content. Those must be produced and may not be conventions (section 5.5).
2. What an admission chart holds, misses or records late. This sets what the documentation model must source (rule 16).
3. Whether a learner may act on a learned model's readings at all (fork 6).
4. Which response features set the pass bar on the whole body (ticket 14).
5. A face-validity read of mixed, unlabelled teacher and learned trajectories. On synthetic numbers it judges shape and never magnitude (ticket 01, section 6).

How much slow drift over two days is too much is not on this list. Record 0004 holds it to sourced within-subject variation.

## Non-goals

- Changing `docs/srs.md`, `docs/adr/`, `GLOSSARY.md`, `crates/` or `tools/`.
- Any prototype before body 28 resolves, beyond what the map already leaves outside the parking.
- Training on the whole body before body 28 resolves (`nightingale#241` resolved native-contract 91; [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) remains open).
- New glossary terms. This ticket uses seam, proposal, emulator, cut and manifest in their plain senses, and each enters the glossary with the first build ticket that needs it.
- Grading, scenarios, debrief and display.
- Any claim of clinical validity from synthetic runs or from a face-validity read.

## Comments

The review pass read the first capture at `2333f26` and changed the things below. They are listed so that a later pass does not reopen them.

- **The ladder became seams.** M0 to M6 read as one line of stages. They are five kinds of model that ask for different trust, and their dependencies are not a line: chart inference needs the generative physiology model for its courses, and the ladder had it the other way round. M0 is the engine's own work and is now section 5's table of what the line waits on. M1 is 5.1, M2 is 5.2, M3 is 5.3, M4 is 5.5, M5 is 5.6, and M6 is 5.4 as research and 5.7 as a product decision.
- **No cost had been stated.** Section 4 is new, and it sets the order more than any other fact.
- **The planted-discrepancy experiment is not parked.** The map makes it the next step after a first experiment that earns it. The capture had parked it behind M6.
- **Engine claims corrected.** A run's identity already names the start it reached. Rule 14 and I15 already put disease in the event log, and foundation 07's C16 holds what is missing. I4 and I7 were each cited for I18's rule. Only a row's reads are public, not a column's, so the mask is not free. The body has no electrical model, and who makes the ECG is the engine's open question, so a template waveform was not this line's to choose. The whole body runs on its holds: nine systems have a crate, four have none, and 318 pools are held. The body's history is not bounded on disk until native-contract 95. A branch cannot withhold an input that already waits.
- **Reference claims corrected** (section 6):
  - sbi holds no sequential flow matching.
  - Its NaN tolerance is padding for whole missing trials, not an irregular series, and `accepts_nan_input` is in no release.
  - H2NCM's causal loss ranks hypothetical inputs by domain rules. It is not a contrast against simulated counterfactual outcomes.
  - H2NCM's read lists, and its frozen closure with the reused network, belong to two different models.
  - SepsisAgent trains on retrospective records and rejects none.
  - MeanFlow is not a distillation.
  - PhysicalAI labels a failed draw and keeps it, its correlations live in a latent space fitted to 146 patients, and it reads no laboratory value.
- **Dropped.** Reduced-precision weights: the models are small, and determinism is the constraint. Comparisons with other industries. Two acceptance boxes: prototypes before body 28, which the map's parking excludes, and a change to the gate's path rules, which the capture's own non-goals excluded. A sampled discrete latent (L4). Three provenance tiers for learned models, which section 2's two rules replace.
- **Moved.** The site-class graph model left the parked list. It is section 5.4's kernels shared by law and site class, which ticket 08 already half holds.
- **Kept.** The cheapest baseline first. `admit` as the only judge of a start. An emulator that proposes while the engine verifies. Refused runs kept and labelled. Observer models before chart inference. An exact ledger. A statechart for the handover.
- **Not checked.** No paper was read in full. Section 4's worker-hours are arithmetic on figures measured on two machines at two commits. No claim about the whole body was rerun.

Follow-up pass (after merging main `9f5ba6c`):

- **Framing adopted as working base.** Section 2's four-seam framing and section 3's draft rules (`L1` to `L14`) are adopted for upcoming `pstack` passes, recorded in [the map](../map.md).
- **Acute vs. chronic entry separated (fork 9).** Chronic resting conditions admit directly from chronic packet traits, while acute non-equilibrium arrivals (such as hours into hemorrhage or sepsis) are neither periodic nor cheap to simulate from rest per proposal at 2.5x real time; fork 9 weighs a pre-run checkpoint library against a future non-periodic admission judge.
- **Sourced covariate scaling noted (section 5).** Independent uniform draws across dozens of traits risk anatomically incompatible combinations; clinical population sampling needs sourced allometric and covariate links (GLOSSARY: Population record).
- **Speed sensitivity noted (section 4).** Course-scale budgets scale linearly with engine speed, so engine speed work after body 30 can directly expand feasible teacher datasets.
- **Engine-process questions filed.** Cross-references added to `native-contract 86`, `foundation 07` (C16), `ticket 15`, and `body/map.md` (ECG accounting); the developer-reviewer `paths.toml` item stays open as its own Acceptance box.

Reconciliation pass (after merging main `ffcd1a7a`):

- **Merged `nightingale#234` to `nightingale#241` facts.** Updated section 4's speed aim to `docs/srs.md` section 6's `1×` live and `>= 10×` unbounded single-core aim (`nightingale#234`) with recomputed distance ratios (`1.3×` short on the slice, near `4×` on the body at `body 26`'s commit) and noted `nightingale#235`, `nightingale#236`, `nightingale#239` and `nightingale#240` ahead of `body 30`; ticked Acceptance box 3 and updated section 7's final row for `tools/generative-physiology/**`'s `["10-docs", "10-rust"]` gate rule (`nightingale#238`); marked `native-contract 91` resolved in `nightingale#241` while keeping `body 28` parking, `body 30` and `native-contract 95` open; and re-verified all `file:line` citations against `ffcd1a7a`.
