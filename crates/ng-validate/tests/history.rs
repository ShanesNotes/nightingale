use core::cell::{Cell, RefCell};
use core::num::NonZeroU32;
use ng_contract::*;
use ng_record::{
    CheckpointError, CompactLink, HistorySegment, HistoryStore, Running, Session, compact, expand,
};
use ng_solver::Core;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;

struct Ticks(Cell<u64>);
impl WallClock for Ticks {
    fn elapsed_ns(&self) -> Option<u64> {
        self.0.set(self.0.get() + SECOND);
        Some(self.0.get())
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

#[derive(Default)]
struct Compacting {
    held: RefCell<Vec<(Digest, Vec<u8>)>>,
    given: RefCell<Vec<(Digest, Vec<u8>)>>,
}
impl HistoryStore for Compacting {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let held = self.held.borrow();
        let at = held.iter().position(|(held, _)| held == seal)?;
        let previous = at.checked_sub(1).map(|before| CompactLink {
            seal: held[before].0,
            bytes: &held[before].1,
        });
        expand(previous, &held[at].1)
    }
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        self.given.borrow_mut().push((seal, link.to_vec()));
        let mut held = self.held.borrow_mut();
        if held.iter().any(|(held, _)| *held == seal) {
            return Ok(());
        }
        let kept = compact(
            held.last().map(|(seal, kept)| CompactLink {
                seal: *seal,
                bytes: kept,
            }),
            link,
        );
        held.push((seal, kept));
        Ok(())
    }
}

#[test]
fn a_store_that_keeps_end_values_only_gives_back_every_link_of_a_slice_run() {
    let body = arena_body().expect("compiled");
    let core = Core::new(10 * MILLISECOND);
    let store = Compacting::default();
    let end = SimTime::from_ns(anchor_moment().nanoseconds() + 3 * SECOND);
    let spec = ArenaSpec {
        window: Interval::new(anchor_moment(), end).expect("an interval"),
        packet: arena_packet(&body).expect("a packet"),
        lens: arena_lens(&body).expect("a lens"),
        boundary: BoundaryPort::new(Vec::new()).expect("no event"),
        seed: arena_seed(),
        repeats: NonZeroU32::new(2).expect("positive"),
    };
    let arena = run(&body, &core, spec, &Ticks(Cell::new(0)), &store).expect("the slice runs");
    let session = arena.session();
    let (seal, revision) = (session.record().seal(), session.record().revision());
    let saved = session.checkpoint().expect("a checkpoint");
    let resumed = Session::resume(&saved, session.record().identity(), &body, &core, &store);
    assert_eq!(
        resumed.expect("every link chains to its seal").record().seal(),
        seal
    );

    let (given, held) = (store.given.borrow(), store.held.borrow());
    assert!(revision > 200, "{revision} links");
    assert_eq!(
        given.len() as u64,
        2 * revision,
        "each repeat hands over each link"
    );
    assert_eq!(held.len() as u64, revision, "two repeats hold one chain");
    for (seal, link) in given.iter() {
        assert_eq!(
            store.resolve(seal).as_ref(),
            Some(link),
            "the link kept under {seal:?}"
        );
    }
    for ((_, kept), (_, link)) in held.iter().zip(given.iter()).skip(1) {
        assert!(
            kept.len() <= link.len() / 2 + 1,
            "{} of {} bytes",
            kept.len(),
            link.len()
        );
    }
}

#[derive(Default)]
struct Counting {
    kept: MemoryStore,
    resolved: Cell<u64>,
    lost: RefCell<Vec<Digest>>,
    altered: RefCell<Vec<Digest>>,
}
impl HistoryStore for Counting {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        self.resolved.set(self.resolved.get() + 1);
        if self.lost.borrow().contains(seal) {
            return None;
        }
        let mut link = self.kept.resolve(seal)?;
        if self.altered.borrow().contains(seal) {
            let last = link.len() - 1;
            link[last] ^= 1;
        }
        Some(link)
    }
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        self.kept.keep(seal, link)
    }
}

const STEP: u64 = 100 * MILLISECOND;

fn twenty_minutes() -> TwoReservoirs {
    let end = SimTime::from_ns(1_200 * SECOND);
    let inflow = Delivery {
        id: b"inflow",
        from: SimTime::from_ns(0),
        to: end,
        amount: Measure::new(120.0).expect("an amount"),
    };
    let start = [600.0, 424.0].map(|amount| Measure::new(amount).expect("a start"));
    TwoReservoirs::delivering(start, vec![inflow])
        .expect("a body")
        .until(end)
}

fn run_marked<'s>(
    case: &'s TwoReservoirs,
    store: &'s Counting,
    core: &'s Core,
    marks: &[u64],
) -> (Session<'s, Running>, Vec<Digest>) {
    let mut session = case.admit(core, store).expect("admitted");
    let mut seals = Vec::new();
    for &mark in marks {
        let Ok(at) = run_until(session, SimTime::from_ns(mark * SECOND)) else {
            panic!("the run faulted");
        };
        seals.push(at.record().seal().digest());
        session = at;
    }
    let Ok(session) = run_until(session, case.end()) else {
        panic!("the run faulted");
    };
    (session, seals)
}

fn values(segment: &HistorySegment, basis: &ConservedBasis) -> Vec<[u64; 2]> {
    let piece = segment.interval();
    let mut values = Vec::new();
    for quantity in basis.ids() {
        for curve in [
            segment.inventory(quantity),
            segment.inflow(quantity),
            segment.outflow(quantity),
        ] {
            let ends = curve.on(piece).expect("its own segment");
            values.extend(ends.iter().map(|value| value.bits()));
        }
    }
    values
}

#[test]
fn the_steps_in_time_order_are_the_steps_newest_first_reversed_and_a_cursor_holds_one_page_at_most() {
    let (case, store, core) = (twenty_minutes(), Counting::default(), Core::new(STEP));
    let (session, _) = run_marked(&case, &store, &core, &[]);
    let view = session.conservation();
    let mut newest = view
        .newest_first()
        .collect::<Result<Vec<_>, _>>()
        .expect("a whole chain");
    newest.reverse();
    let steps = newest.len() as u64;
    assert_eq!(steps, 12_000, "twenty minutes of 0.1 s steps");

    let before = store.resolved.get();
    let order = view.in_time_order().expect("a whole chain");
    let passed = store.resolved.get();
    assert_eq!(
        passed - before,
        steps - 1,
        "the walk reads each link once, and the head from memory"
    );

    let mut served = 0;
    let mut most_held = 0;
    for (item, expected) in order.from(newest[0].interval().start()).zip(&newest) {
        let (revision, segment) = item.expect("a whole chain");
        served += 1;
        let held = store.resolved.get() - passed - served.min(steps - 1);
        most_held = most_held.max(held);
        assert!(held <= 64, "{held} decoded steps held at revision {revision}");
        assert_eq!(revision, served, "oldest first, each with its revision");
        assert_eq!(segment.interval(), expected.interval());
        assert_eq!(values(&segment, view.basis()), values(expected, view.basis()));
    }
    assert_eq!(served, steps, "every step once, the head last");
    assert_eq!(most_held, 63, "a page of 64 read, one served");
    let first_cursor = store.resolved.get() - passed;
    assert_eq!(first_cursor, steps - 1, "the cursor reads each link once");

    let late = SimTime::from_ns(newest[0].interval().start().nanoseconds() + 400 * SECOND);
    let read = order.from(late).count() as u64;
    let second_cursor = store.resolved.get() - passed - first_cursor;
    assert!(
        read < steps && second_cursor < read,
        "{second_cursor} links read for {read} steps"
    );
    assert!(
        store.resolved.get() - before <= 3 * (steps - 1),
        "each link is read at most three times"
    );
}

#[test]
fn a_cursor_starts_before_its_instant_and_reaches_the_step_that_holds_it_within_a_page() {
    let (case, store, core) = (twenty_minutes(), Counting::default(), Core::new(STEP));
    let (session, _) = run_marked(&case, &store, &core, &[]);
    let view = session.conservation();
    let mut newest = view
        .newest_first()
        .collect::<Result<Vec<_>, _>>()
        .expect("a whole chain");
    newest.reverse();
    let order = view.in_time_order().expect("a whole chain");
    let admission = newest[0].interval().start().nanoseconds();
    let mut instants = vec![admission, admission + 1];
    for segment in newest.iter().step_by(37).chain(newest.last()) {
        let start = segment.interval().start().nanoseconds();
        instants.extend([start, start + 1, segment.interval().end().nanoseconds()]);
    }
    for instant in instants {
        let at = SimTime::from_ns(instant);
        let served: Vec<_> = (order.from(at).map(|item| item.expect("a whole chain"))).collect();
        let (first, _) = served[0];
        let opening = newest[first as usize - 1].interval().start();
        assert!(
            opening < at || first == 1,
            "the first step served at {instant} starts before it, or opens the history"
        );
        let holding = served
            .iter()
            .position(|(_, segment)| at <= segment.interval().end());
        assert!(
            holding.is_some_and(|holding| holding < 64),
            "the step holding {instant} is {holding:?} steps in"
        );
        let revisions: Vec<u64> = served.iter().map(|(revision, _)| *revision).collect();
        let expected: Vec<u64> = (first..=newest.len() as u64).collect();
        assert_eq!(revisions, expected, "then every later step, in order");
    }
}

#[test]
fn a_walk_in_time_order_refuses_the_link_a_walk_newest_first_refuses() {
    let (case, store, core) = (twenty_minutes(), Counting::default(), Core::new(STEP));
    let (session, seals) = run_marked(&case, &store, &core, &[300, 900]);
    let view = session.conservation();
    let (older, newer) = (seals[0], seals[1]);
    let refusal = |lost: &[Digest], altered: &[Digest]| {
        *store.lost.borrow_mut() = lost.to_vec();
        *store.altered.borrow_mut() = altered.to_vec();
        let newest_first = view.newest_first().find_map(Result::err);
        let in_time_order = view.in_time_order().err();
        assert_eq!(in_time_order, newest_first, "lost {lost:?}, altered {altered:?}");
        in_time_order
    };
    assert_eq!(
        refusal(&[older], &[]),
        Some(CheckpointError::HistoryMissing { seal: older })
    );
    assert_eq!(
        refusal(&[], &[older]),
        Some(CheckpointError::HistoryAltered { seal: older })
    );
    assert_eq!(
        refusal(&[older], &[newer]),
        Some(CheckpointError::HistoryAltered { seal: newer }),
        "the newer of two bad links"
    );
    assert_eq!(
        refusal(&[newer], &[older]),
        Some(CheckpointError::HistoryMissing { seal: newer }),
        "the newer of two bad links"
    );
    assert_eq!(refusal(&[], &[]), None);
}
