# Generative physiology wayfinder map

## Destination

An architecture pass can start the generative physiology model from a reviewed research and concept brief. The brief says what the hidden patient state means, how its dynamics are learned, which small first experiment is most informative, and which test would show that the model learned a physiological response rather than the assumptions of whatever trained it.

## Notes

- The generative physiology model is a learned, action-conditioned model of a patient's physiology. It is a separate line of work from the engine. Nothing in this effort changes [the SRS](../../docs/srs.md), the solver contract or the engine's build order. A proposal that needs an engine change goes to the engine's own process as a question.
- SRS scope stage 5, "Learning", means replay and debrief. It is not machine learning, and the SRS neither allows nor limits machine learning.
- A model's output is never a source or a target ([SRS rule 16](../../docs/srs.md#3-hard-rules), [record 0003](../../docs/adr/0003-inferred-parameters.md)). That holds for a learned model too: its outputs are never the reference they are judged against.
- Briefs, candidate drafts and review rounds stay out of the repo ([governance section 2](../../docs/governance.md#2-what-belongs-in-the-repo)). A ticket's Answer holds the result.

## Decisions so far

None yet.

## Not yet specified

- **The architecture pass's tickets.** They follow from [the concept brief](issues/01-concept-brief.md) once it resolves.

## Out of scope

- Code, training runs and new dependencies in this effort's first ticket.
- Teaching-product features such as grading, scenario authoring, learner feedback and display. They are not parts of the model.
- Research-grade or clinical decision-support claims.
