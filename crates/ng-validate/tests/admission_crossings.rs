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
            flows.extend([("synthetic.up", UP), ("synthetic.down", DOWN)]);
            units.push(("synthetic.guard-scale", scale));
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

/// `x` after `span` of relaxing to `target` at `speed`.
fn relaxed(x: f64, target: f64, speed: f64, span: f64) -> f64 {
    target + (x - target) * exp(-speed * span)
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
    let refused = relaxer.admitted(8, shape, 1.0, &[Relaxer::memory("y")]);
    assert!(refused.is_err(), "{refused:?}");
}
