# 13: The arena run reports its numerical health

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** `RunEvidence` carries a `NumericalHealth` report beside the certificate, as the sketch has it (`docs/architecture/solver-contract-sketch/src/ng_validate.rs:348`). The report counts refusals by reason and by refusal source, faults at the minimum interval and validity excursions. A step's receipt and the refusals a `Session` records feed it.

Blocked by: 12

Status: resolved

- [x] A short slice window's `run` reports refusals by reason and by source, floor faults and validity excursions, apart from the certificate

## Answer

`RunEvidence::health()` returns the run's `NumericalHealth`, beside `throughput()` and never in the certificate. `ArenaFailure::Faulted` is now `{ report: Box<FaultReport>, health }`, so a faulted run reports its health up to the fault.

- **A short slice window's `run` reports refusals by reason and by source, floor faults and validity excursions, apart from the certificate.** In `crates/ng-validate/tests/throughput.rs`, `a_short_slice_window_reports_what_its_session_refused_once_per_repeat` runs the slice's 2 s from 06:55 on the reference core at 5 ms, twice. It steps a second session from the same start through `Session::step` and `retry`, outside `run`. The two agree on every refusal reason and count and on the excursion count, with no floor fault. Measured once on that window: 455 accepted steps, 110 refusals, all `Accuracy` (70 on row 8 and 40 on row 41), and no excursion, so the one-vessel test below is what proves the excursion count. `a_slice_window_that_faults_at_the_floor_reports_its_health_up_to_the_fault` plants a core that refuses every attempt from 50 ms after 06:55 on, naming two row sources in turn. The run faults with `NoAdmissibleStep`. Its health holds one floor fault, the planted refusals per source in `refusal_sources` order, and the earlier natural refusals.
- `refusals()` lists each `NumericalRefusal` with the row or source it names, in the order each first refused. `by_source()` derives the `TrialDomain` refusals per `SourceId`. A reason that names a row, such as `Accuracy`, is not a refusal source's set or domain, so it is counted by reason only. The sketch keyed `by_source` on `RowId`, and it now keys on `SourceId`.
- A floor fault is `NoAdmissibleStep`. The contract cannot tell a refusal at the floor from one past the retry budget, because `retry_budget` is crate-private, so the count includes both, as the glossary says. Another fault, such as `BindingMismatch`, is no floor fault (`a_fault_off_the_floor_is_no_floor_fault`).
- Excursions count each `ValidityExcursion` on each accepted step's receipt. A one-vessel body whose trusted range leaves out its zero flow has one per accepted step over each of three repeats.
- Health is counted over one repeat. A repeat whose health differs from repeat 0's is `RepeatDiffered`, even when it ends on the same seal. In `a_repeat_that_refused_for_another_reason_differed`, a core refusing once at 500 ms for the same reason in each repeat runs clean, and the same core naming another reason in repeat 1 differs. So one repeat's counts stand for all of them by check, not by assumption.
- `tools/bench`'s `arena` mode prints the health under `health` lines.
- Not in this ticket: listing the recorded rate classes (the comment from 09 and 10), and telling a floor fault from a budget fault.

## Comments

From 04: `NumericalHealth` is not built. A receipt's excursions and a `Session`'s refusals feed it.

From 10: a stage past a guard is `GuardInside`, a new refusal that the report counts.

From 09 and 10, on finding 17: a step stops at the body's tightest rate class and at a forcing's knot, and the record enforces both, so the report can list the recorded classes. An unmoved row's rate class is the slowest band with no limit, so a list of longest intervals skips it.

From the engine bench (nightingale#31): on the slice at 5 ms, 29 percent of attempts were refused and an accepted step took 20 Newton updates, so the refusal counts are worth reading beside throughput.

From ticket 12: `NumericalHealth` joins `RunEvidence` beside `throughput` (`crates/ng-validate/src/run.rs`). `run` steps through the crate's `advance`, which retries each refusal and keeps no count, so the report needs a loop that counts them. Every repeat must end on repeat 0's seal, so one repeat's counts stand for all of them.
