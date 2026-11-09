# 07: Cost budget and the step plan

Type: prototype
Status: open
Blocked by: none

**What to build:** A measured account of where a step's cost goes and what limits its length, a budget each later ticket is held to, and a step plan that can reach the speed aim on the whole body. The aim is 100 times real time with full waveforms (SRS section 6), which is 10 ms of wall time for each simulated second.

Where it stands on the slice, from [native-contract 25](../../native-contract/issues/25-history-on-disk.md)'s run on the heavy-run machine (one repeat): 212 accepted steps a simulated second at 0.69 ms each. That is 145 ms for each simulated second, 14.5 times over. Three facts shape the work:

- The record's own share of a step (its replay of the stages, the hash and the ledger) is about 37% in the one profile on record. At today's step count that share alone allows about 21 times real time, whatever the core costs.
- The mean step is 4.7 ms under a 10 ms cap, about 156 steps a beat. Four in five refusals are accuracy refusals on the two valve flow rows of the left heart, and each firing restarts the core's step control. What limits the step is not established, and no histogram of accepted steps exists.
- One coupled step covers the whole body, so every new row pays for the heart's step count. A body with three times the rows at today's cost would be about 43 times over.

- [ ] A histogram of accepted steps over a beat on the slice, and the limiter named from a profile or from counters, not from reading the code: the cap, one row's accuracy, or the restart after a firing
- [ ] The cost of a step by part (evaluate, linearize, factor, the record's replay, the hash, the ledger) on the slice and on [ticket 06](06-whole-body-on-holds.md)'s body, with how each part grows with rows
- [ ] A budget, in wall milliseconds for each simulated second, that each loop ticket must stay inside, stated with the run that set it
- [ ] Two or three step plans prototyped and measured on the slice, each still passing the replay gates and the accuracy gate. Candidates: step control that survives a firing, a split where the heart's rows step apart from the slow rows (the split gate of native-contract 20 exists and no core uses it), and a cheaper record that keeps the same guarantees
- [ ] A pick with its measured gain and what it costs, written as a decision record if it changes the contract. Resolution is part of a run's identity (foundation map), so a plan that changes results is a different run and says so
- [ ] Every number carries its machine, its run count and its spread (pstack `benchmark-checklist`)

**Non-goals:** the arena's pick between cores, and any gain bought by dropping a law.

The first two boxes can start on the slice today. This runs as pstack's Hillclimb playbook: one metric, one hypothesis at a time, a decision log, and one commit for each accepted gain. Evidence so far: `~/lanes/logs/ng-body-plan/body-cost.md`.
