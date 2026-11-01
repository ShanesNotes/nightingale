//! Ticket 08: its four checks and every refusal its design adds, through the public API only. Check 4 is a
//! compile-fail doctest in `src/lib.rs`, because it is a type rule.
//!
//! The manufactured case. Two vessels `a -> b` hold water, `q = k (a - b)` with `k = g e`. A pulse of `P` enters
//! `a` over `[0, tau)` of each period `T`, and `P` leaves `b` evenly over the whole period, so
//!
//!     a' = -q + u(t)    b' = q - v    u = P / tau on [0, tau), else 0    v = P / T
//!
//! With `s = a + b` and `d = a - b`:
//!
//!     s' = u - v, which rises by `H = P (1 - tau / T)` and falls back, so `mean s = S + H / 2` for `S = s(0)`
//!     d' = -2 k d + u + v, whose integral over a closed cycle gives `mean d = (mean u + v) / (2 k) = v / k`
//!
//! So `mean a = (S + H / 2 + v / k) / 2`, and the elastance that makes the cycle mean of `a` equal `A` is
//!
//!     e = v / (g (2 A - S - H / 2))
//!
//! The ring of `N` backward Euler stages keeps both means exactly when the drive's edges sit on points. Summing the
//! stages of `d` around the ring gives the same `v / k`, and on a uniform ring the mean of the points of the
//! piecewise linear `s` is its integral. Only the wave's shape carries the grid's error: `d` steps by
//! `r = 1 / (1 + 2 k h)` where the continuous orbit decays by `exp(-2 k h)`, and with `D1 = (P / tau + v) / (2 k)`,
//! `D2 = v / (2 k)`, `E1 = r^(N tau / T)` and `E2 = r^(N (1 - tau / T))`,
//!
//!     d(0) = (D2 (1 - E2) + D1 E2 (1 - E1)) / (1 - E1 E2)

use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_observe::*;
use ng_record::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const PERIOD_NS: u64 = SECOND;
const PULSE_NS: u64 = SECOND / 4;
/// Half a step of a reference core at `PERIOD_NS / 32` is one interval, so that core's orbit is these points'.
const INTERVALS: u32 = 64;
const REFERENCE_NS: u64 = PERIOD_NS / 32;

const DELIVERED: f64 = 8.0;
const CONDUCTANCE: f64 = 0.5;
const TOTAL: f64 = 100.0;
const MEAN_A: f64 = 55.5;
const GUESS: f64 = 1.0;
/// Unequal, because `q` reads `e (a - b)`: a guess with no head leaves the fit nothing to move.
const START: [f64; 2] = [53.0, 47.0];

const PULSE: &str = "pulse";
const DRAIN: &str = "drain";
const GAUGE: &str = "a synthetic gauge";
const E: ParameterKey = ParameterKey::new("cv.e");
const G: ParameterKey = ParameterKey::new("cv.g");
const TONE: ParameterKey = ParameterKey::new("cv.tone");
const RATIO: ParameterKey = ParameterKey::new("cv.ratio");
const DOUBLED: ParameterKey = ParameterKey::new("cv.doubled");
const PACE: ParameterKey = ParameterKey::new("cv.pace");
const SET: ParameterKey = ParameterKey::new("cv.set");
const KNOTS: ParameterKey = ParameterKey::new("cv.knots");
const WOBBLE: ParameterKey = ParameterKey::new("cv.wobble");
const MEAN: AnchorId = AnchorId::new("fixture.mean-a");
const NETWORK: Binding = Binding::Network(Network::Vascular);
const NOTE: &str = "synthetic";
const SETTLED: &str = "settled";
const READING: &str = "reading";
const ECHO: &str = "echo";
/// One unit of water over the one second of the instant's delivery.
const CHANNEL_FLOW: f64 = 3.0;

fn at(ns: u64) -> SimTime {
    SimTime::from_ns(ns)
}

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn count(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("not zero")
}

fn hi(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn name(local: &'static str) -> LawName {
    LawName { owner: "CV", local }
}

fn lumen(site: SiteId) -> RowKind {
    RowKind::Amount {
        site,
        space: Space::Lumen,
    }
}

fn reference(interval_ns: u64) -> ReferenceIntegrator {
    ReferenceIntegrator::new(interval_ns)
}

fn cycle(intervals: u32) -> Cycle {
    let intervals = NonZeroU32::new(intervals).expect("not zero");
    Cycle::new(count(PERIOD_NS), intervals).expect("even")
}

fn steady_k() -> f64 {
    let v = DELIVERED / seconds(PERIOD_NS);
    let rise = DELIVERED * (1.0 - PULSE_NS as f64 / PERIOD_NS as f64);
    v / (2.0 * MEAN_A - TOTAL - rise / 2.0)
}

fn seconds(ns: u64) -> f64 {
    ns as f64 / SECOND as f64
}

/// `e = v / (g (2 A - S - H / 2))`.
fn fitted_elastance() -> f64 {
    steady_k() / CONDUCTANCE
}

/// `a(0) = (S + d(0)) / 2` on a ring of `intervals` stages.
fn start_a(intervals: u32) -> f64 {
    let (k, period, tau) = (steady_k(), seconds(PERIOD_NS), seconds(PULSE_NS));
    let v = DELIVERED / period;
    let (d1, d2) = ((DELIVERED / tau + v) / (2.0 * k), v / (2.0 * k));
    let h = period / f64::from(intervals);
    let r = 1.0 / (1.0 + 2.0 * k * h);
    let power = |steps: u32| (0..steps).fold(1.0, |product, _| product * r);
    let pulse_steps = intervals / 4;
    let (e1, e2) = (power(pulse_steps), power(intervals - pulse_steps));
    let d = (d2 * (1.0 - e2) + d1 * e2 * (1.0 - e1)) / (1.0 - e1 * e2);
    (TOTAL + d) / 2.0
}

#[derive(Clone, Copy, PartialEq)]
enum Variant {
    Plain,
    /// Its laws in the minutes band, so an instant anchor on an amount is legal.
    Slow,
    /// A memory row that reads three more parameters, for the provenance kinds the flow has no slot for.
    Extra,
    /// A hazard guard on a mode variable.
    Hazard,
    /// An algebraic row that is `x - p`.
    PinnedRead,
    /// A memory row that relaxes to `p`.
    PinnedRelax,
    /// The flow divides by `b`.
    Divided,
    /// A forcing scales the flow.
    Forced,
    /// A forcing scales the flow, and a row reads the forcing's period as a parameter.
    PeriodRead,
    /// An algebraic row in the boundary reads a drive, in the minutes band, so an instant anchor can hold on it.
    Instant,
    /// A second pair of vessels, joined to nothing else and reached by no drive.
    Apart,
}

struct Body {
    body: CompiledBody,
    a: SiteId,
    b: SiteId,
}

fn declared(variant: Variant) -> Result<Body, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let a = map.site::<VesselSegment>("a", origin());
    let b = map.site::<VesselSegment>("b", origin());
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    if variant == Variant::Apart {
        let c = map.site::<VesselSegment>("c", origin());
        let d = map.site::<VesselSegment>("d", origin());
        map.contain(root.id(), c.id());
        map.contain(root.id(), d.id());
        map.connect(Network::Vascular, c.id(), d.id());
    }
    let mut schema = Schema::new(map.finish()?)?;
    schema.water_boundary(PULSE, BoundaryDirection::IntoBody, a.id(), Space::Lumen)?;
    schema.water_boundary(DRAIN, BoundaryDirection::OutOfBody, b.id(), Space::Lumen)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let e = laws.end_parameter::<Elastance>(E, Admissible::positive());
        let g = laws.edge_parameter::<Conductance<Water>>(G, Admissible::nonnegative());
        let class = match variant {
            Variant::Slow | Variant::Instant => RateClass::new(RateBand::Minutes, 60 * SECOND)?,
            _ => RateClass::new(RateBand::Wave, 1_000_000)?,
        };
        let forced = variant == Variant::Forced || variant == Variant::PeriodRead;
        let wobble = forced.then(|| laws.forcing("wobble", KNOTS, WOBBLE));
        let flow = laws.edge_kernel(|mut k| {
            let upstream = k.at_edge(&route.from_amount());
            let downstream = k.at_edge(&route.to_amount());
            let upstream = k.volume(upstream);
            let downstream = k.volume(downstream);
            let upstream_elastance = k.upstream_parameter(&e);
            let downstream_elastance = k.downstream_parameter(&e);
            let upstream = k.multiply(upstream_elastance, upstream);
            let downstream = k.multiply(downstream_elastance, downstream);
            let upstream = k.as_dimension::<_, Pressure>(upstream);
            let downstream = k.as_dimension::<_, Pressure>(downstream);
            let head = k.subtract(upstream, downstream);
            let conductance = k.edge_parameter(&g);
            let flow = k.conductance_flow(conductance, head);
            if let Some(wobble) = &wobble {
                let factor = k.forcing(wobble);
                let flow = k.scale(flow, factor);
                return k.finish(flow, laws.every_mode(), class);
            }
            if variant == Variant::Divided {
                let reach = k.at_edge(&route.from_amount());
                let reach = k.volume(reach);
                let below = k.at_edge(&route.to_amount());
                let below = k.volume(below);
                let share = k.divide(reach, below);
                let share = k.as_dimension::<_, Dimensionless>(share);
                let flow = k.scale(flow, share);
                return k.finish(flow, laws.every_mode(), class);
            }
            k.finish(flow, laws.every_mode(), class)
        });
        laws.transfer_along(&route, flow);
        let beat = RateClass::new(RateBand::Beat, 100_000_000)?;
        match variant {
            Variant::Plain
            | Variant::Slow
            | Variant::Divided
            | Variant::Forced
            | Variant::PeriodRead
            | Variant::Instant
            | Variant::Apart => {}
            Variant::Extra => {
                let tone = laws.parameter::<Dimensionless>(TONE, Admissible::finite());
                let ratio = laws.parameter::<Dimensionless>(RATIO, Admissible::finite());
                let doubled = laws.parameter::<Dimensionless>(DOUBLED, Admissible::finite());
                let speed = laws.parameter::<Rate<Dimensionless>>(PACE, Admissible::positive());
                let settled = laws.state::<Dimensionless>(SETTLED, Admissible::finite());
                let rate = laws.kernel(|mut k| {
                    let (tone, ratio, doubled) =
                        (k.parameter(&tone), k.parameter(&ratio), k.parameter(&doubled));
                    let sum = k.add(tone, ratio);
                    let target = k.add(sum, doubled);
                    let current = k.state(&settled);
                    let speed = k.parameter(&speed);
                    let rate = k.relax(target, current, speed);
                    k.finish(rate, laws.every_mode(), beat)
                });
                laws.memory_rate(&settled, rate);
            }
            Variant::Hazard => {
                let pace = laws.parameter::<Rate<Dimensionless>>(PACE, Admissible::nonnegative());
                let dice = laws.stream("beat");
                let integral = laws.state::<Dimensionless>("hazard", Admissible::nonnegative());
                let [sinus, ectopic] = laws.modes("rhythm", ["sinus", "ectopic"]);
                let rate = laws.kernel(|mut k| {
                    let rate = k.parameter(&pace);
                    k.finish(rate, laws.every_mode(), beat)
                });
                laws.memory_rate(&integral, rate);
                laws.hazard(HazardGuard::new(&integral, &dice, sinus.to(&ectopic)?));
            }
            Variant::PinnedRead => {
                let set = laws.parameter::<Dimensionless>(SET, Admissible::positive());
                let read = laws.algebraic::<Dimensionless>("read", Admissible::finite());
                let closure = laws.kernel(|mut k| {
                    let x = k.algebraic(&read);
                    let p = k.parameter(&set);
                    let residual = k.subtract(x, p);
                    k.finish(residual, laws.every_mode(), beat)
                });
                laws.residual(&read, closure);
            }
            Variant::PinnedRelax => {
                let set = laws.parameter::<Dimensionless>(SET, Admissible::positive());
                let speed = laws.parameter::<Rate<Dimensionless>>(PACE, Admissible::positive());
                let settled = laws.state::<Dimensionless>(SETTLED, Admissible::finite());
                let rate = laws.kernel(|mut k| {
                    let target = k.parameter(&set);
                    let current = k.state(&settled);
                    let speed = k.parameter(&speed);
                    let rate = k.relax(target, current, speed);
                    k.finish(rate, laws.every_mode(), beat)
                });
                laws.memory_rate(&settled, rate);
            }
        }
        body.add(laws.finish());
        let (mut laws, ports) = body.boundary()?.parts();
        for boundary in [PULSE, DRAIN] {
            let BoundaryWater { route: pair, drive } = ports.water(boundary)?;
            let rate = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&pair, rate));
            if variant == Variant::PeriodRead && boundary == PULSE {
                let period = laws.parameter::<Time>(KNOTS, Admissible::positive());
                let timed = laws.algebraic::<Product<Rate<Water>, Time>>(READING, Admissible::finite());
                let closure = laws.kernel(|mut k| {
                    let x = k.algebraic(&timed);
                    let drive = k.read(&drive);
                    let period = k.parameter(&period);
                    let product = k.multiply(drive, period);
                    let residual = k.subtract(x, product);
                    k.finish(residual, laws.every_mode(), class)
                });
                laws.residual(&timed, closure);
            }
            if variant == Variant::Instant && boundary == PULSE {
                let gain = laws.parameter::<Dimensionless>(SET, Admissible::positive());
                let reading = laws.algebraic::<Rate<Water>>(READING, Admissible::finite());
                let closure = laws.kernel(|mut k| {
                    let x = k.algebraic(&reading);
                    let drive = k.read(&drive);
                    let gain = k.parameter(&gain);
                    let scaled = k.scale(drive, gain);
                    let residual = k.subtract(x, scaled);
                    k.finish(residual, laws.every_mode(), class)
                });
                laws.residual(&reading, closure);
                let echo = laws.algebraic::<Rate<Water>>(ECHO, Admissible::finite());
                let copy = laws.kernel(|mut k| {
                    let x = k.algebraic(&echo);
                    let read = k.algebraic(&reading);
                    let residual = k.subtract(x, read);
                    k.finish(residual, laws.every_mode(), class)
                });
                laws.residual(&echo, copy);
            }
        }
        body.add(laws.finish());
        body.compile()
    })?;
    Ok(Body {
        body,
        a: a.id(),
        b: b.id(),
    })
}

fn water(tag: &[u8], index: u64, boundary: &'static str, from: u64, to: u64, amount: f64) -> BoundaryEvent {
    let mut id = tag.to_vec();
    id.extend_from_slice(&index.to_be_bytes());
    let delivery = WaterDelivery {
        boundary,
        amount: measure(amount),
        ends: at(to),
    };
    BoundaryEvent::new(
        InputId::new(Digest::of_canonical(&id)),
        at(from),
        BoundaryInput::Water(delivery),
    )
}

/// One pulse and one even drain per cycle.
fn deliveries(cycles: core::ops::Range<u64>) -> Vec<BoundaryEvent> {
    let mut events = Vec::new();
    for index in cycles {
        let from = index * PERIOD_NS;
        events.push(water(b"pulse", index, PULSE, from, from + PULSE_NS, DELIVERED));
        events.push(water(b"drain", index, DRAIN, from, from + PERIOD_NS, DELIVERED));
    }
    events
}

fn port(cycles: core::ops::Range<u64>) -> BoundaryPort {
    BoundaryPort::new(deliveries(cycles)).expect("parsed")
}

fn empty_port() -> BoundaryPort {
    BoundaryPort::new(Vec::new()).expect("no event")
}

struct PulsatileCase {
    declared: Body,
    lens: ObservationLens,
    intervals: u32,
    cycles: u64,
}
impl PulsatileCase {
    fn new(variant: Variant) -> Self {
        let declared = declared(variant).expect("a body");
        let lens = gauges(&declared, GAUGE, u64::from(INTERVALS));
        Self {
            declared,
            lens,
            intervals: INTERVALS,
            cycles: 4,
        }
    }
    fn plain() -> Self {
        Self::new(Variant::Plain)
    }
    fn with_intervals(self, intervals: u32) -> Self {
        let lens = gauges(&self.declared, GAUGE, u64::from(intervals));
        Self {
            intervals,
            lens,
            ..self
        }
    }
    fn body(&self) -> &CompiledBody {
        &self.declared.body
    }
    fn a(&self) -> RowKind {
        lumen(self.declared.a)
    }
    fn b(&self) -> RowKind {
        lumen(self.declared.b)
    }
    fn row_a(&self) -> RowId {
        (self.body().rows().iter())
            .find(|row| row.kind() == self.a())
            .expect("the body declares it")
            .id()
    }
    fn row_b(&self) -> RowId {
        (self.body().rows().iter())
            .find(|row| row.kind() == self.b())
            .expect("the body declares it")
            .id()
    }
    fn mean_anchor(&self, id: AnchorId, row: RowKind, value: f64) -> Synthetic<Water> {
        let read = Snapshot::new(row, GAUGE, AnchorStatistic::CycleMean, measure::<Water>(value));
        Synthetic::new(id, read, NOTE)
    }
    fn packet_without_fit(&self) -> ParameterPacket<SyntheticFixture> {
        let mut packet = ParameterPacket::periodic(cycle(self.intervals));
        packet.within(admission_tolerance());
        let g = measure::<Conductance<Water>>(CONDUCTANCE);
        packet.synthetic(G, NETWORK, g, NOTE).expect("once");
        packet
            .start(self.a(), measure::<Water>(START[0]), NOTE)
            .expect("once");
        packet
            .start(self.b(), measure::<Water>(START[1]), NOTE)
            .expect("once");
        packet
    }
    /// A cycle-mean anchor on `a`, and `cv.e` fitted to it: one entry that fills both ends' slots.
    fn packet(&self) -> ParameterPacket<SyntheticFixture> {
        let mut packet = self.packet_without_fit();
        packet
            .anchor(self.mean_anchor(MEAN, self.a(), MEAN_A))
            .expect("once");
        packet
            .calibrated(E, NETWORK, measure::<Elastance>(GUESS), MEAN)
            .expect("once");
        packet
    }
    fn system(&self, packet: ParameterPacket<SyntheticFixture>) -> InitializationSystem<'_> {
        let parameters = self.body().bind(packet).expect("bound");
        let request = InitializationRequest::at(at(0)).reading(&self.lens);
        self.body()
            .initialization(parameters, port(0..1), request)
            .expect("a system")
    }
    fn history(&self) -> MethodHistory {
        MethodHistory::new(Digest::of_canonical(b"a schema"), at(0), Vec::new())
    }
    fn admitted<'m>(&'m self, core: &'m dyn CoupledSolver) -> Admitted<'m> {
        admit_solved(self.body(), self.packet(), port(0..self.cycles), core, &self.lens).expect("solved")
    }
}
/// Only what can arrive late. A delivery inside the window shaped the start state, so every run knows it.
impl ReplayCase for PulsatileCase {
    fn body(&self) -> &CompiledBody {
        &self.declared.body
    }
    fn end(&self) -> SimTime {
        at(self.cycles * PERIOD_NS)
    }
    fn events(&self) -> Result<Vec<BoundaryEvent>, AdmissionError> {
        Ok(deliveries(1..self.cycles))
    }
    fn admit<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        let boundary = BoundaryPort::new(deliveries(0..self.cycles))?;
        Ok(admit_solved(self.body(), self.packet(), boundary, core, &self.lens)?.session)
    }
    fn admit_withheld<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        let boundary = BoundaryPort::new(deliveries(0..1))?;
        Ok(admit_solved(self.body(), self.packet(), boundary, core, &self.lens)?.session)
    }
}

/// A channel on each vessel by `method`, one reading per `samples` grid instants, on a grid of a cycle's `samples`.
fn gauges(body: &Body, method: &'static str, samples: u64) -> ObservationLens {
    let channels =
        [body.a, body.b].map(|site| ChannelSpec::new(method, lumen(site)).averaged(count(samples)));
    (body.body.lens(channels.into(), count(PERIOD_NS / samples))).expect("a lens")
}

fn step<'m>(session: Session<'m, Running>) -> Session<'m, Running> {
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, _) => return session,
            Step::Refused(session) => session.retry(),
            Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
        };
    }
}

/// `a`'s reading per whole cycle, through the anchor's own channel, and the head after them.
fn cycle_means<'m>(
    mut session: Session<'m, Running>,
    lens: &ObservationLens,
    cycles: u64,
) -> (Vec<f64>, Session<'m, Running>) {
    let mut observer = Observer::new(lens, ObservationSeed::new([5, 6, 7, 8]));
    let view = session.observation(lens).expect("this body's lens");
    observer.advance(&view).expect("the first read");
    let mut means = Vec::new();
    while session.record().accepted().time() < at(cycles * PERIOD_NS) {
        session = step(session);
        let view = session.observation(lens).expect("this body's lens");
        for seen in observer.advance(&view).expect("the next revision") {
            if let (0, ObservedValue::Present(value)) = (seen.channel(), seen.value()) {
                means.push(hi(value));
            }
        }
    }
    (means, session)
}

fn record(provenance: &InitializationProvenance, name: ValueName) -> ValueRecord {
    *(provenance.values().iter())
        .find(|record| record.name == name)
        .expect("every value has a record")
}

// Check 1: a cycle-mean anchor on a pulsatile output is met as a cycle mean, not an instant.

#[test]
fn a_cycle_mean_anchor_is_met_as_a_cycle_mean_not_an_instant() {
    let case = PulsatileCase::plain();
    let core = reference(REFERENCE_NS);
    let Admitted { session, provenance } = case.admitted(&core);
    let name = ValueName::Parameter {
        key: E,
        binding: NETWORK,
    };
    assert!((hi(record(&provenance, name).value) - fitted_elastance()).abs() < 1e-13);
    let instant = hi(session.record().accepted().value(case.row_a()));
    assert!(
        (instant - MEAN_A).abs() > 2.0,
        "the start is no cycle mean: {instant}"
    );
    let (means, _) = cycle_means(session, &case.lens, 1);
    assert!((means[0] - MEAN_A).abs() < 1e-12, "{means:?}");
}

#[test]
fn the_fit_is_its_closed_form_on_any_grid_because_the_ring_keeps_the_means() {
    for intervals in [4, 8, 64, 256] {
        let case = PulsatileCase::plain().with_intervals(intervals);
        let core = reference(REFERENCE_NS);
        let name = ValueName::Parameter {
            key: E,
            binding: NETWORK,
        };
        let solved = hi(record(&case.admitted(&core).provenance, name).value);
        assert!(
            (solved - fitted_elastance()).abs() < 1e-12,
            "{intervals}: {solved}"
        );
    }
}

#[test]
fn the_start_state_is_the_point_of_the_backward_euler_orbit() {
    let case = PulsatileCase::plain();
    let core = reference(REFERENCE_NS);
    let session = case.admitted(&core).session;
    let start = hi(session.record().accepted().value(case.row_a()));
    assert!((start - start_a(INTERVALS)).abs() < 1e-11, "{start}");
}

#[test]
fn a_core_on_the_grids_own_step_closes_the_first_cycle_to_rounding() {
    let case = PulsatileCase::plain();
    let core = reference(REFERENCE_NS);
    let session = case.admitted(&core).session;
    let start = hi(session.record().accepted().value(case.row_a()));
    let (_, session) = cycle_means(session, &case.lens, 1);
    let end = hi(session.record().accepted().value(case.row_a()));
    assert!((end - start).abs() < 1e-11, "{}", end - start);
}

#[test]
fn a_core_off_the_grid_starts_near_its_own_orbit_and_closes_on_it() {
    let case = PulsatileCase::plain();
    for interval_ns in [PERIOD_NS / 128, PERIOD_NS / 16, PERIOD_NS / 4] {
        let core = reference(interval_ns);
        let (means, _) = cycle_means(case.admitted(&core).session, &case.lens, 4);
        let (first, last) = ((means[0] - MEAN_A).abs(), (means[3] - MEAN_A).abs());
        assert!(first < 0.05 && last < first / 100.0, "{interval_ns}: {means:?}");
    }
}

#[test]
fn an_instant_anchor_on_a_pulsatile_row_is_refused_and_on_a_slow_row_is_read() {
    let case = PulsatileCase::plain();
    let read = Snapshot::new(
        case.a(),
        GAUGE,
        AnchorStatistic::Instant,
        measure::<Water>(MEAN_A),
    );
    let mut packet = case.packet_without_fit();
    packet.anchor(Synthetic::new(MEAN, read, NOTE)).expect("once");
    packet
        .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
        .expect("once");
    assert_eq!(
        case.body().bind(packet).err(),
        Some(AdmissionError::InstantOnPulsatile { anchor: MEAN })
    );

    let slow = declared(Variant::Slow).expect("a body");
    let bare = ChannelSpec::new(GAUGE, lumen(slow.a));
    let lens = slow.body.lens(vec![bare], count(SECOND)).expect("a lens");
    let packet = |value| {
        let mut packet = ParameterPacket::new();
        let read = Snapshot::new(
            lumen(slow.a),
            GAUGE,
            AnchorStatistic::Instant,
            measure::<Water>(value),
        );
        packet.anchor(Synthetic::new(MEAN, read, NOTE)).expect("once");
        packet
            .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
            .expect("once");
        let g = measure::<Conductance<Water>>(CONDUCTANCE);
        packet.synthetic(G, NETWORK, g, NOTE).expect("once");
        packet
            .start(lumen(slow.a), measure::<Water>(START[0]), NOTE)
            .expect("once");
        packet
            .start(lumen(slow.b), measure::<Water>(START[1]), NOTE)
            .expect("once");
        packet
    };
    let core = reference(SECOND);
    assert!(admit_solved(&slow.body, packet(START[0]), empty_port(), &core, &lens).is_ok());
    assert_eq!(
        admit_solved(&slow.body, packet(START[0] + 1.0), empty_port(), &core, &lens).err(),
        Some(InitializationFailure::Inconsistent {
            row: AdmissionRow::Anchor(MEAN)
        })
    );
}

#[test]
fn a_cycle_mean_anchor_needs_a_cycle() {
    let case = PulsatileCase::plain();
    let mut packet = ParameterPacket::new();
    packet
        .anchor(case.mean_anchor(MEAN, case.a(), MEAN_A))
        .expect("once");
    packet
        .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
        .expect("once");
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .start(case.a(), measure::<Water>(START[0]), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(START[1]), NOTE)
        .expect("once");
    assert_eq!(
        case.body().bind(packet).err(),
        Some(AdmissionError::NoCycle { anchor: MEAN })
    );
}

#[test]
fn an_anchor_no_channel_reads_by_its_method_and_window_is_refused() {
    let case = PulsatileCase::plain();
    let core = reference(REFERENCE_NS);
    let refused = Some(InitializationFailure::Admission(AdmissionError::AnchorUnread {
        anchor: MEAN,
    }));
    let solved =
        |lens: &ObservationLens| admit_solved(case.body(), case.packet(), port(0..1), &core, lens).err();
    let other_method = gauges(&case.declared, "another method", u64::from(INTERVALS));
    assert_eq!(solved(&other_method), refused);
    let grid = count(PERIOD_NS / u64::from(INTERVALS));
    let half_a_cycle = ChannelSpec::new(GAUGE, case.a()).averaged(count(32));
    let half_a_cycle = case.body().lens(vec![half_a_cycle], grid).expect("a lens");
    assert_eq!(solved(&half_a_cycle), refused);
    let other_row = ChannelSpec::new(GAUGE, case.b()).averaged(count(64));
    let other_row = case.body().lens(vec![other_row], grid).expect("a lens");
    assert_eq!(solved(&other_row), refused);
    let other_body = PulsatileCase::new(Variant::Slow);
    assert_eq!(solved(&other_body.lens), refused);
    let one_instant_a_beat = ChannelSpec::new(GAUGE, case.a());
    let one_instant_a_beat = (case.body().lens(vec![one_instant_a_beat], count(PERIOD_NS))).expect("a lens");
    assert_eq!(solved(&one_instant_a_beat), refused);
    let coarser_than_the_ring = gauges(&case.declared, GAUGE, 32);
    assert_eq!(solved(&coarser_than_the_ring), refused);
    let off_the_rings_points = gauges(&case.declared, GAUGE, 125);
    assert_eq!(solved(&off_the_rings_points), refused);
    let finer_than_the_ring = gauges(&case.declared, GAUGE, 128);
    assert_eq!(solved(&finer_than_the_ring), refused);
    let parameters = case.body().bind(case.packet()).expect("bound");
    let unread = InitializationRequest::at(at(0));
    assert_eq!(
        case.body().initialization(parameters, port(0..1), unread).err(),
        Some(AdmissionError::AnchorUnread { anchor: MEAN })
    );
}

#[test]
fn the_lens_that_reads_an_anchor_moves_no_bit_of_the_start() {
    let case = PulsatileCase::plain();
    let core = reference(REFERENCE_NS);
    let wider = {
        let extra = ChannelSpec::new("a bare gauge", case.b());
        let channels = vec![ChannelSpec::new(GAUGE, case.a()).averaged(count(64)), extra];
        (case.body().lens(channels, count(PERIOD_NS / 64))).expect("a lens")
    };
    let one = case.admitted(&core).session;
    let other = admit_solved(case.body(), case.packet(), port(0..case.cycles), &core, &wider)
        .expect("solved")
        .session;
    assert_eq!(one.record().seal(), other.record().seal());
}

// Check 2: an initializer that returns a guess violating a row or anchor is refused with the row named.

#[test]
fn an_initializer_that_returns_its_guess_is_refused_with_the_row_named() {
    let case = PulsatileCase::plain();
    let system = case.system(case.packet());
    let guess = system.workspace();
    let row = AdmissionRow::Body(case.body().rows()[0].id());
    assert_eq!(
        system.admit(guess, case.history()).err(),
        Some(InitializationFailure::Inconsistent { row })
    );
}

/// The steady orbit of an even drive: `d = v / k` at every point, so the packet's own values solve the ring.
fn steady_packet(case: &PulsatileCase) -> (ParameterPacket<SyntheticFixture>, BoundaryPort, f64) {
    steady_packet_missing(case, 0.0)
}

fn steady_packet_missing(
    case: &PulsatileCase,
    by: f64,
) -> (ParameterPacket<SyntheticFixture>, BoundaryPort, f64) {
    let elastance = 2.0;
    let difference = DELIVERED / (CONDUCTANCE * elastance);
    let (a, b) = ((TOTAL + difference) / 2.0, (TOTAL - difference) / 2.0);
    let mut packet = ParameterPacket::periodic(cycle(4));
    packet.within(admission_tolerance());
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet.start(case.a(), measure::<Water>(a), NOTE).expect("once");
    packet.start(case.b(), measure::<Water>(b), NOTE).expect("once");
    packet
        .anchor(case.mean_anchor(MEAN, case.a(), a + by))
        .expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(elastance), MEAN)
        .expect("once");
    let events = vec![
        water(b"even in", 0, PULSE, 0, PERIOD_NS, DELIVERED),
        water(b"even out", 0, DRAIN, 0, PERIOD_NS, DELIVERED),
    ];
    (packet, BoundaryPort::new(events).expect("parsed"), a)
}

fn steady_system<'m>(
    case: &'m PulsatileCase,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
) -> InitializationSystem<'m> {
    let parameters = case.body().bind(packet).expect("bound");
    let lens = gauges(&case.declared, GAUGE, 4);
    let request = InitializationRequest::at(at(0)).reading(&lens);
    (case.body().initialization(parameters, boundary, request)).expect("a system")
}

#[test]
fn a_guess_that_already_solves_the_system_is_admitted() {
    let case = PulsatileCase::plain();
    let (packet, boundary, _) = steady_packet(&case);
    let system = steady_system(&case, packet, boundary);
    let guess = system.workspace();
    assert!(system.admit(guess, case.history()).is_ok());
}

#[test]
fn a_solved_orbit_that_misses_a_second_anchor_is_refused_with_that_anchor_named() {
    let case = PulsatileCase::plain();
    let second = AnchorId::new("fixture.mean-b");
    let mut packet = case.packet();
    packet
        .anchor(case.mean_anchor(second, case.b(), 1.0))
        .expect("once");
    let core = reference(REFERENCE_NS);
    assert_eq!(
        admit_solved(case.body(), packet, port(0..1), &core, &case.lens).err(),
        Some(InitializationFailure::Inconsistent {
            row: AdmissionRow::Anchor(second)
        })
    );
}

#[test]
fn a_moved_conserved_total_is_refused_with_its_quantity() {
    let case = PulsatileCase::plain();
    let quantity = case.body().conserved_basis().combinations()[0].id();
    let rows = case.body().rows().len();
    for change in [0.5, -0.5] {
        let (packet, boundary, _) = steady_packet(&case);
        let system = steady_system(&case, packet, boundary);
        let mut moved = system.workspace();
        // Both vessels at every point move alike, so every stage still holds and only the total can tell.
        for unknown in moved.unknowns_mut().iter_mut().take(4 * rows) {
            *unknown += change;
        }
        assert_eq!(
            system.admit(moved, case.history()).err(),
            Some(InitializationFailure::Inconsistent {
                row: AdmissionRow::Total(quantity)
            })
        );
    }
}

/// Each row moves inside its own scale at an instant, and together they move the total past its own.
#[test]
fn an_instant_moves_no_conserved_total_past_its_scale() {
    let case = PulsatileCase::new(Variant::Slow);
    let lens = case.body().lens(Vec::new(), count(SECOND)).expect("a lens");
    let mut packet = ParameterPacket::new();
    packet.within(admission_tolerance());
    packet
        .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
        .expect("once");
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .start(case.a(), measure::<Water>(START[0]), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(START[1]), NOTE)
        .expect("once");
    let parameters = case.body().bind(packet).expect("bound");
    let request = InitializationRequest::at(at(0)).reading(&lens);
    let system = (case.body().initialization(parameters, empty_port(), request)).expect("a system");
    let mut moved = system.workspace();
    moved.unknowns_mut()[0] += 9e-10;
    moved.unknowns_mut()[1] += 9e-10;
    let quantity = case.body().conserved_basis().combinations()[0].id();
    assert_eq!(
        system.admit(moved, case.history()).err(),
        Some(InitializationFailure::Inconsistent {
            row: AdmissionRow::Total(quantity)
        })
    );
}

#[test]
fn a_witness_that_breaks_one_stage_at_one_point_is_refused_with_its_row() {
    let case = PulsatileCase::plain();
    let rows = case.body().rows().len();
    for (point, change) in [(1, 1e-3), (2, -1e-3), (3, 1e-3)] {
        let (packet, boundary, _) = steady_packet(&case);
        let system = steady_system(&case, packet, boundary);
        let mut witness = system.workspace();
        witness.unknowns_mut()[point * rows] += change;
        assert_eq!(
            system.admit(witness, case.history()).err(),
            Some(InitializationFailure::Inconsistent {
                row: AdmissionRow::Body(case.row_a())
            }),
            "point {point}"
        );
    }
}

#[test]
fn a_witness_that_holds_every_row_and_misses_its_anchor_is_refused_with_the_anchor_named() {
    let case = PulsatileCase::plain();
    for by in [1.0, -1.0] {
        let (packet, boundary, _) = steady_packet_missing(&case, by);
        let system = steady_system(&case, packet, boundary);
        let guess = system.workspace();
        assert_eq!(
            system.admit(guess, case.history()).err(),
            Some(InitializationFailure::Inconsistent {
                row: AdmissionRow::Anchor(MEAN)
            })
        );
    }
}

#[test]
fn a_net_boundary_flow_over_the_cycle_is_refused_with_its_quantity() {
    let case = PulsatileCase::plain();
    let core = reference(REFERENCE_NS);
    let boundary = BoundaryPort::new(vec![
        water(b"pulse", 0, PULSE, 0, PULSE_NS, DELIVERED),
        water(b"short drain", 0, DRAIN, 0, PERIOD_NS, DELIVERED - 1.0),
    ])
    .expect("parsed");
    let quantity = case.body().conserved_basis().combinations()[0].id();
    assert_eq!(
        admit_solved(case.body(), case.packet(), boundary, &core, &case.lens).err(),
        Some(InitializationFailure::NetBoundaryFlow { quantity })
    );
}

/// Every stage holds to its own scale and all of them lean one way: the flow lands on no one row.
#[test]
fn a_net_flow_spread_over_every_row_is_still_refused_with_its_quantity() {
    let case = PulsatileCase::plain();
    let (elastance, intervals) = (2.0, 4.0);
    let net = 8e-9;
    let flow = DELIVERED - net / 2.0;
    let difference = flow / (CONDUCTANCE * elastance);
    let (a, b) = ((TOTAL + difference) / 2.0, (TOTAL - difference) / 2.0);
    let mut packet = ParameterPacket::periodic(cycle(intervals as u32));
    packet.within(admission_tolerance());
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .synthetic(E, NETWORK, measure::<Elastance>(elastance), NOTE)
        .expect("once");
    packet.start(case.a(), measure::<Water>(a), NOTE).expect("once");
    packet.start(case.b(), measure::<Water>(b), NOTE).expect("once");
    let boundary = BoundaryPort::new(vec![
        water(b"even in", 0, PULSE, 0, PERIOD_NS, DELIVERED),
        water(b"short out", 0, DRAIN, 0, PERIOD_NS, DELIVERED - net),
    ])
    .expect("parsed");
    let quantity = case.body().conserved_basis().combinations()[0].id();
    let core = reference(REFERENCE_NS);
    assert_eq!(
        admit_fixture(case.body(), packet, boundary, &core).err(),
        Some(InitializationFailure::NetBoundaryFlow { quantity })
    );
}

/// The small vessel's last stage is the row the total replaced. A defect far past that row's own scale must be
/// read there, though the total beside it is large.
#[test]
fn the_row_a_total_replaced_is_read_at_its_own_scale() {
    let case = PulsatileCase::plain();
    let (small, large) = (1.0, 1e9);
    let mut packet = ParameterPacket::periodic(cycle(4));
    packet.within(admission_tolerance());
    let g = measure::<Conductance<Water>>(4.0);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    let (at_a, at_b) = (Binding::Site(case.declared.a), Binding::Site(case.declared.b));
    let stiff = measure::<Elastance>(large / small);
    packet.synthetic(E, at_a, stiff, NOTE).expect("once");
    packet
        .synthetic(E, at_b, measure::<Elastance>(1.0), NOTE)
        .expect("once");
    packet
        .start(case.a(), measure::<Water>(small), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(large), NOTE)
        .expect("once");
    let parameters = case.body().bind(packet).expect("bound");
    let request = InitializationRequest::at(at(0)).reading(&case.lens);
    let system = (case.body().initialization(parameters, empty_port(), request)).expect("a system");
    let mut witness = system.workspace();
    let last = 3 * case.body().rows().len();
    witness.unknowns_mut()[last + 1] += 5e-4;
    let row = AdmissionRow::Body(case.row_a());
    assert_eq!(
        system.admit(witness, case.history()).err(),
        Some(InitializationFailure::Inconsistent { row })
    );
}

#[test]
fn a_guess_with_no_head_gives_the_fit_nothing_to_move() {
    let case = PulsatileCase::plain();
    let mut packet = ParameterPacket::periodic(cycle(INTERVALS));
    packet.within(admission_tolerance());
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .start(case.a(), measure::<Water>(50.0), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(50.0), NOTE)
        .expect("once");
    packet
        .anchor(case.mean_anchor(MEAN, case.a(), MEAN_A))
        .expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(GUESS), MEAN)
        .expect("once");
    let core = reference(REFERENCE_NS);
    assert!(matches!(
        admit_solved(case.body(), packet, port(0..1), &core, &case.lens).err(),
        Some(InitializationFailure::Inconsistent { .. })
    ));
}

#[test]
fn a_fit_outside_its_set_is_refused_by_its_key() {
    let case = PulsatileCase::plain();
    let (packet, boundary, _) = steady_packet(&case);
    let system = steady_system(&case, packet, boundary);
    let mut solved = system.workspace();
    let slots = solved.unknowns_mut().len();
    solved.unknowns_mut()[slots - 1] = -1.0;
    assert_eq!(
        system.admit(solved, case.history()).err(),
        Some(InitializationFailure::Admission(
            AdmissionError::ParameterOutsideSet {
                key: E,
                binding: NETWORK
            }
        ))
    );
}

#[test]
fn a_forcing_is_held_at_its_mean_around_the_ring() {
    let case = PulsatileCase::new(Variant::Forced);
    let (mut packet, boundary, _) = steady_packet(&case);
    packet
        .synthetic(KNOTS, Binding::Body, measure::<Time>(1.0), NOTE)
        .expect("once");
    let amplitude = measure::<Dimensionless>(0.5);
    packet
        .synthetic(WOBBLE, Binding::Body, amplitude, NOTE)
        .expect("once");
    let system = steady_system(&case, packet, boundary);
    let guess = system.workspace();
    assert!(system.admit(guess, case.history()).is_ok());
}

#[test]
fn a_fit_no_law_behind_its_anchor_reads_is_refused_by_its_key() {
    let forced = PulsatileCase::new(Variant::Forced);
    let second = AnchorId::new("fixture.mean-b");
    let (mut packet, _, a) = steady_packet(&forced);
    packet
        .anchor(forced.mean_anchor(second, forced.b(), TOTAL - a))
        .expect("once");
    packet
        .calibrated(KNOTS, Binding::Body, measure::<Time>(1.0), second)
        .expect("once");
    let amplitude = measure::<Dimensionless>(0.5);
    packet
        .synthetic(WOBBLE, Binding::Body, amplitude, NOTE)
        .expect("once");
    assert_eq!(
        forced.body().bind(packet).err(),
        Some(AdmissionError::FitUnread {
            key: KNOTS,
            binding: Binding::Body
        })
    );

    // At an instant no amount moves, so a law on an amount cannot carry its anchor to a fit.
    let slow = PulsatileCase::new(Variant::Slow);
    let read = Snapshot::new(
        slow.a(),
        GAUGE,
        AnchorStatistic::Instant,
        measure::<Water>(START[0]),
    );
    let mut packet = ParameterPacket::new();
    packet.anchor(Synthetic::new(MEAN, read, NOTE)).expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(GUESS), MEAN)
        .expect("once");
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .start(slow.a(), measure::<Water>(START[0]), NOTE)
        .expect("once");
    packet
        .start(slow.b(), measure::<Water>(START[1]), NOTE)
        .expect("once");
    assert_eq!(
        slow.body().bind(packet).err(),
        Some(AdmissionError::FitUnread {
            key: E,
            binding: NETWORK
        })
    );
}

/// The drain comes first, so the water falls below zero inside the beat. The start itself is positive, and every
/// row of the orbit holds.
#[test]
fn an_orbit_that_leaves_a_rows_set_inside_the_beat_is_refused_with_its_row() {
    let case = PulsatileCase::plain();
    let mut packet = ParameterPacket::periodic(cycle(INTERVALS));
    packet.within(admission_tolerance());
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .synthetic(E, NETWORK, measure::<Elastance>(100.0), NOTE)
        .expect("once");
    packet.start(case.a(), measure::<Water>(2.0), NOTE).expect("once");
    packet.start(case.b(), measure::<Water>(2.0), NOTE).expect("once");
    let events = vec![
        water(b"even in", 0, PULSE, 0, PERIOD_NS, DELIVERED),
        water(b"early out", 0, DRAIN, 0, PULSE_NS, DELIVERED),
    ];
    let boundary = BoundaryPort::new(events).expect("parsed");
    let core = reference(REFERENCE_NS);
    let row = AdmissionRow::Body(case.row_b());
    assert_eq!(
        admit_solved(case.body(), packet, boundary, &core, &case.lens).err(),
        Some(InitializationFailure::Inconsistent { row })
    );
}

#[test]
fn a_law_with_no_value_at_a_later_point_is_refused_by_its_name() {
    let case = PulsatileCase::new(Variant::Divided);
    let (packet, boundary, _) = steady_packet(&case);
    let system = steady_system(&case, packet, boundary);
    let mut solved = system.workspace();
    let rows = case.body().rows().len();
    let b = 1;
    solved.unknowns_mut()[2 * rows + b] = 0.0;
    assert_eq!(
        system.admit(solved, case.history()).err(),
        Some(InitializationFailure::Undefined {
            law: name("circulation")
        })
    );
}

// Under an instant, only algebraic rows move.

fn filter_body() -> CompiledBody {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let filter = laws.state::<Dimensionless>("filter", Admissible::finite());
        let reading = laws.algebraic::<Dimensionless>("reading", Admissible::finite());
        let speed = laws.parameter::<Rate<Dimensionless>>(PACE, Admissible::positive());
        let gain = laws.parameter::<Dimensionless>(SET, Admissible::finite());
        let reflex = RateClass::new(RateBand::Reflex, SECOND)?;
        let rate = laws.kernel(|mut k| {
            let now = k.state(&filter);
            let speed = k.parameter(&speed);
            let zero = k.zero::<Dimensionless>();
            let rate = k.relax(zero, now, speed);
            k.finish(rate, laws.every_mode(), reflex)
        });
        laws.memory_rate(&filter, rate);
        let closure = laws.kernel(|mut k| {
            let x = k.algebraic(&reading);
            let g = k.parameter(&gain);
            let now = k.state(&filter);
            let scaled = k.scale(g, now);
            let residual = k.subtract(x, scaled);
            k.finish(residual, laws.every_mode(), reflex)
        });
        laws.residual(&reading, closure);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

fn filter_packet(reading: f64) -> ParameterPacket<SyntheticFixture> {
    let mut packet = ParameterPacket::new();
    let nervous = |local| LawName { owner: "NS", local };
    let speed = measure::<Rate<Dimensionless>>(0.01);
    packet.synthetic(PACE, Binding::Body, speed, NOTE).expect("once");
    packet
        .synthetic(SET, Binding::Body, measure::<Dimensionless>(3.0), NOTE)
        .expect("once");
    let filter = RowKind::Memory(nervous("filter"));
    packet
        .start(filter, measure::<Dimensionless>(1.0), NOTE)
        .expect("once");
    let row = RowKind::Algebraic(nervous("reading"));
    packet
        .start(row, measure::<Dimensionless>(reading), NOTE)
        .expect("once");
    packet
}

#[test]
fn under_an_instant_only_algebraic_rows_move() {
    let body = filter_body();
    let core = reference(SECOND);
    let lens = body.lens(Vec::new(), count(SECOND)).expect("a lens");
    let Admitted { session, provenance } =
        admit_solved(&body, filter_packet(4.0), empty_port(), &core, &lens).expect("solved");
    let nervous = |local| LawName { owner: "NS", local };
    let reading = RowKind::Algebraic(nervous("reading"));
    assert_eq!(
        record(&provenance, ValueName::Start(reading)).provenance,
        ValueProvenance::Solved
    );
    let filter = RowKind::Memory(nervous("filter"));
    assert_eq!(
        record(&provenance, ValueName::Start(filter)).provenance,
        ValueProvenance::Fixture
    );
    assert_eq!(hi(session.record().accepted().value(body.rows()[1].id())), 3.0);
}

// The tolerance.

/// The model default's scale at a value near 3.
const NEAR_THREE: f64 = 1e-9 + 3e-12;

#[test]
fn the_tolerance_decides_a_near_row_and_is_reported_with_the_worst_row() {
    let body = filter_body();
    let core = reference(SECOND);
    let row = AdmissionRow::Body(body.rows()[1].id());
    let tolerance = admission_tolerance();
    let within = |reading| {
        let mut packet = filter_packet(reading);
        packet.within(tolerance);
        packet
    };
    let far = 3.0 + 1.1 * NEAR_THREE;
    let refused = admit_fixture(&body, within(far), empty_port(), &core).err();
    assert_eq!(refused, Some(InitializationFailure::Inconsistent { row }));

    let lens = body.lens(Vec::new(), count(SECOND)).expect("a lens");
    let parameters = body.bind(within(3.0 + 0.9 * NEAR_THREE)).expect("bound");
    let request = InitializationRequest::at(at(0)).reading(&lens);
    let system = body
        .initialization(parameters, empty_port(), request)
        .expect("a system");
    let guess = system.workspace();
    let history = MethodHistory::new(Digest::of_canonical(b"a schema"), at(0), Vec::new());
    let origin = system.admit(guess, history).expect("within the tolerance");
    assert_eq!(origin.provenance().tolerance(), tolerance);
    let (worst, normalized) = origin.provenance().worst().expect("a body with rows");
    assert_eq!(worst, row);
    assert!(hi(normalized) > 0.8 && hi(normalized) <= 1.0, "{normalized:?}");
}

#[test]
fn a_packet_that_states_no_tolerance_is_held_exactly() {
    let body = filter_body();
    let core = reference(SECOND);
    let row = AdmissionRow::Body(body.rows()[1].id());
    assert!(admit_fixture(&body, filter_packet(3.0), empty_port(), &core).is_ok());
    let near = 3.0 + 4.0 * f64::EPSILON;
    let refused = admit_fixture(&body, filter_packet(near), empty_port(), &core).err();
    assert_eq!(refused, Some(InitializationFailure::Inconsistent { row }));
    let parameters = body.bind(filter_packet(3.0)).expect("bound");
    let system = body
        .initialization(parameters, empty_port(), InitializationRequest::at(at(0)))
        .expect("a system");
    let mut moved = system.workspace();
    moved.unknowns_mut()[0] += 1e-13;
    let history = MethodHistory::new(Digest::of_canonical(b"a schema"), at(0), Vec::new());
    let row = AdmissionRow::Body(body.rows()[0].id());
    assert_eq!(
        system.admit(moved, history).err(),
        Some(InitializationFailure::Inconsistent { row })
    );
}

/// Pivoting can move a memory row by rounding while it solves the algebraic row beside it.
#[test]
fn an_instant_solve_stands_though_rounding_moves_a_row_inside_the_tolerance() {
    let body = filter_body();
    let core = reference(SECOND);
    let lens = body.lens(Vec::new(), count(SECOND)).expect("a lens");
    for (filter, gain, reading) in [(1e-3, 1.5, 1.0), (1e-3, 3.0, 4.0), (7e-3, 1.5, 1.0)] {
        let mut packet = ParameterPacket::new();
        packet.within(admission_tolerance());
        let speed = measure::<Rate<Dimensionless>>(1.0);
        packet.synthetic(PACE, Binding::Body, speed, NOTE).expect("once");
        packet
            .synthetic(SET, Binding::Body, measure::<Dimensionless>(gain), NOTE)
            .expect("once");
        let nervous = |local| LawName { owner: "NS", local };
        packet
            .start(
                RowKind::Memory(nervous("filter")),
                measure::<Dimensionless>(filter),
                NOTE,
            )
            .expect("once");
        packet
            .start(
                RowKind::Algebraic(nervous("reading")),
                measure::<Dimensionless>(reading),
                NOTE,
            )
            .expect("once");
        let solved = admit_solved(&body, packet, empty_port(), &core, &lens).expect("solved");
        let read = hi(solved.session.record().accepted().value(body.rows()[1].id()));
        assert!((read - gain * filter).abs() < 1e-12, "{read}");
    }
}

/// A flow this small on amounts this large, at the true anchor, from a guess of half the fit.
fn tiny_flow<'m>(
    case: &'m PulsatileCase,
    core: &'m ReferenceIntegrator,
    tiny: f64,
) -> Result<Admitted<'m>, InitializationFailure> {
    let (v, tau) = (tiny / seconds(PERIOD_NS), seconds(PULSE_NS));
    let mean = TOTAL / 2.0 + tiny * (1.0 - tau) / 4.0 + v / (2.0 * 2.0 * CONDUCTANCE) / 2.0 * 2.0;
    let mut packet = ParameterPacket::periodic(cycle(INTERVALS));
    packet.within(admission_tolerance());
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .start(case.a(), measure::<Water>(START[0]), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(START[1]), NOTE)
        .expect("once");
    packet
        .anchor(case.mean_anchor(MEAN, case.a(), mean))
        .expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(GUESS), MEAN)
        .expect("once");
    let boundary = BoundaryPort::new(vec![
        water(b"tiny pulse", 0, PULSE, 0, PULSE_NS, tiny),
        water(b"tiny drain", 0, DRAIN, 0, PERIOD_NS, tiny),
    ])
    .expect("parsed");
    admit_solved(case.body(), packet, boundary, core, &case.lens)
}

/// The solve runs to rounding and reads no tolerance, so a flow too small for the rows to speak for at the
/// tolerance still sets the fit.
#[test]
fn a_fit_the_tolerance_could_not_see_is_still_solved() {
    let case = PulsatileCase::plain();
    let name = ValueName::Parameter {
        key: E,
        binding: NETWORK,
    };
    let core = reference(REFERENCE_NS);
    let solved = tiny_flow(&case, &core, 1e-7).expect("solved");
    let fitted = hi(record(&solved.provenance, name).value);
    assert!((fitted - 2.0).abs() < 1e-5, "{fitted}");
}

/// Past rounding on the rows no value of the fit can be told from another, so it is refused, not stamped.
#[test]
fn a_fit_only_rounding_can_feel_is_refused_by_its_key() {
    let case = PulsatileCase::plain();
    let core = reference(REFERENCE_NS);
    assert_eq!(
        tiny_flow(&case, &core, 1e-14).err(),
        Some(InitializationFailure::Admission(AdmissionError::FitUnread {
            key: E,
            binding: NETWORK
        }))
    );
}

/// A ring whose first point sits between a lens's instants is read at no point of it.
#[test]
fn a_run_that_starts_off_the_lenss_grid_is_refused() {
    let case = PulsatileCase::plain();
    let lens = gauges(&case.declared, GAUGE, 4);
    let reading = |moment| {
        let (packet, _, _) = steady_packet(&case);
        let parameters = case.body().bind(packet).expect("bound");
        let request = InitializationRequest::at(at(moment)).reading(&lens);
        (case.body().initialization(parameters, empty_port(), request)).err()
    };
    let step = PERIOD_NS / 4;
    assert_eq!(reading(7 * step), None);
    assert_eq!(
        reading(7 * step + 1),
        Some(AdmissionError::AnchorUnread { anchor: MEAN })
    );
}

/// One identity has one start whatever made it: a start or fit moved inside the tolerance, or another core's
/// history there, names another run.
#[test]
fn a_start_moved_inside_the_tolerance_or_another_history_names_another_run() {
    let case = PulsatileCase::plain();
    let named = |slot: usize, change: f64, history: MethodHistory| {
        let (packet, boundary, _) = steady_packet(&case);
        let system = steady_system(&case, packet, boundary);
        let mut start = system.workspace();
        start.unknowns_mut()[slot] += change;
        let revision = system.admit(start, history);
        revision.expect("inside the tolerance").packet_hash()
    };
    let last = 2 * 4;
    let plain = named(0, 0.0, case.history());
    assert_eq!(plain, named(0, 0.0, case.history()));
    assert_ne!(plain, named(0, 1e-10, case.history()));
    assert_ne!(plain, named(last, 1e-10, case.history()));
    let counted = MethodHistory::new(Digest::of_canonical(b"a core"), at(0), vec![1]);
    assert_ne!(plain, named(0, 0.0, counted));
}

/// Every amount is far below the default's floor, and the rows still speak for the fit on their own scale.
#[test]
fn a_small_body_is_solved_on_its_own_scale() {
    let case = PulsatileCase::plain();
    let small = 1e-16;
    let floor = measure::<Dimensionless>(1e-30);
    let mut packet = ParameterPacket::periodic(cycle(INTERVALS));
    packet.within(ErrorScales::uniform(floor, measure(1e-12)).expect("positive"));
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .start(case.a(), measure::<Water>(START[0] * small), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(START[1] * small), NOTE)
        .expect("once");
    packet
        .anchor(case.mean_anchor(MEAN, case.a(), MEAN_A * small))
        .expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(GUESS), MEAN)
        .expect("once");
    let boundary = BoundaryPort::new(vec![
        water(b"pulse", 0, PULSE, 0, PULSE_NS, DELIVERED * small),
        water(b"drain", 0, DRAIN, 0, PERIOD_NS, DELIVERED * small),
    ])
    .expect("parsed");
    let core = reference(REFERENCE_NS);
    let solved = admit_solved(case.body(), packet, boundary, &core, &case.lens).expect("solved");
    let name = ValueName::Parameter {
        key: E,
        binding: NETWORK,
    };
    let fitted = hi(record(&solved.provenance, name).value);
    assert!((fitted - fitted_elastance()).abs() < 1e-9, "{fitted}");
}

#[test]
fn a_wider_tolerance_is_another_packet_so_it_cannot_share_a_runs_identity() {
    let case = PulsatileCase::plain();
    let wide = ErrorScales::uniform(measure(1e6), measure(1e6)).expect("positive");
    let run = |tolerance: Option<ErrorScales>| {
        let mut packet = case.packet();
        if let Some(tolerance) = tolerance {
            packet.within(tolerance);
        }
        let parameters = case.body().bind(packet).expect("bound");
        let request = InitializationRequest::at(at(0)).reading(&case.lens);
        let system = (case.body().initialization(parameters, port(0..1), request)).expect("a system");
        let guess = system.workspace();
        system
            .admit(guess, case.history())
            .map(|origin| origin.packet_hash())
    };
    let row = AdmissionRow::Body(case.body().rows()[0].id());
    assert_eq!(run(None), Err(InitializationFailure::Inconsistent { row }));
    let hash = |tolerance| {
        let mut packet = case.packet();
        packet.within(tolerance);
        case.body().bind(packet).expect("bound").packet_hash()
    };
    assert_ne!(hash(wide), hash(admission_tolerance()));
    assert!(run(Some(wide)).is_ok());
}

#[test]
fn a_stage_is_read_in_its_rows_own_unit_not_as_a_rate() {
    let case = PulsatileCase::plain();
    let rows = case.body().rows().len();
    let (a, b) = {
        let (_, _, a) = steady_packet(&case);
        (a, TOTAL - a)
    };
    let scale = 1e-9 + 1e-12 * a;
    let admitted = |nudge: f64| {
        let (packet, boundary, _) = steady_packet(&case);
        let system = steady_system(&case, packet, boundary);
        let mut witness = system.workspace();
        witness.unknowns_mut()[rows] = a + nudge;
        witness.unknowns_mut()[rows + 1] = b - nudge;
        system.admit(witness, case.history()).err()
    };
    assert_eq!(admitted(0.2 * scale), None);
    assert!(admitted(3.0 * scale).is_some());
}

#[test]
fn a_workspace_of_another_shape_is_refused() {
    let case = PulsatileCase::plain();
    let refused = Some(InitializationFailure::Admission(AdmissionError::ForeignWorkspace));
    let (packet, boundary, _) = steady_packet(&case);
    let small = steady_system(&case, packet, boundary);
    let large = case.system(case.packet());
    let (too_short, too_long) = (small.workspace(), large.workspace());
    assert_eq!(large.admit(too_short, case.history()).err(), refused);
    assert_eq!(small.admit(too_long, case.history()).err(), refused);
}

fn boundary_row(local: &'static str) -> RowKind {
    RowKind::Algebraic(LawName {
        owner: "Boundary",
        local,
    })
}

/// The instant case's packet, with one fit on an anchor at `row`.
fn instant_packet(case: &PulsatileCase, row: RowKind, target: f64) -> ParameterPacket<SyntheticFixture> {
    let mut packet = ParameterPacket::new();
    packet.within(admission_tolerance());
    packet
        .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
        .expect("once");
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .start(case.a(), measure::<Water>(START[0]), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(START[1]), NOTE)
        .expect("once");
    for local in [READING, ECHO] {
        packet
            .start(boundary_row(local), measure::<Rate<Water>>(1.0), NOTE)
            .expect("once");
    }
    let read = Snapshot::new(
        row,
        GAUGE,
        AnchorStatistic::Instant,
        measure::<Rate<Water>>(target),
    );
    packet.anchor(Synthetic::new(MEAN, read, NOTE)).expect("once");
    let gain = measure::<Dimensionless>(1.0);
    packet.calibrated(SET, Binding::Body, gain, MEAN).expect("once");
    packet
}

#[test]
fn a_fit_solves_at_an_instant_on_a_body_with_a_conserved_total() {
    let case = PulsatileCase::new(Variant::Instant);
    let reading = boundary_row(READING);
    let bare = ChannelSpec::new(GAUGE, reading);
    let lens = case.body().lens(vec![bare], count(SECOND)).expect("a lens");
    let packet = instant_packet(&case, reading, 2.0 * CHANNEL_FLOW);
    let events = vec![water(b"steady", 0, PULSE, 0, PERIOD_NS, CHANNEL_FLOW)];
    let boundary = BoundaryPort::new(events).expect("parsed");
    let core = reference(SECOND);
    let Admitted { provenance, .. } =
        admit_solved(case.body(), packet, boundary, &core, &lens).expect("solved");
    let name = ValueName::Parameter {
        key: SET,
        binding: Binding::Body,
    };
    assert_eq!(
        record(&provenance, name).provenance,
        ValueProvenance::Calibrated { anchor: MEAN }
    );
    assert!((hi(record(&provenance, name).value) - 2.0).abs() < 1e-12);
}

#[test]
fn a_row_that_reaches_a_drive_through_another_row_can_carry_an_anchor() {
    let case = PulsatileCase::new(Variant::Instant);
    let packet = instant_packet(&case, boundary_row(ECHO), 2.0 * CHANNEL_FLOW);
    assert!(case.body().bind(packet).is_ok());
}

// Check 3: provenance per value, an authored value inside its range, no parameter pinned to an anchor.

const OF_THE_RATIO: (ParameterKey, Binding) = (RATIO, Binding::Body);

fn twice() -> Measure<Dimensionless> {
    measure(2.0)
}

fn extra_packet(seed: PacketSeed) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let case = PulsatileCase::new(Variant::Extra);
    let mut packet = case.packet();
    let speed = measure::<Rate<Dimensionless>>(1.0);
    packet.synthetic(PACE, Binding::Body, speed, NOTE)?;
    packet.start(
        RowKind::Memory(name(SETTLED)),
        measure::<Dimensionless>(1.0),
        NOTE,
    )?;
    let (low, high) = (measure::<Dimensionless>(1.0), measure::<Dimensionless>(3.0));
    packet.population(TONE, low, high, NOTE)?;
    packet.sampled(TONE, Binding::Body, seed)?;
    let (low, high) = (measure::<Dimensionless>(0.0), measure::<Dimensionless>(9.0));
    packet.population(RATIO, low, high, NOTE)?;
    packet.authored(
        RATIO,
        Binding::Body,
        measure::<Dimensionless>(4.5),
        "a teaching case",
    )?;
    Ok(packet)
}

#[test]
fn provenance_is_recorded_per_value_in_all_five_kinds_and_a_solved_start() {
    let case = PulsatileCase::new(Variant::Extra);
    let mut packet = extra_packet(PacketSeed::new([9, 9, 9, 9])).expect("a packet");
    packet
        .derived(DOUBLED, Binding::Body, "twice the ratio", OF_THE_RATIO, twice())
        .expect("once");
    let core = reference(REFERENCE_NS);
    let Admitted { provenance, .. } =
        admit_solved(case.body(), packet, port(0..1), &core, &case.lens).expect("solved");
    let entry = |key, binding| ValueName::Parameter { key, binding };
    let kinds = [
        (entry(E, NETWORK), ValueProvenance::Calibrated { anchor: MEAN }),
        (entry(G, NETWORK), ValueProvenance::Fixture),
        (entry(TONE, Binding::Body), ValueProvenance::Sampled),
        (
            entry(RATIO, Binding::Body),
            ValueProvenance::Authored {
                reason: "a teaching case",
            },
        ),
        (
            entry(DOUBLED, Binding::Body),
            ValueProvenance::Derived {
                identity: "twice the ratio",
                source: RATIO,
                binding: Binding::Body,
            },
        ),
        (ValueName::Start(case.a()), ValueProvenance::Solved),
        (ValueName::Start(case.b()), ValueProvenance::Solved),
    ];
    for (name, kind) in kinds {
        assert_eq!(record(&provenance, name).provenance, kind, "{name:?}");
    }
    assert_eq!(provenance.values().len(), 6 + case.body().rows().len());
    let derived = record(&provenance, entry(DOUBLED, Binding::Body));
    assert_eq!(hi(derived.value), 2.0 * 4.5);
}

#[test]
fn a_derived_value_rests_on_one_settled_entry_of_its_packet() {
    let case = PulsatileCase::new(Variant::Extra);
    let refused = |source: (ParameterKey, Binding)| {
        let mut packet = extra_packet(PacketSeed::new([1, 1, 1, 1])).expect("a packet");
        packet
            .derived(DOUBLED, Binding::Body, "an identity", source, twice())
            .expect("once");
        case.body().bind(packet).err()
    };
    let refusal = |source| Some(AdmissionError::IdentitySource { key: DOUBLED, source });
    let absent = ParameterKey::new("cv.absent");
    assert_eq!(refused((absent, Binding::Body)), refusal(absent));
    assert_eq!(refused((RATIO, NETWORK)), refusal(RATIO));
    assert_eq!(refused((DOUBLED, Binding::Body)), refusal(DOUBLED));
    assert_eq!(refused((E, NETWORK)), refusal(E));
}

#[test]
fn a_derived_value_takes_its_sources_dimension() {
    let case = PulsatileCase::new(Variant::Extra);
    let mut packet = extra_packet(PacketSeed::new([1, 1, 1, 1])).expect("a packet");
    packet
        .derived(DOUBLED, Binding::Body, "an identity", (G, NETWORK), twice())
        .expect("once");
    assert_eq!(
        case.body().bind(packet).err(),
        Some(AdmissionError::ParameterDimension {
            key: DOUBLED,
            binding: Binding::Body
        })
    );
}

#[test]
fn an_authored_value_lies_inside_its_population_records_range() {
    let mut packet = ParameterPacket::<SyntheticFixture>::new();
    let authored = |packet: &mut ParameterPacket<SyntheticFixture>, value| {
        packet.authored(TONE, Binding::Body, measure::<Dimensionless>(value), "a reason")
    };
    assert_eq!(
        authored(&mut packet, 2.0),
        Err(AdmissionError::NoPopulation { key: TONE })
    );
    let (low, high) = (measure::<Dimensionless>(1.0), measure::<Dimensionless>(3.0));
    packet.population(TONE, low, high, NOTE).expect("once");
    let outside = AdmissionError::OutsidePopulation {
        key: TONE,
        binding: Binding::Body,
    };
    assert_eq!(authored(&mut packet, 3.5), Err(outside));
    assert_eq!(authored(&mut packet, 0.5), Err(outside));
    assert_eq!(
        packet.authored(TONE, Binding::Body, measure::<Pressure>(2.0), "a reason"),
        Err(AdmissionError::ParameterDimension {
            key: TONE,
            binding: Binding::Body
        })
    );
    assert_eq!(authored(&mut packet, 3.0), Ok(()));
}

#[test]
fn a_population_record_is_one_interval_per_key() {
    let mut packet = ParameterPacket::<SyntheticFixture>::new();
    let (low, high) = (measure::<Dimensionless>(3.0), measure::<Dimensionless>(1.0));
    assert_eq!(
        packet.population(TONE, low, high, NOTE),
        Err(AdmissionError::PopulationRange { key: TONE })
    );
    let (low, high) = (measure::<Dimensionless>(1.0), measure::<Dimensionless>(3.0));
    packet.population(TONE, low, high, NOTE).expect("once");
    let (low, high) = (measure::<Dimensionless>(1.0), measure::<Dimensionless>(3.0));
    assert_eq!(
        packet.population(TONE, low, high, NOTE),
        Err(AdmissionError::PopulationRange { key: TONE })
    );
    let (low, high) = (measure::<Dimensionless>(-1e308), measure::<Dimensionless>(1e308));
    assert_eq!(
        packet.population(RATIO, low, high, NOTE),
        Err(AdmissionError::PopulationRange { key: RATIO })
    );
}

#[test]
fn a_sampled_value_needs_a_record_and_moves_only_with_the_packet_seed() {
    let mut packet = ParameterPacket::<SyntheticFixture>::new();
    assert_eq!(
        packet.sampled(TONE, Binding::Body, PacketSeed::new([1, 2, 3, 4])),
        Err(AdmissionError::NoPopulation { key: TONE })
    );
    let case = PulsatileCase::new(Variant::Extra);
    let hash = |seed| {
        let mut packet = extra_packet(seed).expect("a packet");
        packet
            .derived(DOUBLED, Binding::Body, "twice the ratio", OF_THE_RATIO, twice())
            .expect("once");
        case.body().bind(packet).expect("bound").packet_hash()
    };
    let seed = PacketSeed::new([1, 2, 3, 4]);
    assert_eq!(hash(seed), hash(seed));
    assert_ne!(hash(seed), hash(PacketSeed::new([1, 2, 3, 5])));
}

fn drawn(seed: PacketSeed, tone_first: bool) -> [f64; 2] {
    let case = PulsatileCase::new(Variant::Extra).with_intervals(4);
    let mut packet = case.packet();
    let speed = measure::<Rate<Dimensionless>>(1.0);
    packet.synthetic(PACE, Binding::Body, speed, NOTE).expect("once");
    let settled = RowKind::Memory(name(SETTLED));
    packet
        .start(settled, measure::<Dimensionless>(1.0), NOTE)
        .expect("once");
    let doubled = measure::<Dimensionless>(9.0);
    packet
        .synthetic(DOUBLED, Binding::Body, doubled, NOTE)
        .expect("once");
    let keys = if tone_first { [TONE, RATIO] } else { [RATIO, TONE] };
    for key in keys {
        let (low, high) = (measure::<Dimensionless>(1.0), measure::<Dimensionless>(3.0));
        packet.population(key, low, high, NOTE).expect("once");
        packet.sampled(key, Binding::Body, seed).expect("a record");
    }
    let core = reference(REFERENCE_NS);
    let solved = admit_solved(case.body(), packet, port(0..1), &core, &case.lens).expect("solved");
    [TONE, RATIO].map(|key| {
        let binding = Binding::Body;
        let name = ValueName::Parameter { key, binding };
        assert_eq!(
            record(&solved.provenance, name).provenance,
            ValueProvenance::Sampled
        );
        hi(record(&solved.provenance, name).value)
    })
}

#[test]
fn a_sampled_value_lies_inside_its_range_and_no_other_entry_moves_it() {
    let mut seen = Vec::new();
    for word in 0..8 {
        let seed = PacketSeed::new([word, 2, 3, 4]);
        let values = drawn(seed, true);
        assert_eq!(values.map(f64::to_bits), drawn(seed, false).map(f64::to_bits));
        assert!(
            values.iter().all(|value| (1.0..=3.0).contains(value)),
            "{values:?}"
        );
        assert_ne!(values[0].to_bits(), values[1].to_bits());
        seen.push(values[0].to_bits());
    }
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 8);
}

#[test]
fn a_law_whose_row_is_a_parameter_read_cannot_carry_an_anchor() {
    let core = |variant, row: RowKind| {
        let case = PulsatileCase::new(variant);
        let read = Snapshot::new(
            row,
            GAUGE,
            AnchorStatistic::CycleMean,
            measure::<Dimensionless>(2.0),
        );
        let mut packet = case.packet_without_fit();
        packet
            .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
            .expect("once");
        packet
            .start(row, measure::<Dimensionless>(1.0), NOTE)
            .expect("once");
        packet.anchor(Synthetic::new(MEAN, read, NOTE)).expect("once");
        let guess = measure::<Dimensionless>(1.0);
        packet.calibrated(SET, Binding::Body, guess, MEAN).expect("once");
        if variant == Variant::PinnedRelax {
            let speed = measure::<Rate<Dimensionless>>(1.0);
            packet.synthetic(PACE, Binding::Body, speed, NOTE).expect("once");
        }
        case.body().bind(packet).err()
    };
    let refused = Some(AdmissionError::AnchorOnParameters { anchor: MEAN });
    assert_eq!(
        core(Variant::PinnedRead, RowKind::Algebraic(name("read"))),
        refused
    );
    assert_eq!(
        core(Variant::PinnedRelax, RowKind::Memory(name(SETTLED))),
        refused
    );
}

#[test]
fn a_fit_names_one_anchor_its_packet_holds_and_an_anchor_takes_one_fit() {
    let case = PulsatileCase::plain();
    let mut packet = case.packet_without_fit();
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(GUESS), MEAN)
        .expect("once");
    assert_eq!(
        case.body().bind(packet).err(),
        Some(AdmissionError::UnknownAnchor {
            key: E,
            binding: NETWORK
        })
    );
    let mut packet = ParameterPacket::periodic(cycle(INTERVALS));
    packet
        .anchor(case.mean_anchor(MEAN, case.a(), MEAN_A))
        .expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(GUESS), MEAN)
        .expect("once");
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.calibrated(G, NETWORK, g, MEAN).expect("once");
    packet
        .start(case.a(), measure::<Water>(START[0]), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(START[1]), NOTE)
        .expect("once");
    assert_eq!(
        case.body().bind(packet).err(),
        Some(AdmissionError::AnchorFittedTwice { anchor: MEAN })
    );
}

#[test]
fn a_fit_on_an_anchor_its_packet_does_not_hold_is_refused_though_others_are_there() {
    let case = PulsatileCase::plain();
    let mut packet = case.packet_without_fit();
    let held = AnchorId::new("fixture.held");
    packet
        .anchor(case.mean_anchor(held, case.b(), TOTAL - MEAN_A))
        .expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(GUESS), MEAN)
        .expect("once");
    assert_eq!(
        case.body().bind(packet).err(),
        Some(AdmissionError::UnknownAnchor {
            key: E,
            binding: NETWORK
        })
    );
}

#[test]
fn a_refusal_names_the_same_anchor_however_the_packet_was_built() {
    let case = PulsatileCase::plain();
    let (first, second) = (AnchorId::new("fixture.a"), AnchorId::new("fixture.b"));
    let core = reference(REFERENCE_NS);
    let refused = |anchors: [AnchorId; 2]| {
        let mut packet = case.packet();
        for id in anchors {
            let row = if id == first { case.a() } else { case.b() };
            packet.anchor(case.mean_anchor(id, row, 1.0)).expect("once");
        }
        admit_solved(case.body(), packet, port(0..1), &core, &case.lens).err()
    };
    let named = Some(InitializationFailure::Inconsistent {
        row: AdmissionRow::Anchor(first),
    });
    assert_eq!(refused([first, second]), named);
    assert_eq!(refused([second, first]), named);
}

#[test]
fn an_anchor_id_is_used_once_and_an_anchor_reads_a_row_of_its_own_dimension() {
    let case = PulsatileCase::plain();
    let mut packet = case.packet();
    assert_eq!(
        packet.anchor(case.mean_anchor(MEAN, case.b(), MEAN_A)),
        Err(AdmissionError::DuplicateAnchor { anchor: MEAN })
    );
    let second = AnchorId::new("fixture.pressure");
    let read = Snapshot::new(
        case.a(),
        GAUGE,
        AnchorStatistic::CycleMean,
        measure::<Pressure>(1.0),
    );
    packet.anchor(Synthetic::new(second, read, NOTE)).expect("once");
    assert_eq!(
        case.body().bind(packet).err(),
        Some(AdmissionError::AnchorRow { anchor: second })
    );
}

// The cycle and the run's identity.

#[test]
fn a_cycle_refuses_a_grid_that_does_not_divide_it() {
    let intervals = NonZeroU32::new(3).expect("not zero");
    assert_eq!(
        Cycle::new(count(10), intervals).err(),
        Some(AdmissionError::UnevenCycle)
    );
}

#[test]
fn a_delivery_edge_between_two_points_is_refused_and_one_past_the_window_is_taken() {
    let case = PulsatileCase::plain();
    let off = PULSE_NS + 1;
    let system = |event: BoundaryEvent| {
        let events = vec![event, water(b"drain", 0, DRAIN, 0, PERIOD_NS, DELIVERED)];
        let boundary = BoundaryPort::new(events).expect("parsed");
        let parameters = case.body().bind(case.packet()).expect("bound");
        let request = InitializationRequest::at(at(0)).reading(&case.lens);
        case.body().initialization(parameters, boundary, request).err()
    };
    let refused = |error| matches!(error, Some(AdmissionError::DeliveryOffGrid { .. }));
    assert!(refused(system(water(b"late end", 0, PULSE, 0, off, DELIVERED))));
    assert!(refused(system(water(
        b"late stamp",
        0,
        PULSE,
        off,
        PERIOD_NS / 2,
        1.0
    ))));
    let past = PERIOD_NS + 1;
    assert_eq!(system(water(b"past", 0, PULSE, past, past + 1, 1.0)), None);
}

/// A solved value is off the clock by rounding, and a period off the clock can round to nothing at the first step.
#[test]
fn a_fit_on_a_forcings_period_is_refused_though_a_law_reads_it() {
    let case = PulsatileCase::new(Variant::PeriodRead);
    let (mut packet, _, _) = steady_packet(&case);
    let row = boundary_row(READING);
    let read = Snapshot::new(
        row,
        GAUGE,
        AnchorStatistic::CycleMean,
        measure::<Product<Rate<Water>, Time>>(1.0),
    );
    let timed = AnchorId::new("fixture.timed");
    packet.anchor(Synthetic::new(timed, read, NOTE)).expect("once");
    packet
        .calibrated(KNOTS, Binding::Body, measure::<Time>(1.0), timed)
        .expect("once");
    let amplitude = measure::<Dimensionless>(0.5);
    packet
        .synthetic(WOBBLE, Binding::Body, amplitude, NOTE)
        .expect("once");
    packet
        .start(row, measure::<Product<Rate<Water>, Time>>(1.0), NOTE)
        .expect("once");
    assert_eq!(
        case.body().bind(packet).err(),
        Some(AdmissionError::ForcingPeriod { key: KNOTS })
    );
}

#[test]
fn a_fit_no_row_feels_at_the_start_is_refused_by_its_key() {
    let case = PulsatileCase::plain();
    let even = 50.0;
    let mut packet = ParameterPacket::periodic(cycle(INTERVALS));
    packet.within(admission_tolerance());
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .start(case.a(), measure::<Water>(even), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(even), NOTE)
        .expect("once");
    packet
        .anchor(case.mean_anchor(MEAN, case.a(), even))
        .expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(1234.5), MEAN)
        .expect("once");
    let core = reference(REFERENCE_NS);
    assert_eq!(
        admit_solved(case.body(), packet, empty_port(), &core, &case.lens).err(),
        Some(InitializationFailure::Admission(AdmissionError::FitUnread {
            key: E,
            binding: NETWORK
        }))
    );
}

/// `E` fills every end. The anchor reads the pair at rest, and only the other pair feels the fit.
#[test]
fn a_fit_felt_only_away_from_its_anchor_is_refused_by_its_key() {
    let case = PulsatileCase::new(Variant::Apart);
    let kinds = case.body().rows().iter().map(|row| row.kind());
    let at_rest: Vec<RowKind> = kinds
        .filter(|kind| *kind != case.a() && *kind != case.b())
        .collect();
    let (elastance, even) = (2.0, 50.0);
    let difference = DELIVERED / (CONDUCTANCE * elastance);
    let mut packet = ParameterPacket::periodic(cycle(4));
    packet.within(admission_tolerance());
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    for (row, amount) in [
        (case.a(), (TOTAL + difference) / 2.0),
        (case.b(), (TOTAL - difference) / 2.0),
        (at_rest[0], even),
        (at_rest[1], even),
    ] {
        packet.start(row, measure::<Water>(amount), NOTE).expect("once");
    }
    let read = Snapshot::new(
        at_rest[0],
        GAUGE,
        AnchorStatistic::CycleMean,
        measure::<Water>(even),
    );
    packet.anchor(Synthetic::new(MEAN, read, NOTE)).expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(elastance), MEAN)
        .expect("once");
    let channel = ChannelSpec::new(GAUGE, at_rest[0]).averaged(count(4));
    let lens = case
        .body()
        .lens(vec![channel], count(PERIOD_NS / 4))
        .expect("a lens");
    let boundary = BoundaryPort::new(vec![
        water(b"even in", 0, PULSE, 0, PERIOD_NS, DELIVERED),
        water(b"even out", 0, DRAIN, 0, PERIOD_NS, DELIVERED),
    ])
    .expect("parsed");
    let core = reference(REFERENCE_NS);
    assert_eq!(
        admit_solved(case.body(), packet, boundary, &core, &lens).err(),
        Some(InitializationFailure::Admission(AdmissionError::FitUnread {
            key: E,
            binding: NETWORK
        }))
    );
}

#[test]
fn a_first_mode_has_a_record() {
    let case = PulsatileCase::new(Variant::Hazard);
    let mut packet = ParameterPacket::new();
    let g = measure::<Conductance<Water>>(CONDUCTANCE);
    packet.synthetic(G, NETWORK, g, NOTE).expect("once");
    packet
        .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
        .expect("once");
    let pace = measure::<Rate<Dimensionless>>(1.0);
    packet.synthetic(PACE, Binding::Body, pace, NOTE).expect("once");
    packet
        .start(case.a(), measure::<Water>(START[0]), NOTE)
        .expect("once");
    packet
        .start(case.b(), measure::<Water>(START[1]), NOTE)
        .expect("once");
    packet
        .start(
            RowKind::Memory(name("hazard")),
            measure::<Dimensionless>(0.0),
            NOTE,
        )
        .expect("once");
    packet.start_mode(name("rhythm"), "ectopic", NOTE).expect("once");
    let core = reference(SECOND);
    let lens = case.body().lens(Vec::new(), count(SECOND)).expect("a lens");
    let solved = admit_solved(case.body(), packet, empty_port(), &core, &lens).expect("solved");
    let mode = ValueName::Mode {
        variable: name("rhythm"),
        value: "ectopic",
    };
    assert_eq!(
        record(&solved.provenance, mode).provenance,
        ValueProvenance::Fixture
    );
}

#[test]
fn a_mode_variable_is_refused_under_a_cycle() {
    let case = PulsatileCase::new(Variant::Hazard);
    let mut packet = case.packet_without_fit();
    packet
        .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
        .expect("once");
    packet.start_mode(name("rhythm"), "sinus", NOTE).expect("once");
    assert_eq!(
        case.body().bind(packet).err(),
        Some(AdmissionError::ModesNotYetRun {
            variable: name("rhythm")
        })
    );
}

#[test]
fn the_goal_and_its_grid_are_in_the_packet_hash() {
    let case = PulsatileCase::plain();
    let hash = |intervals| {
        let other = PulsatileCase::plain().with_intervals(intervals);
        case.body().bind(other.packet()).expect("bound").packet_hash()
    };
    assert_eq!(hash(64), hash(64));
    assert_ne!(hash(64), hash(32));
    let slow = PulsatileCase::new(Variant::Slow);
    let read_as = |statistic| {
        let read = Snapshot::new(slow.a(), GAUGE, statistic, measure::<Water>(MEAN_A));
        let mut packet = slow.packet_without_fit();
        packet
            .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
            .expect("once");
        packet.anchor(Synthetic::new(MEAN, read, NOTE)).expect("once");
        slow.body().bind(packet).expect("bound").packet_hash()
    };
    assert_ne!(
        read_as(AnchorStatistic::Instant),
        read_as(AnchorStatistic::CycleMean)
    );
}

#[test]
fn two_builds_of_one_packet_start_on_the_same_bits() {
    let case = PulsatileCase::new(Variant::Forced);
    let second = AnchorId::new("fixture.mean-b");
    let mean = |id, row, value| case.mean_anchor(id, row, value);
    let build = |fits_first: bool| {
        let mut packet = ParameterPacket::periodic(cycle(INTERVALS));
        if fits_first {
            packet
                .anchor(mean(second, case.b(), TOTAL - MEAN_A))
                .expect("once");
            packet.anchor(mean(MEAN, case.a(), MEAN_A)).expect("once");
            let g = measure::<Conductance<Water>>(CONDUCTANCE);
            packet.calibrated(G, NETWORK, g, second).expect("once");
            let e = measure::<Elastance>(GUESS);
            packet.calibrated(E, NETWORK, e, MEAN).expect("once");
        } else {
            packet.anchor(mean(MEAN, case.a(), MEAN_A)).expect("once");
            packet
                .anchor(mean(second, case.b(), TOTAL - MEAN_A))
                .expect("once");
            let e = measure::<Elastance>(GUESS);
            packet.calibrated(E, NETWORK, e, MEAN).expect("once");
            let g = measure::<Conductance<Water>>(CONDUCTANCE);
            packet.calibrated(G, NETWORK, g, second).expect("once");
        }
        let period = measure::<Time>(1.0);
        packet
            .synthetic(KNOTS, Binding::Body, period, NOTE)
            .expect("once");
        let amplitude = measure::<Dimensionless>(0.5);
        packet
            .synthetic(WOBBLE, Binding::Body, amplitude, NOTE)
            .expect("once");
        packet
            .start(case.a(), measure::<Water>(START[0]), NOTE)
            .expect("once");
        packet
            .start(case.b(), measure::<Water>(START[1]), NOTE)
            .expect("once");
        packet
    };
    let hash = |packet| case.body().bind(packet).expect("bound").packet_hash();
    assert_eq!(hash(build(true)), hash(build(false)));
    let parts = |fits_first| {
        let system = case.system(build(fits_first));
        let mut workspace = system.workspace();
        let residuals = system.evaluate(&workspace).expect("rows");
        let jacobian = system.linearize(&workspace).expect("a matrix");
        let bits = |values: &[f64]| values.iter().map(|value| value.to_bits()).collect::<Vec<_>>();
        (
            bits(workspace.unknowns_mut()),
            bits(&residuals),
            bits(jacobian.view().values()),
        )
    };
    assert_eq!(parts(true), parts(false));
}

#[test]
fn which_anchor_a_fit_names_is_in_the_hash() {
    let case = PulsatileCase::plain();
    let (first, second) = (AnchorId::new("fixture.a"), AnchorId::new("fixture.b"));
    let hash = |target| {
        let mut packet = case.packet_without_fit();
        packet
            .anchor(case.mean_anchor(first, case.a(), MEAN_A))
            .expect("once");
        packet
            .anchor(case.mean_anchor(second, case.b(), TOTAL - MEAN_A))
            .expect("once");
        packet
            .calibrated(E, NETWORK, measure::<Elastance>(GUESS), target)
            .expect("once");
        case.body().bind(packet).expect("bound").packet_hash()
    };
    assert_ne!(hash(first), hash(second));
}

#[derive(Clone, Copy)]
struct Stated {
    method: &'static str,
    value: f64,
    anchor_note: &'static str,
    range: [f64; 2],
    other_range: [f64; 2],
    record_note: &'static str,
    reason: &'static str,
    identity: &'static str,
    scale: f64,
    seed: u64,
    tolerance: f64,
}
const STATED: Stated = Stated {
    method: GAUGE,
    value: MEAN_A,
    anchor_note: NOTE,
    range: [1.0, 3.0],
    other_range: [0.0, 9.0],
    record_note: NOTE,
    reason: "a reason",
    identity: "an identity",
    scale: 2.0,
    seed: 1,
    tolerance: 1e-9,
};

fn stated_hash(case: &PulsatileCase, stated: Stated) -> PacketHash {
    let mut packet = case.packet_without_fit();
    let tolerance = ErrorScales::uniform(measure(stated.tolerance), measure(1e-12)).expect("positive");
    packet.within(tolerance);
    let speed = measure::<Rate<Dimensionless>>(1.0);
    packet.synthetic(PACE, Binding::Body, speed, NOTE).expect("once");
    let settled = RowKind::Memory(name(SETTLED));
    packet
        .start(settled, measure::<Dimensionless>(1.0), NOTE)
        .expect("once");
    let read = Snapshot::new(
        case.a(),
        stated.method,
        AnchorStatistic::CycleMean,
        measure::<Water>(stated.value),
    );
    packet
        .anchor(Synthetic::new(MEAN, read, stated.anchor_note))
        .expect("once");
    packet
        .calibrated(E, NETWORK, measure::<Elastance>(GUESS), MEAN)
        .expect("once");
    let [low, high] = stated.range.map(measure::<Dimensionless>);
    packet
        .population(TONE, low, high, stated.record_note)
        .expect("once");
    let seed = PacketSeed::new([stated.seed, 2, 3, 4]);
    packet.sampled(TONE, Binding::Body, seed).expect("a record");
    let [low, high] = stated.other_range.map(measure::<Dimensionless>);
    packet.population(RATIO, low, high, NOTE).expect("once");
    let ratio = measure::<Dimensionless>(4.5);
    packet
        .authored(RATIO, Binding::Body, ratio, stated.reason)
        .expect("inside");
    packet
        .derived(
            DOUBLED,
            Binding::Body,
            stated.identity,
            OF_THE_RATIO,
            measure(stated.scale),
        )
        .expect("once");
    case.body().bind(packet).expect("bound").packet_hash()
}

#[test]
fn a_delivery_inside_the_window_is_in_the_hash_a_run_names_and_one_past_it_is_not() {
    let case = PulsatileCase::plain();
    let core = reference(REFERENCE_NS);
    let named = |events: Vec<BoundaryEvent>| {
        let boundary = BoundaryPort::new(events).expect("parsed");
        let solved = admit_solved(case.body(), case.packet(), boundary, &core, &case.lens);
        let session = solved.expect("solved").session;
        session.record().identity().digest()
    };
    let pulse = |from| water(b"pulse", 0, PULSE, from, from + PULSE_NS, DELIVERED);
    let drain = || water(b"drain", 0, DRAIN, 0, PERIOD_NS, DELIVERED);
    let later = || water(b"later", 0, PULSE, PERIOD_NS, 2 * PERIOD_NS, 1.0);
    let base = named(vec![pulse(0), drain()]);
    assert_ne!(base, named(vec![pulse(PERIOD_NS / 2), drain()]));
    assert_eq!(base, named(vec![pulse(0), drain(), later()]));
}

#[test]
fn the_moment_a_run_starts_is_in_the_hash_it_names() {
    let case = PulsatileCase::new(Variant::Slow);
    let named = |moment: u64| {
        let mut packet = ParameterPacket::new();
        packet
            .synthetic(E, NETWORK, measure::<Elastance>(GUESS), NOTE)
            .expect("once");
        let g = measure::<Conductance<Water>>(CONDUCTANCE);
        packet.synthetic(G, NETWORK, g, NOTE).expect("once");
        packet
            .start(case.a(), measure::<Water>(START[0]), NOTE)
            .expect("once");
        packet
            .start(case.b(), measure::<Water>(START[1]), NOTE)
            .expect("once");
        let parameters = case.body().bind(packet).expect("bound");
        let request = InitializationRequest::at(at(moment));
        let system = (case.body().initialization(parameters, empty_port(), request)).expect("a system");
        let guess = system.workspace();
        system.admit(guess, case.history()).expect("exact").packet_hash()
    };
    assert_eq!(named(0), named(0));
    assert_ne!(named(0), named(5 * SECOND));
}

#[test]
fn every_part_of_a_packet_is_in_its_hash() {
    let case = PulsatileCase::new(Variant::Extra);
    let base = stated_hash(&case, STATED);
    assert_eq!(base, stated_hash(&case, STATED));
    let changed = [
        Stated {
            method: "another",
            ..STATED
        },
        Stated {
            value: MEAN_A + 1.0,
            ..STATED
        },
        Stated {
            anchor_note: "another",
            ..STATED
        },
        Stated {
            range: [0.5, 3.0],
            ..STATED
        },
        Stated {
            range: [1.0, 3.5],
            ..STATED
        },
        Stated {
            other_range: [0.0, 10.0],
            ..STATED
        },
        Stated {
            other_range: [1.0, 9.0],
            ..STATED
        },
        Stated {
            record_note: "another",
            ..STATED
        },
        Stated {
            reason: "another",
            ..STATED
        },
        Stated {
            identity: "another",
            ..STATED
        },
        Stated { scale: 3.0, ..STATED },
        Stated { seed: 2, ..STATED },
        Stated {
            tolerance: 1e6,
            ..STATED
        },
    ];
    for (at, stated) in changed.into_iter().enumerate() {
        assert_ne!(stated_hash(&case, stated), base, "change {at}");
    }
}

// The fence.

#[test]
fn a_late_input_inside_the_window_is_refused_and_one_at_its_end_is_taken() {
    let case = PulsatileCase::plain();
    let core = reference(REFERENCE_NS);
    let mut session = admit_solved(case.body(), case.packet(), port(0..1), &core, &case.lens)
        .expect("solved")
        .session;
    assert_eq!(session.record().earliest_stamp(), at(PERIOD_NS));
    let late = |tag, stamp| {
        let event = water(tag, 0, PULSE, stamp, stamp + PERIOD_NS / 8, 1.0);
        AuthorizedInput::from_command_receipt(Digest::of_canonical(b"a command"), event)
    };
    let id = InputId::new(Digest::of_canonical(b"inside\0\0\0\0\0\0\0\0"));
    assert_eq!(
        session.enqueue(late(b"inside", PERIOD_NS - 1)).err(),
        Some(AdmissionError::EventBeforeFence {
            id,
            fence: at(PERIOD_NS)
        })
    );
    assert!(session.enqueue(late(b"at its end", PERIOD_NS)).is_ok());
}

struct ScriptedClock {
    next: core::cell::Cell<u64>,
}
impl WallClock for ScriptedClock {
    fn elapsed_ns(&self) -> Option<u64> {
        let now = self.next.get();
        self.next.set(now + PERIOD_NS / 8);
        Some(now)
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

#[test]
fn every_replay_gate_passes_with_deliveries_inside_the_window() {
    let case = PulsatileCase::plain();
    let cores = || reference(REFERENCE_NS);
    let clock = ScriptedClock {
        next: core::cell::Cell::new(0),
    };
    let plan = ReplayPlan {
        clock: &clock,
        simulated_per_wall: 1,
        chunks: vec![PERIOD_NS / 3, PERIOD_NS / 7],
        resume_every: 16,
        chance: None,
    };
    assert!(replay_gates(&case, &cores, &plan).is_ok());
}

// Finding 15's extension.

fn bound_rate(
    rate: impl for<'b, 'k> FnOnce(
        &mut Kernel<'b, 'k, Cardiovascular>,
        &Parameter<'b, Cardiovascular, Pressure>,
        &Parameter<'b, Cardiovascular, Pressure>,
    ) -> Expr<'b, 'k, Cardiovascular, Rate<Dimensionless>>,
) -> Option<AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _ports) = body.cardiovascular()?.parts();
        let p = laws.parameter::<Pressure>(ParameterKey::new("fixture.p"), Admissible::positive());
        let q = laws.parameter::<Pressure>(ParameterKey::new("fixture.q"), Admissible::positive());
        let slot = laws.state::<Dimensionless>("memory", Admissible::finite());
        let beat = RateClass::new(RateBand::Beat, 100_000_000)?;
        let kernel = laws.kernel(|mut k| {
            let value = rate(&mut k, &p, &q);
            k.finish(value, laws.every_mode(), beat)
        });
        laws.memory_rate(&slot, kernel);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    body.bind(ParameterPacket::<SyntheticFixture>::new()).err()
}

fn zero_times<'b, 'k>(
    k: &mut Kernel<'b, 'k, Cardiovascular>,
    ratio: Expr<'b, 'k, Cardiovascular, Dimensionless>,
) -> Expr<'b, 'k, Cardiovascular, Rate<Dimensionless>> {
    let nothing = k.zero::<Rate<Dimensionless>>();
    k.scale(nothing, ratio)
}

#[test]
fn finding_15_two_equal_expressions_divided_are_refused_by_name() {
    let refused = Some(AdmissionError::UnbackedConstant { law: name("memory") });
    let sums = bound_rate(|k, p, _| {
        let (a, b, c, d) = (k.parameter(p), k.parameter(p), k.parameter(p), k.parameter(p));
        let (top, bottom) = (k.add(a, b), k.add(c, d));
        let ratio = k.divide(top, bottom);
        let one = k.as_dimension::<_, Dimensionless>(ratio);
        zero_times(k, one)
    });
    assert_eq!(sums, refused);
    let swapped = bound_rate(|k, p, q| {
        let (a, b, c, d) = (k.parameter(p), k.parameter(q), k.parameter(q), k.parameter(p));
        let (top, bottom) = (k.add(a, b), k.add(c, d));
        let ratio = k.divide(top, bottom);
        let one = k.as_dimension::<_, Dimensionless>(ratio);
        zero_times(k, one)
    });
    assert_eq!(swapped, refused);
    let limits = bound_rate(|k, p, q| {
        let (a, b, c, d) = (k.parameter(p), k.parameter(q), k.parameter(q), k.parameter(p));
        let (one, other) = (k.parameter(q), k.parameter(q));
        let (top, bottom) = (k.smooth_max(a, b, one), k.smooth_max(c, d, other));
        let ratio = k.divide(top, bottom);
        let one = k.as_dimension::<_, Dimensionless>(ratio);
        zero_times(k, one)
    });
    assert_eq!(limits, refused);
    let floors = bound_rate(|k, p, q| {
        let (a, b, c, d) = (k.parameter(p), k.parameter(q), k.parameter(q), k.parameter(p));
        let (one, other) = (k.parameter(q), k.parameter(q));
        let (top, bottom) = (k.smooth_min(a, b, one), k.smooth_min(c, d, other));
        let ratio = k.divide(top, bottom);
        let one = k.as_dimension::<_, Dimensionless>(ratio);
        zero_times(k, one)
    });
    assert_eq!(floors, refused);
}

/// No identity reads a quotient of products, so a row of its own dimension holds it as it stands.
#[test]
fn finding_15_two_equal_products_divided_are_refused_by_name() {
    type Ratio = Quotient<Product<Pressure, Pressure>, Product<Pressure, Pressure>>;
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", origin());
    let schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    let body = assemble(schema, |mut body| {
        let (mut laws, _ports) = body.cardiovascular()?.parts();
        let p = laws.parameter::<Pressure>(ParameterKey::new("fixture.p"), Admissible::positive());
        let q = laws.parameter::<Pressure>(ParameterKey::new("fixture.q"), Admissible::positive());
        let slot = laws.algebraic::<Ratio>("ratio", Admissible::finite());
        let beat = RateClass::new(RateBand::Beat, 100_000_000)?;
        let kernel = laws.kernel(|mut k| {
            let x = k.algebraic(&slot);
            let (a, b, c, d) = (k.parameter(&p), k.parameter(&q), k.parameter(&q), k.parameter(&p));
            let (top, bottom) = (k.multiply(a, b), k.multiply(c, d));
            let ratio = k.divide(top, bottom);
            let residual = k.subtract(x, ratio);
            k.finish(residual, laws.every_mode(), beat)
        });
        laws.residual(&slot, kernel);
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled");
    assert_eq!(
        body.bind(ParameterPacket::<SyntheticFixture>::new()).err(),
        Some(AdmissionError::UnbackedConstant { law: name("ratio") })
    );
}

#[test]
fn finding_15_two_different_expressions_divided_are_backed() {
    let outcome = bound_rate(|k, p, q| {
        let (a, b, c, d) = (k.parameter(p), k.parameter(p), k.parameter(p), k.parameter(q));
        let (top, bottom) = (k.add(a, b), k.add(c, d));
        let ratio = k.divide(top, bottom);
        let ratio = k.as_dimension::<_, Dimensionless>(ratio);
        zero_times(k, ratio)
    });
    assert!(!matches!(outcome, Some(AdmissionError::UnbackedConstant { .. })));
}
