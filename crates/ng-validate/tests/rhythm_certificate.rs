//! Windows between a rhythm's edges, on one capillary bed whose phase is declared a rhythm. The phase drives an oxygen
//! swing that whole breaths cancel, each rated quantity drifts steadily, and a room's humidity gates an exact, typed
//! leak on one of them. The room's air speed paces the phase, so a still room holds the breath. Every value is
//! synthetic.

mod certificate_oracle;

use certificate_oracle::{Ledger, certified};
use ng_contract::*;
use ng_record::{ConservationView, HistorySeal};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const STEP: u64 = 250 * MILLISECOND;
const LENGTH: u64 = 10 * SECOND;
const WIDEST: u64 = 15 * SECOND;
const LOOSE: f64 = 1e9;
const SWING: f64 = 32.0;
const NOTE: &str = "synthetic";
const OXYGEN: &str = "oxygen";
const BREATH: &str = "breath";
const CARBON: &str = "carbon";
const CARBON_OXYGEN: &str = "carbon's oxygen";
const NITROGEN: &str = "nitrogen";
const VASOPRESSIN: &str = "vasopressin";

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn named(local: &'static str) -> LawName {
    LawName {
        owner: "Boundary",
        local,
    }
}

fn clock() -> Slot {
    Slot::from(named("phase"))
}

fn at(ns: u64) -> SimTime {
    SimTime::from_ns(ns)
}

fn interval(from: u64, to: u64) -> Interval {
    Interval::new(at(from), at(to)).expect("an interval")
}

fn hi(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn burned(laws: &mut FragmentBuilder<'_, Metabolism>, class: RateClass) {
    let proportions = Proportions::<Glucose>::using(Space::Interstitium)
        .uses::<Oxygen, 6>(Space::Interstitium)
        .makes::<CarbonDioxide, 6>(Space::Interstitium)
        .makes::<Water, 6>(Space::Interstitium)
        .energy_into(Space::Thermal);
    let burn = laws.reaction::<CapillaryBed, Glucose, _>("burn", proportions);
    let speed = laws.site_parameter::<CapillaryBed, Rate<Dimensionless>>(
        ParameterKey::new("mt.burn-speed"),
        Admissible::nonnegative(),
    );
    let fuel = burn.amount::<Glucose>();
    let rate = laws.site_kernel(|mut k| {
        let (fuel, zero, speed) = (k.at_site(&fuel), k.zero(), k.site_parameter(&speed));
        let rate = k.relax(fuel, zero, speed);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.react_at(&burn, rate);
}

fn leaking<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    route: &'static str,
    [drift, leak]: [&'static str; 2],
    class: RateClass,
) -> Result<(), AdmissionError> {
    let BoundaryRoute { route, .. } = ports.route::<Q>(route)?;
    let drift = laws.parameter::<Rate<Q>>(ParameterKey::new(drift), Admissible::finite());
    let leak = laws.parameter::<Rate<Q>>(ParameterKey::new(leak), Admissible::finite());
    let rate = laws.kernel(|mut k| {
        let (drift, leak, gate) = (k.parameter(&drift), k.parameter(&leak), k.room_humidity());
        let leaked = k.scale(leak, gate);
        let value = k.add(drift, leaked);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
    Ok(())
}

/// Nothing burns, but the burn joins glucose, oxygen, carbon dioxide and water, so oxygen and carbon are element
/// quantities, while nitrogen gas and vasopressin keep their own.
fn breathing(declares: bool) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let vessel = map.site::<VesselSegment>("vessel", origin());
    map.contain(root.id(), vessel.id());
    let bed = map.site::<CapillaryBed>("bed", origin()).id();
    map.contain(root.id(), bed);
    map.connect(Network::Vascular, vessel.id(), bed);
    map.connect(Network::Vascular, bed, vessel.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Plasma, "the bed's interstitium alone");
    schema.leave_out(Space::Cells, "the bed's interstitium alone");
    schema.leave_out(Space::Lumen, "no blood moves");
    schema.carry::<Glucose, CapillaryBed>(Space::Interstitium);
    schema.carry::<Oxygen, CapillaryBed>(Space::Interstitium);
    schema.carry::<CarbonDioxide, CapillaryBed>(Space::Interstitium);
    schema.carry::<NitrogenGas, CapillaryBed>(Space::Gas);
    schema.carry::<Vasopressin, CapillaryBed>(Space::Interstitium);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    schema.energy_content::<Glucose>(measure(1_000.0))?;
    let (into, out, space) = (
        BoundaryDirection::IntoBody,
        BoundaryDirection::OutOfBody,
        Space::Interstitium,
    );
    schema.boundary::<Oxygen>(OXYGEN, into, bed, space)?;
    schema.boundary::<Oxygen>(BREATH, into, bed, space)?;
    schema.boundary::<CarbonDioxide>(CARBON, into, bed, space)?;
    schema.boundary::<Oxygen>(CARBON_OXYGEN, out, bed, space)?;
    schema.boundary::<NitrogenGas>(NITROGEN, into, bed, Space::Gas)?;
    schema.boundary::<Vasopressin>(VASOPRESSIN, into, bed, space)?;
    let class = RateClass::new(RateBand::Wave, 3_600 * SECOND)?;
    assemble(schema, |mut body| {
        let held = HeldDeclaration::<Metabolism>::new(Vec::new(), Vec::new(), NOTE);
        let (mut laws, _) = body.hold(held)?.parts();
        burned(&mut laws, class);
        body.add(laws.finish());

        let (mut laws, ports) = body.boundary()?.parts();
        let key = ParameterKey::new;
        let phase = laws.state::<Dimensionless>("phase", Admissible::nonnegative());
        let pace = laws.parameter::<Rate<Dimensionless>>(key("bd.pace"), Admissible::positive());
        let running = laws.kernel(|mut k| {
            let (pace, moving) = (k.parameter(&pace), k.room_air_speed());
            let value = k.scale(pace, moving);
            k.finish(value, laws.every_mode(), class)
        });
        laws.memory_rate(&phase, running);
        let threshold = laws.parameter::<Dimensionless>(key("bd.threshold"), Admissible::positive());
        let [tick, tock] = laws.modes("beat", ["tick", "tock"]);
        for (from, to) in [(&tick, &tock), (&tock, &tick)] {
            let past = laws.kernel(|mut k| {
                let (now, threshold) = (k.state(&phase), k.parameter(&threshold));
                let gap = k.subtract(now, threshold);
                k.finish(gap, from.only(), class)
            });
            laws.event(Guard::new(past, Crossing::Rising, from.to(to)?).resetting(&phase));
        }
        if declares {
            laws.rhythm(&phase);
        }
        let BoundaryRoute { route, .. } = ports.route::<Oxygen>(BREATH)?;
        let swing = laws.parameter::<Rate<Oxygen>>(key("bd.swing"), Admissible::nonnegative());
        let half = laws.parameter::<Dimensionless>(key("bd.half"), Admissible::positive());
        let swung = laws.kernel(|mut k| {
            let (now, half, swing) = (k.state(&phase), k.parameter(&half), k.parameter(&swing));
            let off = k.subtract(now, half);
            let value = k.scale(swing, off);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, swung));
        let oxygen = ["bd.oxygen-drift", "bd.oxygen-leak"];
        leaking::<Oxygen>(&mut laws, &ports, OXYGEN, oxygen, class)?;
        let carbon = ["bd.carbon-drift", "bd.carbon-leak"];
        leaking::<CarbonDioxide>(&mut laws, &ports, CARBON, carbon, class)?;
        let returned = ["bd.carbon-oxygen-drift", "bd.carbon-oxygen-leak"];
        leaking::<Oxygen>(&mut laws, &ports, CARBON_OXYGEN, returned, class)?;
        let nitrogen = ["bd.nitrogen-drift", "bd.nitrogen-leak"];
        leaking::<NitrogenGas>(&mut laws, &ports, NITROGEN, nitrogen, class)?;
        let vasopressin = ["bd.vasopressin-drift", "bd.vasopressin-leak"];
        leaking::<Vasopressin>(&mut laws, &ports, VASOPRESSIN, vasopressin, class)?;
        body.add(laws.finish());
        body.compile()
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Leak {
    Oxygen,
    Carbon,
    Nitrogen,
    Vasopressin,
}
impl Leak {
    const ALL: [Leak; 4] = [Leak::Oxygen, Leak::Carbon, Leak::Nitrogen, Leak::Vasopressin];
    fn book(self) -> Book {
        match self {
            Leak::Oxygen => Book::Element(Element::Oxygen),
            Leak::Carbon => Book::Element(Element::Carbon),
            Leak::Nitrogen => Book::Of(Species::NitrogenGas),
            Leak::Vasopressin => Book::Of(Species::Vasopressin),
        }
    }
    fn sizes(self) -> [f64; 3] {
        match self {
            Leak::Oxygen => [20.0, 12.0, 5.0],
            Leak::Carbon => [9.0, 6.0, 3.0],
            Leak::Nitrogen => [20.0, 7.0, 4.0],
            Leak::Vasopressin => [3e-4, 2e-4, 1e-4],
        }
    }
}

fn room(leaking: bool, breathing: bool) -> BoundaryInput {
    let gate = |open: bool| measure(if open { 1.0 } else { 0.0 });
    BoundaryInput::Room(Room {
        air: measure(307.0),
        radiant: measure(308.0),
        humidity: gate(leaking),
        air_speed: gate(breathing),
        pressure: measure(765.0),
        oxygen: measure(0.21),
        carbon_dioxide: measure(0.0),
        clothing: measure(0.9),
    })
}

struct Course {
    period: u64,
    leak: Option<(Leak, f64)>,
    leaking: Vec<(u64, u64)>,
    held: Vec<(u64, u64)>,
    end: u64,
}
impl Course {
    /// The hold is 10.5 s, not whole periods, so the edges after it fall half a second off the grid before it.
    fn standard(period: u64, leak: Option<(Leak, f64)>) -> Self {
        let held = 20 * SECOND + period / 2;
        Self {
            period,
            leak,
            leaking: vec![(0, 55 * SECOND)],
            held: vec![(held, held + 10_500 * MILLISECOND)],
            end: 55 * SECOND,
        }
    }
    fn packet(&self) -> ParameterPacket<SyntheticFixture> {
        let mut packet = ParameterPacket::new();
        let size = |which: Leak| match self.leak {
            Some((leak, size)) if leak == which => size,
            _ => 0.0,
        };
        let (key, body) = (ParameterKey::new, Binding::Body);
        let pace = measure::<Rate<Dimensionless>>(SECOND as f64 / self.period as f64);
        packet.synthetic(key("bd.pace"), body, pace, NOTE).expect("once");
        let one = measure::<Dimensionless>(1.0);
        packet
            .synthetic(key("bd.threshold"), body, one, NOTE)
            .expect("once");
        let half = measure::<Dimensionless>(0.5);
        packet.synthetic(key("bd.half"), body, half, NOTE).expect("once");
        let swing = measure::<Rate<Oxygen>>(SWING);
        packet
            .synthetic(key("bd.swing"), body, swing, NOTE)
            .expect("once");
        let oxygen = |value| measure::<Rate<Oxygen>>(value);
        packet
            .synthetic(key("bd.oxygen-drift"), body, oxygen(0.0), NOTE)
            .expect("once");
        let atoms = size(Leak::Oxygen) / 2.0;
        packet
            .synthetic(key("bd.oxygen-leak"), body, oxygen(atoms), NOTE)
            .expect("once");
        let carbon = |value| measure::<Rate<CarbonDioxide>>(value);
        packet
            .synthetic(key("bd.carbon-drift"), body, carbon(1.0), NOTE)
            .expect("once");
        let leaked = size(Leak::Carbon);
        packet
            .synthetic(key("bd.carbon-leak"), body, carbon(leaked), NOTE)
            .expect("once");
        packet
            .synthetic(key("bd.carbon-oxygen-drift"), body, oxygen(0.0), NOTE)
            .expect("once");
        packet
            .synthetic(key("bd.carbon-oxygen-leak"), body, oxygen(leaked), NOTE)
            .expect("once");
        let nitrogen = |value| measure::<Rate<NitrogenGas>>(value);
        packet
            .synthetic(key("bd.nitrogen-drift"), body, nitrogen(1.0), NOTE)
            .expect("once");
        let leaked = size(Leak::Nitrogen);
        packet
            .synthetic(key("bd.nitrogen-leak"), body, nitrogen(leaked), NOTE)
            .expect("once");
        let vasopressin = |value| measure::<Rate<Vasopressin>>(value);
        packet
            .synthetic(key("bd.vasopressin-drift"), body, vasopressin(5e-5), NOTE)
            .expect("once");
        let leaked = size(Leak::Vasopressin);
        packet
            .synthetic(key("bd.vasopressin-leak"), body, vasopressin(leaked), NOTE)
            .expect("once");
        let beds = Binding::Class(Class::CapillaryBed);
        let still = measure::<Rate<Dimensionless>>(0.0);
        packet
            .synthetic(key("mt.burn-speed"), beds, still, NOTE)
            .expect("once");
        let bed = AnatomyBuilder::new().site::<CapillaryBed>("bed", origin()).id();
        let amount = |species| RowKind::Amount {
            site: bed,
            space: Space::Interstitium,
            species,
        };
        let starts = [
            packet.start(amount(Species::Water), measure::<Water>(1_000.0), NOTE),
            packet.start(amount(Species::Glucose), measure::<Glucose>(10.0), NOTE),
            packet.start(amount(Species::Oxygen), measure::<Oxygen>(2_000.0), NOTE),
            packet.start(
                amount(Species::CarbonDioxide),
                measure::<CarbonDioxide>(2_000.0),
                NOTE,
            ),
            packet.start(amount(Species::Vasopressin), measure::<Vasopressin>(1.0), NOTE),
        ];
        for start in starts {
            start.expect("once");
        }
        let heat = RowKind::Amount {
            site: bed,
            space: Space::Thermal,
            species: Species::Energy,
        };
        packet
            .start(heat, measure::<Energy>(1_000.0), NOTE)
            .expect("once");
        let gas = RowKind::Amount {
            site: bed,
            space: Space::Gas,
            species: Species::NitrogenGas,
        };
        packet
            .start(gas, measure::<NitrogenGas>(4_000.0), NOTE)
            .expect("once");
        let phase = RowKind::Memory(named("phase").into());
        packet
            .start(phase, measure::<Dimensionless>(0.0), NOTE)
            .expect("once");
        packet.start_mode(named("beat"), "tick", NOTE).expect("once");
        packet
    }
    fn events(&self) -> Vec<BoundaryEvent> {
        let spans = self.leaking.iter().chain(&self.held);
        let mut times: Vec<u64> = ([0].into_iter())
            .chain(spans.flat_map(|&(from, to)| [from, to]))
            .filter(|&instant| instant < self.end)
            .collect();
        times.sort_unstable();
        times.dedup();
        let inside = |spans: &[(u64, u64)], instant: u64| {
            spans.iter().any(|&(from, to)| (from..to).contains(&instant))
        };
        (times.into_iter())
            .map(|instant| {
                let id = InputId::new(Digest::of_canonical(&instant.to_be_bytes()));
                let room = room(inside(&self.leaking, instant), !inside(&self.held, instant));
                BoundaryEvent::new(id, at(instant), room)
            })
            .collect()
    }
}

fn running<T>(body: &CompiledBody, course: &Course, look: impl FnOnce(&ConservationView<'_>) -> T) -> T {
    let (core, store) = (ReferenceIntegrator::new(STEP), MemoryStore::default());
    let port = BoundaryPort::new(course.events()).expect("a course");
    let session = admit_fixture(body, course.packet(), port, &core, &store).expect("admitted");
    let Ok(session) = run_until(session, at(course.end)) else {
        panic!("the run faulted");
    };
    look(&session.conservation())
}

fn fixed(view: &ConservationView<'_>, drift: f64) -> Ledger {
    let span = view.span().expect("a span").length_ns();
    Ledger::sliding(view.basis(), LENGTH, span, 1e-6, 1e-9, drift)
}

fn rates(basis: &ConservedBasis, early: Option<f64>) -> Vec<(QuantityId, f64, Option<f64>)> {
    (basis.ids())
        .filter_map(|quantity| {
            let leak = (Leak::ALL.into_iter()).find(|leak| leak.book() == quantity.book())?;
            let [_, bound, _] = leak.sizes();
            Some((quantity, bound, early.map(|factor| factor * bound)))
        })
        .collect()
}

fn with(
    rates: &[(QuantityId, f64, Option<f64>)],
    book: Book,
    rate: (f64, Option<f64>),
) -> Vec<(QuantityId, f64, Option<f64>)> {
    (rates.iter())
        .map(|&(quantity, settled, settling)| match quantity.book() == book {
            true => (quantity, rate.0, rate.1),
            false => (quantity, settled, settling),
        })
        .collect()
}

fn quantity(view: &ConservationView<'_>, book: Book) -> QuantityId {
    view.basis()
        .ids()
        .find(|quantity| quantity.book() == book)
        .expect("in the basis")
}

fn marks(view: &ConservationView<'_>, quantity: QuantityId) -> Vec<(u64, f64)> {
    let (rhythm, _) = view.rhythms().next().expect("one rhythm");
    let mut segments = (view.newest_first().collect::<Result<Vec<_>, _>>()).expect("a whole history");
    segments.reverse();
    (segments.iter())
        .filter(|segment| segment.ends_on_edge(rhythm))
        .map(|segment| {
            let end = segment
                .inventory(quantity)
                .on(segment.interval())
                .expect("its own")[1];
            (segment.interval().end().nanoseconds(), hi(end))
        })
        .collect()
}

fn edges(view: &ConservationView<'_>) -> Vec<u64> {
    let any = view.basis().ids().next().expect("a quantity");
    marks(view, any).into_iter().map(|(edge, _)| edge).collect()
}

fn steepest(view: &ConservationView<'_>, book: Book) -> f64 {
    let marks = marks(view, quantity(view, book));
    let mut steepest = 0.0;
    for (index, &(opened, before)) in marks.iter().enumerate() {
        for &(closed, after) in &marks[index + 1..] {
            let rate = (after - before).abs() * SECOND as f64 / (closed - opened) as f64;
            if closed - opened >= LENGTH && rate > steepest {
                steepest = rate;
            }
        }
    }
    steepest
}

fn pair(edges: &[u64], settling: u64, length: u64) -> Interval {
    let opened = *edges.iter().find(|&&edge| edge >= settling).expect("an opening");
    let closed = *edges
        .iter()
        .find(|&&edge| edge >= opened + length)
        .expect("a close");
    interval(opened, closed)
}

fn placements(edges: &[u64]) -> [u64; 3] {
    let held = (edges.windows(2))
        .max_by_key(|pair| pair[1] - pair[0])
        .expect("two edges")[0];
    let closes = |opened: u64| edges.iter().any(|&closed| closed >= opened + LENGTH);
    let tail = *edges
        .iter()
        .rev()
        .find(|&&opened| closes(opened))
        .expect("a window");
    [0, held, tail]
}

fn refused_at_both_paces_with_its_margins(leak: Leak) {
    let body = breathing(true).expect("compiled");
    let [old, bound, small] = leak.sizes();
    for period in [2 * SECOND, 4 * SECOND] {
        let honest = running(&body, &Course::standard(period, None), |view| {
            steepest(view, leak.book())
        });
        let (above, below) = (old - bound - honest, bound - honest - small);
        assert!(
            above > 0.0 && below > 0.0,
            "{leak:?} every {period} ns: {old} is {above} past the bound {bound} with the drift {honest}, and {small} is \
             {below} short of it"
        );
        for sign in [1.0, -1.0] {
            let refused = Course::standard(period, Some((leak, sign * old)));
            running(&body, &refused, |view| {
                let edges = edges(view);
                for settling in placements(&edges) {
                    let window = pair(&edges, settling, LENGTH);
                    let rated = rates(view.basis(), None);
                    let ledger =
                        fixed(view, LOOSE).between_edges(clock(), (settling, LENGTH, WIDEST), &rated);
                    let failure = certified(view, &ledger).expect_err("a leak past its bound");
                    let named = (
                        failure.check(),
                        failure.quantity().map(QuantityId::book),
                        failure.window(),
                    );
                    assert_eq!(
                        named,
                        (CertificateCheck::EdgeDrift, Some(leak.book()), Some(window))
                    );
                    let booked = hi(failure.residual());
                    let allowed = bound * window.length_ns() as f64 / SECOND as f64;
                    assert!(
                        booked > allowed,
                        "{leak:?} {} every {period} ns on {window:?}: booked {booked} beside allowed {allowed}",
                        sign * old
                    );
                }
                assert!(certified(view, &fixed(view, LOOSE)).is_ok(), "closure holds");
            });
            let passed = Course::standard(period, Some((leak, sign * small)));
            running(&body, &passed, |view| {
                for settling in placements(&edges(view)) {
                    let rated = rates(view.basis(), None);
                    let ledger =
                        fixed(view, LOOSE).between_edges(clock(), (settling, LENGTH, WIDEST), &rated);
                    let certificate =
                        certified(view, &ledger).expect("a leak under its bound less the drift");
                    let rhythm = certificate.rhythm().expect("windows between edges");
                    let result = (rhythm.results().iter())
                        .find(|result| result.quantity().book() == leak.book())
                        .expect("rated");
                    let (booked, allowed) = (hi(result.change()), hi(result.allowance()));
                    assert!(
                        result.change().at_most(result.allowance()),
                        "{leak:?} {} every {period} ns: booked {booked} beside allowed {allowed}",
                        sign * small
                    );
                }
            });
        }
    }
}

#[test]
fn an_oxygen_leak_of_20_atoms_a_second_is_refused_between_edges_at_either_pace_and_either_sign() {
    refused_at_both_paces_with_its_margins(Leak::Oxygen);
}

#[test]
fn a_carbon_leak_of_9_atoms_a_second_is_refused_between_edges_at_either_pace_and_either_sign() {
    refused_at_both_paces_with_its_margins(Leak::Carbon);
}

#[test]
fn a_nitrogen_gas_leak_of_20_a_second_is_refused_between_edges_at_either_pace_and_either_sign() {
    refused_at_both_paces_with_its_margins(Leak::Nitrogen);
}

#[test]
fn a_vasopressin_leak_of_3e_4_a_second_is_refused_between_edges_at_either_pace_and_either_sign() {
    refused_at_both_paces_with_its_margins(Leak::Vasopressin);
}

#[test]
fn the_honest_swing_fails_tight_fixed_windows_at_a_pace_that_cuts_them_and_passes_between_edges() {
    let body = breathing(true).expect("compiled");
    running(&body, &Course::standard(4 * SECOND, None), |view| {
        let (tight, oxygen) = (4.0, Leak::Oxygen.book());
        let ten = tight * (LENGTH / SECOND) as f64;
        let drifted = certified(view, &fixed(view, ten)).expect_err("a window that cuts breaths");
        let named = (drifted.check(), drifted.quantity().map(QuantityId::book));
        assert_eq!(named, (CertificateCheck::Drift, Some(oxygen)));
        let rated = with(&rates(view.basis(), None), oxygen, (tight, None));
        let ledger = fixed(view, LOOSE).between_edges(clock(), (0, LENGTH, WIDEST), &rated);
        let certificate = certified(view, &ledger).expect("whole breaths");
        let rhythm = certificate.rhythm().expect("windows between edges");
        let result = (rhythm.results().iter())
            .find(|result| result.quantity().book() == oxygen)
            .expect("rated");
        assert!(
            result.change().at_most(result.allowance()),
            "booked {} beside allowed {} on {:?}, where a fixed window moved {} against {ten}",
            hi(result.change()),
            hi(result.allowance()),
            result.window(),
            hi(drifted.residual())
        );
    });
}

#[test]
fn two_openings_that_mature_on_one_edge_are_each_judged_and_only_the_older_refuses_a_leak_it_alone_holds() {
    let body = breathing(true).expect("compiled");
    let course = Course {
        period: 4 * SECOND,
        leak: Some((Leak::Nitrogen, 100.0)),
        leaking: vec![(5 * SECOND, 7 * SECOND)],
        held: vec![(14 * SECOND, 15 * SECOND)],
        end: 30 * SECOND,
    };
    running(&body, &course, |view| {
        let edges = edges(view);
        let [first, second, third, fourth] = edges[..4] else {
            panic!("four edges");
        };
        assert_eq!([first, second, third, fourth], [4, 8, 12, 17].map(|s| s * SECOND));
        let missed = third - first + 1;
        let rated = rates(view.basis(), None);
        let judged = |settling, length| {
            let ledger = fixed(view, LOOSE).between_edges(clock(), (settling, length, WIDEST), &rated);
            certified(view, &ledger)
        };
        let older = judged(first, missed).expect_err("the older window holds the leak");
        let named = (
            older.check(),
            older.quantity().map(QuantityId::book),
            older.window(),
        );
        let nitrogen = Some(Leak::Nitrogen.book());
        assert_eq!(
            named,
            (
                CertificateCheck::EdgeDrift,
                nitrogen,
                Some(interval(first, fourth))
            ),
            "the third edge misses the length by one nanosecond, so both openings mature on the fourth"
        );
        assert!(judged(second, missed).is_ok(), "the younger pair, judged alone");
        let loose = with(&rated, Leak::Nitrogen.book(), (LOOSE, None));
        let ledger = fixed(view, LOOSE).between_edges(clock(), (first, missed, WIDEST), &loose);
        let certificate = certified(view, &ledger).expect("a loose nitrogen rate");
        let closes = |opened: u64| edges.iter().any(|&closed| closed >= opened + missed);
        let windows = (edges.iter()).filter(|&&opened| opened >= first && closes(opened));
        assert_eq!(
            certificate.rhythm().expect("windows between edges").judged(),
            windows.count() as u64,
            "the fourth edge closes both the first's window and the second's"
        );
        let exact = judged(first, third - first).expect_err("at the length, the third edge closes it");
        assert_eq!(exact.window(), Some(interval(first, third)));
    });
}

#[test]
fn an_amount_the_opening_step_books_cancels_and_one_the_closing_step_books_counts_once() {
    let body = breathing(true).expect("compiled");
    let course = Course {
        period: 4 * SECOND,
        leak: Some((Leak::Nitrogen, 400.0)),
        leaking: vec![(11_750 * MILLISECOND, 12 * SECOND)],
        held: Vec::new(),
        end: 30 * SECOND,
    };
    running(&body, &course, |view| {
        let nitrogen = quantity(view, Leak::Nitrogen.book());
        let mut segments = (view.newest_first().collect::<Result<Vec<_>, _>>()).expect("a whole history");
        segments.reverse();
        let received = |end: u64| {
            let segment = (segments.iter())
                .find(|segment| segment.interval().end() == at(end))
                .expect("a step end");
            let [before, after] = segment.inflow(nitrogen).on(segment.interval()).expect("its own")[..]
            else {
                panic!("two ends");
            };
            hi(after.minus(before))
        };
        let (closing, after) = (received(12 * SECOND), received(12 * SECOND + STEP));
        assert!(
            closing > 100.0 && after < 1.0,
            "the step to the edge books {closing}, the next {after}"
        );
        let length = 8 * SECOND;
        let rated = rates(view.basis(), None);
        let judged = |settling| {
            let ledger = fixed(view, LOOSE).between_edges(clock(), (settling, length, WIDEST), &rated);
            certified(view, &ledger)
        };
        let counted = judged(0).expect_err("the window that closes on the edge counts it");
        let named = (counted.check(), counted.quantity(), counted.window());
        let window = Some(interval(4 * SECOND, 12 * SECOND));
        assert_eq!(named, (CertificateCheck::EdgeDrift, Some(nitrogen), window));
        let opened = judged(12 * SECOND).expect("the window that opens on the edge starts after it");
        assert_eq!(
            opened.rhythm().expect("windows").widest(),
            interval(0, 4 * SECOND)
        );
    });
}

#[test]
fn a_window_takes_the_rate_of_the_stretch_its_opening_edge_is_in() {
    let body = breathing(true).expect("compiled");
    let course = Course {
        period: 2 * SECOND,
        leak: Some((Leak::Nitrogen, 5.0)),
        leaking: vec![(0, 25 * SECOND)],
        held: Vec::new(),
        end: 25 * SECOND,
    };
    running(&body, &course, |view| {
        let settling = 9 * SECOND;
        let rated = with(
            &rates(view.basis(), Some(1.0)),
            Leak::Nitrogen.book(),
            (4.0, Some(8.0)),
        );
        let ledger = fixed(view, LOOSE).between_edges(clock(), (settling, LENGTH, WIDEST), &rated);
        let failure = certified(view, &ledger).expect_err("six a second against four once settled");
        assert_eq!(
            (failure.check(), failure.window()),
            (
                CertificateCheck::EdgeDrift,
                Some(interval(10 * SECOND, 20 * SECOND))
            ),
            "the windows that open before 9 s and close after it take eight a second and pass"
        );
    });
}

#[test]
fn a_windows_allowance_is_its_rate_times_its_whole_seconds_and_its_part_second() {
    let body = breathing(true).expect("compiled");
    running(&body, &Course::standard(4 * SECOND, None), |view| {
        let edges = edges(view);
        let held = pair(&edges, 20 * SECOND, LENGTH);
        assert_eq!(
            held,
            interval(20 * SECOND, 34_500 * MILLISECOND),
            "fourteen and a half seconds"
        );
        let nitrogen = Leak::Nitrogen.book();
        let judged = |rate| {
            let rated = with(&rates(view.basis(), None), nitrogen, (rate, None));
            let ledger = fixed(view, LOOSE).between_edges(clock(), (0, LENGTH, WIDEST), &rated);
            certified(view, &ledger)
        };
        let drift = steepest(view, nitrogen);
        let length = held.length_ns() as f64 / SECOND as f64;
        let (over, under) = (1.02 * length - drift * length, drift * length - 1.02 * 14.0);
        assert!(
            over > 0.0 && under > 0.0,
            "{drift} a second over {length} s sits {over} under 1.02 a second's allowance and {under} past its whole \
             seconds' share"
        );
        assert!(judged(1.02).is_ok(), "the part second counts");
        let refused = judged(0.98).expect_err("under the drift");
        let named = (refused.check(), refused.quantity().map(QuantityId::book));
        assert_eq!(named, (CertificateCheck::EdgeDrift, Some(nitrogen)));
    });
}

#[test]
fn an_empty_rate_list_opens_no_window_before_the_settling_time() {
    let body = breathing(true).expect("compiled");
    running(&body, &Course::standard(4 * SECOND, None), |view| {
        let edges = edges(view);
        let first = *edges.first().expect("an opening");
        let last = *edges.last().expect("a close");
        let span = view.span().expect("a span").length_ns();
        let judged = |settling| {
            let ledger = fixed(view, LOOSE).between_edges(clock(), (settling, last - first, span), &[]);
            let certificate = certified(view, &ledger).expect("the fixed windows and coverage hold");
            let rhythm = certificate.rhythm().expect("windows between edges");
            (rhythm.judged(), rhythm.last_close())
        };
        assert_eq!(judged(first), (1, Some(at(last))));
        assert_eq!(judged(first + 1), (0, None));
    });
}

#[test]
fn the_worst_result_kept_is_the_window_furthest_past_its_allowance_not_the_largest_change() {
    let body = breathing(true).expect("compiled");
    let course = Course {
        period: 2 * SECOND,
        leak: Some((Leak::Nitrogen, 5.0)),
        leaking: vec![(0, 9 * SECOND)],
        held: Vec::new(),
        end: 30 * SECOND,
    };
    running(&body, &course, |view| {
        let (settling, nitrogen) = (10 * SECOND, Leak::Nitrogen.book());
        let rated = rates(view.basis(), Some(1.0));
        let (_, settled, _) = *(rated.iter())
            .find(|(quantity, _, _)| quantity.book() == nitrogen)
            .expect("rated");
        let rated = with(&rated, nitrogen, (settled, Some(1e3)));
        let ledger = fixed(view, LOOSE).between_edges(clock(), (settling, LENGTH, WIDEST), &rated);
        let certificate = certified(view, &ledger).expect("a loose rate while the leak runs");
        let rhythm = certificate.rhythm().expect("windows between edges");
        let kept = (rhythm.results().iter())
            .find(|result| result.quantity().book() == nitrogen)
            .expect("rated");
        assert!(
            kept.window().start() >= at(settling),
            "kept {:?}, which moved {} against {}, where a window open on the leak moved more under a far larger \
             allowance",
            kept.window(),
            hi(kept.change()),
            hi(kept.allowance())
        );
    });
}

#[test]
fn a_leak_before_the_settling_time_is_judged_only_where_the_policy_states_a_rate_for_that_stretch() {
    let body = breathing(true).expect("compiled");
    let course = Course {
        period: 2 * SECOND,
        leak: Some((Leak::Nitrogen, 50.0)),
        leaking: vec![(3 * SECOND, 9 * SECOND)],
        held: Vec::new(),
        end: 35 * SECOND,
    };
    running(&body, &course, |view| {
        let judged = |early| {
            let rated = rates(view.basis(), early);
            let ledger = fixed(view, LOOSE).between_edges(clock(), (20 * SECOND, LENGTH, WIDEST), &rated);
            certified(view, &ledger)
        };
        let failure = judged(Some(1.0)).expect_err("a rate for the settling stretch");
        let window = Some(interval(2 * SECOND, 12 * SECOND));
        assert_eq!(
            (failure.check(), failure.window()),
            (CertificateCheck::EdgeDrift, window)
        );
        assert!(judged(None).is_ok(), "no edge before 20 s opens a window");
    });
}

#[test]
fn the_tail_runs_from_the_last_close_judged_and_not_from_the_last_edge() {
    let body = breathing(true).expect("compiled");
    let course = Course {
        period: SECOND,
        leak: None,
        leaking: Vec::new(),
        held: vec![
            (51_500 * MILLISECOND, 59_500 * MILLISECOND),
            (62_500 * MILLISECOND, 71_500 * MILLISECOND),
        ],
        end: 71_500 * MILLISECOND,
    };
    let (core, store) = (ReferenceIntegrator::new(STEP), MemoryStore::default());
    let port = BoundaryPort::new(course.events()).expect("a course");
    let session = admit_fixture(&body, course.packet(), port, &core, &store).expect("admitted");
    let judged = |view: &ConservationView<'_>, widest| {
        let rated = rates(view.basis(), None);
        let ledger = fixed(view, LOOSE).between_edges(clock(), (0, LENGTH, widest), &rated);
        certified(view, &ledger)
    };
    let Ok(session) = run_until(session, at(62 * SECOND)) else {
        panic!("the run faulted");
    };
    let view = session.conservation();
    let edges = edges(&view);
    assert_eq!(edges[edges.len() - 4..], [51, 60, 61, 62].map(|s| s * SECOND));
    let certificate = judged(&view, 9 * SECOND).expect("a stretch at its bound");
    let rhythm = certificate.rhythm().expect("windows between edges");
    let close = rhythm.last_close().expect("a close");
    assert_eq!(
        (close, rhythm.widest()),
        (at(61 * SECOND), interval(51 * SECOND, 60 * SECOND))
    );
    assert_eq!(
        certificate.span().end().nanoseconds() - close.nanoseconds(),
        SECOND,
        "a tail of one second"
    );
    let stopped = judged(&view, 9 * SECOND - 1).expect_err("one nanosecond past");
    let named = (stopped.check(), stopped.window());
    assert_eq!(
        named,
        (
            CertificateCheck::RhythmStopped,
            Some(interval(51 * SECOND, 60 * SECOND))
        )
    );

    let Ok(session) = run_until(session, at(course.end)) else {
        panic!("the run faulted");
    };
    let view = session.conservation();
    let tail = Some(interval(61 * SECOND, course.end));
    let stopped =
        judged(&view, 10 * SECOND).expect_err("10.5 s from the last close, 9.5 s from the last edge");
    assert_eq!(
        (stopped.check(), stopped.window()),
        (CertificateCheck::RhythmStopped, tail)
    );
    let certificate = judged(&view, 10_500 * MILLISECOND).expect("a tail at its bound");
    let rhythm = certificate.rhythm().expect("windows between edges");
    assert_eq!(
        (rhythm.last_close(), Some(rhythm.widest())),
        (Some(at(61 * SECOND)), tail)
    );
}

#[test]
fn a_run_in_which_no_window_closed_is_one_stretch_from_admission_to_the_head() {
    let body = breathing(true).expect("compiled");
    let mut course = Course::standard(4 * SECOND, None);
    course.end = running(&body, &course, |view| {
        edges(view)
            .into_iter()
            .find(|&edge| edge >= LENGTH)
            .expect("an edge after a full fixed window")
    });
    running(&body, &course, |view| {
        let span = view.span().expect("a span");
        let mut instants = vec![span.start().nanoseconds()];
        instants.extend(edges(view));
        assert_eq!(instants.last().copied(), Some(span.end().nanoseconds()));
        let widest = (instants.windows(2))
            .map(|ends| ends[1] - ends[0])
            .max()
            .expect("an edge");
        assert!(widest < span.length_ns(), "the run holds several edges");
        let rated = rates(view.basis(), None);
        let judged = |bound| {
            let ledger = fixed(view, LOOSE).between_edges(clock(), (span.length_ns(), LENGTH, bound), &rated);
            certified(view, &ledger)
        };
        let stopped = judged(widest).expect_err("no window closed, so no stretch ends before the head");
        assert_eq!(
            (stopped.check(), stopped.window()),
            (CertificateCheck::RhythmStopped, Some(span))
        );
        let certificate = judged(span.length_ns()).expect("a bound that reaches the run's length");
        let rhythm = certificate.rhythm().expect("windows between edges");
        assert_eq!((rhythm.judged(), rhythm.last_close()), (0, None));
    });
}

#[test]
fn a_leak_before_the_first_edge_is_the_fixed_windows_to_see_and_one_after_the_last_close_is_theirs_and_coverages()
 {
    let body = breathing(true).expect("compiled");
    let early = Course {
        period: 4 * SECOND,
        leak: Some((Leak::Nitrogen, 100.0)),
        leaking: vec![(500 * MILLISECOND, 3_500 * MILLISECOND)],
        held: Vec::new(),
        end: 30 * SECOND,
    };
    let nitrogen = Some(Leak::Nitrogen.book());
    running(&body, &early, |view| {
        let rated = rates(view.basis(), None);
        let ledger = |drift| fixed(view, drift).between_edges(clock(), (0, LENGTH, WIDEST), &rated);
        assert!(
            certified(view, &ledger(LOOSE)).is_ok(),
            "no window between edges holds it"
        );
        let seen = certified(view, &ledger(100.0)).expect_err("a fixed window holds it");
        let named = (seen.check(), seen.quantity().map(QuantityId::book), seen.window());
        assert_eq!(
            named,
            (CertificateCheck::Drift, nitrogen, Some(interval(0, LENGTH)))
        );
    });
    let late = Course {
        period: 4 * SECOND,
        leak: Some((Leak::Nitrogen, 100.0)),
        leaking: vec![(28 * SECOND, 31 * SECOND)],
        held: vec![(26 * SECOND, 40 * SECOND)],
        end: 40 * SECOND,
    };
    running(&body, &late, |view| {
        let rated = rates(view.basis(), None);
        let ledger = |drift, widest| fixed(view, drift).between_edges(clock(), (0, LENGTH, widest), &rated);
        let certificate =
            certified(view, &ledger(LOOSE, 16 * SECOND)).expect("no window between edges holds it");
        let rhythm = certificate.rhythm().expect("windows between edges");
        let tail = interval(24 * SECOND, 40 * SECOND);
        assert_eq!(
            (rhythm.last_close(), rhythm.widest()),
            (Some(at(24 * SECOND)), tail)
        );
        let stopped = certified(view, &ledger(LOOSE, 15 * SECOND)).expect_err("coverage bounds the tail");
        assert_eq!(
            (stopped.check(), stopped.window()),
            (CertificateCheck::RhythmStopped, Some(tail))
        );
        let seen = certified(view, &ledger(100.0, 16 * SECOND)).expect_err("a fixed window holds it");
        let named = (seen.check(), seen.quantity().map(QuantityId::book));
        assert_eq!(named, (CertificateCheck::Drift, nitrogen));
    });
}

#[test]
fn fixed_windows_report_the_admission_drift_when_another_quantity_drifts_one_step_later() {
    let body = breathing(true).expect("compiled");
    let course = Course {
        period: 4 * SECOND,
        leak: Some((Leak::Oxygen, 30.0)),
        leaking: vec![(0, LENGTH)],
        held: vec![(0, 30 * SECOND)],
        end: 30 * SECOND,
    };
    running(&body, &course, |view| {
        let judged = |coverage| {
            let ledger = Ledger::sliding(view.basis(), LENGTH, coverage, 1e-6, 1e-9, 4.0);
            let failure = certified(view, &ledger).expect_err("the admission window drifts");
            (failure.check(), failure.window())
        };
        let first = (CertificateCheck::Drift, Some(interval(0, LENGTH)));
        assert_eq!(judged(LENGTH), first, "the admission window judged alone");
        assert_eq!(judged(view.span().expect("a span").length_ns()), first);
    });
}

#[test]
fn a_fixed_drift_outranks_a_stopped_rhythm_which_outranks_an_earlier_drift_between_edges() {
    let body = breathing(true).expect("compiled");
    running(
        &body,
        &Course::standard(4 * SECOND, Some((Leak::Nitrogen, 20.0))),
        |view| {
            let rated = rates(view.basis(), None);
            let ledger =
                |drift, widest| fixed(view, drift).between_edges(clock(), (0, LENGTH, widest), &rated);
            let failed = |drift, widest| {
                let failure = certified(view, &ledger(drift, widest)).expect_err("a leak");
                (failure.check(), failure.window())
            };
            let first = interval(0, LENGTH);
            assert_eq!(failed(100.0, 12 * SECOND), (CertificateCheck::Drift, Some(first)));
            let held = Some(interval(20 * SECOND, 34_500 * MILLISECOND));
            assert_eq!(
                failed(LOOSE, 12 * SECOND),
                (CertificateCheck::RhythmStopped, held)
            );
            let opening = Some(interval(4 * SECOND, 16 * SECOND));
            assert_eq!(failed(LOOSE, WIDEST), (CertificateCheck::EdgeDrift, opening));
            let span = view.span().expect("a span").length_ns();
            let long = Ledger::sliding(view.basis(), 40 * SECOND, span, 1e-6, 1e-9, LOOSE);
            let long = long.between_edges(clock(), (0, LENGTH, WIDEST), &rated);
            let failure = certified(view, &long).expect_err("a leak");
            assert_eq!(
                (failure.check(), failure.window()),
                (CertificateCheck::EdgeDrift, opening),
                "fixed windows of 40 s begin reading a page past the first edges, and the windows between edges \
                 still read from admission"
            );
        },
    );
}

#[test]
fn the_fixed_windows_judge_as_they_did_and_the_windows_between_edges_run_to_the_head_past_one_fixed_window() {
    let body = breathing(true).expect("compiled");
    let mut course = Course::standard(4 * SECOND, Some((Leak::Nitrogen, 20.0)));
    course.leaking = vec![(35 * SECOND, course.end)];
    running(&body, &course, |view| {
        let span = view.span().expect("a span").length_ns();
        let loose = with(&rates(view.basis(), None), Leak::Nitrogen.book(), (LOOSE, None));
        let rated = rates(view.basis(), None);
        let failure = |outcome: Result<Certificate, CertificateFailure>| {
            let failure = outcome.expect_err("a fixed drift");
            (
                failure.check(),
                failure.quantity(),
                failure.window(),
                failure.residual().bits(),
            )
        };
        let parts = |certificate: &Certificate| {
            let parts = (certificate.results(), certificate.support(), certificate.pieces());
            format!("{parts:?}")
        };
        for coverage in [LENGTH, span] {
            let ledger = |drift| Ledger::sliding(view.basis(), LENGTH, coverage, 1e-6, 1e-9, drift);
            let edged = |drift, rates: &[(QuantityId, f64, Option<f64>)]| {
                ledger(drift).between_edges(clock(), (0, LENGTH, WIDEST), rates)
            };
            let plain = certified(view, &ledger(LOOSE)).expect("loose");
            let paced = certified(view, &edged(LOOSE, &loose)).expect("loose");
            assert_eq!(parts(&plain), parts(&paced), "coverage {coverage}");
            let tight = failure(certified(view, &ledger(5.0)));
            assert_eq!(
                tight,
                failure(certified(view, &edged(5.0, &rated))),
                "coverage {coverage}"
            );
        }
        let one = || Ledger::sliding(view.basis(), LENGTH, LENGTH, 1e-6, 1e-9, LOOSE);
        let edges = edges(view);
        let before = *edges
            .iter()
            .rev()
            .find(|&&edge| edge < 35 * SECOND)
            .expect("an edge");
        let late = certified(view, &one().between_edges(clock(), (0, LENGTH, WIDEST), &rated));
        let late = late.expect_err("past the one fixed window");
        let named = (late.check(), late.window());
        assert_eq!(
            named,
            (CertificateCheck::EdgeDrift, Some(pair(&edges, before, LENGTH)))
        );
        let stopped = certified(
            view,
            &one().between_edges(clock(), (0, LENGTH, 12 * SECOND), &rated),
        );
        let stopped = stopped.expect_err("a breath held past the bound");
        let held = Some(interval(20 * SECOND, 34_500 * MILLISECOND));
        assert_eq!(
            (stopped.check(), stopped.window()),
            (CertificateCheck::RhythmStopped, held)
        );
    });
}

#[test]
fn an_edge_drift_reports_the_allowance_that_the_change_exceeded() {
    // Solver contract section 7 requires "the certificate names the quantity and the window's two edges,
    // with the change beside its edge rate times the window's length".
    let body = breathing(true).expect("compiled");
    running(
        &body,
        &Course::standard(4 * SECOND, Some((Leak::Nitrogen, 20.0))),
        |view| {
            let nitrogen = quantity(view, Leak::Nitrogen.book());
            let window = interval(4 * SECOND, 16 * SECOND);
            let report = |rate| {
                let rated = with(&rates(view.basis(), None), nitrogen.book(), (rate, None));
                let ledger = fixed(view, LOOSE).between_edges(clock(), (0, LENGTH, WIDEST), &rated);
                let failure = certified(view, &ledger).expect_err("the nitrogen leak exceeds the rate");
                assert_eq!(
                    (failure.check(), failure.quantity(), failure.window()),
                    (CertificateCheck::EdgeDrift, Some(nitrogen), Some(window))
                );
                (failure.residual().bits(), format!("{failure:?}"))
            };
            let [(first_change, first_report), (second_change, second_report)] = [6.0, 7.0].map(report);
            assert_eq!(first_change, second_change);
            assert_ne!(first_report, second_report, "the same change exceeded 72 and 84");
        },
    );
}

#[test]
fn an_edge_drift_carries_the_change_beside_the_allowance_it_passed() {
    let body = breathing(true).expect("compiled");
    let course = Course::standard(4 * SECOND, Some((Leak::Nitrogen, 20.0)));
    running(&body, &course, |view| {
        let nitrogen = quantity(view, Leak::Nitrogen.book());
        for (rate, allowed) in [(6.0, 72.0), (7.0, 84.0)] {
            let rated = with(&rates(view.basis(), None), nitrogen.book(), (rate, None));
            let ledger = fixed(view, LOOSE).between_edges(clock(), (0, LENGTH, WIDEST), &rated);
            let failure = certified(view, &ledger).expect_err("the nitrogen leak exceeds the rate");
            let named = (failure.check(), failure.quantity(), failure.window());
            let window = Some(interval(4 * SECOND, 16 * SECOND));
            assert_eq!(named, (CertificateCheck::EdgeDrift, Some(nitrogen), window));
            let (change, allowance) = (hi(failure.residual()), failure.allowance().map(hi));
            assert_eq!((change, allowance), (252.0, Some(allowed)), "at {rate} a second");
        }
    });
}

#[test]
fn one_policy_resolves_its_clock_where_a_model_declares_it_and_refuses_it_by_name_where_one_does_not() {
    let (declared, undeclared) = (
        breathing(true).expect("compiled"),
        breathing(false).expect("compiled"),
    );
    let basis = declared.conserved_basis();
    assert_eq!(
        basis.digest(),
        undeclared.conserved_basis().digest(),
        "a declaration is no part of the basis"
    );
    let course = Course {
        end: 30 * SECOND,
        ..Course::standard(4 * SECOND, None)
    };
    let ledger = || {
        let rated = rates(basis, None);
        Ledger::sliding(basis, LENGTH, 30 * SECOND, 1e-6, 1e-9, LOOSE).between_edges(
            clock(),
            (0, LENGTH, WIDEST),
            &rated,
        )
    };
    running(&declared, &course, |view| {
        let certificate = certified(view, &ledger()).expect("its clock resolves");
        assert_eq!(
            certificate.rhythm().map(|rhythm| rhythm.windows().clock()),
            Some(clock())
        );
        let other = Slot::from(named("other"));
        let rated = rates(basis, None);
        let elsewhere = Ledger::sliding(basis, LENGTH, 30 * SECOND, 1e-6, 1e-9, LOOSE);
        let elsewhere = elsewhere.between_edges(other, (0, LENGTH, WIDEST), &rated);
        let refused = certified(view, &elsewhere).expect_err("a clock this model declares no rhythm of");
        assert_eq!(refused.check(), CertificateCheck::UndeclaredRhythm);
    });
    running(&undeclared, &course, |view| {
        let refused = certified(view, &ledger()).expect_err("the same memory and reset, undeclared");
        assert_eq!(refused.check(), CertificateCheck::UndeclaredRhythm);
    });
    let (core, store) = (ReferenceIntegrator::new(STEP), MemoryStore::default());
    let port = BoundaryPort::new(course.events()).expect("a course");
    let admitted = admit_fixture(&undeclared, course.packet(), port, &core, &store).expect("admitted");
    let refused = certified(&admitted.conservation(), &ledger()).expect_err("no step yet");
    assert_eq!(
        refused.check(),
        CertificateCheck::UndeclaredRhythm,
        "the name is checked before the span"
    );
    let start = [600.0, 424.0].map(measure);
    let case = TwoReservoirs::delivering(start, Vec::new())
        .expect("a body")
        .until(at(30 * SECOND));
    let (core, store) = (ReferenceIntegrator::new(STEP), MemoryStore::default());
    let session = case.admit(&core, &store).expect("admitted");
    let Ok(session) = run_until(session, case.end()) else {
        panic!("the run faulted");
    };
    let refused = certified(&session.conservation(), &ledger()).expect_err("another basis");
    assert_eq!(
        refused.check(),
        CertificateCheck::Coverage,
        "the basis check comes first"
    );
}

#[test]
fn a_fixed_policy_keeps_its_old_digest_and_each_field_of_the_windows_between_edges_moves_it() {
    let body = breathing(true).expect("compiled");
    let basis = body.conserved_basis();
    let coverage = 20 * SECOND;
    let ledger = || Ledger::sliding(basis, LENGTH, coverage, 1e-6, 1e-9, LOOSE);
    let mut bytes = basis.digest().as_bytes().to_vec();
    for number in [0, LENGTH, coverage] {
        bytes.extend_from_slice(&number.to_be_bytes());
    }
    for _ in basis.ids() {
        bytes.extend_from_slice(&0_u64.to_be_bytes());
        for value in [1e-6, 1e-9, LOOSE] {
            bytes.extend_from_slice(&value.to_bits().to_be_bytes());
            bytes.extend_from_slice(&0_u64.to_be_bytes());
        }
    }
    assert_eq!(
        ledger().digest(),
        Digest::of_canonical(&bytes),
        "the bytes it had before"
    );
    let rated = rates(basis, None);
    let edged = |clock, (settling, length, widest), rates: &[(QuantityId, f64, Option<f64>)]| {
        ledger()
            .between_edges(clock, (settling, length, widest), rates)
            .digest()
    };
    let windows = (0, LENGTH, WIDEST);
    let (first, settled) = (rated[0].0, rated[0].1);
    let equal = |other: usize| [(first, 1.0, None), (rated[other].0, 1.0, None)];
    let bed = AnatomyBuilder::new().site::<CapillaryBed>("bed", origin()).id();
    let elsewhere = Slot {
        at: Context::Site(bed),
        ..clock()
    };
    let digests = [
        ledger().digest(),
        edged(clock(), windows, &rated),
        edged(Slot::from(named("other")), windows, &rated),
        edged(clock(), (SECOND, LENGTH, WIDEST), &rated),
        edged(clock(), (0, LENGTH + 1, WIDEST), &rated),
        edged(clock(), (0, LENGTH, WIDEST + 1), &rated),
        edged(
            clock(),
            windows,
            &with(&rated, first.book(), (settled * 2.0, None)),
        ),
        edged(clock(), windows, &rates(basis, Some(1.0))),
        edged(clock(), windows, &rated[1..]),
        edged(clock(), windows, &[]),
        edged(clock(), windows, &equal(1)),
        edged(clock(), windows, &equal(2)),
        edged(elsewhere, windows, &rated),
    ];
    for (index, digest) in digests.iter().enumerate() {
        assert!(
            !digests[..index].contains(digest),
            "digest {index} repeats one before it"
        );
    }
}

#[test]
fn rates_are_refused_in_another_dimension_out_of_basis_order_twice_or_settling_for_some_quantities_only() {
    let body = breathing(true).expect("compiled");
    let basis = body.conserved_basis();
    let nitrogen = (basis.ids()).find(|quantity| quantity.book() == Leak::Nitrogen.book());
    let vasopressin = (basis.ids()).find(|quantity| quantity.book() == Leak::Vasopressin.book());
    let (nitrogen, vasopressin) = (nitrogen.expect("nitrogen gas"), vasopressin.expect("vasopressin"));
    let refused = EdgeRate::new(nitrogen, measure::<Rate<Water>>(1.0), None).err();
    assert_eq!(
        refused,
        Some(AdmissionError::QuantityDimension { quantity: nitrogen })
    );
    let gas = |settling: Option<f64>| {
        EdgeRate::new(nitrogen, measure::<Rate<NitrogenGas>>(1.0), settling.map(measure)).expect("its own")
    };
    let hormone = |settling: Option<f64>| {
        EdgeRate::new(
            vasopressin,
            measure::<Rate<Vasopressin>>(1.0),
            settling.map(measure),
        )
        .expect("its own")
    };
    let length = core::num::NonZeroU64::new(LENGTH).expect("positive");
    let windows = EdgeWindows::new(clock(), 0, length, WIDEST);
    let judged = |rates: Vec<EdgeRate>| {
        let ledger = Ledger::sliding(basis, LENGTH, 20 * SECOND, 1e-6, 1e-9, LOOSE);
        ledger.policy.between_edges(windows, rates).err()
    };
    let in_order = |first: EdgeRate, second: EdgeRate| match nitrogen.index() < vasopressin.index() {
        true => vec![first, second],
        false => vec![second, first],
    };
    let coverage = Some(AdmissionError::PolicyCoverage);
    assert_eq!(judged(in_order(gas(None), hormone(None))), None);
    assert_eq!(judged(in_order(gas(Some(2.0)), hormone(Some(2.0)))), None);
    let mut reversed = in_order(gas(None), hormone(None));
    reversed.reverse();
    assert_eq!(judged(reversed), coverage, "out of basis order");
    assert_eq!(judged(vec![gas(None), gas(None)]), coverage, "one quantity twice");
    assert_eq!(
        judged(in_order(gas(Some(2.0)), hormone(None))),
        coverage,
        "settling for one only"
    );
}

#[test]
fn a_policy_refuses_a_second_clock() {
    let body = breathing(true).expect("compiled");
    let basis = body.conserved_basis();
    let length = core::num::NonZeroU64::new(LENGTH).expect("positive");
    let windows = EdgeWindows::new(clock(), 0, length, WIDEST);
    let policy = Ledger::sliding(basis, LENGTH, 20 * SECOND, 1e-6, 1e-9, LOOSE).policy;
    let once = policy.between_edges(windows, Vec::new()).expect("one clock");
    let twice = once.between_edges(windows, Vec::new()).err();
    assert_eq!(twice, Some(AdmissionError::PolicyCoverage));
}

fn derived(fields: &[(&str, &dyn core::fmt::Debug)], pretty: bool) -> String {
    if pretty {
        let mut out = String::from("Certificate {\n");
        for (name, value) in fields {
            let value = format!("{value:#?}").replace('\n', "\n    ");
            out.push_str(&format!("    {name}: {value},\n"));
        }
        out + "}"
    } else {
        let fields: Vec<String> = (fields.iter())
            .map(|(name, value)| format!("{name}: {value:?}"))
            .collect();
        format!("Certificate {{ {} }}", fields.join(", "))
    }
}

fn base_fields(certificate: &Certificate) -> [(&'static str, Box<dyn core::fmt::Debug + '_>); 9] {
    let seal: HistorySeal = certificate.seal();
    [
        ("seal", Box::new(seal)),
        ("basis", Box::new(certificate.basis())),
        ("policy", Box::new(certificate.policy())),
        ("windows", Box::new(certificate.windows())),
        ("span", Box::new(certificate.span())),
        ("results", Box::new(certificate.results())),
        ("coverage", Box::new(certificate.coverage())),
        ("support", Box::new(certificate.support())),
        ("pieces", Box::new(certificate.pieces())),
    ]
}

#[test]
fn a_certificate_prints_as_its_derive_did_and_adds_its_rhythm_only_where_one_was_judged() {
    let body = breathing(true).expect("compiled");
    running(&body, &Course::standard(2 * SECOND, None), |view| {
        let plain = certified(view, &fixed(view, LOOSE)).expect("honest");
        assert!(plain.rhythm().is_none());
        let fields = base_fields(&plain);
        let fields: Vec<(&str, &dyn core::fmt::Debug)> = (fields.iter())
            .map(|(name, value)| (*name, value.as_ref()))
            .collect();
        assert_eq!(format!("{plain:?}"), derived(&fields, false));
        assert_eq!(format!("{plain:#?}"), derived(&fields, true));
        let rated = rates(view.basis(), None);
        let ledger = fixed(view, LOOSE).between_edges(clock(), (0, LENGTH, WIDEST), &rated);
        let paced = certified(view, &ledger).expect("honest");
        let rhythm = paced.rhythm().expect("windows between edges");
        let fields = base_fields(&paced);
        let fields: Vec<(&str, &dyn core::fmt::Debug)> = (fields.iter())
            .map(|(name, value)| (*name, value.as_ref()))
            .chain([("rhythm", rhythm as &dyn core::fmt::Debug)])
            .collect();
        assert_eq!(format!("{paced:?}"), derived(&fields, false));
        assert_eq!(format!("{paced:#?}"), derived(&fields, true));
    });
}

fn assert_fold_matches_oracle(
    session: &ng_record::Session<'_, ng_record::Running>,
    fold: &CertificateFold<'_, '_>,
    ledger: &Ledger,
    label: &str,
) {
    let want = certified(&session.conservation(), ledger);
    let got = fold.certificate();
    match (got, want) {
        (Ok(a), Ok(b)) => assert_eq!(format!("{a:?}"), format!("{b:?}"), "{label}: ok"),
        (Err(a), Err(b)) => assert_eq!(
            (
                a.check(),
                a.quantity(),
                a.window(),
                a.residual().bits(),
                a.allowance().map(Compensated::bits),
            ),
            (
                b.check(),
                b.quantity(),
                b.window(),
                b.residual().bits(),
                b.allowance().map(Compensated::bits),
            ),
            "{label}: err"
        ),
        (a, b) => panic!("{label}: mismatch fold={a:?} certify/oracle={b:?}"),
    }
}

#[test]
fn the_live_fold_matches_certify_and_the_oracle_on_every_window_between_edges() {
    let body = breathing(true).expect("compiled");
    let basis = body.conserved_basis();
    let cases = [
        (
            "passing standard",
            Course::standard(2 * SECOND, None),
            0,
            None,
            WIDEST,
        ),
        (
            "passing with settling",
            Course::standard(4 * SECOND, None),
            10 * SECOND,
            Some(2.0),
            WIDEST,
        ),
        (
            "edge drift oxygen",
            Course::standard(4 * SECOND, Some((Leak::Oxygen, 20.0))),
            0,
            None,
            WIDEST,
        ),
        (
            "edge drift pre-settling nitrogen",
            Course {
                end: 40 * SECOND,
                ..Course::standard(2 * SECOND, Some((Leak::Nitrogen, 20.0)))
            },
            12 * SECOND,
            Some(0.5),
            WIDEST,
        ),
        (
            "rhythm stopped mid-run",
            Course::standard(2 * SECOND, None),
            0,
            None,
            8 * SECOND,
        ),
    ];
    for (label, course, settling, early, widest) in cases {
        let (core, store) = (ReferenceIntegrator::new(STEP), MemoryStore::default());
        let port = BoundaryPort::new(course.events()).expect("a course");
        let mut session = admit_fixture(&body, course.packet(), port, &core, &store).expect("admitted");
        let rated = rates(basis, early);
        let ledger = Ledger::sliding(basis, LENGTH, 20 * SECOND, 1e-6, 1e-9, LOOSE).between_edges(
            clock(),
            (settling, LENGTH, widest),
            &rated,
        );
        let mut fold = CertificateFold::open(&session, &ledger.policy).expect("opened");
        let mut steps = 0u64;
        while session.record().accepted().time() < at(course.end) {
            let mut attempt = session.step();
            session = loop {
                match attempt {
                    ng_record::Step::Accepted(next, _) => break next,
                    ng_record::Step::Refused(retrying) => attempt = retrying.retry(),
                    ng_record::Step::Faulted(stopped) => panic!("{label}: faulted {:?}", stopped.fault()),
                }
            };
            fold.advance(&session).expect("same session");
            steps += 1;
            if steps.is_multiple_of(40) {
                assert_fold_matches_oracle(&session, &fold, &ledger, label);
            }
        }
        assert_fold_matches_oracle(&session, &fold, &ledger, label);
    }
}
