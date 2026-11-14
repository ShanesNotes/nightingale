# 15: Nothing in the gate sees the step path get slower

Type: task
Status: open
Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours, and how long that run takes rests on what a step costs. A change can make every step slower without changing a bit, and today no test and no gate step would see it. The aim of 100 times real time ([SRS section 6](../../../docs/srs.md#6-fidelity-aims)) is far off already, so a slower step path that lands unseen widens the gap.

**What happened.** The plan for part 2 of [native-contract 42](../../native-contract/issues/42-veins-collapse-branch.md) (`~/lanes/logs/ng-wave-4/nc42-pr2/PLAN.md`, the paragraph on its void first pass and its flag on ruling 3) added a helper on the refusal path, off the step path. In its first form the helper called two evaluator functions directly, so the compiler stopped inlining them into the step's own evaluation. A build with no vein marked, on main's model, ran 12.6% more instructions a step. No test and no gate step noticed. A profile found it during the plan's own cost measurement. The plan flags that a check should pin the no-vein build's instructions a step within a band of main's.

The boxes ask what such a check could read and where it would run. They do not design it.

- [ ] What the check reads: user instructions a step of the slice, or another count. The Answer says how steady that count is from run to run and from load to load on one worker
- [ ] On which named build worker it runs, and whether a reading on one worker means anything on another
- [ ] What band around main's figure it holds, and what a change does when it moves the figure on purpose
- [ ] Where it runs: the merge proof, the nightly run or a bench a PR runs by hand, with the time each would add
- [ ] Whether the void first pass's form, put back, would trip the check that the Answer proposes

**Non-goals:** building the check. A cost budget for the whole body, which is [body 26](../../body/issues/26-cost-budget.md).
