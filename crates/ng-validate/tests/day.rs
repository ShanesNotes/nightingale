use core::num::NonZeroU32;
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;

fn told(failure: &DayFailure) -> String {
    match failure {
        DayFailure::Initialization(failure) => format!("{failure:?}"),
        DayFailure::Faulted(report) => format!("fault {:?}", report.fault()),
        DayFailure::Checkpoint(error) => format!("{error:?}"),
        DayFailure::Input(error) => format!("input {error:?}"),
        DayFailure::MissingRow(kind) => format!("no row {kind:?}"),
    }
}

#[test]
fn a_slice_run_checks_every_row_and_scores_none_before_a_window_closes() {
    let body = arena_body().expect("compiled");
    let core = ng_solver::Core::new(10_000_000);
    let seven = anchor_moment().nanoseconds() + 300 * SECOND;
    let (start, end) = (
        SimTime::from_ns(seven - 2 * SECOND),
        SimTime::from_ns(seven + 11 * SECOND),
    );
    let spec = DaySpec {
        window: Interval::new(start, end).expect("a window"),
        packet: arena_packet(&body).expect("a packet"),
        lens: arena_lens(&body).expect("a lens"),
        seed: Seed::new([1, 2, 3, 4]),
        plan: DayPlan::reference(NonZeroU32::MIN),
    };
    let store = MemoryStore::default();
    let report =
        day_checks(&body, &core, spec, &store).unwrap_or_else(|failure| panic!("{}", told(&failure)));
    let mut rows: Vec<u8> = report.checks().iter().map(DayCheck::row).collect();
    rows.dedup();
    assert_eq!(rows, (1..=17).collect::<Vec<u8>>());
    let not_yet = Verdict::NotAvailableYet;
    for check in report.checks() {
        let expected = match check.row() {
            1..=7 => not_yet(Unavailable::NotCovered),
            8 | 11..=13 => not_yet(Unavailable::Unscored),
            9 | 10 => not_yet(Unavailable::NotReached),
            _ => not_yet(Unavailable::NoPayload),
        };
        assert_eq!(
            check.verdict(),
            expected,
            "row {} {:?}\n{report}",
            check.row(),
            check.quantity()
        );
    }
    let seven = SimTime::from_ns(seven);
    let at_seven = report.checks().iter().filter(|check| check.row() <= 8);
    assert!(at_seven.clone().all(|check| check.at() == Some(seven)));
    assert_eq!(at_seven.count(), 8);
}
