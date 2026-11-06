# 20: A split is measured from accepted history

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The crates build split measurement as the contract lays it out: `PatientRecord::split_trace` draws a trace from accepted history, and `SplitMeasurement::qualify` is the only producer of a `SplitQualification`. It scores the four-family metric (state, exchange integral, observable, event time) against the reference integrator. §7's split gate then reads it, and runs under measurement are never scored.

Blocked by: None (can start immediately)

Status: resolved

- [x] A split qualification comes only from a measurement over traces from accepted history, scored against the reference integrator, and the sketch's two split probes hold in the crates

## Answer

A split solve is now trusted only on a measurement. `ng-contract` holds split measurement, `ng-record` reads accepted heads into it, and `ng-validate` runs §7's split gate against the reference integrator. All values are synthetic.

- **Traces from accepted history.** The history store keeps each step's ledger and no row's past value (ticket 11), so a trace is drawn while the run goes. `PatientRecord::split_trace` opens a `TraceDraw` on a head, then reads each accepted head with the receipt of the step that made it. The draw takes only what `verify` made: the head's state and the step's `StepExtents` (its inputs, gross extents and firings). It refuses a head that does not follow the seal it read last, a head of another run and a receipt of another head. Only `TraceDraw::finish` makes a `SplitTrace`. The families are every amount and memory row, every column's net extent since the grid's first instant, every lens channel, and each firing's time.
- **The only producer.** `SplitMeasurement::qualify` normalizes each error by its family's scales at the reference's value. It adds the worst family to the reference's own error, and it refuses a total over one (`OverBudget`) or a reference error over its share (`ReferenceTooCoarse`). `SplitMeasurement::new` refuses traces of another patient or seed, grid, lens, input schedule or event set, a run compared with itself, and a reference error measured on another reference. A trace binds the patient by its bound values, because the packet hash also covers each core's method history at admission.
- **The split gate.** `split_gate` draws the reference integrator's trace, a second at half its interval for the reference's own error, and the measured core's trace. It returns the qualification and nothing else, so no run it measured reaches a score. The tests are in `crates/ng-validate/tests/split.rs`. Measured with a throwaway probe on the stepped inflow case and a 7 s grid: a coupled core at 16 s against the 8 s reference errs by 1.87 in each grid family, against the reference's own 0.93. The ratio is 2.02, and the test asserts it lies between 1.9 and 2.1 for each family. At scales of 10 the core qualifies with a total of 0.28. At 256 s it errs by 19.85 and is refused (total 2.08). On the ectopy case a 0.2 s core places firings up to 4.9 ms from the 0.1 s reference, whose own error is 2.5 ms.
- **Probes.** Bytes as a qualification (E0308), a qualification written by hand (E0451) and step extents written by hand (E0451) are in `crates/ng-contract/src/lib.rs`. A trace from invented samples (E0599) is in `crates/ng-record/src/lib.rs`. `gate.py codes` passes on all four.
- **Mutation sweep.** 22 rows, one per rule, against the split tests: `~/lanes/logs/ng-nc-11-24/20/mutants-20.toml`.
- **Not built.** `EvaluationPlan` holds `Coupled` and `QualifiedSplit`. The sketch's `UnderMeasurement` waits for the block seam, where a run's declared resolution can say it is under measurement.
- **Limit.** A crate that admits an image can run `verify` outside a record, because the pending inputs' `attempt` is public, and feed a draw verified heads that no record validated. The record pairs each accepted head with its own step. Nothing in this crate graph can make the record the only caller of `TraceDraw::read`.

## Comments

From 03: the sketch's probes at `docs/architecture/solver-contract-sketch/src/lib.rs:218` (a split qualification is not bytes, E0308) and `:232` (no split trace from invented samples, E0599) are rows of §7's must-fail table. No ticket scoped split measurement itself, which both of them need. Ticket 03's third box stays open on them.

From 04: block scope is additive in the sketch. `evaluate_block`, `linearize_block` and `quadrature_for` sit beside the whole-body calls, so a whole-scope core never changes.

From the split of ticket 11: the split gate judges solver-B (contract §7). Whether solver-B passes the full metric without dt-aware organ updates and incomplete sweep acceptance is a question only the arena answers (foundation 07).
