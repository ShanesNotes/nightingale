# 84: A mutant row's budget follows its test's own time

Spec: `tools/mutate.py` and `tools/gate/gate.py` (the mutant sweep), and `tools/verify.d/40-mutants.sh`

Type: task

Status: claimed

Claimed by: nc84-build

Blocked by: none

**Why.** The first milestone (SRS invariant I19) is held by its gate, and the gate's mutant sweep is how a test that once missed a mistake keeps seeing it. A row that cannot finish inside the sweep's budget stops the proof without a verdict, and a check worth keeping can then only be pointed at a cheaper test or dropped.

**What body ticket 10's second PR found.** The sweep gives every row and every baseline 420 s of wall time, whatever its test costs.

- A row whose test admits the whole body takes 200 to 290 s on an idle beelink: in nightingale#198, `breath-never-starts` took 200 s and `pleural-reads-no-drive` 288 s, beside a baseline of 212 s.
- The proof of nightingale#198's first head on ryzen ran that test's baseline in 370 s, beside the desk's share, and then gave `pleural-reads-no-drive` `UNCLEAR` at 420 s. The sweep stopped there with 561 rows not run.
- The same row runs again whenever its code or its test changes, and the whole body's assembly, packet and run tests change in most body PRs.

**What body ticket 10's last PR found.** In its proof on ryzen the body breath test's baseline took 334 s and its row `breath-never-starts` 342 s of the 420 s limit, where 10c's proof took 248 s. The lung's water slowed admission (native-contract 86 holds the counts). Another admission rise of that size passes the limit.

- [ ] A budget that follows each test's own baseline in the same sweep (for example a multiple of the baseline's time), or one counted in CPU time rather than wall time, so a loaded worker gives a row the same chance as an idle one
- [ ] The sweep's report says which budget it gave each row, and a row over it still reads `UNCLEAR`
- [ ] The rule's probe in `tools/gate/gate.py` holds the new budget the way it holds 420 s today

**Non-goals:** which rows the gate keeps, and the worker's load.
