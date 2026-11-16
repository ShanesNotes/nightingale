use core::num::NonZeroU64;
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;

/// Two reservoirs of different size relax toward each other with nothing crossing a boundary. Each step moves an amount
/// finer than the larger one's last place, so the two rows round it to different places and their total moves.
fn certified(per_update: u64) -> Result<Certificate, CertificateFailure> {
    let end = SimTime::from_ns(6 * 3_600 * SECOND);
    let start = [Measure::new(600.0).expect("a"), Measure::new(424.0).expect("b")];
    let case = TwoReservoirs::delivering(start, Vec::new()).expect("a body").until(end);
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
        Measure::new(1e-12).expect("relative"),
        Measure::<Water>::new(1.0).expect("drift"),
    )
    .expect("water");
    let policy = CertificatePolicy::new(view.basis(), vec![limits], ExpectedDrift::Zero, windows)
        .and_then(|policy| policy.admitting_row_rounding(case.body(), per_update))
        .expect("a policy");
    certify(&view, &policy)
}

#[test]
fn two_reservoirs_whose_rows_round_their_transfer_close_within_their_rows_rounding() {
    let refused = certified(0);
    assert!(
        matches!(&refused, Err(failure) if failure.check() == CertificateCheck::Closure),
        "with no rounding admitted the rows' rounding refuses closure: {refused:?}"
    );
    let passed = certified(7).expect("within 7 last places a row update");
    let water = &passed.results()[0];
    let place = f64::from_bits(water.closure_place().bits()[0]);
    let used = f64::from_bits(water.worst_closure().bits()[0]) / place;
    println!(
        "the worst window used {used} last places of the {} it allowed, {} for one update",
        water.closure_last_places(),
        water.row_rounding()
    );
    assert!(used > 1.0, "the rows rounded");
}
