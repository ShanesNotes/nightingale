# 16: A slice window meets the reference integrator

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The accuracy gate measures a window of the slice against `ReferenceIntegrator` at three refinements, which give the reference's own error and a core's observed order. The window is §7's first: admission through the first posture change and meal. The metrics cover states, exchanges, outputs and event times. A core is never compared with itself.

Blocked by: 11, 12, 15

Status: resolved

- [x] The first slice window's distances come from the reference integrator's own refinement, and a core outside them is rejected

## Answer

`AccuracyCase::SliceWindow(Box<SliceWindow>)` is §7's first window. `SliceWindow::first()` measures it once against `ReferenceIntegrator`, and `accuracy` then judges any core against what it measured. The sketch's `SliceReference { reference, factor }` becomes `SliceWindow(SliceWindow)` with `first()`, because the measurements below fix the reference and its refinements.

- **The first slice window's distances come from the reference integrator's own refinement, and a core outside them is rejected.** `the_first_window_holds_each_core_to_the_references_own_distances` (`crates/ng-validate/tests/window.rs`) builds the window once and judges five cores. The reference at a fixed 5 ms, a step no rung takes, passes, and so does the reference at 5 ms on the arena's scales, which adapts its step. `AcceptsAfterSweepLimit` stops each Newton stage after one update and declares scales loose enough for `validate` to accept it. It passes every manufactured case, and the window rejects it as `Outside`. The window also rejects a core that lands transitions only to within 1 ms (`Outside`) and one that lands them only to its 10 ms step (`ModesApart`). Three unit tests in `crates/ng-validate/src/window.rs` hold `TransitionMoved` on runs built by hand, because on the body a moved transition moves values too, and values are judged first.
- **The window.** Admission at 06:55, supine to seated in eleven steps from 1.2 s, past the first cycle's fence, and a meal of 300 mL water, 50 sodium, 2 potassium and 52 chloride delivered from 2.5 s to 4.5 s. It is compared to 6 s on a 10 ms grid. §7 names the window by its events, so it runs on a compressed synthetic day. §6's day, with the ramp at 07:00 and the meal at 07:30, is 35 simulated minutes to the meal alone (see the cost below).
- **The reference's refinements.** The body's longest step, 10 ms, then 2.5 ms and 0.625 ms, at a fixed step: the scales are so loose the estimate never refuses, and the landing tolerance is 0.1 ms. The finest stands for the solution. Measured on the tower: at the arena's scales the 10 ms reference refused 782 steps for accuracy, so its rungs stopped being refinements. 10, 5 and 2.5 ms do not converge on this slice: the median ratio of successive differences was 1.35, and the valves that flutter made 87, 91 and 112 transitions. 2.5, 1.25 and 0.625 ms held 382 of 467 moving quantities at first order, and 1.25 and 0.625 ms made the same 126 transitions. A 1 µs landing tolerance left the reference's Newton unable to settle (`NoAdmissibleStep`).
- **What it compares.** Every row (amounts, memory and produced quantities) and every column's exchange since admission, 498 quantities, at each instant where the run holds the finest's modes. Then each kind of mode transition (a variable and the value it enters), and the instants spent in other modes. A coarser step steps over a brief valve flutter that a finer one resolves, so the window asks only that each transition a core makes stand near one the finest makes. What a core steps over shows as instants in other modes.
- **The distances.** A core may stand no more than twice as far from the finest as the farther of the two coarser rungs: in each quantity (never under a millionth of what that quantity reaches), in each kind of transition (never under the landing tolerance), and in instants spent in other modes. The factor is a model default. On a quantity that a landing moves, such as the gravity work across the leg veins' valve, the rungs are two draws of what a 0.1 ms landing leaves. Without it the reference at 5 ms with the arena's scales stood 0.2 percent past the farther rung, and `ng_solver::Core` capped at 2.5 ms stood 0.3 percent past it.
- **Not compared with itself.** The finest refinement is the target. Each rung is a different step of the reference, and no core under test supplies a distance.
- **Verdicts, measured on the tower** (release build, one run each, largest share of a distance; 1 or less passes):

  | Core | Verdict |
  |---|---|
  | reference fixed at 10, 5, 2.5 and 1.25 ms | pass: 0.50, 0.45, 0.50, 0.50 |
  | reference at 5 ms on the arena's scales (1, 1e-3) | pass: 0.61 |
  | reference fixed at 5 ms, landing to 0.2 ms | pass: 0.59 |
  | reference fixed at 5 ms, landing to 0.3, 0.5 or 1 ms | `Outside` (gravity work across the leg veins' valve, or a row after the stir) |
  | reference fixed at 10 ms, landing to 5 or 10 ms | `ModesApart`: 145 and 317 instants in other modes, 106 allowed |
  | `ng_solver::Core` capped at 10, 5, 2.5 and 1.25 ms | pass: 0.84, 0.56, 0.50, 0.44 |
  | `AcceptsAfterSweepLimit` at 10 ms | `Outside` |

- **Observed order.** The window gives a nonzero worst at every refinement, so `observed_order` can read it, but the shares above do not fall at first order: the reference's barely move from 10 ms to 1.25 ms, and `Core`'s fall by 1.5, 1.1 and 1.1 times. On the slice the largest share is set by quantities that a 0.1 ms landing moves whatever the step, so the window cannot hold a first-order band. The distances never assume an order: each is measured.
- **Cost, measured on the tower.** Building the window took 32.6 s in a release build: one admission solve, then 2.3, 5.3 and 18.2 s for the three rungs (704, 2,658 and 10,460 accepted steps over 6.5 s with the tail). The crate test, which builds the window and judges five cores, ran 45.2 s and 45.9 s in the test profile, with peak resident sets of 414 MB and 217 MB. The finest rung's history stays in a `MemoryStore`, about 195 MB at 18.6 KB a step.
- **§6's own day, inferred by arithmetic from those rates.** Admission at 06:55 to the meal at 07:30 is 2,100 simulated seconds: about 2.3 hours on the tower for the three rungs, and about 63 GB of history for the finest, which no `MemoryStore` holds. Running it needs a store outside the process, as `tools/bench`'s `LinkLog` is, and an entry point that takes one. No crate test or bench mode runs it.
- **Left as they were.** `AccuracyPolicy::manufactured()` keeps its five cases without the window: its ladders of cores refine from 16 s down, every slice ticket is capped at the body's 10 ms, and the window costs the reference's three runs. The arena's policy adds the window itself (ticket 22). No row scales moved: a quantity's distance is its own, so the window needs none.

## Comments

From 06 and 10: choose the accuracy and tolerance distances from the reference's own refinement on the slice. A run settles on its own orbit. At 1 ms steps mean pressure reads about 102 against the anchor of 100 on the ring's 10 ms grid, and the rate drifts down over the first beats.

From 08: a core off the admission grid starts near its own orbit and settles over a few cycles, so a window that reads a cycle mean from admission says which core and step it used.

From 10: the fence refuses a posture input inside the first cycle, so a window that stands must run past it.

From 04 and 10: row scales were left to tickets 10 and 11. Ticket 10 gave each row its natural size, which the reference's Newton stop reads. The per-row error scales that this gate compares at are this ticket's. The slice's are uniform today (`crates/ng-validate/src/slice.rs:487`).

From the split of ticket 11: §7's other slice windows, sleep onset and the second day's waking, need sleep and clock payloads, and no ticket builds those yet.

From the solver arena (in this PR): the two candidate cores agreed on every body-response verdict and on the two-reservoir accuracy errors, but per-beat cardiac output in the first beats after 07:00 differed between them by up to about 0.6 mL/s, about 1 percent. Only this ticket's window against the reference integrator can say which is closer.

From ticket 11: `accuracy` reads the last segment from `session.conservation().head()` and admits each case with its own `MemoryStore`. A slice window run through admission and the first posture change and meal is short enough for that store; a longer one needs a host store outside the process.

From 15: `AccuracyPolicy::new` turns each `AccuracyCase` into a fixture and a kind (`Manufactured` in `crates/ng-validate/src/accuracy.rs`), and `accuracy` runs a closed form per case. A slice window is a new kind with its own comparison against the reference integrator, and `AccuracyPolicy::manufactured()` gains it so the arena policy runs every case. `observed_order` reads one worst error per case, so a window needs a nonzero worst error at each refinement.
