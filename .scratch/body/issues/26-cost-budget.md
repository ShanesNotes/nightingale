# 26: A cost budget for the whole body

Type: task
Status: resolved
Claimed by: b26-measure
Blocked by: 06, 07

**What to build:** The budget each loop ticket is held to. Measure [ticket 06](06-whole-body-on-holds.md)'s body against the slice on one named build worker, and from [ticket 07](07-what-limits-the-step.md)'s limiter set how much wall time for a simulated second each loop may add, and a ceiling for the whole body after pass 1.

- [ ] The body on holds measured against the slice under one repeat protocol: unknowns, rows, the cost of a step by part, and wall time for each simulated second. Met but for the cost of a step by part: eight jobs on the beelink, below. Its split by part was not profiled on the body; [ticket 07](07-what-limits-the-step.md) has it for the slice, and [ticket 30](30-step-plans-on-the-whole-body.md) measures the body's step
- [x] Admission's wall time and the wall time of a full `tools/verify` run, measured the same way, each with a ceiling. A run pays admission once, however short it is, and the gate's time limits how many tickets merge in a day. Admission from the cost line's runs, the gate from the beelink's proof records and this ticket's proof, both below
- [x] From [ticket 06](06-whole-body-on-holds.md)'s plan: a ceiling on the time the body's replay gates add to a full `tools/verify` run. Each replay gate runs a core several times over its run, so the body's gates cost in proportion to the body's step, which ticket 06's last part measures
  - Inputs from ticket 06 part 4c, each wall seconds on the beelink in one `builders/job`. One body admission solve: 188 to 229 s, five samples, alone or beside one other job, load 2 to 9 on 16 threads. The seven gates on the body, solve excluded: 78.2 and 85.8 s, two samples, of which the four starts (one check and recheck each) are 35.5 to 37.0 s. What the gates' test adds to a proof: `body_run.rs`'s binary took 280.5 and 284.8 s, two samples, each with no other job in the beelink's slots at load 2 to 5, against main's 239.0 s, one sample alone, so the test adds 41.5 to 45.8 s of wall time because it solves on its own thread beside the run test. Ryzen was not measured: native-contract 50's recorded ratios put it at 1.0 to 2.0 times the beelink. This part sets no cap; this ticket decides the cap and its unit
  - The cap: `body_run.rs`'s binary takes at most 420 s on the beelink alone, below
- [x] A budget for each loop and a ceiling for the synthetic day, in the map's order table, with the run that set them. The ceiling after pass 1 stands for the day until ticket 24 checks it against the first measured day
- [x] One command that prints a ticket's cost line the same way every time: `python3 -I tools/bench/run.py cost body`
- [x] The rule the map states: a ticket over its budget does not merge until the budget's owner records an exception with its reason

**Non-goals:** reaching the speed aim. That is [ticket 27](27-step-plan.md).

## Answer

The budget is stated in user instructions for each simulated second over the body's first minute, read on the beelink from the [cost line](../../../GLOSSARY.md). Today's body takes 5,960.1 M. Each pass 1 loop may add 12% of the figure it starts from, and its tickets share that 12%. After pass 1 the body takes at most 11,920 M, twice today's. The body's run test holds four count ceilings, and the cost line a loop PR quotes holds the instructions. Wall time is reported beside them and holds nothing: on one machine it moved 3.2 times with load while the instructions moved 14 parts a million. The body's cost line fell from 6,964.4 M to 4,958.9 M by [native-contract 69](../../native-contract/issues/69-the-ledger-walks-every-combination-against-every-column.md), which changed no bit, and a loop's budget is counted from the line at its base.

### The measured table

Every row is one `builders/job` on the beelink (8 cores, 16 threads) at `71c7549`, with release builds and `ng_solver::Core` at 10 ms. Each run admits at 06:55 with no boundary input and runs its first 60 simulated seconds. Each job ran the slice, then the body, then the slice again, for 5, 3 and 5 repeats from one admission each. A repeat that took other steps, other solves or another seal would have stopped the job, and none did. Perf counted user instructions inside the repeats only. Other load is the 1-minute load average over the body's run. In the last three jobs it came from 4, 8 or 15 slice runs started inside the job, because the machine stayed quiet. The logs are in `~/lanes/logs/ng-wave-4/b26/runs/`.

| Job, other load | Slice ms a simulated second, before and after | Body ms a simulated second | Body admission, s | Body over slice |
|---|---|---|---|---|
| r1, 1.5 to 2.0 | 66.6, 66.9 | 422.4 (406.8 to 423.1) | 190.3 | 6.33 |
| r2, 1.5 to 2.2 | 66.9, 63.3 | 396.9 (396.4 to 418.5) | 190.6 | 6.10 |
| r3, 2.8 to 1.6 | 63.8, 65.5 | 416.1 (403.8 to 428.4) | 183.0 | 6.43 |
| r4, 1.5 to 1.7 | 64.3, 63.8 | 399.4 (398.1 to 429.8) | 187.5 | 6.23 |
| r5, history on disk, 2.0 to 4.6 | 62.4, 62.4 | 413.6 (397.5 to 533.3) | 183.9 | 6.63 |
| r8, 4 processes, 12.1 to 7.1 | 75.0, 74.4 | 509.2 (507.8 to 510.5) | 214.8 | 6.81 |
| r6, 8 processes, 10.2 to 11.3 | 131.1, 89.4 | 595.1 (589.7 to 613.0) | 297.6 | 5.40 |
| r7, 15 processes, 16.5 to 18.8 | 166.0, 179.8 | 1,271.3 (1,226.1 to 1,331.2) | 523.3 | 7.35 |

Every run took the same counts, so these figures have no spread at all.

| | Slice | Body |
|---|---|---|
| Rows, unknowns, columns, Jacobian nonzeros | 241, 241, 257, 1,578 | 1,372, 1,372, 1,288, 8,892 |
| Steps over 60 s, steps a simulated second | 11,613, 193.6 | 11,797, 196.6 |
| Solves over 60 s, solves a step | 14,806, 1.275 | 14,511, 1.230 |
| User instructions a simulated second | 887.8 M, within 1.7 parts a million over 7 jobs | 5,960.1 M, within 14 parts a million over 7 jobs |
| Slice admission | 9.3 to 10.5 s quiet, 26.1 s among 15 processes | |

The body's 11,797 steps equal the count the run test pins. Since nightingale#166 the slice takes 11,614 steps, 14,749 solves and 896.6 M instructions a simulated second, and the body takes the same counts and seal as before, with 5,960.2 M (one job in this ticket's PR, and the same figure at its last commit in the check of nightingale#167). A history kept on disk adds 0.17% to the body's instructions. The run is one thread and bound by the processor in user mode. Perf's `task-clock` equals its wall time within 2% on a quiet machine, and perf counts 3.5 `instructions` for each of its `cycles`. Load takes the processor away and leaves the work as it was. Each candidate unit's spread over the eight jobs:

- Wall time a simulated second: 396.9 to 422.4 ms quiet (6.4%), and up to 1,271.3 ms with load. It does not reproduce.
- The body's wall time over the slice's in one job: 6.10 to 6.43 quiet, and 5.40 to 7.35 with load. It does not reproduce either, because the two runs see different load.
- The counts: none.
- User instructions: 14 parts a million. The body takes 6.71 times the slice's.

Against ticket 06 part 4c, the body admits in 177.5 to 190.6 s quiet over six samples. That overlaps part 4c's 188 to 229 s, which ran at a load of 2 to 9. Its steps are unchanged.

### The unit, the check and the command

- **Unit.** User instructions a simulated second, on the beelink. A person reads them as time at the quiet beelink's 0.068 ns an instruction, which is 408 ms over 5,960 M.
- **Check.** The run test, `the_whole_body_admits_runs_its_first_minute_and_holds_its_pins` in `crates/ng-validate/tests/body_run.rs`, counts the core's solves over the first minute. It compares four counts with their ceilings through `over_ceilings`: rows at most 2,744, Jacobian nonzeros at most 17,784, steps at most 14,000 and solves at most 29,000. The rows and nonzeros ceilings are twice today's counts, and the solves ceiling is 22 under twice. The heart sets the steps on both assemblies, so the steps ceiling is today's count plus 19%. The check adds no run. `a_count_over_its_ceiling_fails_and_names_that_count` tests the function in a fraction of a second, with no admission, and the row `cost-ceiling-lets-one-solve-over` mutates it. The counts cannot see a law that grows heavier at the same counts, so the instructions are the budget. A loop PR that passes a ceiling moves that ceiling in the same PR, against its cost line.
- **Command.** `python3 -I tools/bench/run.py cost body` (or `slice`) prints one line in this order: head, worker, load_1min, assembly, core, span_s, repeats, rows, unknowns, columns, jacobian_nonzeros, steps, solves, steps_a_simulated_s, solves_a_step, minstructions_a_simulated_s, wall_ms_a_simulated_s, wall_spread_pct, admission_wall_s, seal. Where perf cannot count user instructions the field reads `none`. A tree with changed tracked files marks the head `-dirty`. Neither line can show a budget met. A loop PR quotes the line at its base and at its head, both from the beelink.
- **Owner.** The oversight seat records an exception, with its reason, in the ticket over its budget. The developer reviewer may review the exception.

### The budgets

The speed aim is 10 ms a simulated second, 41 times faster than today's body. Tickets 27 and 30 and native-contract 54 and 55 own that gap, and this budget only stops the loops from widening it without a record. The ceiling after pass 1 rests on real time: the two-day run must finish within two days on the beelink, with margin for the day's events. The first minute is rest, and its first two seconds, which hold the inputs at the fence, cost 1,542 ms a simulated second, 3.8 times the minute's rate (one quiet run).

| | Instructions a simulated second | Wall a simulated second, quiet | The day, 24 h | Two days, 48 h |
|---|---|---|---|---|
| Today's body | 5,960 M | 408 ms | 9.8 h | 19.6 h |
| Ceiling after pass 1, twice today | 11,920 M | 816 ms | 19.6 h | 39.2 h |
| The same ceiling, loaded as r6 (1.45 times) | 11,920 M | 1,180 ms | 28.3 h | 56.6 h |
| The real-time line | 14,600 M | 1,000 ms | 24 h | 48 h |

The day's figure is 86,400 s times the wall time of a simulated second, at rest. The day's own ceiling is checked against the first measured day at ticket 24, and pass 2's allowance is set there inside the real-time line.

- **Each loop.** Pass 1 has six units: the gases (08 to 11 and 38), heat (12), sleep reaching the circulation (25), the fuels (13 to 16), water and salt (17) and acid (18). Each may add 12% to the instructions it starts from, and 1.12 to the sixth power is 1.97, inside the ceiling of 2.0. The map's order table carries each unit's share.
- **Admission.** At most 300 s alone on the beelink. Today it takes 177.5 to 190.6 s quiet, and 215, 298 and 523 s among 4, 8 and 15 processes. A mutant row of the run test pays one rebuild, one admission and the minute, and the sweep gives a row 420 s. This ceiling is wall time and is reported, not held.
- **The body's tests in a proof.** `body_run.rs`'s binary takes at most 420 s alone on the beelink. Ticket 06 measured 280.5 and 284.8 s. Its two slow tests each admit on their own thread, so the binary costs about one admission plus the replay gates, which grow with a simulated second's cost: about 360 s at the ceiling (inferred). This is reported, not held.
- **A full `tools/verify`.** At most 3,600 s on the beelink beside at most one other job. Six recorded proofs there took 2,158 to 2,924 s on 11 November, two took 2,840 and 3,598 s on 13 November, and two took 3,991 and 4,001 s on 14 November, under loads not recorded. The serial mutant sweep took 1,708 to 2,887 s of each. This ticket's own proof took 4,135 s beside one other job, over this ceiling: the sweep took 3,025 s of it, 20 s of that for this ticket's row. A proof over this ceiling is a finding for foundation work, and [foundation 14](../../foundation/issues/14-slow-mutant-rows-run-long-tests.md) has this one. This too is reported, not held.

Since nightingale#172 the body takes 6,257.2 M, 5.0% over 5,960.2 M. The rise is not the laws' work. The pulmonary vein's mark makes the guard landing re-solve on the one step a beat that crosses the end of systole: 754 more core solves over the first minute, 15,264 against 14,510, while a solve takes 24.6 M as before ([native-contract 64](../../native-contract/issues/64-the-landing-reads-a-guard-that-did-not-cross.md)). With nightingale#174 the landing's count fell back to 1,988 landing solves and the line read 5,987.2 M, so the line moves with the landing and not with the laws. Native-contract 64 removes the switch: the landing reads the guard that crossed first, and the end-systole step lands in 3 calls with the vein's mark on or off. At the commit before body 10's first PR the line falls from 5,386.6 M to 5,157.1 M (landing solves 2,745 to 1,988), and on main it holds at 5,174.3 against 5,175.8 M, the same 14,452 solves and the same seal. The ceiling after pass 1 stands at 11,920 M. From 6,257.2 M six loops of 12% reach 12,351 M, so without the landing's fix each loop has 11.3%, and the fix is due before pass 1 is checked. The gas loop's 12% stays 750.9 M, 12% of the 6,257.2 M it starts from, and its shares among tickets 08 to 11 are set after ticket 08's three PRs have measured lines. A model default of the oversight seat, open to the developer reviewer.

The gas loop straddles the removal of the ledger's walk (native-contract 69), which lowered the body's line from 6,964.4 M to 4,958.9 M with no law changed. The oversight seat restates its budget on that line: tickets 09 to 11 may add 12% of 4,958.9 M, which is 595.1 M, in place of the 73.2 M that was left, and ticket 08's share stays as it was measured. Pass 1's ceiling of 11,920 M stands. [Native-contract 83](../../native-contract/issues/83-the-elimination-factors-the-pattern-the-body-has.md) then lowered the line from 5,175.7 M to 4,016.5 M with no bit changed, as native-contract 69 did, so a loop's budget counts from the line at its base.

The gas loop's lines since: 10a left about 120 M of the 595.1 M. Body 10's second PR, the breath and the pleural pressure, measured 5,175.7 M at its base and 5,171.1 M at its head, and about 18.3 M over the base at the base's solve count (inferred), so about 100 M is left on that figure. Body 10's third PR, the pump and its work, measured 4,014.6 M at its base (main at `5fe4e63c98d4`, after native-contract 83 lowered the line) and 4,028.7 M at its head, back to back on the beelink (loads 8.6 and 13.4; the head's line reads `-dirty` for two edited ticket files only): rows 1,449 to 1,453, nonzeros 10,053 to 10,072, steps 11,671 to 11,675, solves 14,388 to 14,396. That is 14.1 M over the base, and about 11.9 M at the base's solve count (inferred), so about 88 M is left. Body 10's fourth PR, the lung's water, measured 4,029.0 M at its base (main at `b640ea3a2084`) and 4,048.7 M at its head, back to back on the beelink (loads 4.3 and 3.1): columns 1,458 to 1,460, nonzeros 10,072 to 10,092, steps 11,675 to 11,674, solves 14,396 to 14,405. That is about 17.2 M over the base at its solve count (inferred), so about 72 M is left. Its first build cost 213 M: a lining that read the gas's water joined two gas water pools to the elimination's largest block, and their factoring cost about 186 M. [Native-contract 89](../../native-contract/issues/89-the-eliminations-order-inside-its-largest-block.md) holds the four forms' figures and asks for the block's order, the solver's largest known lever on cost. Admission took 152.0 s at the base and 219.4 s at the head, and why is not shown. Body 11's first PR, the drive released at the pump's junction, measured 4,048.7 M at its base (main at `a7c461a3516c`) and 4,049.5 M at its head, back to back on the beelink (loads 2.8 and 3.7): rows 1,453 to 1,457, nonzeros 10,092 to 10,107, steps 11,674 to 11,679, solves 14,405 to 14,387. That is 0.8 M over the base, and about 5.9 M at the base's solve count (inferred), so about 66 M is left. Its largest block grew from 644 to 651 rows, which [native-contract 90](../../native-contract/issues/90-a-release-reads-the-nerve-that-serves-its-junction.md) takes. Admission took 193.3 s at the base and 197.7 s at the head. Body 11's second PR, the sensors and the drive scale on the breath's rate, measured 4,049.5 M at its base (main at `d1cf68f5c801`) and 4,079.8 M at its head, back to back on the beelink (loads 13.2 falling to 2.9, and 2.9 to 2.3): rows 1,457 to 1,464, nonzeros 10,107 to 10,150, steps 11,679 to 11,666, solves 14,387 to 14,397, the largest block 651 at both. That is 30.3 M over the base, so about 36 M is left for 11c. Admission took 220.7 s at the base and 217.6 s at the head. Body 11's third PR, sleep through the day's input, measured 4,079.7 M at its base (main at `e9e3f784d3ae`) and 4,083.7 M at its head, back to back on the beelink (loads 3.65 falling to 2.01, and 2.01 to 2.22): rows 1,464, nonzeros 10,150, steps 11,666 and solves 14,397 at both, the largest block 651 at both. That is 4.0 M over the base, so about 32 M is left. Admission took 214.5 s at the base and 216.7 s at the head.

### Not measured

- A full `tools/verify` alone takes 34 to 67 minutes, and none ran beyond this ticket's proof.
- Ryzen and asrog took nothing timed, and no one checked whether perf counts user instructions there. The same binary can take other library paths on another processor, so the budget names the beelink (inferred).
- Admission's work in counts: the bench cannot see the ring solve's iterations.
- The day's cost with meals, posture and sleep, because no such body exists yet.
- Load from other jobs: the extra load ran inside the measuring job, which has no share of the processor apart from that load, so it is harsher than as many other jobs (inferred).

### Not held

The check of nightingale#167 found three gaps, none of which moves a figure above.

- The line marks a changed tree and not the build's environment. On a clean tree `CARGO_PROFILE_RELEASE_OPT_LEVEL=1` printed a bare head and 1,392.3 M for the slice. Until [ticket 32](32-the-cost-line-names-its-build.md) closes it, a line stands only from a plain environment.
- Between clones the slice's figure read 896.5 M through `run.py` in one, 896.6 M in another and 896.65 M from perf on the binary, 100 parts in a million apart, while repeats within one way of running agree to 1.3. Against a 12% allowance that changes nothing, and ticket 32 asks why.
- No row holds the run test's own comparison of the counts with `CEILINGS`, its count of solves, or the accessor that feeds the nonzeros, because each mutant needs a body admission (read, not run). Foundation 14 holds the body's rows.
