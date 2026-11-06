# 01: Research and system concept brief

Type: research
Status: open
Blocked by: none

**Question:** Can a learned generative model itself carry a simulated patient's physiology through an encounter, responding to assessment and intervention? What is the smallest experiment that tests this, and what evidence would tell a model that learned a physiological response apart from one that copies its teacher?

The Answer is a research and system concept brief that an architecture pass can build on. It recommends and labels; it does not specify interfaces, crates or a state schema.

- [ ] Intended system and boundaries: training, the start state, the encounter at run time, assessment as a query that returns an observation, intervention as an input that changes the body, and what must persist so the patient stays the same patient
- [ ] The learning problem: what the hidden state means (named, learned or partly named), how its dynamics are produced, and how each knowledge source becomes a training signal, a constraint or a test
- [ ] Time and physiological consistency: elapsed time, delays, observation cadence against the underlying process, and which balance and range checks are hard guarantees, penalties or diagnostics
- [ ] What is established, what is a hypothesis and what is still open
- [ ] The smallest informative experiment: question, physiology, teacher, inputs, outputs, rollout, source of variation, fair baselines and one result that would falsify the claimed advantage
- [ ] Levels of evidence, including one check whose reference comes from neither the engine nor the training source
- [ ] The next decision for the architecture pass, and what must be validated before formal requirements are useful

Every repository claim cites a file and the commit read. Every resource or dataset figure is labelled an estimate until measured.

## Answer
