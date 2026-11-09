//! A synthetic pacemaker beside a closed store, for what a cycle's interval stage must hold whatever its order.
//!
//! A `phase` runs at `rate = gain * drive` and wraps at one, as the arena's heart does. `drive` relaxes to `3` at
//! speed `2` through the first quarter of the phase and to `1` at speed `5` after it, or holds still when both
//! targets are one. An anchor sets the cycle mean of `rate`, sampled at the grid's points, to one beat a period, and
//! `gain` is fitted to it. The rate also reads the store over its wall, which the closure holds at one, so the
//! anchor rests on an amount. Beside the clock an elastic store is held to a wall by the power it sheds into a thermal
//! store: the closed row has no slope of its own, so only the store's stage reaches the power, an index-2 pair. Work
//! enters the store in the first quarter only, and heat leaves the thermal store at `speed * held^2 / scale`, so energy
//! answers its own flows, and not in proportion: a linear loss under one speed in every mode would give both stages the
//! same sum round the ring, and no check could tell their weights apart.

use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_validate::*;

mod dense;

const SECOND: u64 = 1_000_000_000;
const HEAT: &str = "heat";
const GAUGE: &str = "a synthetic gauge";
const NOTE: &str = "synthetic";
const WORK: f64 = 6.0;
const WALL: f64 = 2.0;
const HEAT_SPEED: f64 = 0.5;
const HEAT_SCALE: f64 = 1.0;
const RATE: AnchorId = AnchorId::new("synthetic.rate");

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

fn key(local: &'static str) -> ParameterKey {
    ParameterKey::new(local)
}

/// How the packet times the beat: a paced cycle whose rate anchor is a time mean, the same read as grid samples, or
/// grid samples with no clock named.
#[derive(Clone, Copy)]
enum Clock {
    Timed,
    Sampled,
    Unpaced,
}

struct Pacemaker {
    body: CompiledBody,
    root: SiteId,
}

fn pacemaker() -> Result<Pacemaker, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let mut schema = Schema::new(map.finish()?)?;
    schema.pool::<Energy>(root.id(), Space::Elastic)?;
    schema.pool::<Energy>(root.id(), Space::Thermal)?;
    schema.boundary::<Energy>(HEAT, BoundaryDirection::OutOfBody, root.id(), Space::Thermal)?;
    let body = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let wave = RateClass::new(RateBand::Wave, SECOND)?;
        let [systole, diastole] = cv.modes("rhythm", ["systole", "diastole"]);
        let phase = cv.state::<Dimensionless>("phase", Admissible::finite());
        let drive = cv.state::<Dimensionless>("drive", Admissible::positive());
        let rate = cv.algebraic::<Rate<Dimensionless>>("rate", Admissible::positive());
        let gain = cv.parameter::<Rate<Dimensionless>>(key("synthetic.gain"), Admissible::positive());
        let split = cv.parameter::<Dimensionless>(key("synthetic.split"), Admissible::positive());
        let end = cv.parameter::<Dimensionless>(key("synthetic.end"), Admissible::positive());
        let targets = ["synthetic.systole-target", "synthetic.diastole-target"]
            .map(|local| cv.parameter::<Dimensionless>(key(local), Admissible::positive()));
        let speeds = ["synthetic.systole-speed", "synthetic.diastole-speed"]
            .map(|local| cv.parameter::<Rate<Dimensionless>>(key(local), Admissible::positive()));
        let store = ports.dissipation(&mut cv);
        let wall = cv.parameter::<Energy>(key("synthetic.wall"), Admissible::positive());
        let paced = cv.kernel(|mut k| {
            let (row, gain, drive) = (k.algebraic(&rate), k.parameter(&gain), k.state(&drive));
            let (held, wall) = (k.read(&store.amount::<Energy>()), k.parameter(&wall));
            let share = k.divide(held, wall);
            let share = k.as_dimension::<_, Dimensionless>(share);
            let value = k.scale(gain, drive);
            let value = k.scale(value, share);
            let residual = k.subtract(row, value);
            k.finish(residual, cv.every_mode(), wave)
        });
        cv.residual(&rate, paced);
        let running = cv.kernel(|mut k| {
            let value = k.algebraic(&rate);
            k.finish(value, cv.every_mode(), wave)
        });
        cv.memory_rate(&phase, running);
        for ((mode, target), speed) in [&systole, &diastole].into_iter().zip(&targets).zip(&speeds) {
            let relaxing = cv.kernel(|mut k| {
                let (target, current, speed) = (k.parameter(target), k.state(&drive), k.parameter(speed));
                let value = k.relax(target, current, speed);
                k.finish(value, mode.only(), wave)
            });
            cv.memory_rate(&drive, relaxing);
        }
        let past_split = cv.kernel(|mut k| {
            let (now, split) = (k.state(&phase), k.parameter(&split));
            let gap = k.subtract(now, split);
            k.finish(gap, systole.only(), wave)
        });
        cv.event(Guard::new(past_split, Crossing::Rising, systole.to(&diastole)?));
        let past_end = cv.kernel(|mut k| {
            let (now, end) = (k.state(&phase), k.parameter(&end));
            let gap = k.subtract(now, end);
            k.finish(gap, diastole.only(), wave)
        });
        cv.event(Guard::new(past_end, Crossing::Rising, diastole.to(&systole)?).resetting(&phase));

        let power = cv.algebraic::<Rate<Energy>>("power", Admissible::finite());
        let closing = cv.kernel(|mut k| {
            let (held, wall) = (k.read(&store.amount::<Energy>()), k.parameter(&wall));
            let gap = k.subtract(held, wall);
            k.finish(gap, cv.every_mode(), wave)
        });
        cv.closes(&power, closing);
        let shedding = cv.kernel(|mut k| {
            let value = k.algebraic(&power);
            k.finish(value, cv.every_mode(), wave)
        });
        cv.transfer(Transfer::new(&store, shedding));
        let work = cv.parameter::<Rate<Energy>>(key("synthetic.work"), Admissible::positive());
        let paid = ports.wall_work(&mut cv);
        let working = cv.kernel(|mut k| {
            let value = k.parameter(&work);
            k.finish(value, systole.only(), wave)
        });
        cv.transfer(Transfer::new(&paid, working));
        let resting = cv.kernel(|mut k| {
            let value = k.zero::<Rate<Energy>>();
            k.finish(value, diastole.only(), wave)
        });
        cv.transfer(Transfer::new(&paid, resting));
        body.add(cv.finish());

        let (mut laws, ports) = body.boundary()?.parts();
        let heat = ports.route::<Energy>(HEAT)?.route;
        let loss = laws.parameter::<Rate<Dimensionless>>(key("synthetic.heat-speed"), Admissible::positive());
        let scale = laws.parameter::<Energy>(key("synthetic.heat-scale"), Admissible::positive());
        let leaving = laws.kernel(|mut k| {
            let (held, zero, pace) = (k.read(&heat.amount::<Energy>()), k.zero(), k.parameter(&loss));
            let value = k.relax(held, zero, pace);
            let (held, scale) = (k.read(&heat.amount::<Energy>()), k.parameter(&scale));
            let share = k.divide(held, scale);
            let share = k.as_dimension::<_, Dimensionless>(share);
            let value = k.scale(value, share);
            k.finish(value, laws.every_mode(), wave)
        });
        laws.transfer(Transfer::new(&heat, leaving));
        body.add(laws.finish());
        body.compile()
    })?;
    Ok(Pacemaker {
        body,
        root: root.id(),
    })
}

impl Pacemaker {
    fn store(&self, space: Space) -> RowKind {
        RowKind::Amount {
            site: self.root,
            space,
            species: Species::Energy,
        }
    }
    fn row(&self, kind: RowKind) -> RowId {
        (self.body.rows().iter())
            .find(|row| row.kind() == kind)
            .expect("the body declares it")
            .id()
    }
    fn rate(&self) -> RowKind {
        RowKind::Algebraic(name("rate").into())
    }
    /// The rate read as its time mean over each beat, or as the mean of the grid's samples.
    fn lens(&self, intervals: u32, clock: Clock) -> ObservationLens {
        let channel = ChannelSpec::new(GAUGE, self.rate());
        let channel = match clock {
            Clock::Timed => channel.time_averaged(count(SECOND)),
            Clock::Sampled | Clock::Unpaced => channel.averaged(count(u64::from(intervals))),
        };
        let grid = count(SECOND / u64::from(intervals));
        self.body.lens(vec![channel], grid).expect("a lens")
    }
    /// `targets` are the drive's in systole and in diastole; `end` is where the phase wraps.
    fn packet(
        &self,
        intervals: u32,
        targets: [f64; 2],
        clock: Clock,
        end: f64,
    ) -> ParameterPacket<SyntheticFixture> {
        let cycle = Cycle::new(count(SECOND), NonZeroU32::new(intervals).expect("not zero")).expect("even");
        let cycle = match clock {
            Clock::Unpaced => cycle,
            Clock::Timed | Clock::Sampled => cycle.paced(name("phase").into(), RATE),
        };
        let mut packet = ParameterPacket::periodic(cycle);
        packet.within(admission_tolerance());
        let unit = |local, value| (key(local), measure::<Dimensionless>(value));
        for (key, value) in [
            unit("synthetic.split", 0.25),
            unit("synthetic.end", end),
            unit("synthetic.systole-target", targets[0]),
            unit("synthetic.diastole-target", targets[1]),
        ] {
            packet.synthetic(key, Binding::Body, value, NOTE).expect("once");
        }
        for (local, value) in [
            ("synthetic.systole-speed", 2.0),
            ("synthetic.diastole-speed", 5.0),
            ("synthetic.heat-speed", HEAT_SPEED),
        ] {
            let value = measure::<Rate<Dimensionless>>(value);
            packet
                .synthetic(key(local), Binding::Body, value, NOTE)
                .expect("once");
        }
        let heat_scale = measure::<Energy>(HEAT_SCALE);
        packet
            .synthetic(key("synthetic.heat-scale"), Binding::Body, heat_scale, NOTE)
            .expect("once");
        let wall = measure::<Energy>(WALL);
        packet
            .synthetic(key("synthetic.wall"), Binding::Body, wall, NOTE)
            .expect("once");
        let work = measure::<Rate<Energy>>(WORK);
        packet
            .synthetic(key("synthetic.work"), Binding::Body, work, NOTE)
            .expect("once");
        let statistic = match clock {
            Clock::Timed => AnchorStatistic::CycleTimeMean,
            Clock::Sampled | Clock::Unpaced => AnchorStatistic::CycleMean,
        };
        let read = Snapshot::new(self.rate(), GAUGE, statistic, measure::<Rate<Dimensionless>>(1.0));
        packet.anchor(Synthetic::new(RATE, read, NOTE)).expect("once");
        let guess = measure::<Rate<Dimensionless>>(1.0);
        packet
            .calibrated(key("synthetic.gain"), Binding::Body, guess, RATE)
            .expect("once");
        let stores = [(Space::Elastic, WALL), (Space::Thermal, 2.0)];
        for (space, value) in stores {
            packet
                .start(self.store(space), measure::<Energy>(value), NOTE)
                .expect("once");
        }
        packet.start_mode(name("rhythm"), "systole", NOTE).expect("once");
        let guess = |kind| match kind {
            RowKind::Memory(slot) if slot.law == name("drive") => targets[1],
            RowKind::Memory(_) => 0.0,
            _ => 1.0,
        };
        packet.start_rest(&self.body, guess, NOTE).expect("every row");
        packet
    }
    fn system(&self, intervals: u32, lens: &ObservationLens) -> InitializationSystem<'_> {
        let packet = self.packet(intervals, [1.0, 1.0], Clock::Timed, 1.0);
        let parameters = self.body.bind(packet).expect("bound");
        let request = InitializationRequest::at(SimTime::from_ns(0)).reading(lens);
        let boundary = BoundaryPort::new(Vec::new()).expect("no event");
        self.body
            .initialization(parameters, boundary, request)
            .expect("a system")
    }
    /// A system on `intervals` whose modes and crossings a ramp of the phase has swept round the ring: the end of
    /// systole at a quarter, inside an interval or on a knot, and the wrap at the cycle's end.
    fn swept(
        &self,
        intervals: u32,
        lens: &ObservationLens,
    ) -> (InitializationSystem<'_>, AdmissionWorkspace) {
        let system = self.system(intervals, lens);
        let mut workspace = system.workspace();
        let phase = self.row(RowKind::Memory(name("phase").into())).index();
        let points = system.points();
        for point in 0..points {
            let end = if point == 0 {
                1.0
            } else {
                point as f64 / points as f64
            };
            let slot = workspace.endpoint(point).start + phase;
            workspace.unknowns_mut()[slot] = end;
            let slot = workspace.interior(point).start + phase;
            workspace.unknowns_mut()[slot] = end - 2.0 / (3.0 * points as f64);
        }
        for _ in 0..2 * points {
            if !system.sweep(&mut workspace).expect("every guard reads") {
                break;
            }
        }
        (system, workspace)
    }
    /// The admitted start's value of each of `rows`, and the crossings the cycle located.
    fn admitted(
        &self,
        intervals: u32,
        targets: [f64; 2],
        clock: Clock,
        end: f64,
        rows: &[RowKind],
    ) -> Result<(Vec<f64>, Vec<LocatedCrossing>), InitializationFailure> {
        let core = ReferenceIntegrator::new(SECOND / u64::from(intervals));
        let store = MemoryStore::default();
        let lens = self.lens(intervals, clock);
        let boundary = BoundaryPort::new(Vec::new())?;
        let packet = self.packet(intervals, targets, clock, end);
        let admitted = admit_solved(&self.body, packet, boundary, &core, &lens, &store)?;
        let start = admitted.session.record().accepted();
        let values = rows.iter().map(|&kind| hi(start.value(self.row(kind)))).collect();
        Ok((values, admitted.provenance.crossings().to_vec()))
    }
    fn start(
        &self,
        intervals: u32,
        targets: [f64; 2],
        rows: &[RowKind],
    ) -> Result<Vec<f64>, InitializationFailure> {
        Ok(self.admitted(intervals, targets, Clock::Timed, 1.0, rows)?.0)
    }
}

/// Work enters only in systole, so the power the closed store sheds steps with the mode, and the store's two stages
/// must each give it: the wall row pins the store at both, and only the store's own row reaches the power. The start
/// is the state just after the wrap fires systole, and a firing solves no algebraic row again, so the power there is
/// the last of diastole's: none.
#[test]
fn a_closed_store_sheds_its_work_at_the_start_on_any_ring() {
    let pacemaker = pacemaker().expect("a body");
    let rows = [
        RowKind::Algebraic(name("power").into()),
        pacemaker.store(Space::Elastic),
    ];
    for intervals in [4, 8] {
        let start = pacemaker.start(intervals, [1.0, 1.0], &rows);
        let [power, held] = start.unwrap_or_else(|failure| panic!("{intervals}: {failure:?}"))[..] else {
            panic!("two rows");
        };
        assert!(power.abs() < 1e-9 * WORK, "{intervals}: {power}");
        assert!((held - WALL).abs() < 1e-9 * WALL, "{intervals}: {held}");
    }
}

/// A drive that relaxes inside the beat makes the rate vary. Radau's leg advances the phase by the stage weights'
/// integral of the rate, which no sampled mean pins, so a clock paced by a sampled anchor is refused before anything
/// solves; the same clock paced by its time mean closes its phase on the wrap and stands.
#[test]
fn a_varying_clock_is_paced_by_its_time_mean_and_refused_by_a_sampled_one() {
    let pacemaker = pacemaker().expect("a body");
    let rows = [RowKind::Memory(name("phase").into())];
    let sampled = pacemaker
        .admitted(8, [1.5, 1.0], Clock::Sampled, 1.0, &rows)
        .err();
    let refused = InitializationFailure::Admission(AdmissionError::PeriodAnchor { anchor: RATE });
    assert_eq!(sampled, Some(refused));
    let (phase, _) = pacemaker
        .admitted(8, [1.5, 1.0], Clock::Timed, 1.0, &rows)
        .expect("a start");
    assert_eq!(phase, [0.0]);
}

/// The clock wraps once a period: an anchor of one beat a second over a one-second cycle reaches a threshold of one,
/// and a threshold the period does not reach is refused by the anchor that claims it.
#[test]
fn a_clock_whose_anchor_does_not_reach_its_threshold_in_one_period_is_refused() {
    let pacemaker = pacemaker().expect("a body");
    let rows = [RowKind::Memory(name("phase").into())];
    let refused = pacemaker.admitted(8, [1.0, 1.0], Clock::Timed, 0.9, &rows).err();
    let mismatch = InitializationFailure::Admission(AdmissionError::PeriodMismatch { anchor: RATE });
    assert_eq!(refused, Some(mismatch));
}

/// With no clock named and no drive that moves, the beat may start anywhere along its orbit: only where the grid's
/// samples fall would place it, which is no physiology. The start is refused, not placed.
#[test]
fn an_autonomous_beat_no_clock_paces_is_refused() {
    let pacemaker = pacemaker().expect("a body");
    let rows = [RowKind::Memory(name("phase").into())];
    let refused = pacemaker
        .admitted(8, [1.5, 1.0], Clock::Unpaced, 1.0, &rows)
        .err();
    assert_eq!(refused, Some(InitializationFailure::Unpaced));
}

/// The paced phase runs at one beat a second, so systole ends at a quarter of the beat. On five intervals that is
/// inside the second, where admission locates it; the wrap is the cycle's own end and adds no crossing.
#[test]
fn the_end_of_systole_is_located_where_the_phase_reaches_it() {
    let pacemaker = pacemaker().expect("a body");
    let rows = [RowKind::Memory(name("phase").into())];
    let (phase, crossings) = pacemaker
        .admitted(5, [1.0, 1.0], Clock::Timed, 1.0, &rows)
        .expect("a start");
    assert_eq!(phase, [0.0]);
    let [crossing] = crossings[..] else {
        panic!("{crossings:?}");
    };
    assert_eq!((crossing.leaves, crossing.enters), ("systole", "diastole"));
    let at = hi(Compensated::of(&crossing.at()));
    assert!((at - 0.25).abs() < 1e-9, "{at}");
}

/// A reset phase, a closed store, a regulated total read through a law on the pool, and a fit, at every slot off the
/// guess.
#[test]
fn the_lifted_ring_is_the_dense_solve_of_the_differenced_rows_through_a_reset() {
    let pacemaker = pacemaker().expect("a body");
    for intervals in [2, 4] {
        let lens = pacemaker.lens(intervals, Clock::Timed);
        let (system, mut workspace) = pacemaker.swept(intervals, &lens);
        for (slot, unknown) in workspace.unknowns_mut().iter_mut().enumerate() {
            *unknown += 1e-3 * ((slot * 7 % 11) as f64 - 5.0);
        }
        dense::cross_check(&system, &mut workspace, &format!("{intervals} intervals"));
    }
}

/// Energy's end rows telescope round the ring, so their sum is the cycle's stage-weighted boundary flow. Moving the
/// thermal store by one at every interior stage alone, or at every end alone, moves the sum by each interval times that
/// stage's weight times the change in the heat leaving there.
#[test]
fn energys_end_rows_sum_to_its_stage_weighted_boundary_flow() {
    let pacemaker = pacemaker().expect("a body");
    let intervals = 4;
    let lens = pacemaker.lens(intervals, Clock::Timed);
    let (system, mut workspace) = pacemaker.swept(intervals, &lens);
    let held = workspace.unknowns_mut().to_vec();
    let rows = [Space::Elastic, Space::Thermal].map(|space| pacemaker.row(pacemaker.store(space)).index());
    let thermal = rows[1];
    let summed = |workspace: &AdmissionWorkspace| -> f64 {
        let residuals = system.evaluate(workspace).expect("every row");
        let ends =
            (0..system.points()).flat_map(|point| rows.map(|row| workspace.endpoint(point).start + row));
        ends.map(|slot| residuals[slot]).sum()
    };
    let before = summed(&workspace);
    let h = 1.0 / f64::from(intervals);
    for (stage, weight) in [(0, 0.75), (1, 0.25)] {
        let mut moved = workspace.clone();
        moved.unknowns_mut().copy_from_slice(&held);
        let mut heat = 0.0;
        for point in 0..system.points() {
            let slots = [moved.interior(point), moved.endpoint(point)];
            let slot = slots[stage].start + thermal;
            let was = held[slot];
            moved.unknowns_mut()[slot] = was + 1.0;
            heat += h * weight * HEAT_SPEED * ((was + 1.0) * (was + 1.0) - was * was) / HEAT_SCALE;
        }
        let change = summed(&moved) - before;
        assert!(
            (change - heat).abs() < 1e-12 * (1.0 + heat.abs()),
            "{stage}: {change} against {heat}"
        );
    }
}

/// Systole ends at a quarter of the beat: on eight intervals its guard reaches zero exactly on a knot, rising, and fires
/// there; on five it crosses inside an interval. Work enters only in systole, so the heat the thermal store balances
/// reads how long systole lasted, and both grids hold it to their own error. A guard left to the next knot would hold
/// systole an eighth of a beat too long, and the heat a fifth too high.
#[test]
fn a_guard_that_reaches_zero_on_a_knot_fires_on_that_knot() {
    let pacemaker = pacemaker().expect("a body");
    let rows = [pacemaker.store(Space::Thermal)];
    let thermal = |intervals| {
        let (held, crossings) = pacemaker
            .admitted(intervals, [1.0, 1.0], Clock::Timed, 1.0, &rows)
            .unwrap_or_else(|failure| panic!("{intervals}: {failure:?}"));
        (held[0], crossings.len())
    };
    let ((on_knot, none), (inside, one)) = (thermal(8), thermal(5));
    assert_eq!((none, one), (0, 1));
    assert!(
        (on_knot - inside).abs() < 1e-2 * inside,
        "{on_knot} against {inside}"
    );
}
