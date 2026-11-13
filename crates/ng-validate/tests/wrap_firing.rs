//! A paced pacemaker whose last leg ends in systole past both its thresholds. At the wrap the clock is forced and the
//! systole guard is crossed, so the clock must fire there and reset the phase, whatever order compile gives the two
//! guards. Compile sorts guards by a digest of their layout, which names the unknowns they read, so a row that reads
//! nothing and is read by nothing reorders them.

use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const GAUGE: &str = "a synthetic gauge";
const NOTE: &str = "synthetic";
const HEAT: &str = "heat";
const RATE: AnchorId = AnchorId::new("synthetic.rate");
const INTERVALS: u32 = 4;
const PAST_THE_WRAP: f64 = 1.25;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn count(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("not zero")
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

/// The arena heart's two rhythm guards on a clock whose rate reads a held pool, so its anchor rests on an amount.
/// Each name in `inert` is a memory row that reads nothing and that nothing reads.
fn pacemaker(inert: &[&'static str]) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", Position::new(measure(0.0), measure(0.0), measure(0.0)));
    let mut schema = Schema::new(map.finish()?)?;
    schema.pool::<Energy>(root.id(), Space::Thermal)?;
    schema.boundary::<Energy>(HEAT, BoundaryDirection::OutOfBody, root.id(), Space::Thermal)?;
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let heat = ports.route::<Energy>(HEAT)?.route;
        let wave = RateClass::new(RateBand::Wave, SECOND)?;
        let [systole, diastole] = laws.modes("rhythm", ["systole", "diastole"]);
        let phase = laws.state::<Dimensionless>("phase", Admissible::finite());
        let rate = laws.algebraic::<Rate<Dimensionless>>("rate", Admissible::positive());
        let gain = laws.parameter::<Rate<Dimensionless>>(key("synthetic.gain"), Admissible::positive());
        let scale = laws.parameter::<Energy>(key("synthetic.scale"), Admissible::positive());
        let split = laws.parameter::<Dimensionless>(key("synthetic.split"), Admissible::positive());
        let end = laws.parameter::<Dimensionless>(key("synthetic.end"), Admissible::positive());
        let held = laws.kernel(|mut k| {
            let value = k.zero::<Rate<Energy>>();
            k.finish(value, laws.every_mode(), wave)
        });
        laws.transfer(Transfer::new(&heat, held));
        let paced = laws.kernel(|mut k| {
            let (row, gain) = (k.algebraic(&rate), k.parameter(&gain));
            let (held, scale) = (k.read(&heat.amount::<Energy>()), k.parameter(&scale));
            let share = k.divide(held, scale);
            let share = k.as_dimension::<_, Dimensionless>(share);
            let value = k.scale(gain, share);
            let residual = k.subtract(row, value);
            k.finish(residual, laws.every_mode(), wave)
        });
        laws.residual(&rate, paced);
        let running = laws.kernel(|mut k| {
            let value = k.algebraic(&rate);
            k.finish(value, laws.every_mode(), wave)
        });
        laws.memory_rate(&phase, running);
        for &local in inert {
            let still = laws.state::<Dimensionless>(local, Admissible::finite());
            let none = laws.kernel(|mut k| {
                let value = k.zero::<Rate<Dimensionless>>();
                k.finish(value, laws.every_mode(), wave)
            });
            laws.memory_rate(&still, none);
        }
        let past_split = laws.kernel(|mut k| {
            let (now, split) = (k.state(&phase), k.parameter(&split));
            let gap = k.subtract(now, split);
            k.finish(gap, systole.only(), wave)
        });
        laws.event(Guard::new(past_split, Crossing::Rising, systole.to(&diastole)?));
        let past_end = laws.kernel(|mut k| {
            let (now, end) = (k.state(&phase), k.parameter(&end));
            let gap = k.subtract(now, end);
            k.finish(gap, diastole.only(), wave)
        });
        laws.event(Guard::new(past_end, Crossing::Rising, diastole.to(&systole)?).resetting(&phase));
        body.add(laws.finish());
        body.compile()
    })
}

fn packet(body: &CompiledBody) -> ParameterPacket<SyntheticFixture> {
    let cycle = Cycle::new(count(SECOND), NonZeroU32::new(INTERVALS).expect("not zero")).expect("even");
    let mut packet = ParameterPacket::periodic(cycle.paced(name("phase").into(), RATE));
    packet.within(admission_tolerance());
    for (local, value) in [("synthetic.split", 0.25), ("synthetic.end", 1.0)] {
        packet
            .synthetic(key(local), Binding::Body, measure::<Dimensionless>(value), NOTE)
            .expect("once");
    }
    packet
        .synthetic(
            key("synthetic.scale"),
            Binding::Body,
            measure::<Energy>(1.0),
            NOTE,
        )
        .expect("once");
    let rate = RowKind::Algebraic(name("rate").into());
    let read = Snapshot::new(
        rate,
        GAUGE,
        AnchorStatistic::CycleTimeMean,
        measure::<Rate<Dimensionless>>(1.0),
    );
    packet.anchor(Synthetic::new(RATE, read, NOTE)).expect("once");
    packet
        .calibrated(
            key("synthetic.gain"),
            Binding::Body,
            measure::<Rate<Dimensionless>>(1.0),
            RATE,
        )
        .expect("once");
    packet.start_mode(name("rhythm"), "systole", NOTE).expect("once");
    let guess = |kind| match kind {
        RowKind::Memory(_) => 0.0,
        _ => 1.0,
    };
    packet.start_rest(body, guess, NOTE).expect("every row");
    packet
}

/// The first leg's phase origin after the wrap's knot is read again, less what its stages add over the leg, which no
/// guard order changes: the origin is zero where the clock fired and reset it, and the last leg's end where it did not.
fn phase_after_the_wrap(inert: &[&'static str]) -> f64 {
    let body = pacemaker(inert).expect("a body");
    let channel =
        ChannelSpec::new(GAUGE, RowKind::Algebraic(name("rate").into())).time_averaged(count(SECOND));
    let lens = body
        .lens(vec![channel], count(SECOND / u64::from(INTERVALS)))
        .expect("a lens");
    let parameters = body.bind(packet(&body)).expect("bound");
    let request = InitializationRequest::at(SimTime::from_ns(0)).reading(&lens);
    let boundary = BoundaryPort::new(Vec::new()).expect("no event");
    let system = body
        .initialization(parameters, boundary, request)
        .expect("a system");
    let phase = (body.rows().iter())
        .position(|row| row.kind() == RowKind::Memory(name("phase").into()))
        .expect("a phase row");
    let mut workspace = system.workspace();
    let points = system.points();
    for point in 0..points {
        let end = if point == 0 {
            PAST_THE_WRAP
        } else {
            PAST_THE_WRAP * point as f64 / points as f64
        };
        let slot = workspace.endpoint(point).start + phase;
        workspace.unknowns_mut()[slot] = end;
        let slot = workspace.interior(point).start + phase;
        workspace.unknowns_mut()[slot] = end - PAST_THE_WRAP / (1.5 * points as f64);
    }
    let first = workspace.legs_of(1)[0];
    system
        .settle(&mut workspace, first)
        .expect("the wrap's guards read");
    let slot = workspace.endpoint(1).start + phase;
    let state = workspace.unknowns_mut()[slot];
    let residuals = system.evaluate(&workspace).expect("every law has a value");
    state - residuals[slot]
}

/// The leg advances the phase by its length at a rate of one, a quarter, so a reset origin reads 0.25 and a dropped
/// clock reads 1.25 more.
#[test]
fn the_clock_fires_at_the_wrap_whatever_the_guard_order() {
    let variants: [&[&'static str]; 5] = [
        &[],
        &["a0"],
        &["a0", "a1"],
        &["a0", "a1", "a2"],
        &["a0", "a1", "a2", "a3"],
    ];
    let read: Vec<f64> = variants.iter().map(|inert| phase_after_the_wrap(inert)).collect();
    assert!(
        read.iter().all(|&origin| origin < PAST_THE_WRAP),
        "phase after the wrap with 0 to 4 inert rows: {read:?}"
    );
}
