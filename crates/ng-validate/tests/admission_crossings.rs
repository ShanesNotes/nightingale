//! A relaxing memory driven by a pulse, whose guards cross inside the cycle's intervals.
//!
//! `y` relaxes to the pulse's rate, three for the first half of the beat and one after it, at `slow` or at `fast` by
//! the `valve`'s mode. The valve turns fast as `y` rises through `up` and slow as it falls through `down`, at times
//! no grid holds. A `gate` opens while `y` lies inside a band, so its two crossings can fall between one interval's
//! stage points, and `z` relaxes to one while it is open and to zero while it is shut. A `seen` flag follows the
//! pulse's rate over two, so only a drive's edge crosses it, and `u` relaxes to one while it is set. A `touch` guard
//! only reaches zero where `y` passes the band's middle, so it grazes. Every value is synthetic; each exact orbit is a
//! composition of exponential maps between switches, with each switch's time found by bisection.

use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_validate::*;

mod dense;

const SECOND: u64 = 1_000_000_000;
const HALF: u64 = SECOND / 2;
const PULSE: &str = "pulse";
const DRAIN: &str = "drain";
const GAUGE: &str = "a synthetic gauge";
const NOTE: &str = "synthetic";
const HIGH: f64 = 3.0;
const LOW: f64 = 1.0;
const SLOW: f64 = 2.0;
const FAST: f64 = 5.0;
const UP: f64 = 2.2;
const DOWN: f64 = 1.8;
const MIDDLE: f64 = 1.95;
const WIDTH: f64 = 0.1;
const OPENING: f64 = 3.0;
const LEVEL: f64 = 2.0;
const WATER: f64 = 10.0;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn count(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("not zero")
}

fn hi(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn at(ns: u64) -> SimTime {
    SimTime::from_ns(ns)
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn name(local: &'static str) -> LawName {
    LawName {
        owner: "Boundary",
        local,
    }
}

fn key(local: &'static str) -> ParameterKey {
    ParameterKey::new(local)
}

const MEAN_Y: AnchorId = AnchorId::new("synthetic.mean-y");

/// The valve's rising threshold: stated, or fitted so that the mark's cycle time mean is `mean`.
#[derive(Clone, Copy)]
enum Up {
    Stated(f64),
    Fitted { guess: f64, mean: f64 },
}

/// Which guards the body holds besides the pulse's routes and `y`.
#[derive(Clone, Copy, Default)]
struct Shape {
    valve: bool,
    gate: bool,
    seen: bool,
    touch: bool,
}

struct Relaxer {
    body: CompiledBody,
    vessel: SiteId,
}

/// How far `y` stands from the band's middle, either way.
fn band<'b, 'k>(
    k: &mut Kernel<'b, 'k, Boundary>,
    y: &StateSlot<'b, Boundary, Rate<Water>>,
    middle: &Parameter<'b, Boundary, Rate<Water>>,
) -> Expr<'b, 'k, Boundary, Rate<Water>> {
    let (now, centre) = (k.state(y), k.parameter(middle));
    let above = k.subtract(now, centre);
    let (centre, now) = (k.parameter(middle), k.state(y));
    let below = k.subtract(centre, now);
    let zero = k.zero();
    k.smooth_max(above, below, zero)
}

/// A flag that relaxes to one in the second of `modes` and to zero in the first.
fn follow<'b>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    flag: &'static str,
    modes: [&Mode<'b, Boundary>; 2],
    one: &Parameter<'b, Boundary, Dimensionless>,
    opening: &Parameter<'b, Boundary, Rate<Dimensionless>>,
    class: RateClass,
) {
    let held = laws.state::<Dimensionless>(flag, Admissible::finite());
    for (mode, set) in [(modes[0], false), (modes[1], true)] {
        let relaxing = laws.kernel(|mut k| {
            let target = if set { k.parameter(one) } else { k.zero() };
            let (current, speed) = (k.state(&held), k.parameter(opening));
            let value = k.relax(target, current, speed);
            k.finish(value, mode.only(), class)
        });
        laws.memory_rate(&held, relaxing);
    }
}

fn relaxer(shape: Shape) -> Result<Relaxer, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let vessel = map.site::<VesselSegment>("vessel", origin());
    map.contain(root.id(), vessel.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.boundary::<Water>(PULSE, BoundaryDirection::IntoBody, vessel.id(), Space::Lumen)?;
    schema.boundary::<Water>(DRAIN, BoundaryDirection::OutOfBody, vessel.id(), Space::Lumen)?;
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let class = RateClass::new(RateBand::Wave, SECOND)?;
        for route in [PULSE, DRAIN] {
            let BoundaryRoute { route, drive } = ports.route::<Water>(route)?;
            let rate = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&route, rate));
        }
        let pulse = ports.route::<Water>(PULSE)?.drive;
        let y = laws.state::<Rate<Water>>("y", Admissible::finite());
        let one = laws.parameter::<Dimensionless>(key("synthetic.one"), Admissible::positive());
        let slow = laws.parameter::<Rate<Dimensionless>>(key("synthetic.slow"), Admissible::positive());
        if shape.valve {
            let fast = laws.parameter::<Rate<Dimensionless>>(key("synthetic.fast"), Admissible::positive());
            let up = laws.parameter::<Rate<Water>>(key("synthetic.up"), Admissible::positive());
            let down = laws.parameter::<Rate<Water>>(key("synthetic.down"), Admissible::positive());
            let scale = laws.parameter::<Dimensionless>(key("synthetic.guard-scale"), Admissible::positive());
            let [slowly, fastly] = laws.modes("valve", ["slow", "fast"]);
            for (mode, speed) in [(&slowly, &slow), (&fastly, &fast)] {
                let relaxing = laws.kernel(|mut k| {
                    let (target, current, speed) = (k.read(&pulse), k.state(&y), k.parameter(speed));
                    let value = k.relax(target, current, speed);
                    k.finish(value, mode.only(), class)
                });
                laws.memory_rate(&y, relaxing);
            }
            let rising = laws.kernel(|mut k| {
                let (now, up, scale) = (k.state(&y), k.parameter(&up), k.parameter(&scale));
                let gap = k.subtract(now, up);
                let gap = k.scale(gap, scale);
                k.finish(gap, slowly.only(), class)
            });
            laws.event(Guard::new(rising, Crossing::Rising, slowly.to(&fastly)?));
            let falling = laws.kernel(|mut k| {
                let (now, down) = (k.state(&y), k.parameter(&down));
                let gap = k.subtract(now, down);
                k.finish(gap, fastly.only(), class)
            });
            laws.event(Guard::new(falling, Crossing::Falling, fastly.to(&slowly)?));
            let mark = laws.algebraic::<Rate<Water>>("mark", Admissible::finite());
            let marking = laws.kernel(|mut k| {
                let (row, up, now) = (k.algebraic(&mark), k.parameter(&up), k.state(&y));
                let sum = k.add(now, up);
                let residual = k.subtract(row, sum);
                k.finish(residual, laws.every_mode(), class)
            });
            laws.residual(&mark, marking);
        } else {
            let relaxing = laws.kernel(|mut k| {
                let (target, current, speed) = (k.read(&pulse), k.state(&y), k.parameter(&slow));
                let value = k.relax(target, current, speed);
                k.finish(value, laws.every_mode(), class)
            });
            laws.memory_rate(&y, relaxing);
        }
        let middle = laws.parameter::<Rate<Water>>(key("synthetic.middle"), Admissible::positive());
        let width = laws.parameter::<Rate<Water>>(key("synthetic.width"), Admissible::positive());
        let opening = laws.parameter::<Rate<Dimensionless>>(key("synthetic.opening"), Admissible::positive());
        if shape.gate {
            let [shut, open] = laws.modes("gate", ["shut", "open"]);
            let inside = laws.kernel(|mut k| {
                let off = band(&mut k, &y, &middle);
                let width = k.parameter(&width);
                let gap = k.subtract(width, off);
                k.finish(gap, shut.only(), class)
            });
            laws.event(Guard::new(inside, Crossing::Rising, shut.to(&open)?));
            let outside = laws.kernel(|mut k| {
                let off = band(&mut k, &y, &middle);
                let width = k.parameter(&width);
                let gap = k.subtract(off, width);
                k.finish(gap, open.only(), class)
            });
            laws.event(Guard::new(outside, Crossing::Rising, open.to(&shut)?));
            follow(&mut laws, "z", [&shut, &open], &one, &opening, class);
        }
        if shape.seen {
            let level = laws.parameter::<Rate<Water>>(key("synthetic.level"), Admissible::positive());
            let [below, above] = laws.modes("seen", ["below", "above"]);
            let over = laws.kernel(|mut k| {
                let (now, level) = (k.read(&pulse), k.parameter(&level));
                let gap = k.subtract(now, level);
                k.finish(gap, below.only(), class)
            });
            laws.event(Guard::new(over, Crossing::Rising, below.to(&above)?));
            let under = laws.kernel(|mut k| {
                let (now, level) = (k.read(&pulse), k.parameter(&level));
                let gap = k.subtract(now, level);
                k.finish(gap, above.only(), class)
            });
            laws.event(Guard::new(under, Crossing::Falling, above.to(&below)?));
            follow(&mut laws, "u", [&below, &above], &one, &opening, class);
        }
        if shape.touch {
            let [apart, met] = laws.modes("touch", ["apart", "met"]);
            let touching = laws.kernel(|mut k| {
                let off = band(&mut k, &y, &middle);
                let zero = k.zero();
                let gap = k.subtract(zero, off);
                k.finish(gap, apart.only(), class)
            });
            laws.event(Guard::new(touching, Crossing::Rising, apart.to(&met)?));
            let leaving = laws.kernel(|mut k| {
                let off = band(&mut k, &y, &middle);
                let width = k.parameter(&width);
                let gap = k.subtract(off, width);
                k.finish(gap, met.only(), class)
            });
            laws.event(Guard::new(leaving, Crossing::Rising, met.to(&apart)?));
        }
        body.add(laws.finish());
        body.compile()
    })?;
    Ok(Relaxer {
        body,
        vessel: vessel.id(),
    })
}

fn water(tag: &[u8], boundary: &'static str, from: u64, to: u64, rate: f64) -> BoundaryEvent {
    let delivery = WaterDelivery {
        boundary,
        amount: measure(rate * (to - from) as f64 / SECOND as f64),
        ends: at(to),
    };
    BoundaryEvent::new(
        InputId::new(Digest::of_canonical(tag)),
        at(from),
        BoundaryInput::Water(delivery),
    )
}

/// The pulse at three for the first half of each beat and one after it, drained as it comes.
fn port(cycles: u64) -> BoundaryPort {
    let mut events = Vec::new();
    for cycle in 0..cycles {
        let start = cycle * SECOND;
        for (boundary, tag) in [(PULSE, b'p'), (DRAIN, b'd')] {
            let first = [tag, b'h', cycle as u8];
            let second = [tag, b'l', cycle as u8];
            events.push(water(&first, boundary, start, start + HALF, HIGH));
            events.push(water(&second, boundary, start + HALF, start + SECOND, LOW));
        }
    }
    BoundaryPort::new(events).expect("parsed")
}

impl Relaxer {
    fn row(&self, kind: RowKind) -> RowId {
        (self.body.rows().iter())
            .find(|row| row.kind() == kind)
            .expect("the body declares it")
            .id()
    }
    fn mark() -> RowKind {
        RowKind::Algebraic(name("mark").into())
    }
    fn memory(local: &'static str) -> RowKind {
        RowKind::Memory(name(local).into())
    }
    fn lumen(&self) -> RowKind {
        RowKind::Amount {
            site: self.vessel,
            space: Space::Lumen,
            species: Species::Water,
        }
    }
    fn lens(&self, intervals: u32) -> ObservationLens {
        let channel = ChannelSpec::new(GAUGE, Self::memory("y")).averaged(count(u64::from(intervals)));
        let grid = count(SECOND / u64::from(intervals));
        self.body.lens(vec![channel], grid).expect("a lens")
    }
    fn packet(&self, intervals: u32, shape: Shape, scale: f64) -> ParameterPacket<SyntheticFixture> {
        self.packet_up(intervals, shape, scale, Up::Stated(UP))
    }
    fn packet_up(
        &self,
        intervals: u32,
        shape: Shape,
        scale: f64,
        up: Up,
    ) -> ParameterPacket<SyntheticFixture> {
        let cycle = Cycle::new(count(SECOND), NonZeroU32::new(intervals).expect("not zero")).expect("even");
        let mut packet = ParameterPacket::periodic(cycle);
        packet.within(admission_tolerance());
        let (banded, following) = (shape.gate || shape.touch, shape.gate || shape.seen);
        let mut rates = vec![("synthetic.slow", SLOW)];
        let mut flows = Vec::new();
        let mut units = Vec::new();
        if banded {
            flows.extend([("synthetic.middle", MIDDLE), ("synthetic.width", WIDTH)]);
        }
        if following {
            rates.push(("synthetic.opening", OPENING));
            units.push(("synthetic.one", 1.0));
        }
        if shape.valve {
            rates.push(("synthetic.fast", FAST));
            flows.push(("synthetic.down", DOWN));
            units.push(("synthetic.guard-scale", scale));
        }
        match up {
            Up::Stated(value) if shape.valve => flows.push(("synthetic.up", value)),
            Up::Stated(_) => {}
            Up::Fitted { guess, mean } => {
                let read = Snapshot::new(
                    Self::mark(),
                    GAUGE,
                    AnchorStatistic::CycleTimeMean,
                    measure::<Rate<Water>>(mean),
                );
                packet.anchor(Synthetic::new(MEAN_Y, read, NOTE)).expect("once");
                let guess = measure::<Rate<Water>>(guess);
                packet
                    .calibrated(key("synthetic.up"), Binding::Body, guess, MEAN_Y)
                    .expect("once");
            }
        }
        if shape.seen {
            flows.push(("synthetic.level", LEVEL));
        }
        for (local, value) in rates {
            let value = measure::<Rate<Dimensionless>>(value);
            packet
                .synthetic(key(local), Binding::Body, value, NOTE)
                .expect("once");
        }
        for (local, value) in flows {
            let value = measure::<Rate<Water>>(value);
            packet
                .synthetic(key(local), Binding::Body, value, NOTE)
                .expect("once");
        }
        for (local, value) in units {
            let value = measure::<Dimensionless>(value);
            packet
                .synthetic(key(local), Binding::Body, value, NOTE)
                .expect("once");
        }
        packet
            .start(self.lumen(), measure::<Water>(WATER), NOTE)
            .expect("once");
        for (on, variable, value) in [
            (shape.valve, "valve", "slow"),
            (shape.gate, "gate", "shut"),
            (shape.seen, "seen", "above"),
            (shape.touch, "touch", "apart"),
        ] {
            if on {
                packet.start_mode(name(variable), value, NOTE).expect("once");
            }
        }
        let guess = |kind| match kind {
            RowKind::Memory(slot) if slot.law == name("y") => 2.0,
            _ => 0.5,
        };
        packet.start_rest(&self.body, guess, NOTE).expect("every row");
        packet
    }
    fn admitted(
        &self,
        intervals: u32,
        shape: Shape,
        scale: f64,
        rows: &[RowKind],
    ) -> Result<Vec<f64>, InitializationFailure> {
        let core = ReferenceIntegrator::new(SECOND / u64::from(intervals));
        let store = MemoryStore::default();
        let lens = self.lens(intervals);
        let packet = self.packet(intervals, shape, scale);
        let admitted = admit_solved(&self.body, packet, port(2), &core, &lens, &store)?;
        let start = admitted.session.record().accepted();
        Ok(rows.iter().map(|&kind| hi(start.value(self.row(kind)))).collect())
    }
}

/// `x` after `time` of relaxing to `target` at `speed`.
fn relaxed(x: f64, target: f64, speed: f64, time: f64) -> f64 {
    target + (x - target) * exp(-speed * time)
}

/// The first time in `[from, to]` at which `crossed` holds, by bisection on a function that holds after it.
fn first(from: f64, to: f64, crossed: impl Fn(f64) -> bool) -> f64 {
    let (mut lo, mut hi) = (from, to);
    for _ in 0..200 {
        let middle = 0.5 * (lo + hi);
        if crossed(middle) { hi = middle } else { lo = middle }
    }
    0.5 * (lo + hi)
}

/// The value at zero that a map of one beat returns to, for a map that is affine in it.
fn periodic(map: impl Fn(f64) -> f64) -> f64 {
    let at_zero = map(0.0);
    at_zero / (1.0 - (map(1.0) - at_zero))
}

/// The valve's continuous orbit: slow from zero until `y` rises through `up`, fast until it falls through `down` in
/// the low half, slow again to the end. Returns `y` at zero and the two switch times.
fn exact_valve() -> (f64, [f64; 2]) {
    let beat = |y0: f64| {
        let rise = first(0.0, 0.5, |t| relaxed(y0, HIGH, SLOW, t) >= UP);
        let half = relaxed(UP, HIGH, FAST, 0.5 - rise);
        let fall = first(0.5, 1.0, |t| relaxed(half, LOW, FAST, t - 0.5) <= DOWN);
        (relaxed(DOWN, LOW, SLOW, 1.0 - fall), [rise, fall])
    };
    let (mut lo, mut hi) = (LOW, UP - 1e-9);
    for _ in 0..200 {
        let middle = 0.5 * (lo + hi);
        if beat(middle).0 > middle {
            lo = middle
        } else {
            hi = middle
        }
    }
    let y0 = 0.5 * (lo + hi);
    (y0, beat(y0).1)
}

/// `y` relaxing at `slow` with no valve, from its periodic start, at `t`.
fn plain_y(t: f64) -> f64 {
    let beat = |y0: f64| relaxed(relaxed(y0, HIGH, SLOW, 0.5), LOW, SLOW, 0.5);
    let y0 = periodic(beat);
    if t <= 0.5 {
        relaxed(y0, HIGH, SLOW, t)
    } else {
        relaxed(relaxed(y0, HIGH, SLOW, 0.5), LOW, SLOW, t - 0.5)
    }
}

/// The four times `y` crosses the band's edges, rising then falling.
fn band_crossings() -> [f64; 4] {
    let inside = |t: f64| (plain_y(t) - MIDDLE).abs() < WIDTH;
    let enter = first(0.0, 0.25, inside);
    let leave = first(enter, 0.5, |t| plain_y(t) >= MIDDLE + WIDTH);
    let back = first(0.5, 0.75, inside);
    let out = first(back, 1.0, |t| plain_y(t) <= MIDDLE - WIDTH);
    [enter, leave, back, out]
}

/// Each halving of the interval must cut the start's distance from the continuous orbit by about eight while both
/// switches stay off the grid: a switch placed on an interval's end leaves a first-order distance however good the
/// stages are. The bounds on the distance itself keep an answer exact by accident from passing on its ratios.
#[test]
fn a_switch_located_inside_its_interval_keeps_the_cycle_third_order() {
    let shape = Shape {
        valve: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let (exact, _) = exact_valve();
    let distance = |intervals: u32| {
        let start = relaxer.admitted(intervals, shape, 1.0, &[Relaxer::memory("y")]);
        (start.unwrap_or_else(|failure| panic!("{intervals}: {failure:?}"))[0] - exact).abs()
    };
    let distances: Vec<f64> = [16, 32, 64].into_iter().map(distance).collect();
    assert!(distances[0] > 1e-6 && distances[2] < 2e-6, "{distances:?}");
    for pair in distances.windows(2) {
        let ratio = pair[0] / pair[1];
        assert!((6.0..10.0).contains(&ratio), "{ratio} in {distances:?}");
    }
}

/// On the first quarter `y` enters the band and leaves it between the interior stage and the end, so the gate's
/// guard is negative at the interval's start, its interior stage and its end, and positive between. A scan of those
/// points sees no crossing; the gate opens and shuts all the same, and `z` keeps what it was open for.
#[test]
fn two_crossings_between_a_legs_stage_points_are_both_found() {
    let shape = Shape {
        gate: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let [enter, leave, back, out] = band_crossings();
    let leg = 0.25;
    assert!(enter > leg / 3.0 && leave < leg, "{enter} {leave}");
    let beat = |z0: f64| {
        let shut = relaxed(z0, 0.0, OPENING, enter);
        let open = relaxed(shut, 1.0, OPENING, leave - enter);
        let shut = relaxed(open, 0.0, OPENING, back - leave);
        let open = relaxed(shut, 1.0, OPENING, out - back);
        relaxed(open, 0.0, OPENING, 1.0 - out)
    };
    let exact = periodic(beat);
    let start = relaxer.admitted(4, shape, 1.0, &[Relaxer::memory("z")]);
    let z = start.unwrap_or_else(|failure| panic!("{failure:?}"))[0];
    assert!((z - exact).abs() < 2e-3, "{z} against {exact}");
}

/// A guard on `y - up` and the same guard times three cross at one time, so their starts agree.
#[test]
fn a_guard_rescaled_by_a_positive_constant_moves_no_start() {
    let shape = Shape {
        valve: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let rows = [Relaxer::memory("y")];
    let one = relaxer.admitted(16, shape, 1.0, &rows).expect("admitted")[0];
    let three = relaxer.admitted(16, shape, 3.0, &rows).expect("admitted")[0];
    assert!((one - three).abs() <= 1e-12 * one.abs(), "{one} against {three}");
}

/// `seen` follows the pulse's rate, which steps up at the wrap and down at the half beat, both on the grid. The guard
/// is crossed by the drive's edge itself, so it fires on that edge, not an interval later, and `u` relaxes to one over
/// exactly the high half.
#[test]
fn a_guard_a_drive_edge_crosses_fires_on_the_edge() {
    let shape = Shape {
        seen: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let beat = |u0: f64| relaxed(relaxed(u0, 1.0, OPENING, 0.5), 0.0, OPENING, 0.5);
    let exact = periodic(beat);
    let start = relaxer.admitted(8, shape, 1.0, &[Relaxer::memory("u")]);
    let u = start.unwrap_or_else(|failure| panic!("{failure:?}"))[0];
    assert!((u - exact).abs() < 1e-3, "{u} against {exact}");
}

/// `touch`'s guard is `y`'s distance from the band's middle negated: it reaches zero only where `y` passes the middle,
/// and turns back there. A crossing with no slope has no time a root row can hold, so the start is refused, not placed.
#[test]
fn a_guard_that_only_touches_zero_is_refused() {
    let shape = Shape {
        touch: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let refused = relaxer.admitted(8, shape, 1.0, &[Relaxer::memory("y")]).err();
    let variable = name("touch").into();
    assert_eq!(refused, Some(InitializationFailure::Unresolved { variable }));
}

/// The beat's time mean of `y` on the valve's orbit, in closed form: each piece between switches relaxes from where the
/// last left it, slow then fast in the high half and fast then slow in the low half.
fn valve_time_mean() -> f64 {
    let (y0, [rise, fall]) = exact_valve();
    let piece = |from: f64, target: f64, speed: f64, time: f64| {
        target * time + (from - target) * (1.0 - exp(-speed * time)) / speed
    };
    let half = relaxed(UP, HIGH, FAST, 0.5 - rise);
    piece(y0, HIGH, SLOW, rise)
        + piece(UP, HIGH, FAST, 0.5 - rise)
        + piece(half, LOW, FAST, fall - 0.5)
        + piece(DOWN, LOW, SLOW, 1.0 - fall)
}

/// Rule 17 on both sides: an anchor's cycle time mean is what a time-mean channel reads, the integral of each accepted
/// step's chord over the window divided by its length, which a core at a fine step brings to the continuous mean. The
/// mean of the same row's grid samples is another number, so a sampled channel cannot stand in for it.
#[test]
fn a_time_mean_channel_integrates_the_accepted_steps_and_a_sampled_one_does_not() {
    let shape = Shape {
        valve: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let intervals = 32;
    let row = Relaxer::memory("y");
    let channels = vec![
        ChannelSpec::new(GAUGE, row).time_averaged(count(SECOND)),
        ChannelSpec::new(GAUGE, row).averaged(count(4)),
    ];
    let lens = relaxer.body.lens(channels, count(SECOND / 4)).expect("a lens");
    let core = ReferenceIntegrator::new(SECOND / 1_000);
    let store = MemoryStore::default();
    let packet = relaxer.packet(intervals, shape, 1.0);
    let admitted = admit_solved(&relaxer.body, packet, port(3), &core, &lens, &store).expect("admitted");
    let mut session = admitted.session;
    let mut observer = ng_observe::Observer::new(&lens, ObservationSeed::new([1, 2, 3, 4]));
    let mut means = [Vec::new(), Vec::new()];
    loop {
        let view = session.observation(&lens).expect("this body's lens");
        for seen in observer.advance(&view).expect("the next revision") {
            if let ng_observe::ObservedValue::Present(value) = seen.value() {
                means[seen.channel()].push((seen.collected(), hi(value)));
            }
        }
        if session.record().accepted().time() >= at(2 * SECOND) {
            break;
        }
        let mut attempt = session.step();
        session = loop {
            attempt = match attempt {
                ng_record::Step::Accepted(session, _) => break session,
                ng_record::Step::Refused(session) => session.retry(),
                ng_record::Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
            };
        };
    }
    let exact = valve_time_mean();
    let timed: Vec<f64> = means[0].iter().map(|&(_, mean)| mean).collect();
    assert_eq!(
        means[0].iter().map(|&(at, _)| at).collect::<Vec<_>>(),
        [at(SECOND), at(2 * SECOND)]
    );
    // The reference core's own error at a millisecond, about 5e-4 here, bounds how near it comes.
    for mean in &timed {
        assert!((mean - exact).abs() < 2e-3, "{mean} against {exact}");
    }
    let sampled = means[1]
        .iter()
        .find(|&&(at, _)| at == at_second())
        .map(|&(_, mean)| mean)
        .expect("a sample mean");
    assert!((sampled - exact).abs() > 1e-2, "{sampled} against {exact}");
}

fn at_second() -> SimTime {
    at(SECOND)
}

impl Relaxer {
    fn system(&self, intervals: u32, packet: ParameterPacket<SyntheticFixture>) -> InitializationSystem<'_> {
        let mut channels = vec![
            ChannelSpec::new(GAUGE, Self::memory("y")).time_averaged(count(SECOND)),
            ChannelSpec::new(GAUGE, Self::memory("y")).averaged(count(u64::from(intervals))),
        ];
        if self.body.rows().iter().any(|row| row.kind() == Self::mark()) {
            channels.push(ChannelSpec::new(GAUGE, Self::mark()).time_averaged(count(SECOND)));
        }
        let lens = self
            .body
            .lens(channels, count(SECOND / u64::from(intervals)))
            .expect("a lens");
        let parameters = self.body.bind(packet).expect("bound");
        let request = InitializationRequest::at(at(0)).reading(&lens);
        self.body
            .initialization(parameters, port(1), request)
            .expect("a system")
    }
}

/// Newton on the topology the workspace holds, stopping where a crossing wants across a knot or nothing moves. Returns
/// whether it converged with no crossing held at a knot.
fn newton(system: &InitializationSystem<'_>, workspace: &mut AdmissionWorkspace) -> bool {
    for _ in 0..16 {
        let residuals = system.evaluate(workspace).expect("every row");
        let matrix = system.linearize(workspace).expect("a matrix");
        let correction = system
            .solve(workspace, &matrix, &residuals)
            .unwrap_or_else(|slot| panic!("a pivot: {slot} with {} crossings", workspace.crossing_count()));
        let still = correction.slots().iter().all(|change| change.abs() < 1e-15);
        if system.apply(workspace, &correction, 1.0) != Applied::Inside {
            return false;
        }
        if still {
            return true;
        }
    }
    false
}

/// Each leg's stages solved again on their own, in time order from the wrap, twice round, each crossing relocated on
/// its leg's curve: what a sweep's split or merged legs need before Newton reads them.
fn reseed(system: &InitializationSystem<'_>, workspace: &mut AdmissionWorkspace) {
    let points = system.points();
    for _ in 0..2 {
        for step in 1..=points {
            let mut at = 0;
            while let Some(&leg) = workspace.legs_of(step % points).get(at) {
                system.settle(workspace, leg).expect("every guard reads");
                system.seed(workspace, leg);
                for _ in 0..8 {
                    let (residuals, block) = system.stage(workspace, leg).expect("a stage");
                    let update = system.solve_stage(&block, &residuals).expect("a pivot");
                    let slots = workspace.leg_slots(leg);
                    for (unknown, change) in workspace.unknowns_mut()[slots].iter_mut().zip(update) {
                        *unknown -= change;
                    }
                }
                if system.relocate(workspace, leg).expect("every guard reads") == Relocated::Settled {
                    at += 1;
                }
            }
        }
    }
}

/// Newton and sweeps by hand, through the public surface alone, until a sweep changes nothing. Returns the workspace
/// and how many sweeps it took.
fn solved_by_hand(system: &InitializationSystem<'_>) -> (AdmissionWorkspace, u32) {
    let mut workspace = system.workspace();
    for sweeps in 1..=16 {
        let converged = newton(system, &mut workspace);
        if !converged {
            reseed(system, &mut workspace);
        }
        if system.sweep(&mut workspace).expect("every guard reads") {
            reseed(system, &mut workspace);
        } else if converged {
            return (workspace, sweeps);
        }
    }
    panic!("no settled topology in 16 sweeps");
}

fn history() -> MethodHistory {
    MethodHistory::new(Digest::of_canonical(b"a schema"), at(0), Vec::new())
}

/// From a workspace with no crossing, Newton and the sweep settle the valve's two crossings in a few rounds, a second
/// sweep changes nothing, and the result is admitted.
#[test]
fn the_sweep_settles_and_a_second_sweep_changes_nothing() {
    let shape = Shape {
        valve: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let system = relaxer.system(16, relaxer.packet(16, shape, 1.0));
    let (mut workspace, sweeps) = solved_by_hand(&system);
    assert!(sweeps <= 6, "{sweeps}");
    assert_eq!(workspace.crossing_count(), 2);
    assert_eq!(system.sweep(&mut workspace), Ok(false));
    let crossings = system.crossings(&workspace);
    let (_, [rise, fall]) = exact_valve();
    for (crossing, exact) in crossings.iter().zip([rise, fall]) {
        let placed = hi(Compensated::of(&crossing.at()));
        assert!((placed - exact).abs() < 1e-4, "{placed} against {exact}");
    }
    assert!(system.admit(workspace, history()).is_ok());
}

/// A leg's rows alone are the rows its `stage` gives, bit for bit, on every leg of a cycle whose crossings split its
/// intervals, so a march that holds a factored block solves against the rows that block was built for. A workspace
/// another system made is refused.
#[test]
fn a_legs_rows_alone_are_the_rows_its_stage_gives() {
    let shape = Shape {
        valve: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let system = relaxer.system(16, relaxer.packet(16, shape, 1.0));
    let (mut workspace, _) = solved_by_hand(&system);
    for (slot, unknown) in workspace.unknowns_mut().iter_mut().enumerate() {
        *unknown *= 1.0 + 1e-4 * ((slot * 7 % 11) as f64 - 5.0) / 5.0;
    }
    let bits = |rows: &[f64]| rows.iter().map(|row| row.to_bits()).collect::<Vec<_>>();
    let mut legs = 0;
    for point in 0..system.points() {
        for leg in workspace.legs_of(point) {
            let (whole, _) = system.stage(&workspace, leg).expect("a stage");
            let alone = system.stage_residuals(&workspace, leg).expect("its rows");
            assert_eq!(bits(&alone), bits(&whole), "point {point}");
            legs += 1;
        }
    }
    assert_eq!(legs, system.points() + workspace.crossing_count());
    let other = relaxer.system(16, relaxer.packet_up(16, shape, 1.0, Up::Stated(UP + 0.05)));
    assert_eq!(
        other.stage_residuals(&workspace, workspace.legs_of(0)[0]),
        Err(InitializationFailure::Admission(AdmissionError::ForeignWorkspace))
    );
}

/// A solved workspace with one crossing moved, the same workspace solved with no sweep, so its topology is not what
/// its values give, and a workspace another system made are each refused.
#[test]
fn a_moved_crossing_a_stale_topology_and_a_foreign_workspace_are_refused() {
    let shape = Shape {
        valve: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let system = || relaxer.system(16, relaxer.packet(16, shape, 1.0));
    let (mut moved, _) = solved_by_hand(&system());
    moved
        .shift_crossing(0, measure::<Time>(1e-4))
        .expect("inside its interval");
    let refused = system().admit(moved, history()).err();
    assert!(
        matches!(refused, Some(InitializationFailure::Inconsistent { .. })),
        "{refused:?}"
    );

    let stale_system = system();
    let mut stale = stale_system.workspace();
    assert!(newton(&stale_system, &mut stale));
    let refused = stale_system.admit(stale, history()).err();
    assert!(
        matches!(refused, Some(InitializationFailure::GuardSweep { .. })),
        "{refused:?}"
    );

    let (solved, _) = solved_by_hand(&system());
    let other = relaxer.system(16, relaxer.packet_up(16, shape, 1.0, Up::Stated(UP + 0.05)));
    let refused = other.admit(solved, history()).err();
    assert_eq!(
        refused,
        Some(InitializationFailure::Admission(AdmissionError::ForeignWorkspace))
    );
}

/// A guard whose threshold is fitted, by the time mean of a mark that reads `y` and the threshold: its root row reads the fit, and each crossing time reads both legs it bounds. The
/// ring's matrix matches central differences in every slot, fit and time, and its update the dense solve.
#[test]
fn a_guard_that_reads_a_fit_has_the_slopes_its_differences_give() {
    let shape = Shape {
        valve: true,
        ..Shape::default()
    };
    let relaxer = relaxer(shape).expect("a body");
    let up = Up::Fitted {
        guess: 2.15,
        mean: UP + 1.88,
    };
    for intervals in [8, 16] {
        let system = relaxer.system(intervals, relaxer.packet_up(intervals, shape, 1.0, up));
        let (mut workspace, _) = solved_by_hand(&system);
        assert_eq!(workspace.crossing_count(), 2, "{intervals}");
        for (slot, unknown) in workspace.unknowns_mut().iter_mut().enumerate() {
            *unknown *= 1.0 + 1e-4 * ((slot * 7 % 11) as f64 - 5.0) / 5.0;
        }
        dense::cross_check(&system, &mut workspace, &format!("{intervals} intervals"));
    }
}
