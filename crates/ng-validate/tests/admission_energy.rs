//! Ticket native-contract 28: admission places the energy store at the heart work its own cycle computes, and a
//! running core then integrates the same beat to its own accuracy. Where the two disagree, the store starts off its
//! balance and relaxes toward it with heat's time constant, so the first half minute of a run shows the offset as
//! inflow and outflow apart.

use ng_contract::*;
use ng_record::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const WINDOW_NS: u64 = 30 * SECOND;
/// Inflow and outflow agree to 2% of inflow over the first 30 s. At the arena's heart work of about 12,000 a second
/// that is a gap of at most 240 a second, which a store leaving at `bd.heat-speed`, 0.001 a second, reads as a start
/// about `240 * 30 / (1 - exp(-0.03))`, 2.4 × 10⁵, off its balance; over a 24 h window it relaxes all of that, a drift
/// of about 2.4 × 10⁵. Chosen before the Radau cycle was measured; it is a numerical regression bound, not the energy
/// drift limit.
const GAP_SHARE: f64 = 0.02;

fn hi(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

#[test]
#[ignore = "admits the arena and runs 30 s of the core; the full gate runs it"]
fn energy_inflow_and_outflow_agree_over_the_first_30_seconds_after_admission() {
    let body = arena_body().expect("compiled");
    let core = ng_solver::Core::new(10_000_000);
    let origin = anchor_moment();
    let store = MemoryStore::default();
    let lens = arena_lens(&body).expect("a lens");
    let packet = arena_packet(&body).expect("a packet");
    let boundary = BoundaryPort::new(Vec::new()).expect("no event");
    let admitted = admit_at(&body, packet, boundary, &core, &lens, origin, &store);
    let session = admitted.unwrap_or_else(|failure| panic!("{failure:?}")).session;
    let end = SimTime::from_ns(origin.nanoseconds() + WINDOW_NS);
    let session = run_until(session, end).unwrap_or_else(|stopped| panic!("{:?}", stopped.fault()));
    let view = session.conservation();
    let head = view.head().expect("a step");
    let piece = head.interval();
    let end_of = |curve: Curve<'_>| hi(*curve.on(piece).expect("on the head").last().expect("an end"));
    let energy: Vec<QuantityId> = (view.basis().ids())
        .filter(|quantity| quantity.book() == Book::Of(Species::Energy))
        .collect();
    assert_eq!(energy.len(), 1);
    let (inflow, outflow) = (end_of(head.inflow(energy[0])), end_of(head.outflow(energy[0])));
    assert!(
        inflow.is_finite() && outflow.is_finite() && inflow > 0.0,
        "{inflow} {outflow}"
    );
    let gap = inflow - outflow;
    assert!(
        gap.abs() <= GAP_SHARE * inflow,
        "inflow {inflow}, outflow {outflow}: a gap of {} of inflow",
        gap / inflow
    );
}
