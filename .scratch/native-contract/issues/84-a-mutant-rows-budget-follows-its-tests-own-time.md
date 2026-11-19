# 84: A mutant row's budget follows its test's own time

Spec: `tools/mutate.py` and `tools/gate/gate.py` (the mutant sweep), and `tools/verify.d/40-mutants.sh`

Type: task

Status: resolved

Claimed by: nc84-build

Blocked by: none

**Why.** The first milestone (SRS invariant I19) is held by its gate, and the gate's mutant sweep is how a test that once missed a mistake keeps seeing it. A row that cannot finish inside the sweep's budget stops the proof without a verdict, and a check worth keeping can then only be pointed at a cheaper test or dropped.

**What body ticket 10's second PR found.** The sweep gives every row and every baseline 420 s of wall time, whatever its test costs.

- A row whose test admits the whole body takes 200 to 290 s on an idle beelink: in nightingale#198, `breath-never-starts` took 200 s and `pleural-reads-no-drive` 288 s, beside a baseline of 212 s.
- The proof of nightingale#198's first head on ryzen ran that test's baseline in 370 s, beside the desk's share, and then gave `pleural-reads-no-drive` `UNCLEAR` at 420 s. The sweep stopped there with 561 rows not run.
- The same row runs again whenever its code or its test changes, and the whole body's assembly, packet and run tests change in most body PRs.

**What body ticket 10's last PR found.** In its proof on ryzen the body breath test's baseline took 334 s and its row `breath-never-starts` 342 s of the 420 s limit, where 10c's proof took 248 s. The lung's water slowed admission (native-contract 86 holds the counts). Another admission rise of that size passes the limit.

- [x] A budget that follows each test's own baseline in the same sweep (for example a multiple of the baseline's time), or one counted in CPU time rather than wall time, so a loaded worker gives a row the same chance as an idle one. Shown by `limit_case` in `tools/gate/gate.py`: a row that takes 2.5 s beside a 2 s baseline is `CAUGHT` with a least limit of 1 s
- [x] The sweep's report says which budget it gave each row, and a row over it still reads `UNCLEAR`. Shown by the same case: each verdict line carries `limit N s`, and a row that takes 8 s beside a 2 s baseline reads `UNCLEAR  over 4 s` and fails the sweep
- [x] The rule's probe in `tools/gate/gate.py` holds the new budget the way it holds 420 s today. `limit_case` runs in 05-seam's driver cases, and it fails with the multiple taken out and with the skipped baseline's second run taken out

**Non-goals:** which rows the gate keeps, and the worker's load.

## Answer

A row's limit is twice its test's baseline time in the same sweep, and never under `--budget`, which stays 420 s by default. A baseline cannot follow itself, so it gets four times `--budget`, 1,680 s: the unmutated test is known to finish, and that limit only stops a hung build. The `--no-run` build that stands in for a skipped baseline gets the same.

Why twice. A row's rebuild rides on its test's time. The slowest rows measured took 1.0 to 1.4 times their baseline: 342 s beside 343 s on ryzen in body 10d's proof, and 288 s beside 212 s on an idle beelink in nightingale#198. Twice leaves room for that and for load that changes between the baseline and its rows. Wall time was kept: no case was found that a wall multiple fails and CPU time passes, and CPU time across cargo's process tree is harder to hold.

A skipped baseline. In a merge run's rest part the sweep skips each baseline that 10-rust's passed call covered, and then it has no time for that test. A row over `--budget` whose baseline was skipped now runs the baseline and then runs again with its limit, the way a row caught at build already runs its skipped baseline. The record and 10-rust stay as they were. The whole body's breath test is one such test, since 10-rust tests `ng-validate` whole.

Only `UNCLEAR` by time can change. Every row gets at least the 420 s it had. Each verdict line now reads `<verdict>  <row>  <n> s  limit <n> s  <detail>`, and `gate.py`'s verdict pattern reads it unchanged.

The cost on body 10d's proof (933d0bd on ryzen): a green sweep is unchanged at 5,886 s, since the limit only cuts a stuck row. A stuck whole-body row now costs up to 686 s before the sweep stops, against 420 s. If its baseline was skipped it costs up to 1,449 s: 420 s, then the 343 s baseline, then 686 s.
