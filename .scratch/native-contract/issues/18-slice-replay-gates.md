# 18: The slice passes the replay gates

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The slice becomes a `ReplayCase`, and a short slice window passes every replay gate with the reference core. The gates are pace (unbounded, live and chunked), resume, late admission, rewind, extra evaluation, step policy and non-perturbation. The case admits once and resumes each run from a checkpoint of the start, as `crates/ng-validate/tests/arena_body.rs` does.

Blocked by: 11, 12, 17

Status: resolved

- [x] A short slice window passes all seven replay gates with the reference core

## Answer

The slice is a `ReplayCase` now: `SliceCase` in `crates/ng-validate/src/slice.rs`. A window of it passes all seven replay gates with the reference core.

- **A short slice window passes all seven replay gates with the reference core.** `a_slice_window_passes_every_replay_gate_with_the_reference_core` in `crates/ng-validate/tests/arena_body.rs` runs `replay_gates` on `SliceCase` with the reference core at 3 ms, and every gate passes. The window runs from 06:55 to 50 ms past the fence at 06:56. The stir fires inside it, at 528.07 ms on that core (measured). Two inputs wait at the fence with separate ids, a stand and a 400 unit gut water bolus, so the late-admission gate withholds both, and pace and resume meet posture, a delivery, same-time inputs and refusals (30 at 3 ms, measured). Each gate's reference run fires the stir alone, on stream `stir` at ordinal 0, and each gate's evidence names the one run. The test also checks that the withheld start holds no waiting input.
- **The step policies.** The second policy is the reference core at 6 ms. The gate's reference is the first policy, because the core under test is the reference integrator. Its distance is the floor, 1 ms. The stir's rate is a constant, so a policy lands it within one floor after it crosses its draw. Measured landings: 529.000 ms at 1 and 2 ms, 528.367 ms at 2.5 ms, 528.072 ms at 3 ms and 528.031 ms at 4 ms.
- **One admission per resolution.** An admission solve takes about 6.5 s and a run of the window about 0.66 s at 3 ms (measured, Omarchy, test profile), so `SliceCase` admits each core resolution's start once, with the inputs and without, keeps a checkpoint of each, and resumes every run from it. This test admits three: 3 ms with and without the inputs, and 6 ms. A start's checkpoint holds no link, so it resumes into any store.
- **Wall time.** Measured on Omarchy in the test profile under `heavy.sh`: the test takes 61.7 s. The resume gate is about 28 s of it, because it loads on each side of every breakpoint, refusal and firing (43 rows at 3 ms) and runs each load to the end. The three admission solves take about 19 s. The test runs beside the slice's other checks in the `arena_body` binary, whose wall time went from 52.7 s to 61.6 s. At 2 ms the gates take 72.3 s. A run of the window at 1 ms costs about 1.5 s, more than twice the 3 ms run (measured), so the test runs at 3 ms.

## Comments

From 05: each load in the resume gate runs to the end of the case, so choose `resume_every` for the slice's length. `k` loads cost about `k / 2` runs. A resume reads every link (ticket 11).

From 06: the gates take `&dyn ReplayCase`, so the slice is one more implementor. `ChancePlan` needs a second policy, a `ReferenceIntegrator` factory and a distance, and `GateEvidence::tolerance_ns` reports that distance. The distance is the gate's limit: a core that draws per step and moves its events by less than the distance passes. So choose the distance from the reference's own refinement on the slice. The late-admission gate costs one run per input and one more, because it reads each arrival off a run that does not hold that input.

From 07: non-perturbation needs nothing new in `ReplayPlan` or `ReplayCase`. It builds two lenses from `body.rows()`, so it covers the slice once the case lands.

From 08: `ReplayCase::events` lists only what can arrive late. A delivery inside the admission window shaped the start, so every run of a case holds it, and the slice's case does the same.

From 10: one admission takes about 17 s in a release build (`tools/bench/run.py`), so `ReplayCase::admit` on the slice admits once and resumes from a checkpoint. The fence refuses a posture input inside the first cycle, so a window that stands must run past it.

From 04, in ticket 09's comments: a ticket cut below the floor by a breakpoint faults at its first refusal, so the slice's closely spaced events need a floor below their gaps.

From the split of ticket 11: the live pace reads the host clock that ticket 12 passes in. Each core's minimum interval is a policy value. It takes a model default, set in the validation crate and reported with every result (foundation 07).

From the solver arena (in this PR), a lead not yet tested on the slice: the step-policy gate needs two policies on two grids, and an error-controlled core runs both caps on one grid wherever both caps exceed the span its error allows, so the gate would report missing evidence. The picked core passes the gate on the chance case at caps of 1 s and 0.25 s. On the slice, choose two policies whose grids differ.

From ticket 11: every gate admits with its own `MemoryStore`, and the resume and rewind gates share one store between the parent and its loads and forks. A resume walks every link newest first, so the resume gate's cost still grows with the number of loads.

From 19: pace, resume, late admission and rewind now watch every run they compare through a lens on every row with 256 instants over the span a detached reference accepted, so a slice run reads about 256 instants whatever its step. Each of those gates runs its reference twice, detached and watched, so budget one more slice run per gate. A body whose rows `CompiledBody::lens` refuses makes those gates `MissingEvidence`, as non-perturbation already is.

From 17: the slice's stochastic fixture is on the held MS stand-in. The hazard `stir hazard` draws on the stream `stir` and fires once. With seed [1, 2, 3, 4] on the reference core at 1 ms, it fired 529 ms after 06:55 (measured), so a window that runs past about 0.53 s holds the extra-evaluation gate's event. The forcing `skin water variability` scales the skin's water loss at every stage. The stir's rate is a constant, so its instant reads only the seed and the landing search, never the core's accuracy. A step-policy gate on the slice therefore tests where a core lands the stir, not how well it integrates. A hazard whose rate reads the body would test the second. At admission every hazard integral holds its packet value, so a slice checkpoint taken at admission has the stir unfired, at ordinal 0.

From ticket 12: the live pace and `run` read the same `WallClock` trait, and `tools/bench`'s `HostClock` is a host clock for both. `run` admits once and resumes each repeat from a checkpoint of the start, the shape this ticket's case wants.
