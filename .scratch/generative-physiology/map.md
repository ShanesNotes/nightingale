# Generative physiology wayfinder map

## Destination

An architecture pass can start the generative physiology model from a reviewed research and concept brief. The brief says what the hidden patient state means, how its dynamics are learned, which small first experiment is most informative, and which test would show that the model learned a physiological response rather than the assumptions of whatever trained it.

## Notes

- The generative physiology model is a learned, action-conditioned model of a patient's physiology. It is a separate line of work from the engine. Nothing in this effort changes [the SRS](../../docs/srs.md), the solver contract or the engine's build order. A proposal that needs an engine change goes to the engine's own process as a question.
- SRS scope stage 5, "Learning", means replay and debrief. It is not machine learning, and the SRS neither allows nor limits machine learning.
- A model's output is never a source or a target ([SRS rule 16](../../docs/srs.md#3-hard-rules), [record 0003](../../docs/adr/0003-inferred-parameters.md)). That holds for a learned model too: its outputs are never the reference they are judged against.
- Briefs, candidate drafts and review rounds stay out of the repo ([governance section 2](../../docs/governance.md#2-what-belongs-in-the-repo)). A ticket's Answer holds the result.

## Decisions so far

Each is a model default from [the concept brief](issues/01-concept-brief.md), open to review.

- **Model class.** The generative physiology model is an action-conditioned generative state-space model at every scale: hidden state, learned dynamics that inputs condition, and observations read from state. Alternative: a readings-only sequence model, kept as the main comparison arm.
- **First state and dynamics.** The first instance carries named totals of conserved amounts and slow memories, evolved by learned transfers under exact two-ended balance and supervised by the teacher's ledger extents. A latent part is added only if a richer reduction beats the named totals. Alternative: a partly named state with latent memory from the start.
- **Teacher.** The engine teaches offline through public interfaces only: full state, per-column extents, and randomized as well as policy-coupled input sets. Nothing in the engine changes.
- **First experiments.** The slice's posture response comes first, gated by a teacher audit on the core that will teach. Fluid deliveries through the gut routes come second, once a host-built synthetic packet and hour-scale runs exist. Alternative: fluids first, with a retuned synthetic clearance.
- **The central question splits in two.** Out-of-policy paired branches test whether a learner copied its data's sampling assumptions. Only a signal the teacher lacks tests whether it copied the teacher's mechanism errors: a planted-discrepancy experiment and published human checks, used for direction or ordering only.
- **Evaluation discipline.** Readings never feed the dynamics, every evaluation is a free run with no reset to teacher states, the harness is frozen before any arm is tuned, and the teacher is never the only judge.
- **Runtime relationship.** It stays open. A learned law inside the engine would need a provenance decision for its weights under SRS rule 12, which is a question for the engine's process.

## Not yet specified

- **The architecture pass's tickets.** They follow from section 7 of [the concept brief](issues/01-concept-brief.md): the teacher audit, the data host, the reduction map and sufficiency test, the frozen evaluation harness, and the second experiment's prerequisites.

## Out of scope

- Code, training runs and new dependencies in this effort's first ticket.
- Teaching-product features such as grading, scenario authoring, learner feedback and display. They are not parts of the model.
- Research-grade or clinical decision-support claims.
