use core::num::NonZeroU64;
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;

fn two_reservoirs_certified(per_update: u64) -> Result<Certificate, CertificateFailure> {
    certified([600.0, 424.0], Vec::new(), 1e-12, per_update)
}

fn certified(
    start: [f64; 2],
    deliveries: Vec<Delivery>,
    relative: f64,
    per_update: u64,
) -> Result<Certificate, CertificateFailure> {
    let end = SimTime::from_ns(6 * 3_600 * SECOND);
    let start = start.map(|amount| Measure::new(amount).expect("a start"));
    let case = TwoReservoirs::delivering(start, deliveries)
        .expect("a body")
        .until(end);
    let (store, core) = (MemoryStore::default(), ng_solver::Core::new(100_000_000));
    let session = case.admit(&core, &store).expect("admitted");
    let Ok(session) = run_until(session, case.end()) else {
        panic!("the run faulted");
    };
    let view = session.conservation();
    let quantity = view.basis().ids().next().expect("water");
    let window = NonZeroU64::new(end.nanoseconds() / 2).expect("a window");
    let windows = SlidingWindows::new(window, end.nanoseconds()).expect("a coverage");
    let limits = QuantityPolicy::new(
        quantity,
        Measure::<Water>::new(0.0).expect("zero"),
        Measure::new(relative).expect("relative"),
        Measure::<Water>::new(1e9).expect("drift"),
    )
    .expect("water");
    let policy = CertificatePolicy::new(view.basis(), vec![limits], ExpectedDrift::Zero, windows)
        .and_then(|policy| policy.admitting_row_rounding(case.body(), per_update))
        .expect("a policy");
    certify(&view, &policy)
}

#[test]
fn two_reservoirs_whose_rows_round_their_transfer_close_within_their_rows_rounding() {
    let refused = two_reservoirs_certified(0);
    assert!(
        matches!(&refused, Err(failure) if failure.check() == CertificateCheck::Closure),
        "with no rounding admitted the rows' rounding refuses closure: {refused:?}"
    );
    let passed = two_reservoirs_certified(7).expect("within 7 last places a row update");
    let water = &passed.results()[0];
    let place = f64::from_bits(water.closure_place().bits()[0]);
    let used = f64::from_bits(water.worst_closure().bits()[0]) / place;
    assert_eq!(water.row_rounding(), 7);
    assert_eq!(
        water.closure_last_places(),
        2_268_021,
        "7 last places an update, times the updates a step, times the window's steps and one"
    );
    assert!(
        used > 1.0 && used < water.closure_last_places() as f64,
        "the rows rounded: the worst window used {used} last places of the {} it allowed",
        water.closure_last_places()
    );
}

#[test]
fn a_total_that_crosses_powers_of_two_in_a_window_is_allowed_at_its_closing_end() {
    let (from, to) = (10_200 * SECOND, 6 * 3_600 * SECOND);
    let inflow = Delivery {
        id: b"inflow",
        from: SimTime::from_ns(from),
        to: SimTime::from_ns(to),
        amount: Measure::new(f64::from(1 << 20)).expect("an amount"),
    };
    let passed = certified([0.5, 0.5], vec![inflow], 0.0, 7).expect("within 7 last places a row update");
    let water = &passed.results()[0];
    let end = water.closure_window().end().nanoseconds();
    let total = 1.0 + f64::from(1 << 20) * (end.saturating_sub(from) as f64 / (to - from) as f64);
    assert_eq!(
        water.closure_place().bits()[0],
        ((total.to_bits() >> 52) - 105) << 52,
        "the worst window's place is its closing total's, {total} at {end} ns, and not the first window's"
    );
}
