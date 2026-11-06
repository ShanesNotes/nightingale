# 19: Replay gates compare observer bits

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** Pace, resume, late admission and rewind compare observer digests as well as accepted state, ledger and events, as §7's pace and resume gate asks. The gates thread an observer through `advance`, `step_to`, `revision_to` and `traced`, and each gate's digests go in `GateEvidence::observed`.

Blocked by: None (can start immediately)

Status: resolved

- [x] Pace, resume, late admission and rewind compare observer digests, and each rejects an observer built to differ while the accepted state matches

## Answer

A core can no longer pass pace, resume, late admission or rewind while its observer drifts. Each of the four gates reads every run it compares through one lens, chains what the observer returns into a digest at every row, and holds that digest to the reference run's row by row, beside the seal.

- **Pace, resume, late admission and rewind compare observer digests, and each rejects an observer built to differ while the accepted state matches.** `replay::tests::an_observer_that_drifts_while_the_accepted_state_matches_fails_pace_resume_late_admission_and_rewind` runs each gate twice on one case and one core. With `ng_observe::Observer` the gate passes and every run's `RecordDigests` match. With an observer that hears nothing collected from 50 s on, after its first run, each gate fails `Diverged` at an instant at or past 50 s, so every row before it matched in seal and observations. `an_observer_that_forgets_its_state_on_resume_fails_resume_and_rewind_and_passes_the_rest` resumes as a new observer: resume and rewind reject it, and pace and late admission, which never resume an observer, pass it. `every_gate_passes_the_reference_core_with_one_set_of_digests` now holds `GateEvidence::observed` to one digest per run in `runs`, all equal.
- **Threading.** `advance`, `step_to`, `revision_to` and `traced` take an optional watch. An attached watch reads each accepted revision's view and chains it, and `Row` keeps that digest. A detached run (extra evaluation, step policy, the late-admission runs that only read arrivals, and non-perturbation's detached run) reads nothing, and against a detached row only the seals compare.
- **Resume and rewind** save the observer's checkpoint and its digest beside each record checkpoint, as a host keeps its observer's outputs, and each load and fork resumes both.
- **The lens.** Every row, a window of two grid periods and a delay of three, on a grid of 256 instants over the run's own span. The delay is longer than the window, so every head past the first window holds a reading collected and not yet resulted, which a checkpoint must carry. The grid follows the span and not the steps, so a slice window costs 256 instants, not the 16 MB a 45 min delay on a 4 ms grid would hold (this ticket's comment from 07). The lens holds no error, for the reason ticket 07 gives.
- **Why a private seam.** `ng_observe::Observer` is a function of the views it read, and equal seals give equal views, so only an observer built in a test can differ. The gates take their observers from a private `Observers` factory. `replay_gate` and `replay_gates` keep the sketch's signature and use `Observer`. A public factory in `ReplayPlan` was the other shape: it would put in the arena's API a choice whose only honest value is `Observer`.

**Evidence**, all synthetic.

- **Mutants.** `tools/mutate.py` over five rows (`~/lanes/logs/ng-nc-11-24/19/mutants-19*.toml`), each caught on the head: a row compare that ignores observations (caught by both new tests), a resumed watch that restarts its digest, a late run left unwatched, and an `ObserverCheckpoint` that drops its held readings. That last one, a real observer bug, survives main's gates: on main the same test passes with it.
- **No physiology moved.** No pin changed, and every ng-validate test passes in release.

**Limits.** Non-perturbation still compares physiology only, as §7 says. Extra evaluation compares no observer bits, because no ticket asks it to.

## Comments

From 07: only the non-perturbation gate attaches an observer today. Equal seals give equal views, so these comparisons can only catch an impure observer. Threading one through `advance`, `step_to`, `revision_to` and `traced` is about 40 lines, and `GateEvidence::observed` is where the digests go. A delayed channel holds `delay / (period × window)` readings in each observer checkpoint, so a 45 min delay on a 4 ms grid holds about 16 MB.
