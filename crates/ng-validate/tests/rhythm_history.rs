//! A fixture's own phase, declared a rhythm: each edge is sealed in the link of the step whose selected landing reset
//! the phase, and reads back the same from the head, from older links through a store that keeps end values only,
//! and after a resume.

use core::cell::{Cell, RefCell};
use ng_contract::*;
use ng_record::{
    CheckpointError, CompactLink, ConservationView, HistoryStore, Running, Session, Step, compact, expand,
};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const EXTERIOR: &str = "exterior";
const NOTE: &str = "synthetic";
/// The phase runs 0.8 a second to a threshold of one, so an edge every 1.25 s, and fifteen before the end.
const PACE: f64 = 0.8;
const EDGES: usize = 15;
const END: u64 = 19_500 * MILLISECOND;
const CHECKPOINT: u64 = 10 * SECOND;
/// A delivery starts 10 ms before the first edge, inside the one-second blind step that lands on it.
const CUT: u64 = 1_240 * MILLISECOND;

enum Declares {
    Nothing,
    Phase,
    PhaseTwice,
    Unreset,
}

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn named(local: &'static str) -> LawName {
    LawName { owner: "NS", local }
}

/// One vessel filled from outside, and a phase whose two guards each reset it at the threshold and swap the beat.
fn metronome(declares: Declares) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let vessel = map.site::<VesselSegment>("vessel", origin());
    map.contain(root.id(), vessel.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.boundary::<Water>(EXTERIOR, BoundaryDirection::IntoBody, vessel.id(), Space::Lumen)?;
    schema.partial_volume(measure(1.0))?;
    assemble(schema, |mut body| {
        let wave = RateClass::new(RateBand::Wave, 3_600 * SECOND)?;
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryRoute { route, drive } = ports.route::<Water>(EXTERIOR)?;
        let rate = laws.kernel(|mut k| {
            let value = k.read(&drive);
            k.finish(value, laws.every_mode(), wave)
        });
        laws.transfer(Transfer::new(&route, rate));
        body.add(laws.finish());

        let (mut laws, _ports) = body.nervous()?.parts();
        let phase = laws.state::<Dimensionless>("phase", Admissible::nonnegative());
        let pace =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.pace"), Admissible::positive());
        let running = laws.kernel(|mut k| {
            let value = k.parameter(&pace);
            k.finish(value, laws.every_mode(), wave)
        });
        laws.memory_rate(&phase, running);
        let threshold =
            laws.parameter::<Dimensionless>(ParameterKey::new("ns.threshold"), Admissible::positive());
        let [tick, tock] = laws.modes("beat", ["tick", "tock"]);
        for (from, to) in [(&tick, &tock), (&tock, &tick)] {
            let past = laws.kernel(|mut k| {
                let (now, threshold) = (k.state(&phase), k.parameter(&threshold));
                let gap = k.subtract(now, threshold);
                k.finish(gap, from.only(), wave)
            });
            laws.event(Guard::new(past, Crossing::Rising, from.to(to)?).resetting(&phase));
        }
        match declares {
            Declares::Nothing => {}
            Declares::Phase => laws.rhythm(&phase),
            Declares::PhaseTwice => {
                laws.rhythm(&phase);
                laws.rhythm(&phase);
            }
            Declares::Unreset => {
                let other = laws.state::<Dimensionless>("other", Admissible::finite());
                let still = laws.kernel(|mut k| {
                    let value = k.zero::<Rate<Dimensionless>>();
                    k.finish(value, laws.every_mode(), wave)
                });
                laws.memory_rate(&other, still);
                laws.rhythm(&phase);
                laws.rhythm(&other);
            }
        }
        body.add(laws.finish());
        body.compile()
    })
}

fn packet(body: &CompiledBody) -> ParameterPacket<SyntheticFixture> {
    let mut packet = ParameterPacket::new();
    let pace = measure::<Rate<Dimensionless>>(PACE);
    let pace = packet.synthetic(ParameterKey::new("ns.pace"), Binding::Body, pace, NOTE);
    let threshold = measure::<Dimensionless>(1.0);
    let threshold = packet.synthetic(ParameterKey::new("ns.threshold"), Binding::Body, threshold, NOTE);
    let phase = RowKind::Memory(named("phase").into());
    let phase = packet.start(phase, measure::<Dimensionless>(0.0), NOTE);
    let lumen = (body.rows().iter().map(Row::kind))
        .find(|kind| matches!(kind, RowKind::Amount { .. }))
        .expect("the vessel's water");
    let lumen = packet.start(lumen, measure::<Water>(1_000.0), NOTE);
    let beat = packet.start_mode(named("beat"), "tick", NOTE);
    for set in [pace, threshold, phase, lumen, beat] {
        set.expect("once");
    }
    packet
}

fn boundary() -> BoundaryPort {
    let delivery = WaterDelivery {
        boundary: EXTERIOR,
        amount: measure(36.0),
        ends: SimTime::from_ns(CUT + 360 * MILLISECOND),
    };
    let input = InputId::new(Digest::of_canonical(b"a delivery inside a landing"));
    let event = BoundaryEvent::new(input, SimTime::from_ns(CUT), BoundaryInput::Water(delivery));
    BoundaryPort::new(vec![event]).expect("one event")
}

/// Every link a run hands over, and each kept in the compact form that follows the one before.
#[derive(Default)]
struct Compacting {
    held: RefCell<Vec<(Digest, Vec<u8>)>>,
    given: RefCell<Vec<Vec<u8>>>,
    altered: Cell<Option<Digest>>,
}
impl HistoryStore for Compacting {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let held = self.held.borrow();
        let at = held.iter().position(|(held, _)| held == seal)?;
        let previous = at.checked_sub(1).map(|before| CompactLink {
            seal: held[before].0,
            bytes: &held[before].1,
        });
        let mut link = expand(previous, &held[at].1)?;
        if self.altered.get() == Some(*seal) {
            let last = link.len() - 1;
            link[last] ^= 1;
        }
        Some(link)
    }
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        let mut held = self.held.borrow_mut();
        if held.iter().any(|(held, _)| *held == seal) {
            return Ok(());
        }
        self.given.borrow_mut().push(link.to_vec());
        let previous = held
            .last()
            .map(|(seal, bytes)| CompactLink { seal: *seal, bytes });
        let kept = compact(previous, link);
        held.push((seal, kept));
        Ok(())
    }
}

/// Refuses every fourth solve of a first attempt, so some steps land only on a retry and some landings are refused part
/// way, and never a retry.
struct Refusing {
    core: ReferenceIntegrator,
    calls: Cell<u64>,
}
impl CoupledSolver for Refusing {
    fn resolution(&self) -> DeclaredResolution {
        self.core.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.core.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.calls.set(self.calls.get() + 1);
        if ticket.attempt() == 0 && self.calls.get() % 4 == 0 {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.core.solve(problem, ticket)
    }
}

/// Each accepted step and whether it swapped the beat, which only a guard's landing does.
type Steps = Vec<(Interval, bool)>;

fn run<'m>(
    mut session: Session<'m, Running>,
    end: u64,
    steps: &mut Steps,
    retries: &mut u32,
) -> Session<'m, Running> {
    while session.record().accepted().time() < SimTime::from_ns(end) {
        let accepted = session.record().accepted();
        let (start, beat) = (accepted.time(), accepted.modes().to_vec());
        let mut step = session.step();
        session = loop {
            match step {
                Step::Accepted(next, _) => break next,
                Step::Refused(refused) => {
                    *retries += 1;
                    step = refused.retry();
                }
                Step::Faulted(faulted) => panic!("the fixture faulted: {:?}", faulted.fault()),
            }
        };
        let accepted = session.record().accepted();
        let interval = Interval::new(start, accepted.time()).expect("a step");
        steps.push((interval, accepted.modes() != beat.as_slice()));
    }
    session
}

fn admitted<'m>(
    body: &'m CompiledBody,
    core: &'m dyn CoupledSolver,
    store: &'m dyn HistoryStore,
) -> Session<'m, Running> {
    admit_fixture(body, packet(body), boundary(), core, store).expect("admitted")
}

/// Each segment and whether it ends on an edge, oldest first, read through a cursor in time order.
fn edges(view: &ConservationView<'_>) -> Result<Steps, CheckpointError> {
    let [(rhythm, phase)] = view.rhythms().collect::<Vec<_>>()[..] else {
        panic!("one rhythm");
    };
    assert_eq!(phase, Slot::from(named("phase")));
    let order = view.in_time_order()?;
    let read = order.from(SimTime::from_ns(0)).map(|item| {
        let (_, segment) = item?;
        Ok((segment.interval(), segment.ends_on_edge(rhythm)))
    });
    read.collect()
}

#[test]
fn compile_refuses_a_rhythm_whose_phase_no_guard_resets_and_one_declared_twice() {
    let defect = |declares| match metronome(declares) {
        Err(AdmissionError::Compile(defect)) => Some(defect),
        _ => None,
    };
    assert_eq!(
        defect(Declares::Unreset),
        Some(CompileDefect::UnresetRhythm {
            phase: named("other")
        })
    );
    assert_eq!(
        defect(Declares::PhaseTwice),
        Some(CompileDefect::RhythmTwice {
            phase: named("phase")
        })
    );
    assert!(metronome(Declares::Phase).is_ok());
}

#[test]
fn each_edge_is_sealed_where_the_selected_landing_reset_the_phase_and_reads_back_from_every_link_and_a_resume()
 {
    let body = metronome(Declares::Phase).expect("compiled");
    let (core, store) = (ReferenceIntegrator::new(SECOND), Compacting::default());
    let (mut steps, mut retries) = (Steps::new(), 0);
    let session = run(
        admitted(&body, &core, &store),
        CHECKPOINT,
        &mut steps,
        &mut retries,
    );
    let (saved, before) = (session.checkpoint().expect("a checkpoint"), steps.len());
    let session = run(session, END, &mut steps, &mut retries);
    assert_eq!(retries, 0);
    assert_eq!(steps.iter().filter(|(_, edge)| *edge).count(), EDGES);
    let cut = steps
        .iter()
        .position(|(step, _)| step.end() == SimTime::from_ns(CUT));
    let at = |index: usize| (steps[index].0.start().nanoseconds(), steps[index].1);
    assert_eq!(
        cut.map(|cut| (at(cut), at(cut + 1))),
        Some(((SECOND, false), (CUT, true))),
        "the blind step from 1 s landed past the delivery and was thrown away for the step to it"
    );

    let view = session.conservation();
    assert_eq!(edges(&view), Ok(steps.clone()), "live, and from each older link");
    let mut newest: Vec<_> = (view.newest_first())
        .map(|segment| segment.expect("a whole chain").interval())
        .collect();
    newest.reverse();
    assert!(newest.iter().eq(steps.iter().map(|(step, _)| step)));

    let (given, held) = (store.given.borrow().clone(), store.held.borrow().clone());
    assert_eq!(given.len(), steps.len(), "one link a step");
    for ((_, kept), link) in held.iter().zip(&given).skip(1) {
        assert!(
            kept.len() < link.len(),
            "each link after the first is kept compact, edge or not"
        );
    }

    let (fresh, again) = (ReferenceIntegrator::new(SECOND), Compacting::default());
    let mut whole = Steps::new();
    let uninterrupted = run(admitted(&body, &fresh, &again), END, &mut whole, &mut retries);
    let resumed = Session::resume(&saved, session.record().identity(), &body, &fresh, &store);
    let mut later = steps[..before].to_vec();
    let resumed = run(
        resumed.expect("every link chains to its seal"),
        END,
        &mut later,
        &mut retries,
    );
    assert_eq!(later, steps, "the resumed run steps as the first did");
    assert_eq!(
        edges(&resumed.conservation()),
        Ok(steps.clone()),
        "after the resume"
    );
    assert_eq!(resumed.record().seal(), session.record().seal());
    assert_eq!(uninterrupted.record().seal(), session.record().seal());

    let first = steps.iter().position(|(_, edge)| *edge).expect("an edge");
    let seal = held[first].0;
    store.altered.set(Some(seal));
    assert_eq!(
        edges(&session.conservation()),
        Err(CheckpointError::HistoryAltered { seal }),
        "a changed byte of an edge's link"
    );
}

#[test]
fn a_retry_gives_no_edge_and_each_edge_stays_where_the_beat_swapped() {
    let body = metronome(Declares::Phase).expect("compiled");
    let core = Refusing {
        core: ReferenceIntegrator::new(SECOND),
        calls: Cell::new(0),
    };
    let store = Compacting::default();
    let (mut steps, mut retries) = (Steps::new(), 0);
    let session = run(admitted(&body, &core, &store), END, &mut steps, &mut retries);
    assert!(retries > 0);
    assert_eq!(steps.iter().filter(|(_, edge)| *edge).count(), EDGES);
    assert_eq!(edges(&session.conservation()), Ok(steps));
}

/// The bytes one rhythm adds to a link, raw and as kept after the link before, by whether the link ends on its edge.
fn added(_edge: bool) -> (usize, usize) {
    (32, 16)
}

#[test]
fn a_body_that_declares_no_rhythm_writes_its_old_links_and_one_that_does_adds_its_edges_to_each() {
    let read = |declares| {
        let body = metronome(declares).expect("compiled");
        let (core, store) = (ReferenceIntegrator::new(SECOND), Compacting::default());
        let (mut steps, mut retries) = (Steps::new(), 0);
        let session = run(admitted(&body, &core, &store), END, &mut steps, &mut retries);
        let view = session.conservation();
        let shape = (
            view.basis().ids().count(),
            body.columns().len(),
            view.rhythms().len(),
        );
        let kept: Vec<usize> = store.held.borrow().iter().map(|(_, kept)| kept.len()).collect();
        let raw: Vec<usize> = store.given.borrow().iter().map(Vec::len).collect();
        (steps, shape, raw, kept)
    };
    let (steps, shape, raw, kept) = read(Declares::Phase);
    let (plain, (quantities, columns, none), plain_raw, plain_kept) = read(Declares::Nothing);
    assert_eq!((shape, none), ((quantities, columns, 1), 0));
    assert_eq!(steps, plain, "a declaration changes no step");
    let whole = 2 * 32 + 2 * 8 + 2 * 16 * (3 * quantities + 2 * columns);
    assert!(
        plain_raw.iter().all(|&raw| raw == whole),
        "main's layout: {plain_raw:?}"
    );
    for (at, (_, edge)) in steps.iter().enumerate() {
        let (raw_added, kept_added) = added(*edge);
        let kept_added = if at == 0 { raw_added } else { kept_added };
        assert_eq!(
            (raw[at] - plain_raw[at], kept[at] - plain_kept[at]),
            (raw_added, kept_added),
            "step {at}, edge {edge}"
        );
    }
}
