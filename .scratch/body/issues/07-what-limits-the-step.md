# 07: What limits the step

Type: task
Status: claimed
Claimed by: body-07-step-limit
Blocked by: none

**What to build:** A measured account, on the slice, of where a step's cost goes and what limits its length. The aim is 100 times real time with full waveforms (SRS section 6), which is 10 ms of wall time for each simulated second.

Where it stands, from [native-contract 25](../../native-contract/issues/25-history-on-disk.md)'s run on the heavy-run machine (one repeat): 212 accepted steps a simulated second at 0.69 ms each. That is 145 ms for each simulated second, 14.5 times over. Three facts shape the work:

- The record's own share of a step (its replay of the stages, the hash and the ledger) is about 37% in the one profile on record. At today's step count that share alone allows about 21 times real time, whatever the core costs.
- The mean step is 4.7 ms under a 10 ms cap, about 156 steps a beat. Four in five refusals are accuracy refusals on two rows that an earlier dump of the slice names as the left heart's valve flows, and each firing restarts the core's step control. What limits the step is not established, and no histogram of accepted steps exists.
- One coupled step covers the whole body, so every new row pays for the heart's step count. A body with three times the rows at today's cost would be about 43 times over.

- [ ] A histogram of accepted steps over a beat, and the limiter named from a profile or from counters, not from reading the code: the cap, one row's accuracy, or the restart after a firing
- [ ] The cost of a step by part (evaluate, linearize, factor, the record's replay, the hash, the ledger), with how each part grows with rows
- [ ] The cost of the full-waveform observer, which no run has measured
- [ ] Every number carries its machine, its run count and its spread (pstack `benchmark-checklist`)

**Non-goals:** any change to a core or to the record ([ticket 27](27-step-plan.md)), and the budget ([ticket 26](26-cost-budget.md)).

Evidence so far: `~/lanes/logs/ng-body-plan/body-cost.md`.
