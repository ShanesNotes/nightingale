# 27: A step plan that can reach the speed aim

Type: prototype
Status: open
Blocked by: 07

**What to build:** A way of stepping the body that can reach 100 times real time on all 13 systems. [Ticket 07](07-what-limits-the-step.md) says what limits the step today. On its figures no faster core closes the gap alone: the record's own work caps the slice near 21 times real time, and every row the body adds pays for the heart's step count.

- [ ] Two or three plans prototyped and measured on the slice, each still passing the replay gates and the accuracy gate. Candidates: step control that survives a firing, a split where the heart's rows step apart from the slow rows (the split gate of native-contract 20 exists and no core uses it), and a cheaper record that keeps the same guarantees
- [ ] The same plans measured on [ticket 06](06-whole-body-on-holds.md)'s body once it exists
- [ ] A pick with its measured gain and what it costs, written as a decision record if it changes the contract. Resolution is part of a run's identity (foundation map), so a plan that changes results is a different run and says so
- [ ] Every number carries its machine, its run count and its spread (pstack `benchmark-checklist`)

**Non-goals:** the arena's pick between cores, and any gain bought by dropping a law.

This runs as pstack's Hillclimb playbook: one metric, one hypothesis at a time, a decision log, and one commit for each accepted gain.
