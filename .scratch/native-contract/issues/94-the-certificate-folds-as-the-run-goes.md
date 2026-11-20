# 94: The certificate folds as the run goes

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 5, [record 0008](../../../docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md), `crates/ng-validate/src/certificate.rs` (`certify`) and `crates/ng-record/src/history.rs` (`ConservationView`)

Type: task

Status: open

Blocked by: none

Blocks: [native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)

**Why.** The first milestone (SRS invariant I19) holds the whole body for 48 hours under its certificate, and that run cannot keep every ledger chunk. Record 0008 puts every chunk of one 48 h course at 296 to 342 GB, and 74 to 86 GB compressed. Those are the slice's figures, by arithmetic from short windows. The whole body's links are larger, so its course would cost more (inferred). So the certificate becomes a fold: it reads each step a session accepts, judges each sliding window when the step that holds its end is accepted, and gives the certificate `certify` gives on the complete history, bit for bit. Record 0008 and the contract's section 5 state the fold, `crates/ng-validate/src/certificate.rs` holds none, and no open ticket owns it.

Ruled at design. The fold gets its own ticket and lands before native-contract 85's windows between edges. Those windows are built on the fold's one evaluator, so `certify` and the fold cannot differ.

**What main already has of record 0008.** The seal chain: each accepted step's link chains the seal before it, the revision, the image's digest and the ledger chunk's digest (`linked` in `crates/ng-record/src/history.rs`), and a session hands each link to its `HistoryStore` as the step is accepted. `restore`, behind `resume`, `rewind` and `recover`, walks that chain from the checkpoint's seal back to admission and reparses every link, and takes the run's opening time from the oldest link it reads. The checkpoint carries the ledger's opening values. A session lends a `ConservationView` at any revision (`Session::conservation`), bound to its run, branch and base seal, and `certify` walks it newest first.

What main does not have: the fold, a view the record mints at admission, the witness, a trimmed `restore` that takes the opening time from the record's opening, the rolling day of links and the flight recorder. This ticket builds the fold and the view it reads. The rest of the record waits for a ticket that trims the history.

- [ ] One evaluator, in the order the steps were accepted, sits behind `certify` and the fold. `certify` feeds it the complete history from admission and then finishes it, so the two share every line that judges a window
- [ ] Every policy with sliding windows gives the certificate it gives on main, bit for bit. The slice's model hash and pins, the pick protocol's scores and the milestone protocol's certificate stay, and the PR moves no pin
- [ ] The independent oracle gains the closure's row-rounding allowance first. Today a policy that admits row rounding passes `certify` and fails the oracle's forward walk with a closure failure, so agreement with the oracle cannot show parity for such a policy. The test below is red at main `446e29f9`, with `left: Err(Closure)` and `right: Ok(())`, and is green once the oracle has the allowance
- [ ] Before the old walk goes, the evaluator matches it and the independent oracle (`crates/ng-validate/tests/certificate_oracle/`) on every window shape, on passing runs and failing ones: shifted starts, endpoint rounding, which failure comes first, and the earliest window on a tie
- [ ] The fold reads one session through the view the record mints at admission, so no host can feed it two runs. `resume`, `rewind` and `recover` rebuild it from the verified prefix, and a child branch gets its parent's prefix and none of its parent's later windows
- [ ] A window whose start lies 24 h before its end is judged when the step that holds its end is accepted, shown by a test at that lag
- [ ] What the fold holds in memory over 48 h, as a bound in bytes that a run shows, and the slice's cost for each accepted step beside main's on [body 26](../../body/issues/26-cost-budget.md)'s cost protocol
- [ ] A mutant row for each rule above

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

**Non-goals:** trimming the history, windows between edges ([native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)), and any change to `RecordFormat` or a seal.
