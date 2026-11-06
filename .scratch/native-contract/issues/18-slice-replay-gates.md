# 18: The slice passes the replay gates

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The slice becomes a `ReplayCase`, and a short slice window passes every replay gate with the reference core. The gates are pace (unbounded, live and chunked), resume, late admission, rewind, extra evaluation, step policy and non-perturbation. The case admits once and resumes each run from a checkpoint of the start, as `crates/ng-validate/tests/arena_body.rs` does.

Blocked by: 11, 12, 17

Status: ready-for-agent

- [ ] A short slice window passes all seven replay gates with the reference core

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
