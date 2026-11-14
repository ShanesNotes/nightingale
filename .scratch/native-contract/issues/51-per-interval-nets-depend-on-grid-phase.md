# 51: The slice window's per-interval nets depend on where its grid falls in the beat

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §7 (the Accuracy gate and its windows), and `crates/ng-validate/src/window.rs` (`over_interval`, `Measured::new`)

Type: task

Status: resolved

Claimed by: nc51-build

Blocked by: none

Since [ticket 28](28-admission-energy-at-balance.md) the slice window compares each exchange by its net over each 10 ms grid interval. That statistic's distance is the coarser refinements' largest error at the sampled instants, and where those instants fall in the valve events moves it. The window's grid starts at the arena's start, which is a beat edge only because admission's paced cycle starts at the wrap.

[Ticket 31](31-cross-interval-drift-check.md)'s plan measured the window at sixteen grid phases on main 7f0cbfe: 0 to 9 ms by 1 ms, and the six offsets where the finest refinement's beat edges fall. At a phase of 8 ms three honest cores fail the per-interval statistic on column 22, stroke work drawn on chemical energy: `ng_solver::Core::new` at 1.165 and `Core::doubled` at 1.172, both at the body's longest step, and the 16 s reference under the pick's scales at 1.194. Their worst interval ends at 2.898 s, beside the third beat edge, where the honest cores' systole starts about 4 ms before the finest refinement's. At phase 0, where the window runs, they pass at 0.693, 0.697 and 0.750. Over the sixteen phases the fixed 5 ms reference ranges 0.419 to 0.840 and the adapting 5 ms reference 0.472 to 0.932. The plan, its figures and the analysis are in `~/lanes/logs/ng-wave-4/nc31/` (`PLAN.md`, `analysis-2.txt`, `phase8.py`).

The window passes these cores only because its grid starts on a beat edge. A window that starts mid-beat, such as §7's sleep onset or the second day's waking, can fail an honest core on this statistic alone.

No fix is chosen. One option: ticket 31's whole-beat nets fail all four must-fail cores of `crates/ng-validate/tests/window.rs` on their own (plan figures: 4.78 on column 7, 96.7, 41.9 and 206), and they read at beat edges, not at grid instants, so they could replace the per-interval nets. Others would change what the per-interval distance samples.

- [x] The honest cores of the window test, `Core::new`, `Core::doubled` and the 16 s reference pass the window at every grid phase, held by a test or a measured run named here. Held by the lib test `window::tests::honest_cores_pass_wherever_the_grid_falls_in_the_beat` (the 5 ms references from 2 ms, `Core::new` from 8 ms), with the mutant row `window-reads-every-finest-step`, and by one measured run at sixteen phases (below)
- [x] The four must-fail cores still fail the window. Held by `tests/window.rs`, its assertions unchanged, and the rows of `tools/gate/must-fail.toml`
- [x] §7's lines on the window change only as much as the rule needs. One clause: metrics are read at every step of the finest refinement

Carried from the independent check of nightingale#154, because this ticket edits the same lines (`~/lanes/logs/ng-wave-4/nc31-build/verify-coordinator/VERIFY-pr154.md`):

- [x] `Measured::verdict` refuses a run whose beat count differs from the window's, with a test and a mutant row. Today it pairs the two lists, so a run with no beats would pass; `trace` cannot make one. Held by `AccuracyFailure::OtherBeats`, the lib test `a_run_with_other_beats_than_the_window_is_refused` and the row `verdict-refuses-other-beats`
- [x] A test names `TooFewBeats`: a window of one whole beat is refused and one of two is admitted, with a mutant row that removes the refusal. Held by `whole_beats`, the lib test `a_window_of_one_whole_beat_is_refused_and_one_of_two_admitted` and the row `window-refuses-one-beat`
- [x] The whole-beat fixture gains a run just inside its beat distance and one just outside. Today's failing run is 75,000 times the distance, so a far wider margin would pass both tests. Held by `a_one_sign_error_inside_every_interval_fails_on_its_whole_beat`: the rung's beats are off by `RUNG_BEAT_ERROR`, and runs at 0.96 and 1.04 of the beat distance pass and fail
- [x] The comment in `window.rs` on a running total says what ticket 31 found, and the public docs of the window and of the accuracy failure name the whole-beat nets. Held by the comment on `over_span` and the docs of `SliceWindow`, `WindowPolicy::margin`, `WindowPolicy::distance_floor` and `accuracy`

This closes before the scored pick, and before any window that starts off a beat edge is built ([ticket 16](16-slice-accuracy-window.md) and [ticket 21](21-day-checks-read-the-trace.md) name those windows).

**Non-goals:** the whole-beat check ticket 31 added; the energy drift limit; the window's margins.

## Answer

The window reads every compared value at each step of the finest refinement, 0.625 ms, instead of every 10 ms. A level is read as it stands. An exchange is read as its net over the coarsest refinement's step ending at that instant, 10 ms, or since admission while the window is younger. The whole-beat nets of ticket 31 stay as they were. The grid is `refinements_ns[2]` and the net's span `refinements_ns[0]`, so no new number enters. The plan and its measurements are in `~/lanes/logs/ng-wave-4/nc51/PLAN.md` (shape S3b), and the build's evidence in `~/lanes/logs/ng-wave-4/nc51-build/`.

The level check depended on grid phase too. On the 10 ms grid from 2 ms, the fixed and adapting 5 ms references failed on row 160, the memory of the tricuspid valve, at 3.772 s, at 1.137 and 1.011. The instant fell inside the valve's switch. So no shape that kept reading levels every 10 ms could pass every honest core, and the fix moved from the nets to the grid.

Measured once through the built check at the sixteen phases of ticket 31, on the beelink (`probe-head.log`, `phases-summary.txt`): `Core::new` 0.7066, `Core::doubled` 0.7148, the 16 s reference 0.7334 and the adapting 5 ms reference 0.5167 at every phase, and the fixed 5 ms reference 0.4791 to 0.4960. The four must-fail cores fail at every phase on the check they fail on main. Their largest ratios at head: the 1 ms floor 4.78 on column 7, one update 108 on column 34, stale 382 on a level, the 10 ms floor 206 on a beat net; on main at four phases 4.78, 2,858, 382 and 974.

What the new shape stops seeing: within-beat errors on three cardiac work columns. Stroke work c22 was caught on main at a 4 ms displacement and now at 10 ms; stroke work c30 at 5 against 6 ms; wall work c34 at 8 ms against never, to 200 ms. Main saw them only where its grid fell where the rungs' error was small. The honest cores carry a 4 ms offset on c22 themselves, which is why main failed them from 8 ms.

What no check sees: a reshaped pulse that nets to zero each beat on gravity work c7, stroke work c30 and wall work c34. Energy's certificate sees a drift in their total, not that reshape.

The independent check of this change (`~/lanes/logs/ng-wave-4/nc51-build/verify-coordinator/VERIFY-pr158.md`) found more that the new shape stops seeing, and what bounds each loss.

- A distance is one number for a quantity over the whole window: twice the coarser refinements' largest error at any compared instant. Reading sixteen times as many instants found the coarse rung's sharpest errors: a jump at a posture step, read 0.625 ms into the rung's 10 ms step, and a valve event the rung makes 3 to 5 ms early. So 187 of 241 level distances and 177 of 257 net distances grew, 12 rows and 27 columns by more than a tenth, and none fell. Main's tighter numbers were phase luck against the posture steps as well as the beats.
- Four flows of the circulation and the tricuspid valve's memory: rows 201, 196, 203, 199 and 160. Their level distances grew 2.7 to 23 times. A run that holds row 196 flat, or row 160 flat at half its peak, now passes. A flow's error that its water column carries is still caught by that column's net, from 0.64 to 28.5 held for 10 ms.
- Gravity work c7. Main refused a pulse 10 ms late. The new shape passes every delay to 200 ms, because the built net distance, 0.0098, is wider than the column's whole swing, 0.0084. Wall work c34's net distance is wider than its swing too.
- What still holds these: the whole-beat nets for c22, c30, c34 and c7, whose distances did not move; the water column's net for each flow; and the transition times for the valve. No run ties the valve's memory to its columns.
- Thirty real cores, from honest to must-fail, pass or fail at the head as they do on main, at admission. That is a small family, and no proof that no core gets through.
- A window that starts 0.75 s or more after admission fails honest cores, because a young window reads each net since admission. No shipped path starts one. [Native-contract 53](53-a-late-window-fails-honest-cores.md) holds that, and the two rules of this change that no test holds.
