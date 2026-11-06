# 01: Research and system concept brief

Type: research
Status: resolved
Blocked by: none

**Question:** Can a learned generative model itself carry a simulated patient's physiology through an encounter, responding to assessment and intervention? What is the smallest experiment that tests this, and what evidence would tell a model that learned a physiological response apart from one that copies its teacher?

The Answer is a research and system concept brief that an architecture pass can build on. It recommends and labels; it does not specify interfaces, crates or a state schema.

- [x] Intended system and boundaries: training, the start state, the encounter at run time, assessment as a query that returns an observation, intervention as an input that changes the body, and what must persist so the patient stays the same patient (section 1)
- [x] The learning problem: what the hidden state means (named, learned or partly named), how its dynamics are produced, and how each knowledge source becomes a training signal, a constraint or a test (section 2)
- [x] Time and physiological consistency: elapsed time, delays, observation cadence against the underlying process, and which balance and range checks are hard guarantees, penalties or diagnostics (section 3)
- [x] What is established, what is a hypothesis and what is still open (section 4)
- [x] The smallest informative experiment: question, physiology, teacher, inputs, outputs, rollout, source of variation, fair baselines and one result that would falsify the claimed advantage (section 5)
- [x] Levels of evidence, including one check whose reference comes from neither the engine nor the training source (section 6)
- [x] The next decision for the architecture pass, and what must be validated before formal requirements are useful (section 7)

Every repository claim cites a file and the commit read. Every resource or dataset figure is labelled an estimate until measured.

## Answer

**Read at** commit `b8990a5007c840df756a03edb487cd02aa4a3751` (from `git rev-parse HEAD`), branch `research/gpm-concept-brief`, on 6 November 2026, read only. Each code claim carries a label: implemented (workspace code), synthetic test (a workspace test exercises it on a synthetic fixture), planned (docs, the sketch or a ticket only) or author-reported (a figure written in the repo that no test asserts). A citation of `docs/srs.md` or a decision record states a requirement and is labelled so. Path aliases: `contract/` is `crates/ng-contract/src/`, `record/` is `crates/ng-record/src/`, `validate/` is `crates/ng-validate/src/`, `vtests/` is `crates/ng-validate/tests/`, and `issues/` is `.scratch/native-contract/issues/`. Every resource, dataset or time figure is an estimate with its assumptions, and every proposed threshold is marked for review with its grounds. A read-only run of the existing slice, trace, enqueue and branch tests passed at this commit.

### In brief

- **Running the patient.** During an encounter, learned dynamics compute each next state from the patient's own state, traits and inputs. No engine step runs, and nothing resets the state to an engine state. The engine teaches offline.
- **State.** The first instance carries named totals of conserved amounts, evolved by learned transfers under exact two-ended balance and supervised by the teacher's ledger. A small learned latent part is added only if a richer reduction beats the named totals. The only explicit content is which totals exchange and that each transfer has two ends. Every rate and every reading is learned.
- **The central question is two questions.** A model can copy two things from its source: the sampling assumptions of its training data (action policy, start states, horizon, step) and its teacher's mechanism errors. Out-of-policy paired branches test the first in the first experiment. Only a signal the teacher lacks can test the second: a planted-discrepancy experiment and published human evidence. Reproducing the teacher where the teacher is right is not a failure.
- **First experiment.** On the arena slice, learn the response to posture changes: blood moving between central and dependent pools, filtration into dependent tissue, and the baroreflex carried by transmitter amounts. Train on randomized posture schedules and on schedules from a scripted state-triggered rule. Compare with persistence, a linear model, a recurrent readings-only model, an unconstrained state model and the named model with and without a latent part, in free runs and in a closed loop.
- **Second experiment.** Fluid deliveries through the gut routes, the first dose and composition question, once a host-built synthetic packet and hour-scale runs exist (5.10).
- **Feasibility.** Posture already enters a running slice session in tests, so the first experiment needs no new input and no packet change. A teacher audit comes first, because the core picked for the slice has run it only for one second with no input, and no test runs the slice past 30 simulated seconds.
- **Limits.** The first experiment can show whether a learned generator carries a coupled, conserving body through an encounter and answers inputs causally beyond its training policy. It cannot show human validity, because the slice's numbers are synthetic.
- **Next decision.** Adopt this model class and the posture experiment for the first build, gated by the teacher audit on the reference machine.

### 1. Intended system and boundaries

The engine already separates the roles a learned patient needs, so the learned model uses the same words.

| Role | Engine at this commit | Learned model |
|---|---|---|
| Stable traits | the packet's parameters, whose hash is fixed at admission (`docs/srs.md:28`, requirement) | a trait vector drawn with the patient seed and fixed for the encounter |
| Evolving state | amounts, memories, modes and stream positions, changed only by an accepted step (`docs/srs.md:32`, requirement; `contract/state.rs:5-15`, implemented) | named totals and an optional latent part, changed only by the learned dynamics |
| Inputs | timestamped events through the boundary port (rule 13, I18); today water, sodium, potassium and chloride deliveries and posture (`contract/boundary.rs:27-44`, implemented) | the same events, entered as rates or settings over their intervals |
| Queries | lens channels with method, window, delay and error, read by an observer that never writes (I7; `contract/lens.rs:10-46`, implemented) | an observation model that reads the learned state and never writes |
| Observations | one number per reading (`crates/ng-observe/src/observer.rs:31-43`, implemented); the chart is defined, not built (`docs/srs.md:79`, requirement) | numeric readings and, later, qualitative observations |

The learned model needs three things the engine lacks. It needs qualitative observations. It needs a support signal for states outside its training region, since the engine's validity range only reports an excursion (`contract/quantity.rs:442-461`, implemented). And it needs a start state without the engine's admission solve. In the first experiment the start state is the engine's admitted one after a pre-encounter history. Later it is the learned model's own rest point, which must agree with the engine's. That agreement is a check, not a given.

**Phases.** Training is offline: teacher runs become datasets and fitted functions. Initialization draws traits and takes a start state. At encounter run time, the surrounding product advances simulated time while the learned dynamics evolve the state. A query returns an observation after its delay, and an intervention acts from its instant. Grading, scenarios, feedback and display belong to the product, which only sends queries and inputs and advances time.

**Persistence.** The patient is one tuple: traits, state, waiting inputs, simulated time, the event log and three seeds. It stays the same patient when only the learned dynamics change the state, when the tuple is checkpointed whole, and when replaying the event log with the same seeds gives identical bits at any pace. That is the learned model's form of rule 4, I2 and I17. A model that keeps the patient only in a bounded context window loses the patient once the window slides past the last input, so the experiment tests persistent state directly.

**Assessment and action.** A query reads state. An intervention changes what the boundary supplies. A procedure that does both is two events at one instant: an input and a query. An active stand test is expressible today as a posture input with readings taken during it (`contract/boundary.rs:45-75`, implemented). A passive leg raise and a blood draw wait for planned payloads (`docs/architecture/solver-contract.md:340`, planned), and the draw is a removal plus a lab query.

**Readings are not state.** A numeric reading is a produced quantity taken by a method over a window, with error. From named totals, a concentration is an amount over a volume, and a pressure is decoded from volumes, transmitter effects and traits. A qualitative observation, such as an edema grade, skin perfusion or jugular filling, is an ordinal draw whose dependence on an underlying quantity comes from published diagnostic-accuracy and observer-agreement studies. It is judged against those studies and by clinician review. A clinician's overall impression synthesizes such cues. So the model generates the cues (perfusion, mentation, breathing effort, pallor), and any scripted colleague's impression would be a product feature built on them.

**Closed loop.** State yields readings when queried, readings inform the clinician's next input, and the input changes the state. An offline sequence predictor trained on logged trajectories falls short in four ways. Closed-loop actions reach states the logging policy never produced, and the model's own errors change which actions follow. It can read an action as evidence of hidden state (the patient was laid flat because pressure was low) rather than as a cause, which is wrong under another clinician. It can learn the measurement policy from when readings were taken. And a bounded context loses the patient. The first experiment needs no learned clinician: a scripted, state-triggered rule closes the loop.

**Runtime relationship with the engine.** It stays open, and each option has a price.

| Relationship | Needs from the engine | Needs from the learned model |
|---|---|---|
| Offline teacher (first experiments) | only public interfaces that exist: admission, `Session::enqueue`, the accepted state view, the ledger and checkpoints (implemented) | reproducible scoring: CPU rollouts in a fixed order |
| Beside the engine, under its own contract | nothing at run time | its own form of rules 1, 4 and 13 and of I7, I9 and I10: one state tuple, an event log, three seeds, pace-independent bits, read-only observations and a support signal |
| Inside the engine, as learned laws | a contract version for a law whose kernel is a network, still declared as data and compiled (rule 8) | rates on declared two-ended transfers only (rule 3, I10), never an amount or a reading (I6, I14), and identical bits (rule 4) |

Fixed-order CPU evaluation is necessary for the third option but not sufficient. The law would also have to fit the compiled kernel representation, its declared units, the accepted-step checks and replay (`docs/architecture/solver-contract.md:120`, planned design). It also raises a question for the engine's process before any design: fitted weights are neither a source record nor an inference record, and rule 12 requires one of those behind every non-test value (`docs/srs.md:102`, requirement).

### 2. The foundational learning problem

**2.1 What must be carried.** The engine's own structure, written as an illustration, not a requirement:

```
da/dt = S r(x, θ, u)        dm/dt = g(x, θ, u)        y_c(t) = h_c(x(t), θ) + σ_c ε_c(t)
```

`a` is the vector of amounts in pools, and `m` the memories (filters, set points). `x` is the state: amounts, memories, modes and stream positions. `θ` is the packet's parameters, the traits. `u` is the drive frame: delivery rates per boundary route, and the posture. `S` is the fixed incidence matrix: column `k` takes amount from one pool into another pool or into a receipt pool. `r` is one signed net rate per column, as the laws compute it. The ledger keeps the time integrals of each column's positive and negative parts, stage by stage, as gross forward and reverse extents (`contract/quadrature.rs:59-64`, `record/history.rs:14-27`, implemented). `g` gives memory rates. `h_c` is channel `c`'s produced quantity by its method and window, `σ_c` its SD, and `ε_c` a draw from the observation seed. In words: amounts move only along two-ended flows, memories move only by declared rates, and readings are taken from state without feeding back.

The engine's `x` is complete by construction, since the solver continues exactly from it. A learned model at encounter resolution carries a smaller state `z`, and three things can make `z` incomplete. Averaging fast processes is harmless when they relax fast to values that slow state sets. Summing pools whose rates differ causes closure error: the urinary tract, for example, joins a tubule that turns over in seconds with a bladder that empties over minutes. Dropping slow quantities loses memory. Readings alone are never complete, because `y` is many-to-one in `(x, θ)`. In the slice, admission fits three calibrated keys to three anchors (`validate/slice.rs:494-536`, implemented), so trait draws that admit meet the same resting heart rate, mean arterial pressure and filtration. Some traits still show in other resting readings, and the rest show only under load (5.10). A model fed readings must rebuild state and traits from history, with memory as long as the slowest process that still matters.

**2.2 What each knowledge source can become.** A description is not an executable constraint, and a constraint is not an informative objective. "Sympathetic traffic rises when carotid stretch falls" is a description. "Traffic falls with stretch at every visited state" is a constraint that any model can be checked against by finite differences. The teacher's recorded transmitter release under randomized posture schedules is an informative objective: it pins the function down on the visited states, where the sign constraint only rules out some wrong ones.

| Source | Holds | Becomes | Pins down | Leaves open |
|---|---|---|---|---|
| Teacher runs | full state, per-column extents and inputs under any protocol chosen | transfer and rollout targets; exact paired counterfactuals | the teacher's response on visited states and inputs | what the teacher does not encode, and all human validity |
| Equations from the literature | forms and parameters | the teacher's laws; explicit rate forms in a hybrid; residual losses | behaviour beyond the data, only where the form is trusted | missing mechanisms; on complete equations a residual loss only re-solves them |
| Physical constraints | balance, positivity, units, two-ended transfers | hard structure and range checks | nothing alone, though they remove whole directions of error | which rate is right |
| Published summary data | means, spreads, timings or fractions under one protocol in one population | held-out checks, and later the calibration of a few trait degrees of freedom | no more quantities than there are independent statistics | dynamics, and which mechanism a miss implicates |
| Textbook prose | directions, orderings, couplings | sign and order tests once written as checks | nothing until it is executable | targets (rule 16) |
| Clinical time series | partial readings under clinician-chosen actions | likelihood terms that need hidden-state inference (not in the first experiments) | effects only under causal assumptions | counterfactuals without those assumptions |

Fitting a few measurements through a differentiable model pins down what was measured, not the mechanism behind it. *Med-Real2Sim* shows both sides. A network pretrained on simulated samples emulates a left-ventricular circuit model, and a video encoder trained through it recovers parameters that reproduce two labelled volumes per echocardiogram, while the inferred pressures and resistances cannot be checked against measurement. A mechanism that a frontier model authors where the literature has none enters the learned model's research record as a labelled hypothesis. It stays only if it improves agreement with a sourced held-out check, and it is never the target. It does not enter the engine as an inference record: record 0003 lets inference fill only a parameter or a number of the day, never a law form (`docs/adr/0003-inferred-parameters.md:3`, requirement).

**2.3 Learning routes.**

*Route A: learned transfers on named totals, supervised by the teacher's ledger (recommended).* The learner receives each interval's gross extents, summed into named transfers, so each transfer's integral over the interval is observed directly rather than inferred from changes in totals. Opposing flows, such as filtration against lymph return, cancel in a change of totals but stay apart in the ledger. The learner's objective is the error between its own integrated transfers and those extents, plus the rollout error of totals and readings over windows in which it consumes its own predictions. It needs full teacher state and extents, which a host can read through public interfaces: every row and mode through the accepted state view (`contract/state.rs:223-264`, implemented) and per-column forward and reverse curves through the ledger (`record/record.rs:257`, `record/history.rs:442-486`, implemented). Identifiability: an extent fixes the integral over its interval, not the instantaneous rate, so the rate law is pinned down only up to the interval's resolution, and only on visited states if `z` is close to a sufficient state. Elsewhere only the structure limits it. Evidence is indirect. The hybrid in *Learning Insulin-Glucose Dynamics in the Wild* keeps a named mechanistic state whose parameters a latent process varies. Two participants were modelled separately, and for the one whose accuracy is reported, it forecasts glucose with 27, 35 and 36 percent lower mean absolute error than an LSTM at two, three and six hours, while the LSTM and an ARMA model win from 5 to 30 minutes. Under counterfactual meal and insulin schedules, the hybrid moves in the physiological direction and the LSTM barely responds to insulin, though no counterfactual reference exists there. No source read here shows a learned model carrying a coupled body through hours of closed-loop use. That gap is the first experiment's question.

*Route B: equations and constraints without trajectories.* The learner receives equations, parameters, a start state and inputs. Its objective is the equation residual at sampled points, with the start and boundary conditions. With complete equations it re-solves what an integrator already solves, and *Universal Differential Equations for Scientific Machine Learning* notes that residual training grows harder as stiffness grows, which physiology has. With incomplete equations the residual says nothing about the missing part. Identifiability: equations determine the dynamics only where they are complete. Balance, positivity and a healthy equilibrium leave many rate laws open, with different delays, overshoots and failure behaviour. Use: structure and diagnostic checks, not the training signal.

*Route C: explicit equations with learned corrections*, the universal-differential-equation pattern. The learner receives explicit rate forms with gaps, and its objective is trajectory or measurement error with the known parts held fixed. The glucose hybrid is one example: a mechanistic model whose parameters drift under a latent process mapped through a network, fitted by variational inference. It suits learning from partial real readings. Identifiability: sparse observations may not separate a missing term from a wrong parameter, observation error or an omitted process. As the first experiment's main arm it would need explicit equations at the learner's resolution, which do not exist for whole-body totals. Writing them is the small mechanistic simulator this work must not substitute for learning. It is a later comparison arm.

*Route D: reading sequences or learned latents*, the generative-pretrained-transformer analogy and latent world models. The learner receives readings and inputs. Its objective is the likelihood of the next reading, and a latent variant also reconstructs the readings. It needs no hidden state. Identifiability: its latent is pinned down only up to an invertible transformation, so no constraint can be stated on it without a decoder. *Data-Driven Simulator for Mechanical Circulatory Support with Domain Adversarial Neural Process* forecasts mean arterial pressure 15 minutes ahead under a planned pump schedule, offline, from clinician-chosen pump levels in 211 patients plus simulated series aligned by domain-adversarial training. It reports about 5 percent lower error overall and about 19 percent on non-stationary series, and it does not state whether its test split is by patient. *Multiple Physics Pretraining for Spatiotemporal Surrogate Models* trains one step ahead on two-dimensional simulated fluid fields, scores rollouts of up to five steps, and names long-rollout stability, conservation and noisy data as open problems. It shows transfer across related simulated physics, not a learned patient. Route D is the first experiment's main comparison, because it is the default reading of a generative model.

Recommended combination: route A for dynamics, route B as structure and checks, the teacher as the only bulk signal at first, published data as held-out checks, and route C later for real data.

**2.4 The two state decisions.**

| Meaning, by how dynamics are produced | Explicit, with learned corrections | Learned rates under constraints | Mostly learned |
|---|---|---|---|
| Named | checkable, but mostly the teacher's assumptions: the neural-patch risk | **recommended start**: checkable, balanced by construction, engine usable as teacher, comparison and counterfactual source; needs a reduction map and a sufficiency test | readings checkable, balance only as a penalty |
| Partly named | the glucose hybrid's shape, suited to real data | the first experiment's closure variant | the latent absorbs what the named part should show |
| Latent | not meaningful | constraints only through a decoder | most expressive for unwritten physiology, but uncheckable, data hungry, and able to drift off the training region unseen |

Constraints can be stated and checked on named totals, and on a latent only through a decoder. Named totals are exact sums of engine pools, so teacher data map onto them without fitting, and their transfers are read from the ledger. A latent uses the teacher only through readings or a learned encoder and stays defined only up to transformation. Named state does not give up unwritten physiology. The effective transfers between whole-body totals appear in no engine law, because they emerge from per-bed laws, valves and beats. So the named route learns relationships nobody wrote down, in a modest synthetic sense. In long free runs, errors in named totals stay in physical units and can be audited, while latent drift can stay invisible until a decoded reading goes wrong. Only named state can be inspected at a checkpoint.

The lean toward named state holds, with one refinement: the named part is conserved amounts and slow memories, and all their dynamics are learned. The guard against a traditional simulator with a neural patch is checkable: no engine law form is copied into the learner, and every rate, its dependence on state, traits and inputs, and every reading are learned. Two comparisons decide the axes. The meaning axis is decided by named-only, named-with-latent and readings-only arms on out-of-policy branches and long free runs, and by a richer reduction that tests whether the named totals are sufficient (5.5). The dynamics axis is decided first by the unconstrained-against-structured comparison, and later by an arm with explicit forms for some transfers, scored on inputs outside the training range, where only a trusted form can help.

**2.5 Treatment confounding.** With the engine as teacher, the experimenter sets the action policy. Inputs at state-independent times and settings identify the teacher's interventional response on the visited states with no causal assumption, and paired branches give its exact counterfactual for one patient and state. Policy-coupled data, where a rule chooses inputs from readings, fail in two ways. They miss support, because actions occur only in some states. And they bias any learner whose state does not determine what the rule read: sicker patients get more treatment, and the treatment looks harmful. So for a reduced named state, confounding shrinks to a support problem only if that state is sufficient, which is itself under test. Observational clinical data carry both problems and rest on untestable assumptions (no unmeasured confounding, every action possible in every state), so they stay out of training here. Unverified throughout: every human response, and the teacher's response in states it was never run in.

**2.6 Generation, inference and learning.** Generating a patient needs no inference: draw traits, take a start state, evolve, read. The named arm of the first experiment learns from the teacher's full state and needs no inference either. Two arms do learn hidden quantities: the readings-only arm's recurrent memory and the latent part of the closure arm. Neither involves posterior inference, but both are pinned down only up to a transformation, so both are judged only through readings. Statistical inference of hidden quantities becomes necessary later, in three places: fitting a latent part to data without full state, learning from real readings, and calibrating a trait distribution to population statistics. A named-state generator holds its own state, so it needs no inference at encounter run time. Three meanings of inference stay apart: a language model's value for a gap in the packet (record 0003), running a trained model forward, and estimating hidden quantities from readings. A learned model's predictions never become the reference they are judged against (rule 16).

**2.7 Variation, slow processes and discontinuities.**

- Patient differences come from traits drawn with the patient seed (`contract/scheme.rs:282-313`, `contract/packet.rs:262-279`, implemented). The first experiment holds traits fixed, so it cannot test whether the learner uses them (5.3).
- The start state follows a pre-encounter history: how long the patient lay, sat or stood before the encounter, which changes how much fluid sits in dependent tissue. Present.
- Process variation comes from the run seed, entering as rate factors and never as amounts (I10). The slice declares no stream, forcing or hazard. Only `ChanceCase` and test-local bodies do (`validate/fixture.rs:431-433`, `vtests/chance.rs:248`, synthetic test), and a slice forcing is planned (`issues/17-slice-hazard-and-forcing.md:5`). Absent from the first experiments.
- Measurement error comes from the observation seed (`contract/chance.rs:76-82`, implemented). Present.
- Model uncertainty is measured across training seeds.

Diversity in the first experiment is history-driven: the same stand reaches patients whose dependent tissue already holds different amounts, so one stand drops pressure in one patient and barely moves another. It is meaningful if the learner keeps the across-patient order and spread of response features against paired teacher runs. One complete state under the same inputs has one future, as in the teacher, and per-step randomness waits for a teacher or source that supplies it.

The engine puts chronic disease in parameter shifts and acute disease in timed events (rule 14, I15), and the learned model can match that with trait shifts and input events. Progressive injury and recovery are better carried as slow named state than as drifting parameters. The SRS motif plans "a damage integral when demand is unmet" (`docs/srs.md:146`, requirement), so a learned slow state can line up with an engine quantity, while drifting parameters fit data well but have no teacher and no balance meaning. The line between state and trait is a timescale choice. The slice's baroreflex set point relaxes at 1e-6 per second (`validate/slice.rs:681`, implemented, synthetic value), a time constant near twelve days, so it acts as a trait within an encounter.

Valve modes average out at the learner's resolution. The first experiments exclude death and failure thresholds, since the slice has none and an engine fault never writes death (`contract/failure.rs:367`, implemented). Later, modes become discrete state with learned hazard rates that fire against seeded draws, as the engine's hazard guards do (`contract/chance.rs:146-182`, implemented, exercised only by synthetic tests on non-slice bodies), with death an absorbing mode judged by time to event against the teacher and sourced outcome rates.

### 3. Time and physiological consistency

Simulated time is the model's clock. Wall time only sets pace, and any pace or split of a run into calls must give identical results, as rule 4 requires of the engine. The learned object is a rate per unit of simulated time, not a per-step increment. Inputs enter as rates or settings over their intervals, as deliveries and postures already do (`contract/boundary.rs:27-75`, implemented). Dividing an interval, or an input into back-to-back pieces, then leaves the drive unchanged and moves physiology only by integration error. Intervention timing and duration are exact, and delays come from state, such as dependent tissue filling or a hormone clearing, rather than from lag parameters.

Timescale separation decides what is state. The slice spans a 10 ms longest step (`vtests/arena_body.rs:184`, synthetic test), a 1 s cycle, a baroreflex filter at 4 per second, bladder drainage at 1e-3 per second (about 17 minutes) and the set point above (`validate/slice.rs:676-687`, implemented, synthetic values). Vasopressin clearance runs at 2e-3 per second, but only on the central-vein pool (`validate/slice.rs:247-249, 362-364, 682`, implemented), which holds about a tenth of blood water in the fixture's start guess. So whole-body vasopressin washes out with a time constant near 83 minutes, by arithmetic on the code and not by a run. Filtration into dependent tissue after a posture change is planned to take 10 to 35 minutes (`docs/architecture/solver-contract.md:333`, planned), and the slice's own value is unmeasured. At one-cycle resolution, cycle-scale quantities become readings decoded from slower state, minute-to-hour amounts are state, and day-scale memories are near constant. Waveforms stay outside the first experiments. Later they would be an observation model on cycle-mean state.

Three direct tests:

- *Split*: apply one input, and the same input in back-to-back pieces. The physiology should match up to integration error.
- *Refinement*: halve the learned model's integration step twice. Its error against a tightly integrated reference of the same learned dynamics should fall at the scheme's order. Error against the teacher is measured separately, because it falls only to the model's own modelling-error floor and can even rise as the step shrinks. A model that matches the teacher at its training step but whose converged flow does not has learned a step map, not a rate.
- *Cadence*: change how often and which readings are taken. The state-separated model's physiology should stay bit-identical, as I7 requires of the engine.

| Property | Kind in the first experiment | How |
|---|---|---|
| Balance of each named total, inputs and outflows counted | hard | two-ended transfers, with basal intake, urine and skin water as boundary transfers; still checked over windows |
| Nonnegative amounts | hard acceptance condition | each outgoing rate vanishes as its pool empties, and an update that would still leave an amount negative is refused and retried over a shorter interval, as the engine's record does; a vanishing rate alone does not protect a discrete update |
| Units | hard | each transfer carries its unit, with scales fixed per quantity |
| Readings never write state, and variation enters only through rates | hard | a query-only observation model, and rate factors once process variation exists (I10) |
| Species the learner does not carry | outside the claim | the teacher conserves them; the learner's balance covers only its named totals |
| Ranges | diagnostic | the teacher's visited range per total, flagged and never clamped |
| Monotone response to posture where a source supports it | diagnostic, later perhaps a penalty | finite differences on paired branches |
| Rest stability | diagnostic | no input change; drift compared with the teacher's |
| Return toward rest after an input | empirical tendency | measured, not imposed |

Balance by construction proves only that no amount is made or lost. It says nothing about whether amounts move at the right rates, and a few balanced totals do not make a valid body. When the state leaves the supported region, the model publishes it and records the excursion, as the engine does for a validity range. A support signal combines disagreement across training seeds with distance from the training states, traits and inputs, and every metric is reported inside and outside support. Clamping is ruled out, because it hides failures and breaks balance.

### 4. Established, supported, hypothesized and open

| Kind | Content |
|---|---|
| Requirements and decisions | A learned generative model carries a virtual patient's physiological evolution through an encounter, responding to environment, disease, assessment and intervention. The first use is teaching: a plausible, causal response including deterioration, uncertainty and treatment failure, grounded quantitatively where feasible, with every behaviour labelled validated, approximate or untested. Variation has a physiological source. Product features stay outside the model. The engine's plan is unchanged, and the learned model develops beside it. Inference is never a target. Clinical decision support is out of scope. |
| Repository facts | Labelled where cited. In summary: the slice runs four of the 13 atlas systems as laws plus the receptor layer, and holds the other nine as stand-ins (`validate/slice.rs:387-403, 450-463`, implemented). Its numbers are synthetic (`README.md:3`). No learned or surrogate solver core exists, and the only "surrogate" in the crates is a doctest string (`contract/lib.rs:946`, implemented). Scope stage 5, Learning, means replay and debrief (`docs/srs.md:19`, requirement), so the requirements neither allow nor limit machine learning. |
| Supported by sources | Named-state hybrids beat sequence models at long horizons and answer counterfactual inputs in the physiological direction, on one participant's observational glucose data. Offline intervention-conditioned forecasting of arterial pressure works 15 minutes ahead. One-step surrogates leave long rollouts open. Head-up tilt and tilt back give asymmetric responses in healthy people. Isotonic saline is excreted far more slowly than an equal free-water load in healthy men. |
| Hypotheses | H1: learned transfers on named water and transmitter totals carry the slice's posture response through free and closed-loop runs better than a readings-only model. H2: randomized posture schedules generalize out of policy where policy-coupled ones fail. H3: the named totals are close to a sufficient state at one-cycle resolution. H4 (later): trait conditioning keeps the across-patient order of responses. H5 (later): the route can absorb sparse summary evidence and move past its teacher. |
| Labelled leaps | That what is learned on synthetic numbers carries over when the teacher gets reviewed numbers; that a scripted rule stands in for a clinician in the loop; and that a few totals suffice at encounter scale. Each has a test in sections 5 and 6. |
| Open decisions | The runtime relationship; model families after the first experiment; the first domain and data source (proposed here, reviewable); the exact state; disease, death and qualitative observations; and where the data host lives. |

Commit now to these:

- the roles and the class
- readings never feeding the dynamics
- amounts changing only through two-ended transfers, in the learner as in the engine
- teacher data with decoupled, randomized inputs that keep full state and extents
- the teacher never being the only judge
- an evaluation harness frozen before any arm is tuned

Work it as a bounded research loop: one editable training file, a fixed budget per run, a fixed evaluation, and a log of kept and discarded changes. Everything else can wait: the schema, latent size, later model families, process noise, disease, qualitative observations, any place inside the engine, and hardware beyond the reference machine.

### 5. The smallest informative experiment

**5.1 Question.** On the slice, does a learned generator whose state is a few named totals, and whose dynamics are learned transfers under exact balance, carry a patient's response to posture changes through a free-running encounter, including schedules outside its training policy and in a closed loop, better than a model that learns from readings alone?

**5.2 Why posture first.**

- Posture already enters a running slice session. Synthetic tests apply a stand, a sit and a 21-event ramp and check the directions of pressure, heart rate, stroke, filtration and sympathetic traffic up to 30 simulated seconds (`vtests/arena_body.rs:538-562, 739-804, 881`). A stand enqueued after stepping and across a checkpoint gives identical bits (`vtests/arena_body.rs:714-737`).
- A position change is an intervention in the requirements' own words (`docs/srs.md:35`, requirement), and the experiment needs no new input, no packet change and no new reading.
- The response couples a fast loop with a slow one. Posture sets every site's hydrostatic head through the drive frame (`contract/boundary.rs:45-75, 634-665`, implemented). The baroreflex moves norepinephrine and acetylcholine release within seconds, and the heart rate reads their receptor effects at the pacemaker (`crates/ng-cardiovascular/src/rhythm.rs:41-57`, `crates/ng-receptor/src/lib.rs:106-126`, implemented). Filtration into dependent tissue follows over minutes.
- The action has a setting and a timing: the sine of the trunk angle (head-down allowed), three leg settings, and ramps as trains of events (`contract/boundary.rs:45-75`). That leaves room for out-of-policy and closed-loop tests.
- Its readings are rows the slice's lens already reads: heart rate, arterial pressure and filtration (`validate/slice.rs:917-923`, implemented). So they come through an observer, with error from the observation seed.
- Short runs keep history memory to a few gigabytes per run (5.9).
- A published human study gives an outside check of direction (section 6).

Its limits: posture has no substance dose, its balance is mostly internal (basal intake, urine and skin water are its only boundary flows), and it shows no composition dependence. Fluid deliveries add those and come second (5.10). Three alternatives are set aside. The two-reservoir body tests plumbing only, so it stays as the host's smoke test against its closed form. Real-data domains such as glucose bypass the engine and turn the work into forecasting under clinician-chosen actions. Waiting for the healthy-day run delays this question for no gain (5.3).

**5.3 What the slice can teach today.**

It can:
- Run 155 amount rows, 9 memories and 76 algebraic rows on synthetic numbers (`vtests/arena_body.rs:180-184`, synthetic test).
- Take posture and deliveries through `Session::enqueue` (`record/session.rs:141-145`, `contract/boundary.rs:417-444`, implemented).
- Expose every row and mode through the accepted state view (`contract/state.rs:223-264`, implemented, with no public stream-ordinal accessor, which the chance-free slice does not need) and per-column extents through the ledger (implemented).
- Resume a checkpoint with identical bits (`vtests/arena_body.rs:714-737`, synthetic test).
- Read any row through an observer with delay and error (`contract/lens.rs:10-46`, implemented), though no test attaches an observer to the slice.

It cannot:
- Deliver intravenously, give a drug or oxygen, or stop or titrate a running delivery by id. A new payload under a known id is refused (`contract/boundary.rs:428-433`, implemented), and all of these wait for a contract version (`docs/architecture/solver-contract.md:340`, planned).
- Run disease, chance or death.
- Move cell water, which a held stand-in keeps fixed (`validate/slice.rs:387-403`, implemented).
- Offer human-grounded numbers.

Untested, so the teacher audit checks it first:
- The slice with any input on `ng_solver::Core`, the core picked for the slice. Every slice test with an input or a checkpoint runs the reference integrator (`vtests/arena_body.rs:25-27, 44`), while `ng_solver::Core` has run the slice for one simulated second with an empty port (`vtests/trace.rs:40, 132-133`, synthetic test) and in an author bench of 35 minutes with no input (`issues/12-throughput-beside-the-record.md:31`, author-reported).
- Runs beyond the 30 s any test reaches (`vtests/arena_body.rs:753-757`).
- Bit identity of two slice runs with the same event prefix, which the replay gates check only on manufactured bodies (`issues/18-slice-replay-gates.md:5`, planned).

*Traits* stay fixed in the first experiment. A value that only a class or network default binds can be overridden at a site or an edge today, because binding prefers the specific value (`contract/packet.rs:782`, implemented), as with `fl.ion-exchange`, which the beds take from their class default (`validate/slice.rs:806`, implemented). Values the fixture's table binds explicitly cannot be replaced: a second value for the same key and binding is refused (`contract/packet.rs:252-258`, implemented), and the table is a private function (`validate/slice.rs:638`, implemented). Varying those needs a host-built packet or a builder from the engine's process.

*Memory* bounds run length. Each accepted step keeps a history link of 15.6 to 18.5 KB at about 212 steps per simulated second (`issues/11-memory-stays-flat.md:23`, author-reported), about 12 GB per simulated hour. Restoring a checkpoint reads every link back into memory (`record/history.rs:330-356`, implemented), so chunking a run does not help until `issues/11` lands.

Against the healthy-day run (record 0004, I19): it would add reviewed numbers, two-day horizons and day events through the same port. But it excludes interventions by design, it has not run, and at the slice's step density and today's link size it would hold on the order of 600 GB over 48 hours (an estimate). Wait for it for the grounding question, not for this one.

**5.4 The first model, concretely** (an example, not a schema).

- *State*: named totals that posture moves between. Water in arterial blood, in central blood (heart chambers, pulmonary vessels and central veins), in dependent veins (leg and splanchnic veins), in dependent interstitium (the leg bed) and in other interstitium. Water in the tubule and duct, and in the bladder apart, since a fast pool and a slow one with a drain that reads only the slow one cannot share a total. Transmitter totals that carry the reflex: norepinephrine bound at the sinoatrial, myocardial and vascular junctions, and acetylcholine bound at the sinoatrial junction. The closure variant adds a latent part of two to four numbers.
- *Traits*: fixed at the synthetic packet's values.
- *Input*: posture events, each a trunk setting and a leg setting from its instant, with ramps as trains of events.
- *Dynamics*: one learned transfer for each pair of named totals that exchange: blood flow between the groups, filtration and lymph return between blood and interstitium, transmitter release, binding and clearance, renal filtration and reclaim, and urine, basal intake and skin water at the boundary. Each is a small network that reads the totals, the latent part and the current drive frame, and returns a signed rate. Its positive and negative parts, integrated over each cycle, are compared with the ledger's forward and reverse extents for the engine columns the transfer sums.
- *Readings*: heart rate and arterial pressure as cycle means, decoded by learned maps, with error from the observation seed. Filtration and stroke volume are audit readings, and stroke volume is computed from rows as an existing test does (`vtests/arena_body.rs:790`).
- *Latent part, when present*: it starts as a learned function of the named start state, and nothing the teacher shows later enters it. It is trained only through the rollout and reading losses, never against a teacher quantity.

Ions and albumin move with blood water along the same circulation columns. The first model omits them, so its balance covers only the totals it carries. Everything else in the slice reaches the learner only through these transfers. An illustrative objective:

```
L = Σ_k Σ_j ‖P_j(k) − (A e_k)_j‖² + λ Σ_{t≤H} ‖ẑ(t) − z(t)‖²_W + μ Σ_c ‖ŷ_c − y_c‖²
```

`P_j(k)` is the model's transfer `j` integrated over cycle `k` from the model's own state, split into its positive and negative parts. `e_k` is the teacher's vector of per-column forward and reverse extents over cycle `k`, and `A` is the fixed map that sums engine columns into named transfers. `ẑ` is the model's free rollout from the teacher's totals at a window's start, `z` the teacher's totals, `H` the window, and `W` weights each total by its natural size. `ŷ_c` and `y_c` are predicted and teacher readings, and `λ` and `μ` weigh the terms. In words: the learner is told how much moved along each named transfer in each cycle, and it is held on course over windows in which it consumes its own predictions.

Growth comes one gated addition at a time. Each addition must fix a test the previous model failed, not only lower a loss:
1. Fluid deliveries, the second experiment (5.10).
2. Trait draws, once the trait path exists.
3. A trait distribution calibrated to summary evidence, once the planted-discrepancy experiment passes.
4. Readings only, with state and trait inference, scored against the full-state learner on the same tests.
5. Process variation and discrete events, when the slice has its forcing and hazard.
6. More systems and reviewed teachers as the engine adds them, the healthy-day run first.

**5.5 Data and training.** The teacher is the slice on the core that passes the audit: `ng_solver::Core` (`README.md:16`, `crates/ng-solver/src/lib.rs:71-141`, implemented) if it handles posture mid-run and resume, otherwise the reference integrator, costed at its speed. Each run starts at admission at 06:55 (`validate/slice.rs:472-475`, implemented), lives a pre-encounter history of minutes lying, sitting or standing, then runs a 10-minute encounter, a length proposed for review that covers the reflex and the start of the redistribution. A few 30-minute runs measure the slow component. Each run keeps the full state each cycle, the per-column extents, and readings through an observer, which cannot move physiology (I7).

Two training sets of equal size come from the same start histories. *P-rand* has posture events at state-independent random times, with random trunk and leg settings and ramp lengths. *P-pol* has events chosen by a scripted rule that lays the patient flat when arterial pressure reads low and sits them up when it reads high (synthetic thresholds, for review), plus random stands that give the rule something to correct. Five training seeds per arm.

The learner sees the teacher's full state during training. That defers inferring state and traits from readings, which every route to human evidence needs. It also defers the identifiability of a latent learned without named supervision, and informative measurement timing. And it hides a hazard: a full-state learner can lean on quantities no human measurement offers, so its success says nothing about learning from people.

Sufficiency is tested without matching states, since no public interface sets a state (`record/record.rs:26`, implemented). Arm R carries a richer reduction, every amount and memory at one-cycle resolution, including the baroreflex filter and set point, which are memories rather than amounts (`crates/ng-nervous/src/lib.rs:86-87`, implemented). It is trained on the same data and losses. If R beats C4a out of policy, the named totals are insufficient, and the gap shows which quantity to add.

**5.6 Arms and fairness.**

| Arm | What it is | What it isolates |
|---|---|---|
| C0 persistence | holds the last state | a floor for the metrics |
| C1 linear | a linear exchange model on the same totals and drive, with the same losses | whether the tested range needs nonlinearity |
| C2 readings only | a recurrent model with persistent memory over readings and drive, generating noiseless readings internally, with the same measurement model as every arm applied after | named-state access |
| C3 unconstrained | the same inputs, transfer network and losses as C4a, but it predicts each total's change directly instead of deriving it from the transfers | balance by construction |
| C4a named | learned transfers on the named totals | the candidate |
| C4b named and latent | C4a with a small latent part | closure by a latent part |
| C4c named, no ledger | C4a trained only on the rollout and reading losses | the value of ledger supervision |
| R richer reduction | the C4a form on every amount and memory | sufficiency of the named totals |

All arms get the same start histories, schedules, start information, horizons, evaluation, model-size range and tuning budget, and every arm's dynamics read the current drive frame. Each initializer receives only the start state. C2, which has no named state by design, receives the noiseless readings of a rest window before the first event instead. That difference is the one the C2-against-C4 comparison tests, and every result states it. No arm feeds sampled measurement error back into its dynamics. All arms share the teacher runs, and all are scored by CPU rollouts in a fixed order, so results replay exactly.

**5.7 Rollout and actions.** Every evaluation is a free run from the start state over the whole horizon, with no reset to teacher states. The model consumes only its own state and the input schedule. Readings are drawn from its state with the observation seed and never re-enter its dynamics. An encounter action enters as a posture event from its instant, as in the teacher. In the closed loop, the scripted rule drives the teacher and each arm from the same start state. The same rule issues different actions when readings differ, so each arm's resulting event log is also replayed through the teacher, which separates model error from the effect of different actions.

**5.8 Falsifier, and what earns the next step.** The claimed advantage of named state is falsified if, on held-out start histories and out-of-policy paired branches over the full horizon, C2 matches or beats C4 on the readings both produce (heart rate and arterial pressure), within the spread of C4's error across its five training seeds. That rule is proposed for review. Its ground is the comparison's own seed variance, not an invented number. If C1 matches C4, the linear model wins: it is itself a learned generator, so the simpler model is kept and the next experiment needs a harder question. The next step is earned when all of these hold:

- C4a or C4b beats C1, C2 and C3 out of policy and in the closed loop.
- Balance holds exactly, and amounts stay nonnegative throughout.
- The split, refinement and cadence tests pass.
- The across-patient order of responses survives.
- R does not beat the chosen named arm.
- The P-rand learner beats the P-pol learner out of policy on states and actions both sets cover.

Then come fluid deliveries and the planted-discrepancy experiment.

**5.9 Cost, all estimates.** Assumptions: the teaching core runs near 5 times real time, from one author-reported run of 4.91 times over 35 simulated minutes with no input, at a 10 ms cap, on an unnamed machine, before three later speed passes (`issues/12-throughput-beside-the-record.md:31`). The reference integrator is reported at about 1.05 to 1.3 times real time (`issues/12-throughput-beside-the-record.md:15, 31`, author-reported). Posture breakpoints may add steps. History costs about 12 GB per simulated hour (5.3). Each run is a 5-minute history plus a 10-minute encounter, so about 3 GB.

- *Runs*: 256 training runs (two sets of 16 start histories by 8 schedules), 96 held-out runs (8 start histories by 12: 4 out-of-policy schedules, the 7 paired branches of section 6 and 1 closed loop), 64 closed-loop replays (8 arms on 8 histories at one seed) and 8 runs of 30 minutes for the slow component. That is about 420 runs and 110 simulated hours.
- *Time*: about 22 process-hours on the teaching core, or 85 to 105 on the reference integrator. On the teaching core that is about 6 wall hours at four runs at once, with about 12 GB of RAM.
- *Storage*: per-column extents (257 columns by 2 parts) and the cycle-mean state (240 rows) at 8 bytes a value, about 6 KB per cycle, about 5.4 MB per 15-minute run and about 2.3 GB in all.
- *Training*: arms of 10^4 to 10^5 parameters, about 40 trainings (8 arms by 5 seeds) of minutes to an hour each on the RTX 4060's 8 GB, under a day of GPU time. GPU memory does not bind, and teacher runs dominate.

These sizes rest on cost, not on a power calculation, so the audit first checks that responses differ across start histories enough to test diversity. Paired branches are separate runs with identical event prefixes, because a rewind restores the whole history into memory. One audited run on the reference machine, timed and with its peak memory read, confirms or replaces every figure here. The 4060 Ti machine's configuration is unconfirmed.

**5.10 The second experiment: fluid deliveries.** Deliveries through the gut routes are the first dose and composition question. The absorption law adds a delivery's rate to a basal rate with no lag, into splanchnic venous blood (`validate/slice.rs:229-237, 281-297`, implemented), so a gut delivery acts as a venous infusion. Short blocks stand in for stopping and titrating. The loop runs through vasopressin secretion on central-vein sodium and renal water reclaim (`validate/slice.rs:327-370`, `crates/ng-renal/src/lib.rs:37-38, 161-170`, implemented). It adds what posture lacks: a continuous dose and composition family close to fluid therapy, open balance, and patient traits that matter. Interrogation of an earlier draft found what it needs first:

- *Horizon.* Whole-body vasopressin washes out over about 83 minutes (section 3), so a 45-minute run shows only the start of free-water excretion. Either run hours, which waits for `issues/11`, or raise the synthetic clearance in a host-built packet and record the retune.
- *Traits.* The loop's values (filtration conductance, reclaim limits, the vasopressin centre and width) are bound explicitly, so they need a host-built packet. Three of them also set the resting sodium reading, because at rest the duct must reclaim 0.95 of its inflow, by arithmetic on the fixture's intake, skin-water, filtration and reclaim values (`validate/slice.rs:704, 708, 845-848`, implemented), and that fixes the vasopressin effect. A trait that leaves resting readings unchanged hides better.
- *Readings.* No row holds a plasma sodium concentration, a channel reads one row, and the measurement-error draw is crate-private (`contract/lens.rs:11-13`, `contract/chance.rs:78`, implemented). The host computes that reading and its error, so I7 and the seed separation hold by the host's design, not by the engine's types.
- *Transfers.* Basal intake and a delivery share one gut column (`validate/slice.rs:291-295`, implemented), so each gut route is one intake transfer, and its basal part is the extent minus the known delivery's integral.
- *Teacher.* No test enqueues a delivery after the slice has stepped.

Its outside check is the saline-against-dextrose ordering in section 6.

**5.11 What the first experiment cannot establish.** It cannot establish human validity, since the numbers are synthetic. It says nothing about dose, composition, traits, disease, drugs, chance or death, about learning from partial readings, or about population calibration. It does not test whether a learned model is faster at equal fidelity, and it leaves waveforms out. It is smaller than emulating the whole slice (about a dozen totals against 240 rows), more informative than a two-reservoir test (a reflex loop, slow redistribution and out-of-policy structure), and more decisive than a forecasting benchmark on real data, because every counterfactual has an exact teacher reference and the action policy is under control.

### 6. Independent evidence of progress

Levels, each with its first-experiment test:

1. Fit: per-cycle transfer and one-cycle errors on held-out start histories. Necessary and weak.
2. Free run: rest stability against the teacher's own drift, balance, nonnegative amounts and ranges, with no resets.
3. Intervention response within the teacher's assumptions: paired branches and out-of-policy schedules.
4. Closed loop: the scripted rule on the teacher and on each arm, comparing trajectories and the actions issued.
5. Independent evidence: the published relationships below. Clinical validity remains a further claim (`docs/srs.md:185`, requirement).

| Test | Reveals | Reference | Why the reference holds |
|---|---|---|---|
| Rest, no input change | drift, impossible states | teacher | same patient and events |
| Paired branches from one start state: none, a stand, a sit, a stand delayed, a stand split into a ramp, a head-down setting, a setting held out of training | causal response, nonlinearity, temporal consistency | teacher branches | the same packet, seed and event prefix give identical bits (rule 4), so branches differ only in later events; a counterfactual of the teacher, not of a person |
| Out-of-policy schedules: flat when pressure reads high, sit when it reads low, faster ramps, back-to-back changes | what policy-coupled training misses | teacher | state-independent by design |
| Closed loop, with each arm's event log replayed through the teacher | compounding error under feedback | teacher in the same loop | same rule and start state |
| Split, refinement, cadence and observation-seed changes | temporal inconsistency; noise entering physiology | the model's own tightly integrated flow, and the teacher | split, cadence and seed changes should leave physiology unchanged, and refinement should change it only by integration error that falls at the scheme's order |
| Balance, nonnegative amounts, ranges | impossible trajectories | arithmetic | exact by construction, or diagnostic |
| Across-patient order and spread of response features | lost diversity | paired teacher runs | same start histories |
| Seed spread against actual error, and the support signal against large errors | honest uncertainty | teacher | held-out start histories and branches |
| The two-reservoir smoke test | a broken host or rollout | the closed form (`validate/fixture.rs:253-296`, implemented) | mathematics, not an engine run |

Uncertainty is judged honestly only where a reference exists, as in the second-to-last row. Nothing here judges uncertainty about human responses.

**Splits and leakage.** Hold out start histories, schedule families, settings and horizons longer than the training windows. Never split random time points from correlated runs. Keep branches from one parent in one partition. Label each test interpolation or extrapolation, and never reuse evaluation seeds in training. Levels 1 to 4 are judged by the teacher that trained the model, and every report says so.

**Policy-coupled against randomized training.** Train C4 on P-pol and on P-rand, and score both on the out-of-policy set. Then compare them on the states and actions both sets cover. A P-pol learner that fails only off that shared support lacks support. One that also fails on it is biased, which means the named state does not determine what the rule read.

**The outside check for the first experiment.** A study of passive head-up tilt to 30° and tilt back to supine in seven healthy people recorded beat-to-beat stroke volume, heart rate, mean arterial pressure, cardiac output and peripheral conductance, and reports an asymmetry between the two directions (*Dynamic time course of hemodynamic responses after passive head-up tilt and tilt back to supine position*, Journal of Applied Physiology, 2002). Its abstract reports a delayed fall in stroke volume after tilt-up, a faster recovery after tilt-back, and heart-rate changes closer to symmetric. The posture payload can express the static tilted configuration (trunk sine 0.5 with the half-share leg setting, `contract/boundary.rs:45-75`, implemented) but not a finite-speed tilt, which becomes a ramp of events. So only the direction of each asymmetry is a target, never its size or timing, and only the abstract was read.

| Teacher shows the asymmetry | Learner shows it | Reading |
|---|---|---|
| yes | yes | consistent with the source |
| yes | no | the learner missed the response history |
| no | no | an engine gap, or a protocol the slice cannot express; a question for the engine's process once the protocol is ruled out |
| no | yes | suspicious: inductive bias or leakage, to investigate |

**The outside check for the second experiment.** In a double-blind crossover study of ten healthy men, two litres of 0.9 percent saline or of 5 percent dextrose were infused over one hour. About 1,663 mL of urine followed dextrose and about 563 mL followed saline. All the water from dextrose was excreted by two hours after the infusion ended, while only about a third of the saline's sodium and water had been excreted by six hours (*Dilution and redistribution effects of rapid 2-litre infusions...*, Clinical Science, 2001). The context differs from the slice in route, solute, size, population and time, and the slice keeps cell water fixed. So only the ordering of cumulative excretion is a target, and only if the experiment's horizon covers the separation. A miss inside a short horizon may be scope, not an engine gap.

**Population comparisons, later.** A published outcome frequency is a target only when its population, exposure and duration, illness, co-treatments and outcome definition match or are stated. A miss calls for investigation. It does not by itself show which mechanism, in the learned model or in the engine, is wrong.

**The test that answers the central question.** A model can reproduce two kinds of assumption from its source. The sampling assumptions (policy, start states, horizon, step) are exposed by the out-of-policy, paired-branch and temporal tests above. The teacher's mechanism errors are exposed only by a signal the teacher lacks, which a planted-discrepancy experiment supplies after the first experiment passes:

1. Alter one non-anchored parameter that the packet can override, later one law form, in the slice used as teacher.
2. Give the learner dense runs from the altered slice plus a few summary statistics from the unaltered slice under one protocol, such as the across-history mean and spread of the heart-rate rise a fixed time after a stand.
3. On other protocols, score whether the learner's responses move toward the unaltered slice rather than its teacher, against the same learner trained without the summary term.

Success shows that the route can learn past its teacher from evidence of the kind the literature offers. Failure implicates this representation, summary objective or choice of evidence. It does not show that grounding can enter only through the engine. Its limits: both slices share the engine's structure, the summaries are synthetic, and a law swap needs a variant composition.

**Face validity.** Clinicians review selected trajectories, teacher and learned mixed and unlabelled, for shape, order of events and causal plausibility. With synthetic numbers they judge shape, not magnitude. Whether they can tell learned from teacher trajectories apart is a result about fidelity to the teacher, not about physiology. It is reported as face-validity evidence.

**What a failure means.** The concept is undermined if, with full state, randomized inputs, exact balance and a smooth synthetic loop, no learned arm stays coherent over the horizon in free run and closed loop, and neither rollout training nor capacity fixes it. A choice is implicated if C2 fails while C4 holds (generation from readings), P-pol fails on shared support (training data), C4a fails while C4b or R holds (too small a state), C4c fails while C4a holds (no ledger supervision), or C3 breaks balance or drifts (no structure). A boundary is marked if errors grow only beyond trained settings (extrapolation), or if the teacher misses an outside check (an engine gap or an inexpressible protocol). A linear win is a result, not a boundary: it keeps the simpler generator.

### 7. Architecture handoff: Phase A input

**7.1 Grounding the architecture pass can reuse.** The repository facts above hold at `b8990a5`, each with its label. The pass reruns `how` at its own commit over the same five areas before relying on them:

- the accepted state and pools (`contract/state.rs`, `contract/body.rs`)
- the step and what a solver core may propose (`record/session.rs`, `contract/quadrature.rs`)
- the ledger (`record/history.rs`)
- the boundary port and `Session::enqueue` (`contract/boundary.rs`, `record/record.rs`)
- the slice (`validate/slice.rs`, `vtests/arena_body.rs`)

The facts that most shape the design are these. Posture and deliveries enter a running session through public interfaces. Full rows, modes and per-column extents are readable. Checkpoints resume with identical bits. No public interface sets a state. Explicitly bound traits cannot vary without a host-built packet or an engine-side builder. History memory bounds run length. And the core picked for the slice has not yet run it with an input.

**7.2 Open decisions and the evidence each must return.**

| Decision | Evidence to return |
|---|---|
| Is the slice a usable teacher for posture runs, and on which core? | The teacher audit on the reference machine: posture mid-run after stepping and across a checkpoint on `ng_solver::Core`; one 15-minute and one 30-minute run with wall time, accepted steps and peak memory; bit identity of two runs with the same event prefix; response differences across start histories; the slow component's time constant; and the static tilt configuration's response. |
| Where does the data host live? | A program outside the workspace members, as `tools/bench` already is (`issues/12-throughput-beside-the-record.md:19`, author-reported), driving runs through public interfaces, with confirmation that no engine change is needed. |
| Are the named totals a sufficient state? | The reduction map (pools into totals, columns into transfers, cycle-mean extraction), a check that totals and transfers balance exactly on teacher data, and the R-against-C4a result. |
| What is frozen before tuning? | The evaluation harness: tests, metrics, seeds, splits and the outside-check protocol, committed before any arm is tuned. |
| What does the second experiment need? | A host-built synthetic packet or an engine builder; a decision between hour-scale runs and a recorded clearance retune; the host-computed sodium reading; and an audit of deliveries enqueued after stepping. |

**7.3 Recommended next decision.** Adopt the action-conditioned generative state-space class, with named totals and learned transfers supervised by the ledger, for the first build. Run the slice posture experiment first, gated by the teacher audit, then fluid deliveries, then the planted-discrepancy experiment. Posture is the smallest arrangement that tests whether a learned model can carry a patient through inputs it was not trained on. It keeps the physiology checkable and uses the engine as teacher, comparison and counterfactual source without changing it. Four uncertainties remain. The posture response may be too fast or too linear to separate the arms. The teaching core may not handle posture mid-run. Memory bounds horizons. And success on synthetic numbers may not carry over to reviewed teachers.

**7.4 Questions for the engine's process.** May a host outside the workspace drive the slice and read its ledger for this use? Does trait variation come from a host copy of the synthetic table or from an engine builder? Do hour-scale horizons wait for `issues/11`? Not blocking now: what provenance fitted law weights would need under rule 12 if a learned law ever entered the engine.

**7.5 Bridge to a later SRS.** Formal requirements become useful once these exist:

- a list of behaviours, each labelled validated, approximate or untested at a stated level of section 6
- a chosen runtime relationship and its determinism contract
- constraints with tolerances grounded in measured teacher variation (balance, nonnegative amounts, split, refinement and cadence invariance, rest drift and support-signal behaviour)
- the action and timescale envelope the teaching use sets
- an evidence trail that ties every target to a source, never to a model's prediction

### Where this brief departs from the assignment

- **No new intervention input is needed for the first two experiments.** Posture and gut-route deliveries already enter a running session, and short delivery blocks stand in for stopping and titrating.
- **Intervention-rich public data exists, but not interactive or unconfounded data.** High-resolution intraoperative records with drug-infusion rates are public (*VitalDB, a high-fidelity multi-parameter vital signs database in surgical patients*, Scientific Data, 2022, 6,388 cases). They are observational and policy-coupled, so they can later serve as independent checks of specific responses, not as bulk training.
- **The central question is two questions,** and reproducing the teacher where the teacher is right is useful, not a failure. Only the second question, copying the teacher's mechanism errors, needs a signal from outside the teacher.
- **A clinician's overall impression is not a reading.** It is the observer's synthesis of cues, and the model generates the cues.
- **The engine is a ceiling only for a learner with no outside signal.** Whether a learner with one can move past it is an empirical question, which the planted-discrepancy experiment answers for one representation at a time.
- **Inside the engine, determinism is necessary but not sufficient.** A learned law must also fit the compiled kernel representation, and its weights need a provenance decision under rule 12.

### Sources

- *Med-Real2Sim: Non-Invasive Medical Digital Twins using Physics-Informed Self-Supervised Learning*, NeurIPS 2024, https://arxiv.org/abs/2403.00177. Parameter inference for a patient-specific model: a network pretrained on 3,840 simulated samples emulates a left-ventricular circuit model, and a video encoder trained through the frozen emulator infers parameters that reproduce two labelled volumes per echocardiogram. Ejection-fraction error is close to a supervised network's, and inferred pressures and resistances are not checked against measurement. Not an interactive generator.
- *Multiple Physics Pretraining for Spatiotemporal Surrogate Models*, https://arxiv.org/abs/2310.02994. Emulation of two-dimensional simulated fields with one-step training and one-step and five-step scoring. Transfer to held-out compressible flow helps most with little data.
- *Learning Insulin-Glucose Dynamics in the Wild*, Machine Learning for Healthcare 2020, PMLR volume 126, https://arxiv.org/abs/2008.02852. Forecasting with a named-state hybrid on observational data from two participants modelled separately, with accuracy reported for one. Counterfactual inputs are checked for direction only, since no counterfactual reference exists.
- *Data-Driven Simulator for Mechanical Circulatory Support with Domain Adversarial Neural Process*, arXiv preprint 2024, https://arxiv.org/abs/2405.18536. Offline intervention-conditioned forecasting, with no closed-loop or counterfactual test.
- *Universal Differential Equations for Scientific Machine Learning*, arXiv preprint, https://arxiv.org/abs/2001.04385.
- *Dynamic time course of hemodynamic responses after passive head-up tilt and tilt back to supine position*, Journal of Applied Physiology 92:1671-1676, 2002, https://doi.org/10.1152/japplphysiol.00465.2000 (abstract only).
- *Dilution and redistribution effects of rapid 2-litre infusions of 0.9% (w/v) saline and 5% (w/v) dextrose on haematological parameters and serum biochemistry in normal subjects: a double-blind crossover study*, Clinical Science 101:173-179, 2001, https://doi.org/10.1042/cs1010173 (abstract only).
- *VitalDB, a high-fidelity multi-parameter vital signs database in surgical patients*, Scientific Data 9:279, 2022, https://doi.org/10.1038/s41597-022-01411-5.
