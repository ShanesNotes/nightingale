# 85: The certificate's windows end on breath edges

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 5, and `crates/ng-validate/src/certificate.rs` (`SlidingWindows`)

Type: task

Status: open

Blocked by: none

The first PR builds on [native-contract 94](94-the-certificate-folds-as-the-run-goes.md)'s first PR, the evaluator, which nightingale#210 lands.

Blocks: [body ticket 11](../../body/issues/11-breathing-is-controlled.md)

**Why.** The first milestone (SRS invariant I19) holds the whole body for 48 hours under its certificate, and the certificate is what refuses a slow leak. Once the body breathes, a ten-second window that does not hold whole breaths can catch most of a breath's swing in the gas the lungs hold, so [body ticket 10](../../body/issues/10-lungs-breathe.md)'s second PR set the bounds of the breath's books by that swing, to hold at any breathing rate. A bound that size lets through a steady leak far larger than the steady figures would.

**What the check of nightingale#198 found.** `SlidingWindows` takes only windows of one fixed length (`certificate.rs:78`), so no window can start and end on breath edges. The slowest steady leak each bound the breath's swing set refuses today, against the one a bound from the steady figure would refuse, per ten-second window (energy's, water's and hydrogen's bounds are in "What the second PR must size"):

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
- **The second, after 11b and 11c.** It also sizes the bounds for the day's transitions ("What the second PR must size"). Its first run is the control with no leak at each breathing rate (open item 6). Then the body's policy takes the windows, the bounds are restated from the steady rates measured on 11b's hour run, which keeps the amount of each conserved quantity the breath moves at every breath edge, Fick runs over whole breaths in the form its run settles, hydrogen is bounded at each bed, and the whole-body leak tests run. The body's pins move once.

## What the first PR built

- **The statement and the edge.** `FragmentBuilder::rhythm` declares a card's dimensionless memory a rhythm's phase. Compile refuses a phase no guard resets at one of its places (`UnresetRhythm`), one declared twice (`RhythmTwice`), and one a hazard integrates (`HazardRhythm`), since a hazard's firing resets its integral by chance and seals no edge. `CompiledBody::rhythms` numbers them in row order as `RhythmId`. The declared rows enter the model hash, tagged, just before the basis, so the held cards still end its stream. A body that declares none adds no byte. `CoupledProblem::fired` reports each rhythm whose phase a firing's guards reset, once even when two guards reset it. `VerifiedStep` carries that report to `NextHead::rhythm_edges`, and `PatientRecord::attempt` applies it to the selected step's link. No card crate declares a rhythm, so no body pin moved.
- **The sealed form: the count, open item 1 settled.** Both forms were built on the fixture `crates/ng-validate/tests/rhythm_history.rs` and both were eligible. Each passed 1,753 tests and `seam.seal`. Each gave the same edges at the live head, from older links through a compacting store, and after a resume, and each refused a changed byte. The count changes 81 lines of `crates/ng-record/src/history.rs`, the mark 111, so the count is kept. A link carries each rhythm's edges since admission at the step's start and end. With one rhythm that is 32 bytes raw and 16 kept after the first link, which is kept whole. The mark was 9 bytes raw and 9 kept on a step with an edge, and none on one without. A body with no rhythm adds no byte.
- **The windows.** `CertificatePolicy::between_edges` takes `EdgeWindows`: the clock's `Slot`, the settling time, the length and the widest stretch with no edge. It also takes an `EdgeRate` for each rated conserved quantity, in basis order, and refuses a second call, since a policy judges one clock. A policy without them keeps its old digest, and one with them seals every field, the clock's place and each rate's quantity among them. Open item 2's default is built as a form: a window whose opening edge comes before the settling time takes the rate the policy states for that stretch, and the body's figure stays the second PR's. A policy that rates nothing states no such rate, so its windows open from the settling time on. `certify` resolves the clock after the basis check and refuses one the record's body does not declare (`UndeclaredRhythm`). `Windows` judges the windows between edges as it judges the fixed windows, in the order the steps were accepted, out to the head. A failure there ranks after every fixed window. When no window closed, the last stretch runs from admission to the head, since time between edges that sits in no judged window is what the bound limits. A stretch past its bound (`RhythmStopped`) comes before a window past its allowance (`EdgeDrift`), and windows past their allowance rank by earliest close, then oldest opening, then basis order. `Certificate::rhythm` holds what the windows found, and the certificate prints it only where one was judged.
- **The tests.** `crates/ng-validate/tests/rhythm_history.rs` holds the compile refusals, the hazard's among them, the edges sealed and read back, a retry that gives no edge, two guards that reset one phase in one firing, and the bytes a link adds. `crates/ng-validate/tests/rhythm_certificate.rs` holds the leaks of each old size, at a period that divides ten seconds and one that does not and with each margin stated both ways. It also holds the control, the two openings that mature on one edge with the count of windows judged, the opening and closing steps' amounts, the settling stretch's rate, an empty rate list, the worst result kept, the tail, a run in which no window closed, a second clock and the order of failure. The oracle, `crates/ng-validate/tests/certificate_oracle/mod.rs`, agrees with `certify` on each. Eleven compile-fail doctests with passing twins hold the type rules of the statement, the edge, the rates and the clock's name. Each rule of the first PR has a mutant row, and section 7's must-fail table gains five rows.
- **Departures from the design.** The fixture's two openings are told apart by a leak confined to the older window, because the fixture's swing cancels at every edge. `UndeclaredRhythm` carries no `Slot`: clippy's large-error lint refuses one, so the check alone names the refusal. Rates before the settling time are stated for every rated quantity or for none. `Slot::digest` seals the clock's name in the policy's digest. The order of failure is the one above. Each was accepted in the PR's check.
- **Boxes.** None is done. The first PR builds the windows on a fixture, and the body takes them in the second PR.
- **Left for the second PR.**
  - One settling time, and rates before it stated for every rated quantity or for none, serve conserved quantities that settle at different times.
  - A second rhythm's edges are sealed and unread.
  - `Rhythm::finish` consumes its state, which a fold must carry instead.
  - A stopped rhythm outranks an earlier drift between edges, and so hides that drift's window until the stop clears.
  - `UndeclaredRhythm` names its check and not its clock.
  - The bound on the widest stretch must reach the first close: "What the second PR must size" has the figures.

## What the second PR must size

- **A transition between two states of the day.** Ruled: the certificate's policy must admit one. After sleep's onset the work of breathing falls, the muscle bed that pays it burns less, and heat out follows over about 1,000 s, so energy falls about 390 a second. That is the transition's own physiology and no fault. The figures are in [body ticket 11](../../body/issues/11-breathing-is-controlled.md)'s certificate box. The three the sizing needs, from 11c's hour with sleep: windows between breath edges pass 4e3 for about 100 s after an onset; the worst is 4,855 over two breaths of 11.09 s, 438 a second; and waking is inferred to mirror it, and no run shows it. The check of nightingale#214 puts a window between edges at about 440 a second, about 4.9e3 over two sleeping breaths, and near 1e4 under the margin rule's factor of 2. This PR sizes the fixed energy bound and energy's rate between edges for the transition by the margin rule. Until it lands, no suite run sleeps under a certificate for longer than the suite's branch, 30 s: in the hour, the fixed windows that pass 4e3 open 30 to 100 s after the onset.
- **Energy is one of the breath's conserved quantities.** 11b moved its bound from 3e3 to 4e3, countersigned at merge, since twice energy's worst ten-second change under the controller is 3,057.4. At main the body's bounds for the breath's conserved quantities are energy 4e3, oxygen 9e3, carbon 5e2, nitrogen gas 2e4, water 5e-2, vasopressin 5e-2, hydrogen 7e-5 and the paying beds' hydrogen 0.2 each. For this PR's builder: the doc comment above `ENERGY_DRIFT` in `crates/ng-validate/src/body/packet.rs` still reads "Energy's worst change is 1,347, in the window from 47 s." The check of nightingale#211 measured 1,529.6, in windows opening 20 to 30 s.
- **Water and vasopressin pass half their fixed bound, and no cause is known.** In 11b's awake hour (the check of nightingale#211): water passes half its 5e-2 in 137 of 360 ten-second buckets, from 620 s to 3,420 s, its worst 3.835e-2 at 840 s, 1.53 times half; vasopressin passes half only from 3,500 to 3,580 s, at 1.10 times half, 2.75e-2. In 11c's hour with sleep, sampled in the minute before each mark: water's worst is 3.24e-2 at the 5-minute mark, 1.30 times half, and vasopressin's 2.20e-2 at the hour, 0.88 times half. The margin rule asks for a factor of 2, so this PR states each against its bound and finds the cause or files it.
- **A fixed window cuts the breath's swing as the breath's length moves.** A fixed ten-second window cuts the breath's swing differently as the breath's length moves, so a fixed window's worst grows with the mismatch between ten seconds and two breaths. Ticket 11 infers it at 11b: "Under the controller the breath shortens from 5.000 s to 4.883 s by 54.6 s, which moves where each ten-second window cuts the breath's swing of energy (inferred)." The windows between edges remove it, so this PR lands before any run of more than a minute under sleep or under depth.
- **The bound on the widest stretch must reach the first close.** From the check of nightingale#213's windows: with no window closed the tail runs from admission, so for an honest run to pass at every head the bound must reach the first close. That is about the settling time plus the length plus up to two breaths when no rate before the settling time is stated, and about the first edge plus the length plus a breath when rates are stated. For the body at 5 s breaths, a settling time of 20 s and a length of 10 s, that is about 30 s or 15 s, and the same bound then admits a held breath that long for all 48 hours. Under the reading that measures the tail from the last edge, the bound would need only the longest honest stretch with no edge, about 5 s. The bound is a guard on coverage, how long the judgment between edges may lapse, and not a monitor of breathing.
- **Low notes from the check of nightingale#213's fix round.**
  - No test fixes which refusal fires for a phase only a hazard resets. `compile::rhythms` checks `RhythmTwice`, then `HazardRhythm`, then `UnresetRhythm`, and such a phase is both unreset and a hazard's integral, so `HazardRhythm` fires. Either refusal refuses. A fixture case would fix the order.
  - The contract's section 5 said the policy "refuses a second rhythm's" windows, while `between_edges` refuses any second call, of the same clock too, as `a_policy_refuses_a_second_clock` shows. The code is unambiguous, and the sentence now says so.
  - The first PR's `/no-comments` report says its `failure.rs` diff went from 206 lines to 8. It is 6.
  - `docs/architecture/solver-contract-sketch/src/ng_validate.rs` line 55 is a doc line of 161 characters against `max_width = 110`. Rustfmt does not wrap comments, so no gate sees it.

## Open, each settled by a run

2. **The first 20 s.** A leak of 20 oxygen atoms a second confined to the first 20 s moves a fixed window by 200 against 9,000 allowed, and no later edge sees it. Default: the settling stretch gets windows between edges too, with its own stated rate. Measured at main `446e29f9`, over windows that open before 20 s against those that open from 20 s on: carbon 4.58 a second against 4.27, nitrogen gas 3.56 against 3.02, the coronary bed's hydrogen 8.49e-3 against 3.04e-3, the muscle bed's 4.61e-3 against 1.41e-3. Oxygen's and vasopressin's worst windows open later. So the settling rate is at most 3.3 times the later one. The run: the second PR's control at each rate states the settling stretch's rate on the body after 11b.
3. **A leak while the rate moves.** No design so far tested it. Default: it joins the whole-body tests after 11b. The run: the second PR's whole body under the loop, with an exact leak, beside the same run with none.
5. **Fick's form.** Default: the summed form "Fick" above gives. The run: that form on a saved 300 s run of the body, the length of the runs in "The bound", before any tolerance is set.
6. **The whole body's two-rate leak tests.** The fifth box sets the rate by the packet with the drive scale held. _Inferred, not run:_ away from the rate the drive balances, the body gains or loses carbon dioxide by the change in ventilation, so the run with no leak may itself break a steady bound. Default and run: the second PR's first run is that control at each rate. If it breaks the bound, the whole-body tests state their own bound from their own control, with the leak sized above the bound plus the control's rate, and the fixture carries the exact refusal at two rates.

Item 1, the sealed form of the edge, is settled under "What the first PR built". Item 4, the name of the rhythm's clock, is ruled above under "The edge".

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
