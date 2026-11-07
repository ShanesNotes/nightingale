use core::cell::{Cell, RefCell};
use core::num::NonZeroU32;
use ng_contract::*;
use ng_record::{CheckpointError, CompactLink, HistoryStore, Session, compact, expand};
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
