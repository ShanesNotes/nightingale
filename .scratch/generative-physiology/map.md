# Generative physiology wayfinder map

## Destination

An architecture pass can start the generative physiology model from a reviewed research and concept brief. The brief says what the hidden patient state means, how its dynamics are learned, which small first experiment is most informative, and which test would show that the model learned a physiological response rather than the assumptions of whatever trained it.

## Notes

- The generative physiology model is a learned, action-conditioned model of a patient's physiology. It is a separate line of work from the engine. Nothing in this effort changes [the SRS](../../docs/srs.md), the solver contract or the engine's build order. A proposal that needs an engine change goes to the engine's own process as a question.
- SRS scope stage 5, "Learning", means replay and debrief. It is not machine learning, and the SRS neither allows nor limits machine learning.
- A model's output is never a source or a target ([SRS rule 16](../../docs/srs.md#3-hard-rules), [record 0003](../../docs/adr/0003-inferred-parameters.md)). That holds for a learned model too: its outputs are never the reference they are judged against.
- Briefs, candidate drafts and review rounds stay out of the repo ([governance section 2](../../docs/governance.md#2-what-belongs-in-the-repo)). A ticket's Answer holds the result.
- **The line is parked until the whole body lives two synthetic days** ([body 28](../body/issues/28-two-synthetic-days.md)). [Ticket 14](issues/14-first-experiment-on-the-whole-body.md) waits on body 28 and chooses the first experiment on the whole body, and tickets 05 to 12 wait on ticket 14. Two tickets stand outside the parking. [Ticket 04](issues/04-teacher-audit.md)'s audit runs again on the slice once [native-contract 33](../native-contract/issues/33-leg-veins-empty-head-down.md) and the arena's pick land, because its first run found a defect that the engine's own gates missed. [Ticket 15](issues/15-learned-emulator-for-parameters.md) is a question for the engine's process, and it waits only on what it reads.
- **What a learned model may decide.** The engine computes one course from a start, its events and its seeds ([SRS rule 4](../../docs/srs.md#3-hard-rules)). Its ledger admits only courses in which every amount moves by a transfer or a reaction (I10). A generative physiology model carries its own state under the same rule and predicts the course the engine computes. It never picks a course for the body, because the body computes every number from its physics ([SRS section 1](../../docs/srs.md#1-purpose-and-scope)). Variety comes only from the sources the engine already has: a patient drawn from a population, seeded forcing, hazards at cited odds, and events. A learned model may choose only on the input side, deciding what is done to the body from observations and sending it as events through the boundary port, as ticket 06's scripted rule does. That is another kind of model, and no ticket plans one.

## Decisions so far

Made before the architecture pass, and recorded in [ticket 02](issues/02-first-build-architecture.md):

- **Three models first.** The first experiment starts with C1 linear, C2 readings-only and C4a named. Another is added only if their results are too close to call, under a rule fixed before any tuning ([ticket 11](issues/11-validation-call-and-fourth-model.md)).
- **The engine teaches unchanged.** The driver lives outside the workspace members, as `tools/bench` does, and uses only public interfaces ([ticket 03](issues/03-teacher-driver.md)).
- **The harness is frozen before tuning.** FROZEN records it, and checks hold it ([ticket 09](issues/09-frozen-study-and-freeze.md)).
- **Heavy runs only on the heavy-run machine,** the teacher audit included ([ticket 04](issues/04-teacher-audit.md)).
- **No grading, lessons or product features.**
- **Nothing personal enters the repo.**

Model defaults from [the concept brief](issues/01-concept-brief.md), open to review:

- **Model class.** The generative physiology model is an action-conditioned generative state-space model at every scale: hidden state, learned dynamics that inputs condition, and observations read from state. Alternative: a readings-only sequence model, kept as the main comparison arm.
- **First state and dynamics.** The first instance carries named totals of conserved amounts and slow memories, evolved by learned transfers under exact two-ended balance and supervised by the teacher's ledger. Ticket 02 refines it: the totals are read at beat edges, the transfers are derived from the engine's columns, the target is each transfer's net extent per beat, and the richer state R replaces a latent part as the only conditional addition. Alternative: a partly named state with latent memory from the start.
- **Teacher.** The engine teaches offline through public interfaces only: full state, per-column extents, and randomized as well as policy-coupled input sets. Nothing in the engine changes.
- **First experiments.** The slice's posture response came first, and tickets 04 to 12 plan it. With the line parked until the whole body lives two synthetic days, the first experiment moves to the whole body: a meal's response over the hours after it ([ticket 14](issues/14-first-experiment-on-the-whole-body.md)). Alternative: posture on the whole body, which is cheaper and nearly built but tests little that the slice cannot. Fluid deliveries stay a deferred line ([ticket 13](issues/13-fluids-line-deferred.md)).
- **The central question splits in two.** Out-of-policy paired branches test whether a learner copied its data's sampling assumptions. Only a signal the teacher lacks tests whether it copied the teacher's mechanism errors: a planted-discrepancy experiment and published human checks, used for direction or ordering only.
- **Evaluation discipline.** Readings never feed the dynamics, every evaluation is a free run with no reset to teacher states, the harness is frozen before any arm is tuned, and the teacher is never the only judge.
- **Runtime relationship.** It stays open. A learned law inside the engine would need a provenance decision for its weights under SRS rule 12, which is a question for the engine's process.

Model defaults from [the architecture pass](issues/02-first-build-architecture.md), open to review. [Ticket 14](issues/14-first-experiment-on-the-whole-body.md) says which of them carry over to the whole body:

- **Three modules.** A Rust driver outside the workspace owns the engine's lifecycle and the capture; a Python study owns schedules, the scripted rule, metrics and the freeze; Python models own learned evolution. Alternative: one Rust tool that also judges, which needs a second forward pass for every model.
- **Beat edges and beat readings.** State and ledger are read at the accepted step ends where the rhythm's phase resets, and readings are beat readings; fixed windows stay a diagnostic. Alternative: one-second windows, which alias the beat once the heart rate moves.
- **Sixteen named totals** with derived transfers, an exact integer check of the grouping, and net extents per beat as the target ([ticket 05](issues/05-reduction-and-balance.md)). Alternative: the brief's dozen with bound transmitter totals, which do not close.
- **Paired branches are fresh runs of one prefix.** Checkpoint, resume and rewind stay audit cases. Alternative: branches by rewind, which copy every history link.
- **The fourth model is R,** called on validation cases by the seed spread and a bootstrap over start histories, so the held-out set is read once ([ticket 12](issues/12-heldout-evaluation-and-decision.md)). Alternative: C4b, whose latent part starts from the named totals and so cannot recover what they discard.
- **A sufficiency screen before the freeze,** a diagnostic and never a model ([ticket 07](issues/07-sufficiency-screen.md)).
- **The audit gates everything.** Core must pass posture mid-run, a checkpoint, a rewind and a closed loop, and the start histories must differ beyond their noise; the reference integrator is never an automatic fallback. Its first run failed: Core and the reference integrator both fault 30 s into head-down ([native-contract 33](../native-contract/issues/33-leg-veins-empty-head-down.md)). It runs again on the slice once native-contract 33 and the arena's pick land, and the whole body gets its own audit before its first teacher run ([ticket 04](issues/04-teacher-audit.md), [ticket 14](issues/14-first-experiment-on-the-whole-body.md)).

Settled by the driver's build ([ticket 03](issues/03-teacher-driver.md)):

- **The teacher driver runs.** `gp-driver` turns one canonical run spec into one run directory, written whole or not at all, on the unchanged engine. On the heavy-run machine its smoke reproduces the pinned Core seal, and every audit variant and the closed loop give a plain run's seal log. A 20 s run admits in 7 s, steps in 3 s and peaks at 16 MB resident (measured, one run).

## Not yet specified

- **The fluid-delivery experiment.** A deferred line ([ticket 13](issues/13-fluids-line-deferred.md)); one of its prerequisites is a question for the engine's process: where varied packet values come from. Hour-scale runs no longer wait on the engine, because a run's memory stays flat since native-contract ticket 11.
- **The planted-discrepancy experiment,** which follows a first experiment that earns the next step.
- **The runtime relationship,** and the provenance fitted weights would need under SRS rule 12.
- **A learned emulator to fit the body's parameters,** a question for the engine's process ([ticket 15](issues/15-learned-emulator-for-parameters.md)).
- **The verify path rule for research tooling,** a question for the developer reviewer ([ticket 03](issues/03-teacher-driver.md)).
- **Later model families,** trait draws, process variation, disease, death and qualitative observations.

## Out of scope

- Code, training runs and new dependencies in this effort's first ticket.
- Teaching-product features such as grading, scenario authoring, learner feedback and display. They are not parts of the model.
- Research-grade or clinical decision-support claims.
