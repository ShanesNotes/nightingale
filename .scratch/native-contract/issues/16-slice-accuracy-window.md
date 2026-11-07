# 16: A slice window meets the reference integrator

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The accuracy gate measures a window of the slice against `ReferenceIntegrator` at three refinements, which give the reference's own error and a core's observed order. The window is §7's first: admission through the first posture change and meal. The metrics cover states, exchanges, outputs and event times. A core is never compared with itself.

Blocked by: 11, 12, 15

Status: ready-for-agent

- [ ] The first slice window's distances come from the reference integrator's own refinement, and a core outside them is rejected

## Comments

From 06 and 10: choose the accuracy and tolerance distances from the reference's own refinement on the slice. A run settles on its own orbit. At 1 ms steps mean pressure reads about 102 against the anchor of 100 on the ring's 10 ms grid, and the rate drifts down over the first beats.

From 08: a core off the admission grid starts near its own orbit and settles over a few cycles, so a window that reads a cycle mean from admission says which core and step it used.

From 10: the fence refuses a posture input inside the first cycle, so a window that stands must run past it.

From 04 and 10: row scales were left to tickets 10 and 11. Ticket 10 gave each row its natural size, which the reference's Newton stop reads. The per-row error scales that this gate compares at are this ticket's. The slice's are uniform today (`crates/ng-validate/src/slice.rs:487`).

From the split of ticket 11: §7's other slice windows, sleep onset and the second day's waking, need sleep and clock payloads, and no ticket builds those yet.

From the solver arena (in this PR): the two candidate cores agreed on every body-response verdict and on the two-reservoir accuracy errors, but per-beat cardiac output in the first beats after 07:00 differed between them by up to about 0.6 mL/s, about 1 percent. Only this ticket's window against the reference integrator can say which is closer.

From ticket 11: `accuracy` reads the last segment from `session.conservation().head()` and admits each case with its own `MemoryStore`. A slice window run through admission and the first posture change and meal is short enough for that store; a longer one needs a host store outside the process.

From 15: `AccuracyPolicy::new` turns each `AccuracyCase` into a fixture and a kind (`Manufactured` in `crates/ng-validate/src/accuracy.rs`), and `accuracy` runs a closed form per case. A slice window is a new kind with its own comparison against the reference integrator, and `AccuracyPolicy::manufactured()` gains it so the arena policy runs every case. `observed_order` reads one worst error per case, so a window needs a nonzero worst error at each refinement.

From 22: `score` refuses accuracy evidence whose policy digest is not `AccuracyPolicy::manufactured()`'s, or whose core's resolution digest is not the run's. So once `manufactured()` gains the slice window, a score needs that window's evidence from the run's own core. The honest score in `crates/ng-validate/tests/score.rs` comes from a two-reservoir run on the reference integrator at 16 s, so this ticket rebuilds that test on a core and window that can run the slice.
