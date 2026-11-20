# 85: The certificate's windows end on breath edges

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 5, and `crates/ng-validate/src/certificate.rs` (`SlidingWindows`)

Type: task

Status: open

Blocked by: 94

The first PR needs only [native-contract 94](94-the-certificate-folds-as-the-run-goes.md)'s first PR, the evaluator. When that PR merges, the line above becomes `none`.

Blocks: [body ticket 11](../../body/issues/11-breathing-is-controlled.md)

**Why.** The first milestone (SRS invariant I19) holds the whole body for 48 hours under its certificate, and the certificate is what refuses a slow leak. Once the body breathes, a ten-second window that does not hold whole breaths can catch most of a breath's swing in the gas the lungs hold, so [body ticket 10](../../body/issues/10-lungs-breathe.md)'s second PR set the bounds of the breath's books by that swing, to hold at any breathing rate. A bound that size lets through a steady leak far larger than the steady figures would.

**What the check of nightingale#198 found.** `SlidingWindows` takes only windows of one fixed length (`certificate.rs:78`), so no window can start and end on breath edges. The slowest steady leak each breathing bound refuses today, against the one a bound from the steady figure would refuse, per ten-second window:

- oxygen, 900 a second against 20 a second;
- nitrogen gas, 2,000 against 20;
- carbon, 50 against 9;
- vasopressin, 4e-3 against 3e-4. No other check sees vasopressin.

Fick's check over nine beats takes the blood's store change as slack (oxygen 0.88% of the rate, carbon dioxide 7.6%), because nine beats are not whole breaths.

- [ ] The certificate's windows can end on the run's breath edges, so each breathing book keeps a bound from its steady figure at any breathing rate
- [ ] A steady leak of each old size is refused at any breathing rate: oxygen 20 a second, nitrogen gas 20, carbon 9, vasopressin 3e-4, each shown at two rates, one that divides the window and one that does not
- [ ] Fick's window runs over whole breaths, with the store change as a signed term in place of slack
- [ ] When windows end on breath edges, the hydrogen bound at both paying beds, the coronary bed and the muscle bed, 0.2 each today, is restated. The muscle bed's is twice its worst ten-second change, 8.79e-2, of which the breath's ripple inside one breath is 6.59e-2, so the bound passes a steady leak up to 1.1e-2 a second there
- [ ] It restates the breath's books after [body ticket 11](../../body/issues/11-breathing-is-controlled.md)'s second PR, 11b, with the loop moving the rate, and its two-rate leak tests set the rate by the packet with the drive scale held

**Non-goals:** the breath's rate and its control (body ticket 11), and any bound outside the breath's books.

## The design

Ruled at design. The design keeps every book's fixed sliding windows and adds, for the breathing books, windows between breath edges with a tight bound. It lands as two PRs, the first after [native-contract 94](94-the-certificate-folds-as-the-run-goes.md)'s first PR, and the tests below hold it.

**The edge.** A breath edge is the glossary's: the accepted step end at which a guard's landing resets the breath's phase. The card declares which phase is a rhythm's, and the record reads the reset from the contract's own firing (`guard.resets` in `CoupledProblem::fired`), so a discarded trial, a retry or a stage gives no edge. The certificate reads edges and never learns what a breath is, and the same statement serves the heartbeat. One owner states the edge, and the one sealed history holds it.

Ruled at design: the policy names a rhythm's clock the way a paced cycle already names its clock, by `Slot` (`crates/ng-contract/src/packet.rs`, `paced`: a memory reset by its one rising guard, sealed in the packet and proved at admission), so there is one way to name a clock. `certify` resolves that name among the rhythms the record's own card declares and refuses, by name, one the card does not declare. The reason: a `Slot` is a law's name and its place and holds no model (`crates/ng-contract/src/body.rs`), so a slot read from another model's rows equals the target's own, and no slot can refuse it. A run shows it: a phase slot taken from another model passed admission and paced the target. What refuses foreign input on main is the container, a packet by its parameters' model (`AdmissionError::ForeignParameters`) and a policy by its basis digest. So no handle that carries a model is added, and one policy serves every model of its basis.

**The windows.** The policy states a settling time and a length. Every edge from the settling time on opens one window, which closes at the first edge at least the stated length later. An edge can close more than one window, and the evaluator judges each, oldest opening first. A window is judged on the values after the step that ends at each of its two edges, so a bolus booked by the opening step cancels and one booked by the closing step counts once. The windows between edges add a check and take none away: every book keeps its fixed sliding windows, because record 0004 holds drift over "any 24 hours", and only the sliding windows judge every 24 hours.

**The allowance.** Each breathing book's bound is a typed rate in its own units a second, and a window's allowance is that rate times the window's own length: the rate times its whole seconds, plus the rate's exact share of the rest. Every comparison is through `at_most`. So a steady leak is refused at the same size at every breathing rate and across a pause, to within the booked sum's rounding, and equality is no verdict (the margin rule in the tests).

**Coverage.** The certificate keeps the widest stretch with no edge, admission's and the tail's included, beside the close of the last window it judged, and the policy bounds that stretch. The tail is measured from that close and not from the last edge: with final edges at 51, 60, 61 and 62 s, a length of 10 s and the head at 62 s, the window (51, 61] closes and nothing closes at 62, so a tail measured from the last edge reads zero while the last second sits in no window between edges. A breath that stops is a named failure. From admission to the first edge (5.0004 s on the runs in "The bound") there is no window between edges: the fixed windows judge that stretch alone, and the certificate names it.

**The evaluator.** The windows between edges run in native-contract 94's one evaluator, in the order the steps were accepted, behind `certify` and the fold. The independent oracle (`crates/ng-validate/tests/certificate_oracle/`) gains the windows between edges and their allowance by its own arithmetic, which record 0008 needs before any history is trimmed, and it and the slice's pins check the evaluator. A body that declares no rhythm writes main's bytes: its policy digest, its certificate and its seals are main's, so the slice's pins stay. Each new field of a seal or a digest is named in it, as `seam.seal` holds.

**Fick.** Fick's check runs over whole breaths, with integrals and not means, each gas with its own sign for the store, since carbon dioxide runs the other way, and each residual through `at_most`. It states no windows of its own: the policy states the windows once, and Fick reads them from it. Whole breaths alone do not close today's form, the product of a mean flow and a mean oxygen or carbon dioxide content with each store read at the window's two ends, for any set of stores tried. Over two and four whole breaths from the 50 s edge, with no store term: oxygen +2.3e-2 and +8.2e-2 a second against a tolerance of 9.5e-3, and carbon dioxide -2.90 and -2.93 against 7.1e-3. With the systemic beds' own stores: carbon dioxide -5.0e-2 and +2.9e-2, oxygen 0.23 and 0.26. With every pool from the beds to the pulmonary artery: about 0.6 a second over in both gases. _Inferred, not run:_ the beds hold back 2.9 a second of the carbon dioxide they make, and the mean form overstates the transport because the windows cut beats, so the heart's and the great vessels' pools differ by 2 to 11 a second between the two ends. The form to build is settled by a run: flow times oxygen content, or carbon dioxide content, summed at each reading place over the accepted steps, with the pools between the two reading places as the store, signed as each gas runs. It is tried on a saved run before any tolerance is set, and the tolerance is then the residual that run shows, with its reason. Hydrogen at the coronary bed and at the muscle bed is measured and bounded apart, each with its own rate.

**The tests.**

- Each old size as an exact, typed leak on its own book (an element's book for oxygen and for carbon, the species' own for nitrogen gas and for vasopressin): oxygen 20 a second, nitrogen gas 20, carbon 9 and vasopressin 3e-4. Each at a breathing rate that divides ten seconds and one that does not, and each at admission, in a held breath and at the tail. A leak while the rate moves joins the whole-body tests (open item 3).
- The control: the same honest rhythm, at the rate that does not divide ten seconds, fails tight fixed sliding windows. It shows that the tight bound rests on the windows between edges.
- Two openings that mature on one edge, where only the older one's window refuses the leak. On the edges of the 300 s run in "The bound", with a settling time of 15 s and a length of 10.0001 s, the edge at 25.000569272 s misses the length by one nanosecond, so the openings at 15 s and at 20 s both mature on the 30 s edge. Under an exact oxygen leak of -20 a second the window (15.000469273, 30.000619272] s is refused at -20.375020435 a second, and a rule that judged only the later opening at each close refuses nothing. The two rules judge the same 54 windows at a settling time of 20 s and a length of 10 s; at 15 s and 10.0001 s each judges 54, and they share 50.
- After [body ticket 11](../../body/issues/11-breathing-is-controlled.md)'s second PR, the whole body with the drive scale held at one and the packet's breathing rate at 0.20 and at 0.25 a second: each old size as an exact, typed leak, each run beside one with no leak.
- The margin rule. No test sits a leak on its allowance: each states its margin both ways. At equality a constant law passes over 12 s and is refused over 10.3 s by the booked sum's rounding. The test below shows it. It fails at main `446e29f9` with `10300000000 ns: booked Compensated { hi: 0.00412, lo: 0.0 }, allowed Compensated { hi: 0.00412, lo: -2.0057740190981833e-19 }`, after the 12 s window passed. That failure is the reason for the rule, so the test does not join the suite as written. Where a book's bound equals its old size, as oxygen's (20) and carbon's (9) do in "The bound", the fixture states its own bound below the leak.

The margin rule's test, in `crates/ng-validate/tests/arena_certificate.rs`:

```rust
#[test]
fn a_steady_vasopressin_rate_at_the_edge_allowance_passes_at_both_lengths() {
    const SECOND: u64 = 1_000_000_000;
    let rate = 4e-4;
    let per_second = Compensated::of(&Measure::<Rate<Vasopressin>>::new(rate).expect("a rate"));
    let daily = rate * DAY_SECONDS;
    assert_eq!((daily / DAY_SECONDS).to_bits(), rate.to_bits());
    let body = steady_inflows().expect("compiled");
    for (step_ns, duration) in [(4 * SECOND, 12 * SECOND), (5_150_000_000, 10_300_000_000)] {
        let packet = packet(&[(Species::Vasopressin, daily)]).expect("a packet");
        let (core, store) = (ReferenceIntegrator::new(step_ns), MemoryStore::default());
        let boundary = BoundaryPort::new(Vec::new()).expect("no event");
        let session = admit_fixture(&body, packet, boundary, &core, &store).expect("admitted");
        let Ok(session) = run_until(session, SimTime::from_ns(step_ns + duration)) else {
            panic!("the run faulted");
        };
        let view = session.conservation();
        let quantity = view
            .basis()
            .ids()
            .find(|q| q.book() == Book::Of(Species::Vasopressin))
            .expect("vasopressin");
        let segments = view
            .newest_first()
            .collect::<Result<Vec<_>, _>>()
            .expect("complete history");
        let at = |ns| {
            let segment = segments
                .iter()
                .find(|s| s.interval().end() == SimTime::from_ns(ns))
                .expect("accepted end");
            let interval = segment.interval();
            let inventory = segment.inventory(quantity).on(interval).expect("own segment")[1];
            let received = segment.inflow(quantity).on(interval).expect("own segment")[1];
            (inventory, received)
        };
        let (open, received_before) = at(step_ns);
        let (close, received_after) = at(step_ns + duration);
        let change = close.minus(open).abs();
        assert_eq!(change.bits(), received_after.minus(received_before).bits());
        let fraction = Compensated::between(Compensated::ZERO, per_second, duration % SECOND, SECOND);
        let allowed = per_second.scaled((duration / SECOND) as i64).plus(fraction);
        assert!(
            change.at_most(allowed),
            "{duration} ns: booked {change:?}, allowed {allowed:?}"
        );
    }
}
```

## The two PRs

- **The first, after native-contract 94's first PR, the evaluator.** The card's rhythm statement, the sealed edge, the policy's windows between edges with their allowance and settling time, coverage, the compile-fail rows, and the fixture's tests: the exact leaks at two rates, each with its margin both ways, the two openings that mature on one edge, and the control. No body pin moves.
- **The second, after 11b.** Its first run is the control with no leak at each breathing rate (open item 6). Then the body's policy takes the windows, the bounds are restated from the steady rates measured on 11b's hour run, which keeps each breathing book's inventory at every breath edge, Fick runs over whole breaths in the form its run settles, hydrogen is bounded at each bed, and the whole-body leak tests run. The body's pins move once.

## Open, each settled by a run

1. **The sealed form of the edge.** A count in the ledger chunk's own cells on every step, with `compact` and `expand` untouched, or a mark of 9 bytes on the steps that hold an edge, with the link's parity as the switch and `compact` and `expand` split by it. Default: the count, because it leaves the history's format code alone and has no switch on parity. The run: the first PR's first step builds both on the fixture and keeps the one that moves no slice pin, passes `seam.seal`, and changes fewer lines of `crates/ng-record/src/history.rs`.
2. **The first 20 s.** A leak of 20 oxygen atoms a second confined to the first 20 s moves a fixed window by 200 against 9,000 allowed, and no later edge sees it. Default: the settling stretch gets windows between edges too, with its own stated rate. Measured at main `446e29f9`, over windows that open before 20 s against those that open from 20 s on: carbon 4.58 a second against 4.27, nitrogen gas 3.56 against 3.02, the coronary bed's hydrogen 8.49e-3 against 3.04e-3, the muscle bed's 4.61e-3 against 1.41e-3. Oxygen's and vasopressin's worst windows open later. So the settling rate is at most 3.3 times the later one. The run: the second PR's control at each rate states the settling stretch's rate on the body after 11b.
3. **A leak while the rate moves.** No design so far tested it. Default: it joins the whole-body tests after 11b. The run: the second PR's whole body under the loop, with an exact leak, beside the same run with none.
5. **Fick's form.** Default: the summed form "Fick" above gives. The run: that form on a saved 300 s run of the body, the length of the runs in "The bound", before any tolerance is set.
6. **The whole body's two-rate leak tests.** The fifth box sets the rate by the packet with the drive scale held. _Inferred, not run:_ away from the rate the drive balances, the body gains or loses carbon dioxide by the change in ventilation, so the run with no leak may itself break a steady bound. Default and run: the second PR's first run is that control at each rate. If it breaks the bound, the whole-body tests state their own bound from their own control, with the leak sized above the bound plus the control's rate, and the fixture carries the exact refusal at two rates.

Item 4, the name of the rhythm's clock, is ruled above under "The edge".

## Low notes

Each measured at main `446e29f9`, where body 11a merged.

- The second box holds as written only for a book whose steady rate after 11b is under a third of its old size. Any other book's box is restated with the leak's direction and the size refused both ways. A two-sided bound `b` of twice the steady rate `s` refuses a leak with the drift above `b - s` and a leak against it only above `b + s`.
- The 4e-3 that "What the check of nightingale#198 found" gives for the leak today's vasopressin bound passes is 5e-3 at the packet's bound.
- Vasopressin's 3e-4 in the second box is not met there: the rule gives 4e-4. From 100 s on the same rule gives 2e-4.

## The bound

Two whole-body runs at main `446e29f9`, 120 s and then 300 s: 58,327 accepted steps, 59 edges, gaps of 5.000026 to 5.000075 s, each edge a step end at which the breath's guard set the phase to exactly zero. One breathing rate. Every rate below was computed twice from the run's edges, and the two agree.

A window runs from an edge to the first edge at least 10 s later. `s` is the worst rate over the windows that open from 20 s on. `b` is twice `s`, rounded up to one figure.

| Book | Today's bound passes a leak of | `s` a second | `b` | Old size `L` | `L` with the drift | `L` against the drift | Refused both ways from |
|---|---|---|---|---|---|---|---|
| Oxygen | 900 | 7.49 | 20 | 20 | refused | passes | 27.5 |
| Nitrogen gas | 2,000 | 3.02 | 7 | 20 | refused | refused | 10.0 |
| Carbon | 50 | 4.27 | 9 | 9 | refused | passes | 13.3 |
| Vasopressin | 5e-3 | 1.55e-4 | 4e-4 | 3e-4 | refused | passes | 5.5e-4 |

What the figures decide:

1. **The allowance is a rate times the window's own length.** Every book's change grows in step with its window: oxygen 1 : 2.00 : 3.00 over windows of at least 10, 20 and 30 s, carbon 1 : 1.99 : 2.97. An amount sized on one length is wrong at every other.
2. **The windows do what this ticket wants.** The leak each bound passes falls from 900 to 20 a second for oxygen, from 2,000 to 7 for nitrogen gas, from 50 to 9 for carbon and from 5e-3 to 4e-4 for vasopressin.
3. **No book is steady at this head, so the steady sizes wait for 11b**, as the fifth box says. By thirds of the run, nitrogen gas (3.56, 0.53, 0.097) and vasopressin (1.55e-4, 6.9e-5, 3.4e-5) are settling. Oxygen (5.68, 7.27, 7.49) and carbon (4.58, 4.08, 3.96) carry a flow that 300 s does not end. _Inferred:_ the body holds back carbon dioxide at about 4 a second. Oxygen's change over carbon's climbs from 0.02 at 20 s to 1.95 at 285 s, which is two oxygen atoms a carbon, and Fick finds the beds keeping 2.9 a second of what they make. 11b's loop is what closes it. A bound sized on the first minute is broken later by the same honest body: oxygen's worst rate is 2.96 a second for windows that close by 60 s, a bound of 6, and 7.49 at 280 s.
4. **A two-sided bound refuses a leak against the drift only above `b + s`.** With the drift it refuses above `b - s`. With `b` at `2s`, a leak of the old size is refused both ways only where `s` is under a third of it. At this head that is nitrogen gas alone. Default: the bound stays two-sided and through `at_most`. A band about a stated centre is rejected for now: a centre is a claim about accuracy, the certificate shows stability, and a centre would rest on no anchor.
5. **The settling time is the policy's figure, and it costs test time.** The paying beds' hydrogen tightens only from 60 s: the coronary bed's worst rate is 3.04e-3 a second from 20 s and 1.05e-4 from 60 s, the muscle bed's 1.41e-3 and 4.80e-4. Edges hold the coronary bed's ripple whole: at most 1/63 of the swing inside a breath shows at an edge. A body test past the first minute costs about one wall second for each simulated second.
6. **Vasopressin's 3e-4 is not met at these figures.** Twice its worst rate rounds up to 4e-4. From 100 s on the same rule gives 2e-4.

Not measured: a second breathing rate, a body that has settled, and the summed form of Fick. The fuller measurement (the minute on 11b at the moving packet and at 0.20 and 0.25 a second with the drive held, and clean windows against exact challenges at admission, in a held breath and at the tail) runs before the body's PR, once 11b exists.
