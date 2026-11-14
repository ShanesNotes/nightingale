# 26: A cost budget for the whole body

Type: task
Status: open
Blocked by: 06, 07

**What to build:** The budget each loop ticket is held to. Measure [ticket 06](06-whole-body-on-holds.md)'s body against the slice on one named build worker, and from [ticket 07](07-what-limits-the-step.md)'s limiter set how much wall time for a simulated second each loop may add, and a ceiling for the whole body after pass 1.

- [ ] The body on holds measured against the slice under one repeat protocol: unknowns, rows, the cost of a step by part, and wall time for each simulated second
- [ ] Admission's wall time and the wall time of a full `tools/verify` run, measured the same way, each with a ceiling. A run pays admission once, however short it is, and the gate's time limits how many tickets merge in a day
- [ ] From [ticket 06](06-whole-body-on-holds.md)'s plan: a ceiling on the time the body's replay gates add to a full `tools/verify` run. Each replay gate runs a core several times over its run, so the body's gates cost in proportion to the body's step, which ticket 06's last part measures
  - Inputs from ticket 06 part 4c, each wall seconds on the beelink in one `builders/job`. One body admission solve: 188 to 229 s, five samples, alone or beside one other job, load 2 to 9 on 16 threads. The seven gates on the body, solve excluded: 78.2 and 85.8 s, two samples, of which the four starts (one check and recheck each) are 35.5 to 37.0 s. What the gates' test adds to a proof: `body_run.rs`'s binary took 280.5 and 284.8 s, two samples, each with no other job in the beelink's slots at load 2 to 5, against main's 239.0 s, one sample alone, so the test adds 41.5 to 45.8 s of wall time because it solves on its own thread beside the run test. Ryzen was not measured: native-contract 50's recorded ratios put it at 1.0 to 2.0 times the beelink. This part sets no cap; this ticket decides the cap and its unit
- [ ] A budget for each loop and a ceiling for the synthetic day, in the map's order table, with the run that set them
- [ ] One command that prints a ticket's cost line the same way every time
- [ ] The rule the map states: a ticket over its budget does not merge until the budget's owner records an exception with its reason

**Non-goals:** reaching the speed aim. That is [ticket 27](27-step-plan.md).
