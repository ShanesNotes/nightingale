# 94: The certificate folds as the run goes

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 5, [record 0008](../../../docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md), `crates/ng-validate/src/certificate.rs` (`certify`) and `crates/ng-record/src/history.rs` (`ConservationView`)

Type: task

Status: resolved

Blocked by: none

Blocks: [native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)

**Why.** The first milestone (SRS invariant I19) holds the whole body for 48 hours under its certificate, and that run cannot keep every ledger chunk. Record 0008 puts every chunk of one 48 h course at 296 to 342 GB, and 74 to 86 GB compressed. Those are the slice's figures, by arithmetic from short windows. The whole body's links are larger, so its course would cost more (inferred). So the certificate becomes a fold: it reads each step a session accepts, judges each sliding window when the step that holds its end is accepted, and gives the certificate `certify` gives on the complete history, bit for bit. Record 0008 and the contract's section 5 state the fold, `crates/ng-validate/src/certificate.rs` holds none, and no open ticket owns it.

Ruled at design. The fold gets its own ticket, and its evaluator, the first of the two PRs below, lands before native-contract 85's windows between edges. Those windows are built on that one evaluator, so `certify` and the fold cannot differ.

**What main already has of record 0008.** The seal chain: each accepted step's link chains the seal before it, the revision, the image's digest and the ledger chunk's digest (`linked` in `crates/ng-record/src/history.rs`), and a session queues each link as the step is accepted and offers the queue to its `HistoryStore` in order, stopping at the first link the store refuses, which waits with those after it for the next offer (`keep_waiting`). `restore`, behind `resume`, `rewind` and `recover`, walks that chain from the checkpoint's seal back to admission and reparses every link, and takes the run's opening time from the oldest link it reads. At admission's own checkpoint that walk reads none, and the view has no span until a step is accepted. The checkpoint carries the ledger's opening values. A session lends a `ConservationView` at any revision (`Session::conservation`), bound to its run, branch and base seal. `certify` checks that view's chain once newest first, then reads its segments oldest first (`ConservationView::in_time_order`).

What main does not have: the fold, a view the record mints at admission, the witness, a trimmed `restore` that takes the opening time from the record's opening, the rolling day of links and the flight recorder. This ticket builds the fold and the view it reads. The rest of the record waits for [native-contract 95](95-the-record-trims-its-history.md), which trims the history.

- [x] One evaluator, in the order the steps were accepted, sits behind `certify` and the fold. `certify` feeds it the complete history from admission and then finishes it, so the two share every line that judges a window
- [x] Every policy with sliding windows gives the certificate it gives on main, bit for bit. The slice's model hash and pins, the arena's scores and the milestone protocol's certificate stay, and the PR moves no pin
- [x] The independent oracle gains the closure's row-rounding allowance first. Today a policy that admits row rounding passes `certify` and fails the oracle's forward walk with a closure failure, so agreement with the oracle cannot show parity for such a policy. The test below is red at main `446e29f9`, with `left: Err(Closure)` and `right: Ok(())`, and is green once the oracle has the allowance
- [x] Before the old walk goes, the evaluator matches it and the independent oracle (`crates/ng-validate/tests/certificate_oracle/`) on every window shape, on passing runs and failing ones: shifted starts, endpoint rounding, which failure comes first, and the earliest window on a tie
- [x] The fold reads one session through the view the record mints at admission, so no host can feed it two runs. `resume`, `rewind` and `recover` rebuild it from the verified prefix, and a child branch gets its parent's prefix and none of its parent's later windows
- [x] A window whose start lies 24 h before its end is judged when the step that holds its end is accepted, shown by a test at that lag
- [x] What the fold holds in memory over 48 h, as a bound in bytes that a run shows, and the slice's cost for each accepted step beside main's on [body 26](../../body/issues/26-cost-budget.md)'s cost protocol
- [x] A mutant row for each rule above

The oracle's test, in `crates/ng-validate/tests/certificate_oracle/mod.rs`:

```rust
#[test]
fn the_forward_oracle_accepts_the_same_row_rounding_as_certify() {
    let end = SimTime::from_ns(6 * 3_600 * 1_000_000_000);
    let start = [600.0, 424.0].map(|amount| Measure::new(amount).expect("a start"));
    let case = TwoReservoirs::delivering(start, Vec::new())
        .expect("a body")
        .until(end);
    let (store, core) = (MemoryStore::default(), ng_solver::Core::new(100_000_000));
    let session = case.admit(&core, &store).expect("admitted");
    let Ok(session) = run_until(session, case.end()) else {
        panic!("the run faulted");
    };
    let view = session.conservation();
    let span = end.nanoseconds();
    let mut ledger = Ledger::sliding(view.basis(), span / 2, span, 0.0, 1e-12, 1e9);
    ledger.policy = ledger
        .policy
        .admitting_row_rounding(case.body(), 7)
        .expect("a policy");
    let passed = certify(&view, &ledger.policy).expect("within seven last places per row update");
    assert_eq!(passed.results()[0].closure_last_places(), 2_268_021);
    let oracle = in_time_order(&view, &ledger)
        .map(|_| ())
        .map_err(|failure| failure.check);
    assert_eq!(oracle, Ok(()), "the forward oracle must accept the same policy");
}
```

At main `certify` passes the policy that admits seven last places a row update, at 2,268,021 last places, and the forward walk refuses the same run.

## The two PRs

Ruled at design.

- **The first, the evaluator.** One evaluator that reads accepted steps in time order, on a checked forward walk of a complete history that `ng-record` lends. `certify` runs on it, the independent oracle gains its row-rounding allowance first, and the backward walk goes once the evaluator matches it. It closes the oracle box, the bit-for-bit box and the parity box, and the `certify` half of the first box, each with its mutant row.
- **The second, the fold on a session.** The remaining boxes, the view with `resume`, `rewind` and `recover`, the 24 h lag, and memory and cost, each with its mutant row, and the fold half of the first box.

The reason: native-contract 85's first PR needs only the evaluator, and the two halves fail in different ways. The evaluator fails in arithmetic and order. The fold fails in reading history while a run goes and in a session's life.

**Non-goals:** trimming the history, windows between edges ([native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)), and any change to `RecordFormat` or a seal.

## What the first PR built

- **The evaluator.** `Windows` in `crates/ng-validate/src/certificate.rs` takes the segment that holds the next window start and the one that holds its end, each with its revision. It judges every piece the two reach and returns its `Need`: the trailing segment, the leading one, or `Finish` when no window is left or one failed closure. It keeps each quantity's worst window and each held route's largest flow itself, since it alone sees the windows in time order. Of equal values the earliest stays, and only a strictly larger one replaces it. A held route's flow that is not finite has no rule of its own: it cannot reach a certificate that passes, because the route's quantity fails closure in the same window. `certify` feeds it the complete history from admission and finishes it. The row judge, `balance`, takes one window's two inventories, its inflow and outflow and its row-update count, apart from the way a piece restricts its curves. The `certify` half of the first box is done. The fold half waits for the second PR, so the box stays open.
- **The walk.** `ConservationView::in_time_order` walks the chain once from the head to admission and checks each link as `newest_first` does, keeping a mark every 64 links: the page's newest link and its oldest segment's start. `TimeOrder::from` gives a cursor that reads one page of 64 links at a time and serves it oldest first. A cursor holds at most 64 decoded steps, whatever the window length or step density. `certify` uses two cursors and reads each link at most three times.
- **Parity.** At the commit where both walks ran, every call of `certify` in `ng-validate`'s tests ran both and compared the whole result bit for bit: 111 calls from 50 tests, and none differed. That covers the shapes of the fourth box. The backward walk is gone, and `crates/ng-validate/tests/certificate_shapes.rs` holds the shapes through the oracle.
- **The oracle** works out the row-rounding allowance from the policy's inputs, with its own count of row updates and steps.
- **Mutant rows.** Each reachable rule of the old walk's order of judgment has a row: a coverage failure outranks a bad link, the earliest failure wins, closure outranks drift, a tie keeps the earliest window, both pieces judge a shared endpoint, shifted starts are kept, a history error outranks closure, and the walk refuses a bad link. Two rules have none, because no history reaches them: a walk that runs out gives `Coverage`, and a `Coverage` failure inside one piece ranks with closure. The chain is checked whole before either cursor reads it, and the head always reaches past the last window, so neither cursor runs out. Each piece's windows lie inside the two segments it is given, so no piece fails `Coverage`. The second, third and fourth boxes are done with their rows. The last box stays open until the second PR's rows land.

## For the second PR

- **A larger held flow replaces the one held, and no row holds it.** `Windows` keeps each held route's largest flow by `if !flow.at_most(held.0) { *held = (flow, interval); }` in `crates/ng-validate/src/certificate.rs`, so only a strictly larger flow replaces the one held. The row `held-support-tie-keeps-the-earliest-window` holds the tie, and no row holds the replacement; main before nightingale#210 had none either. The check of nightingale#210's last round found a probe that never replaces (`if false && ...`) caught by the slice certificate's comparison with the oracle, so a row is cheap.
- **`Rhythm::finish` consumes its state.** From the check of nightingale#213: `Rhythm::finish(mut self, head)` measures the tail with `self.stretch(self.last_close.unwrap_or(self.admission), head)`, which writes `widest` and `stopped`, and `Windows::finish(self, ...)` consumes the evaluator and calls it. `certify` rereads the history from admission whenever a rhythm is named. A fold that gives a certificate at every head must carry the open queue, `since`, `last_close`, `widest`, each quantity's worst, the count of windows judged (`judged`) and the first stopped and first drifted failures (`stopped`, `drifted`) forward, with a stop taking precedence over a drift as `finish` gives it (`self.stopped.or(self.drifted)`), and compute the tail and the result without consuming them, by reference or on a copy.
- **A store that loses a link during a walk.** `HistoryStore`'s docs say a store that loses a link fails the next walk that needs it, by that link's seal. The check of nightingale#210 compared the old evaluator and the new on stores damaged before a walk, and they report the same missing seal. It did not compare them on a store that loses a link while a walk runs. The break-it pass wrote three such tests in `crates/ng-validate/tests/transaction.rs`, each with a store that gives a marked link once and then loses it, and ran them at main. The first lost a link the walk had already passed (revision 65 of 66, under a window of the first second): it expected a missing-history failure, and main gave a certificate. The second lost a link the walk still needed (revision 1 of 2, under one 120 s window): it expected a certificate, and main gave `HistoryMissing` by that link's seal. The third lost two links (revisions 64 and 128 of 129): it expected the newer link's seal, and main reported a single seal, not the newer one. They were ruled outside the first PR, since the docs do not say what a walk reports when a link is lost during it. A fold on a session reads a store that the running session writes and that [native-contract 95](95-the-record-trims-its-history.md) trims, so this PR states what a walk reports in each of the three cases and holds each by a test.
- **Coverage before a lost link.** `certify` tests the span's coverage before it reads the chain. So one damaged four-hour history fails by the lost link's seal under a coverage of four hours and by `Coverage` under five (`a_history_shorter_than_its_coverage_fails_coverage_before_a_bad_link_is_read` in `crates/ng-validate/tests/certificate_shapes.rs`). The fold reproduces both outcomes.
- **The body's history does not fit in memory.** Measured on main's whole body before native-contract 85's second PR: each accepted step's link is 131,920 bytes and the body accepts about 194 steps a simulated second, so a store in memory holds 10 GB after about 390 s of the run. A disk store of compacted links took 37 GB for 2,900 s, and certifying its 562,271 links took 828 s, 1.47 ms a link. So until the fold lands, a certified run of the body lasts a few minutes in memory, or as long as a disk has 46 GB for each simulated hour. The box on what the fold holds in memory is stated beside these.
- **The earliest failure does not always win.** The first PR's rule is that the earliest failure wins and closure outranks drift, and section 5 has a window judged when the step that holds its end is accepted. `pair` in `crates/ng-validate/src/certificate.rs` judges together the windows whose two ends lie in one pair of steps, and it loops over the quantities before the starts. So where a later window of an earlier quantity and an earlier window of a later quantity both drift, it reports the later window. The oracle loops the same way, so the two agree and neither catches it. A break-it pass on main after the first PRs merged wrote the test below, and it fails at main: under sliding windows the certificate reports `Drift` for 0.25 to 10.25 s, where the window from 0 to 10 s, judged alone, drifts. Any failure still fails the run, so it held nothing. Section 5 ranks closure anywhere above a fixed window's drift, and `crates/ng-validate/tests/certificate_shapes.rs` holds a later closure above an earlier drift: that stays. This PR has the earliest window win among fixed windows' drifts, then the basis's order, in the evaluator and in the oracle by its own arithmetic. The test below compares two drifts and holds that much, and the PR adds the mixed case among windows judged together: a later closure over an earlier drift.

The test, as a patch to `crates/ng-validate/tests/rhythm_certificate.rs`:

```diff
diff --git a/crates/ng-validate/tests/rhythm_certificate.rs b/crates/ng-validate/tests/rhythm_certificate.rs
index 53db2cb77061aa49932fe12d7cbfc74e5c186ba6..53cf51bacff906bec608d9279b8f3a4467dbcac3 100644
--- a/crates/ng-validate/tests/rhythm_certificate.rs
+++ b/crates/ng-validate/tests/rhythm_certificate.rs
@@ -987,6 +987,29 @@
 }
 
 #[test]
+fn fixed_windows_report_the_admission_drift_when_another_quantity_drifts_one_step_later() {
+    // Native-contract 94 says "the earliest failure wins, closure outranks drift".
+    let body = breathing(true).expect("compiled");
+    let course = Course {
+        period: 4 * SECOND,
+        leak: Some((Leak::Oxygen, 30.0)),
+        leaking: vec![(0, LENGTH)],
+        held: vec![(0, 30 * SECOND)],
+        end: 30 * SECOND,
+    };
+    running(&body, &course, |view| {
+        let judged = |coverage| {
+            let ledger = Ledger::sliding(view.basis(), LENGTH, coverage, 1e-6, 1e-9, 4.0);
+            let failure = certified(view, &ledger).expect_err("the admission window drifts");
+            (failure.check(), failure.window())
+        };
+        let first = (CertificateCheck::Drift, Some(interval(0, LENGTH)));
+        assert_eq!(judged(LENGTH), first, "the admission window judged alone");
+        assert_eq!(judged(view.span().expect("a span").length_ns()), first);
+    });
+}
+
+#[test]
 fn a_fixed_drift_outranks_a_stopped_rhythm_which_outranks_an_earlier_drift_between_edges() {
     let body = breathing(true).expect("compiled");
     running(
```

## What the second PR built

- **Earliest window before basis order.** `pair` in `crates/ng-validate/src/certificate.rs` and `in_time_order` in `crates/ng-validate/tests/certificate_oracle/mod.rs` now rank fixed windows judged together by earliest start first and then basis order, while keeping a later closure failure above an earlier drift failure. Covered by `fixed_windows_report_the_admission_drift_when_another_quantity_drifts_one_step_later` in `crates/ng-validate/tests/rhythm_certificate.rs` and `among_windows_judged_together_a_later_closure_outranks_an_earlier_drift_and_an_earlier_closure_wins` in `crates/ng-validate/tests/certificate_shapes.rs`.
- **Non-consuming snapshots.** `Rhythm::snapshot` and `Windows::snapshot` in `crates/ng-validate/src/certificate.rs` evaluate the rhythm tail and certificate without consuming or mutating `open`, `since`, `last_close`, `widest`, `worst`, `judged`, `stopped` or `drifted`. `Windows::finish` delegates to `Windows::snapshot`, and `CertificateFold::certificate` reads the certificate at any accepted head while the run keeps going.
- **Session-bound live view and fold.** `Session::conservation_feed` in `crates/ng-record/src/session.rs` mints a `ConservationFeed` bound to `(run, branch, origin, format)` and strict sequential `(previous, revision)` transitions, refusing foreign runs, foreign branches (even with a shared caller `BranchId`), skipped revisions and repeated revisions via `ConservationRefusal::{OtherRun, Gap}` without mutating state. `Head` holds `HistorySegment` directly so live leading steps borrow with zero clones. `CertificateFold` in `crates/ng-validate/src/certificate.rs` opens on a session (rebuilding from the verified prefix after `resume`, `rewind` or `recover` while isolating child branches), folds each accepted step through the same `Windows` evaluator as `certify`, releases trailing marks once fixed windows settle, and matches `certify` and the independent oracle bit for bit across window shapes (`crates/ng-validate/tests/certificate_fold.rs`). Compile-fail doctests in `crates/ng-record/src/lib.rs` and `crates/ng-validate/src/lib.rs` hold that no caller can push a raw segment into either type.
- **Lost links during a walk and coverage precedence.** `HistoryStore` and `TimeOrder::from` specify the mid-walk lost-link contract: a link lost after the initial check fails only if a cursor later reaches its page, and when links across several pages are lost mid-walk the oldest damaged page reached reports its newest lost link. Held by the three `GivesOnce` tests in `crates/ng-validate/tests/transaction.rs` and the live-fold coverage-before-lost-link and mid-run lost-link tests in `crates/ng-validate/tests/certificate_fold.rs`.
- **24 h lag, memory bound and step cost.** `a_window_at_24h_lag_is_judged_on_the_step_that_holds_its_end` tests both one-window and sliding 24 h lag. `CertificateFold::memory_bytes` holds at most `65 * segment_bytes + ceil(steps_in_window / 64) * 48 B + windows_state_bytes` (`O(1)` in total run duration once `span > window_ns`; one-window releases trailing to under 4 KiB). Measured on `TwoReservoirs` over 48 h (24 h window, 60 s steps): peak under 64 KiB, plateauing at 24 h. Measured on the slice (`arena_body`, 1 s window over 3 s): exact peak 2,304,888 B (2.20 MiB) with a strict plateau (`peak_3 <= peak_2`). Projected over 48 h with a 24 h sliding window: about 14.9 MB (`<= 27.5 MB` `VecDeque` cap) on the slice and about 21.2 MB (`<= 33.8 MB` cap) on the whole body, against about 4.4 TB of raw links. Pre-sizing `History::linked`, `compact` and `expand` buffers via `LedgerChunk::byte_len` leaves step cost unchanged or slightly lower on [body 26](../../body/issues/26-cost-budget.md)'s cost protocol: on `beelink` beside main `9f5ba6ce`, `cost slice` moved from `800.2 M` to `799.5 M` user instructions a simulated second (`-0.1%`, `-0.7 M`, `steps=11614`, `solves=14749`, bit-identical seal `29070aac7d8f758e`). No body or slice pin moved.
- **Mutant rows.** Fifteen new rows live in `tools/gate/mutants.d/nc94-live-fold.toml`, covering the held-support replacement rule, earliest-drift and closure-over-drift ordering in `pair`, non-consuming rhythm snapshots, foreign-run/foreign-branch/gap refusals, prefix replay on `open`, coverage-before-lost-link precedence, trailing release, one-window boundary clamping, and basis/undeclared-rhythm checks.

## Answer

`CertificateFold` in `crates/ng-validate/src/certificate.rs` folds a run's certificate step by step through the same `Windows` evaluator that `certify` uses, reading each accepted step from a `ConservationFeed` minted by `Session::conservation_feed` in `crates/ng-record`. It matches `certify` and the independent oracle bit for bit at every intermediate head without consuming its state, rebuilds cleanly from the verified prefix on `resume`, `rewind` and `recover` while isolating child branches, judges windows at 24 h lag on the step holding their end, holds `O(1)` memory in run duration (exact measured slice peak 2.20 MiB for a 1 s window; projected ~21.2 MB on a 48 h whole-body run under a 24 h window), and moves no pin or step cost (`-0.1%` on `cost slice`). [Native-contract 95](95-the-record-trims-its-history.md) is unblocked to trim the rolling history store.
