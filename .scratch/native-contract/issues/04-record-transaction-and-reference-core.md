# 04: Record transaction with a backward Euler reference core

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** A session steps the two-reservoir body: the reference core solves, the quadrature and stage witness build the proposal, the record validates and publishes or refuses, and the certificate closes over the run.

Blocked by: 03

Status: resolved

Claimed by: claude

- [x] The two-reservoir case with a stepped exterior inflow matches its closed form within the accuracy policy
- [x] A forward Euler core with h > RC closes every ledger and is rejected by the accuracy gate
- [ ] A forced refusal leaves the accepted image, pending inputs and stream positions byte-identical; the retry applies the delivery once. Answered, not ticked: the accepted image and pending inputs are byte-identical and the retry applies the delivery once, but stream positions are an empty list until ticket 06, whose extra-evaluation check carries that part
- [x] A core that books one sample while its endpoint came from other stages faults with `MethodMismatch`
- [x] Compensated amounts reach the certificate; closure holds over every sliding window of the run

## Answer

A session now steps a manufactured two-reservoir body from admission to a certificate. A core can only propose. The contract replays every stage on the record's own values and says whether the declared method was followed, and the record decides whether the result may publish. `ng-record` holds the accepted image, the session latch and the ledger. `ng-validate` holds the fixture body, a backward Euler reference core, the certificate and the accuracy gate. `ng-contract` gains the evaluator, tickets, packets, admission, the quadrature and `StepProposal::verify`. All values are synthetic. The tests are in `crates/ng-validate/tests/transaction.rs`, and a test name below is in that file unless marked.

- **Closed form.** `the_stepped_inflow_matches_its_closed_form_at_first_order_over_three_refinements`. The gate compares accepted dense output with the closed form at each grid instant and segment end: both amounts, the circulation exchange and the gross inflow. Worst errors are 3.69, 1.84 and 0.92 at 16, 8 and 4 s, against a limit of 5, and the adjacent ratios are 2.00 and 2.01 inside a 1.9 to 2.1 band.
- **Forward Euler past RC.** `forward_euler_past_rc_closes_every_ledger_and_fails_the_accuracy_gate`. At 1.02 RC every ledger closes over 48 h while the difference grows. The gate rejects the core at the first grid instant inside the first step, at 100 s, with an error of 9.365 against 5 and both amounts positive, so positivity plays no part.
- **Forced refusal.** `a_forced_refusal_changes_nothing_and_the_retry_applies_the_delivery_once`, once with the record refusing an unsolved stage and once with the core refusing. Every digest from `PatientRecord::digests`, recomputed from the live fields, is equal before and after. The retry applies the delivery once, the next step applies nothing, and the total received equals the amount.
- **One sample.** `a_core_that_books_one_sample_while_its_endpoint_came_from_other_stages_faults`. The core declares Heun and books one stage at weight one, so `verify` faults with `MethodMismatch`. The session latches and the digests do not move.
- **Compensated amounts and every window.** `closure_holds_over_every_window_and_drift_tells_a_closed_body_from_an_open_one`, `the_worst_window_can_start_inside_a_segment` and `the_certificate_reads_compensated_amounts_where_a_plain_ledger_loses_the_exchange`. Window starts merge every segment boundary and its 24 h shift, so a day's change is linear between them and each piece is bounded by its two ends, with no sampling. The worst drift of a stepped body falls inside a segment, at 30,000 s between the 29,960 and 30,030 grid points. On a tiny exchange the test computes a plain `f64` ledger beside the compensated one, and the plain ledger fails the same limit.

`streams` digests an empty list until ticket 06, so in the forced refusal it proves only that the field is read. That is why the third box is answered and not ticked.

No expect-test trace was added. AGENTS.md names physiology slices, and this is numerics on a manufactured body. The nearest thing is `the_routines_bits_are_the_profiles` in `scheme.rs`: a digest of the pinned routines' bits over a fixed grid, tied to `ArithmeticProfile::contract_v1`, so a changed bit fails until it is a new profile.

**Step 0, on the base.** `bash tools/verify --full` printed `verify: OK` and the pre-push hook printed `verify: OK for changed paths only`, both from clean clones of `11680b3d`. `codex review` on ticket 03's code, gpt-6-astra at max effort, raised two P2 findings and both are fixed here in `e6c0b22`. `match_each_mode` was exponential in mode variables (test time 0.02, 0.08, 0.36 and 1.77 s at 12, 14, 16 and 18 variables), and it now matches rows that share no unknown apart and counts values with equal reads once (`crates/ng-contract/tests/matching.rs`; 3 of 3 mutations caught). `gate.py codes` exited 2 with no rustup default and an absolute `CARGO_TARGET_DIR`, and each probe now carries the pinned toolchain.

**The arena.** A pstack `architect` arena with an Opus runner and a Codex runner, judged blind by Codex. All three ran at max effort, Codex as `gpt-6-astra` and Opus as `claude-opus-5-5`, and `architect/models.md` in the evidence directory cites each log. The judge scored 21 to 16 for the Codex candidate and shares its vendor. My own read was close to a tie, and both reads reach one end state: the Codex candidate's transaction shape with the Opus candidate's method boundary and smaller surface. Two judge findings were rejected. Replacing the whole record in `publish` needs shared history, while `Vec::push` aborts on exhaustion and cannot leave a partial write, so `publish` stays infallible. Marking drift `Unavailable` was rejected because the glossary's certificate includes drift, no ticket scopes adding it later, and the walk computes it at no cost.

Choices:

- **Proposal boundary.** `StepProposal` is owned and holds no problem, and `proposal.verify(&problem)` runs on a problem the record built from its own fields. The judge's blocking finding was a core that redirected a proposal at a problem of its own and compiled. `CoupledProblem::new` stays public because it reaches nothing, and that is what makes `OtherValues`, a recorded rate that differs, writable as a fixture.
- **The issued ticket.** `PatientRecord::validate(&self, expected, proposal)`. The session keeps the ticket it issued and the record compares it. `StepTicket::binds` re-issues the ticket and compares digests, because comparing base and identity alone passes a ticket read from a shadow pending list under the real base.
- **`Session<Retrying>`.** `Step::Refused(Session<Retrying>)` has `refused` and `retry` and no `step`. `docs/statecharts/engine.tla`'s `refused` phase exits only by `Retry` or `NoShorterInterval`, and the sketch's shape let a host drop a refusal and restart the budget. A compile-fail probe with a legal twin holds it, in the crate and the sketch.
- **One refuse path.** `ticket.refuse(reason)` gives a retry at half the length, or `NoAdmissibleStep` when the retry would pass the floor or the budget, so nothing accepts excess error at the floor.
- **Delivery, not a rate step.** `BoundaryInput::Water(WaterDelivery { boundary, amount, ends })`, at the rate that delivers the amount over the integer-ns duration. A rate step is idempotent, so applying it twice shows nothing. Applying a delivery twice shows as twice the total received.
- **Frozen drive frame.** A ticket carries the drives running at its start, and it ends at the earliest of its span, the next event and the next delivery's end. Every stage reads the frame, so a stage at the end reads the left limit by construction. A due input is applied when the interval starting at its instant publishes. `the_ending_interval_sees_the_left_value_and_the_next_the_new_one` runs adjacent deliveries at an odd-ns instant through end stages and retries.
- **Modes refused at bind.** `AdmissionError::ModesNotYetRun { variable }`, because a guard that silently never fires is worse than a run that never starts. Tickets 06, 08 and 10 lift it.
- **Closed schemes and exact weights.** `StageScheme::{forward_euler, heun, backward_euler_doubled}` over a private table, because a public tableau promised generality only one case tests. Weights are `Ratio`, so summing to one is an integer equality. Doubling is the whole interval at weight zero, then `floor(h / 2)` and the rest, which carry the increment, and 1 ns has one whole stage and no estimate. A second reference mode was dropped as surface.
- **Pinned routines.** Plain series, not fdlibm's table: Cody-Waite reduction and a degree 13 Taylor polynomial for `exp`, and the atanh series to `z^11` for `ln`. Measured over 290,000 points against 80-digit `decimal`, the worst errors are 0.849 and 0.802 ulp. Host libm on the same points gives 0.505 and 0.513. The arena's synthesis says 60 digits, which is an error; the script uses 80. The vectors in `pinned.rs` come from the same `decimal` runs.
- **History.** One private `LedgerChunk` per accepted step, read through `ConservationView`, with inventory and gross prefixes per combination and forward and reverse prefixes per column. Storage stays private, so ticket 05 can move to fixed-time chunks with no reader changing.
- **Drift is built**, with a synthetic limit standing in for 1.96 times the within-subject variation, and `ExpectedDrift::Zero` only. A test shows a closed ledger is not a stable body.
- **Reference core in `ng-validate`.** It is written on the public seam alone, so it proves the seam is enough, and the crate table's `ng-solver` stays the arena winner's home. The glossary and table now say so.
- **No decision record.** Nothing here is hard to reverse under governance section 4, since no second core depends on this surface yet.

Refusals, faults and what proves them. `transaction.rs` drives `Accuracy`, `IterationLimit`, `StageResidual` (rate, position and published algebraic row), `TrialDomain` (a row's set, a divisor, a logarithm, a non-finite coordinate, value and slope, and the published state), `NoAdmissibleStep` (floor and budget), `MethodMismatch` (structure, replay and restart) and `BindingMismatch` (schema, stale proposal, earlier attempt, swapped ticket, another ticket's refusal, an unissuable ticket, a foreign base, weights, another body and ticket arguments), plus every admission and gate failure by name. A sweep removes one check at a time in a copy and records which test notices: 121 of 122 die on the final code. The survivor, `accuracy: ignores the inflow`, stays because a ticket freezes the drive, so the gross inflow is exact for any step `verify` passes and that comparison only guards the frame itself. The first pass caught 94 of 104: eight survivors were real gaps, each given its own test, and two mutants had not applied. The last pass found one more, a test that passed for the wrong reason, because the root it used squared back to its target exactly. The final full pass ran at `8921d51`, and after the one test change that followed, the mutant it touched was rerun alone. The script and logs are in the evidence directory. The list is chosen, not exhaustive.

Found in review, not by the sweep: a core could finish a quadrature for one ticket and book that `Increment` for the next, and while the state moved less than the scales the record accepted it. `an_increment_finished_for_an_earlier_ticket_is_a_binding_mismatch` failed with the step accepted before the fix. An `Increment` now keeps its ticket's digest. The sweep only removes checks that exist, so it could not see this.

Departures from the synthesis, none of which changes a shape:

- `Session` turns a core's `Refused` naming any ticket but the issued one into `BindingMismatch`, because otherwise a core resets its budget through a fresh ticket. This adds `Refused::ticket` and `StepTicket::digest`.
- `problem.quadrature`, `problem.propose` and `workspace.point` are infallible and `Quadrature` has no lifetime, since `add`, `evaluate`, `verify` and `validate` hold the checks. `verify` opens with a layout check, because a foreign proposal would otherwise index out of bounds.
- `EngineFault` has no public constructor, and `StepTicket::mismatch` replaces it. `ReferenceIntegrator::with_floor` is new, because the floor test needs a floor the estimate can reach.
- `Ratio` takes `u64`, so a negative weight is unrepresentable. `Seed` is a plain struct until ticket 07 needs `DomainSeed<D>`.
- `QuantityId` carries its `Book`, and `Sliding24Hours::covering` also refuses exactly 24 h, which could slide nowhere. `CompiledBody` keeps refusal sources outside its layout, so they stay out of the model hash.
- The receipt is `StepReceipt`, the Opus candidate's name, because `Column::receipt` and the glossary's receipt pool already use the bare word. The sketch and contract follow.
- `Curve::on` returns `None` outside its segment, as `DenseSegment::sample` does, and `AccuracyEvidence` names its policy, as a `Certificate` does. A law undefined at admission is `InitializationFailure::Undefined`, by law. `AccuracyFailure::MixedEvidence` is new, and a policy refuses a case that never steps.
- `crates/ng-validate/tests/transaction.rs` is named by topic like the other test files.

Sketch and contract. Where the design changed, they now match the code: the proposal and `verify`, `validate`, `Session<Retrying>`, `refuse`, `Ratio`, closed schemes, `MethodHistory`, a derived `DenseSegment`, `Curve::on`, per-column `exchange`, `certify`'s results and `accuracy`. `Scope` is gone from the sketch, because a parameter that always says whole would break every core the day it says anything else. `evaluate_block`, `linearize_block` and `quadrature_for` sit beside the whole-body calls instead, additive as the arena meant. `CoupledProblem::transitions` now reads two stage evaluations, because a core holds no `Candidate`. Later tickets' types stay. Every existing compile-fail probe keeps its line, the refused-session probe follows them at line 499, and the sketch has 32 probes. The walk's steps 4 to 8, 11 and 12 exist as running behavior in `transaction.rs`.

| Sketch line | Rule | Code | Where |
|---:|---|---|---|
| 25 | A vital has no setter on the record | E0599 | `ng-record` |
| 54 | An engine fault is not demise | E0277 | `ng-contract` |
| 149 | The certificate cannot pair two records' histories | E0599 | `ng-validate` |
| 166 | A faulted session is latched | E0599 | `ng-record` |
| 177 | A lent record is read-only | E0596 | `ng-record` |
| 309 | Extents come only from the quadrature | E0451 | `ng-contract` |
| 499 | A refused session can only retry | E0599 | `ng-record` |

Model defaults, reported here and set in the validation crate or the fixture:

| Value | Setting | Measured or why |
|---|---|---|
| Accuracy limit | absolute 5, relative 0, grid 100 s | Reference errors 3.69, 1.84 and 0.92; forward Euler 9.365 at 100 s |
| Observed order band | 1.9 to 2.1 for a refinement by two | Ratios 2.00 and 2.01 |
| Reference scales | absolute 1, relative 1e-5 | Worst estimate 4.0 at 64 s on amounts near 5e5, under a scale near 6 |
| Closure floor | absolute 1e-24, relative 1e-9 | Worst closure exactly 0 on amounts near 1e6, 3.7e-30 on the tiny exchange |
| Newton | 8 updates, stop when an update is under 2^-49 of its value | By reasoning: eight ulp above rounding. Not measured |
| Floor and retry budget | 1 ms and 12 | Carried from the Opus candidate. Not measured |
| Forward Euler interval | 1.02 RC | Past RC; the lowest amount over 48 h is 207,121 |
| Drift limits in tests | 1e-6, 1.0, 1e12 | Fixture data, not policy (record 0004) |

Open points:

- **Speed.** A whole step costs about 40 µs on this two-row body in a release build. At a 2 ms step that alone is about 3,500 s for 48 h, against the 1,728 s budget. `chain` rehashes the whole image every step, so that part grows with the state. Ticket 11 measures this on a real body.
- **Storage.** A chunk per accepted step holds six compensated values per combination and four per column. At a 2 ms step that is about 19 GB over 48 h for this one body. Ticket 05's fixed-time chunks answer it.
- **A ticket cut below the floor** by a breakpoint faults at its first refusal.
- **A ramp** would need a frame with a slope, since a frame holds rates only.
- **The chord skips the doubled midpoint.** A two-piece segment would sample closer, and an algebraic row that reads a drive starts its next chord from the old drive's value.
- **`newton`'s stop** has an absolute term of 1, which assumes rows near unit scale. Row scales belong to tickets 10 and 11.

Review. `/correct` found two repeated classes. One is a value bound to one attempt reaching another: a refusal naming another ticket, a ticket read from a shadow pending list and the stale increment. A lifetime tying `Quadrature` to the issued ticket would not remove it, because a core can mint a ticket of its own inside `solve`, so the check stays at run time, and each bound value keeps its ticket's digest with a test that fails without the comparison. Every field of `StepProposal` now has one. The other is a comparison that reads false on a NaN and so passes: the certificate and the accuracy gate each read it, and both now state what must hold, so an overflow fails. `verify`'s defects cannot be a NaN while incidence coefficients are one, because a sum of finite rates reaches an infinity and never its opposite. `/blast-radius`: the safety fact is that this PR changes no compiled body. The two-reservoir model hash is the same at the base, after step 0 and at the head, by running it (`review/blast-radius-model-hash.log`). Every existing sketch probe kept its line, so later tickets' citations hold, and no SRS sentence repeats a glossary line changed here. `/no-comments` deleted 67 comments, trimmed 18 and corrected 18, and a script showed it changed no code. Of its seven reshape flags, four were fixed (`Curve::on`, the evidence's policy, the undefined law, `two_sum` in `Compensated::plus`) and two were renames. One was declined: a public rates reader would add a bare `f64` item to save a sign flip only test cores need. `codex review --base main` on the rebased branch, gpt-6-astra at max effort, raised six P2 findings, each with a probe that reproduced it, and all six are fixed: the certificate and the accuracy gate passed a NaN after overflow, Newton's stop relaxed algebraic rows on short steps, the closed form ignored a boundary's direction and counted a repeated delivery twice, the policy digest named only the body, and `observed_order` took a ladder of two policies. Two more passes over those fixes raised three findings and then one, all on the fixes themselves and all fixed: a stage returned before its last correction, no path for a solved stage with a singular Jacobian, a case that passed the gate by never stepping, and a last update nothing read. The last fix, a loop bound and its test, had no Codex pass of its own.

Verification. `bash tools/verify` printed `verify: OK` after the first rebase, and the PR carries the proof block for its head, on `3367cb53`. Apart from the Rust fixture, `review/closed-form-check.py` derives the closed form from the contract's equations in 60-digit `decimal` and reads accepted amounts from real runs. At 12,000 s the worst errors are 0.00077, 0.00037, 0.00019, 0.000093 and 0.000046 at 16, 8, 4, 2 and 1 s, with ratios from 2.04 down to 2.00, and forward Euler's first step equals the hand value, 499,480.

Evidence: `~/nightingale-evidence/nc04-record/` holds the step 0 logs, the arena (grounding, both candidates, the judge's report, the synthesis and `models.md`), the pinned sweep, the mutation script and logs, the cost measurement and the review logs.

## Comments

From 03: the compiled body keeps, private to `ng-contract`, what the reference core needs. That is each program's instructions with inputs by declared name (`law.rs`, `Op` and `Input`), and each instance's bound inputs (`body.rs`, `Bound`: a parameter slot, an unknown, or a schema boundary's drive in name order). It also keeps the parameter layout with default bindings, the modes, the guards and the validity ranges. Evaluation is this ticket's, with the pinned `exp` and `ln`, the square root inside smooth max and min, the derivatives and `CompiledBody::refusal_sources` (a divisor, a logarithm's argument and each row's admissible set). The two-reservoir body is `crates/ng-contract/tests/reservoirs.rs`. Its closed form under a stepped inflow is in the arena's base design (`~/nightingale-evidence/nc03-declaration/architect/candidate-codex/design.md`, line 78). A block is the strongly connected part of the row graph after matching, with the fastest rate class of its laws. It is structure, never permission to lag a dependency. Sketch probes this ticket owns: lines 25, 54, 149, 166, 177 and 309.
