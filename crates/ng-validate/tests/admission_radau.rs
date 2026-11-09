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
    fn lens(&self, intervals: u32) -> ObservationLens {
        let channel = ChannelSpec::new(GAUGE, self.rate()).averaged(count(u64::from(intervals)));
        let grid = count(SECOND / u64::from(intervals));
        self.body.lens(vec![channel], grid).expect("a lens")
    }
    /// `targets` are the drive's in systole and in diastole.
    fn packet(&self, intervals: u32, targets: [f64; 2]) -> ParameterPacket<SyntheticFixture> {
        let cycle = Cycle::new(count(SECOND), NonZeroU32::new(intervals).expect("not zero")).expect("even");
        let mut packet = ParameterPacket::periodic(cycle);
        packet.within(admission_tolerance());
        let unit = |local, value| (key(local), measure::<Dimensionless>(value));
        for (key, value) in [
            unit("synthetic.split", 0.25),
            unit("synthetic.end", 1.0),
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
        let read = Snapshot::new(
            self.rate(),
            GAUGE,
            AnchorStatistic::CycleMean,
            measure::<Rate<Dimensionless>>(1.0),
        );
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
    /// A system on `intervals` whose modes a ramp of the phase has swept round the ring: the end of systole at a
    /// quarter, the wrap at the last point and its reset into point 0's interval.
    fn swept(&self, intervals: u32, lens: &ObservationLens) -> (InitializationSystem<'_>, SolveWorkspace) {
        let parameters = self.body.bind(self.packet(intervals, [1.0, 1.0])).expect("bound");
        let request = InitializationRequest::at(SimTime::from_ns(0)).reading(lens);
        let boundary = BoundaryPort::new(Vec::new()).expect("no event");
        let mut system = self
            .body
            .initialization(parameters, boundary, request)
            .expect("a system");
        let mut workspace = system.workspace();
        let phase = self.row(RowKind::Memory(name("phase").into())).index();
        let points = system.points();
        for point in 0..points {
            let ramp = (point + 1) as f64 / points as f64;
            workspace.unknowns_mut()[system.endpoint(point).start + phase] = ramp;
            workspace.unknowns_mut()[system.interior(point).start + phase] = ramp - 0.5 / points as f64;
        }
        for _ in 0..2 * points {
            if system.sweep(&workspace, 0..points).expect("every guard reads") == 0 {
                break;
            }
        }
        (system, workspace)
    }
    /// The admitted start's value of each of `rows`.
    fn start(
        &self,
        intervals: u32,
        targets: [f64; 2],
        rows: &[RowKind],
    ) -> Result<Vec<f64>, InitializationFailure> {
        let core = ReferenceIntegrator::new(SECOND / u64::from(intervals));
        let store = MemoryStore::default();
        let lens = self.lens(intervals);
        let boundary = BoundaryPort::new(Vec::new())?;
        let packet = self.packet(intervals, targets);
        let admitted = admit_solved(&self.body, packet, boundary, &core, &lens, &store)?;
        let start = admitted.session.record().accepted();
        Ok(rows.iter().map(|&kind| hi(start.value(self.row(kind)))).collect())
    }
}

/// Work enters only in systole, so the power the closed store sheds steps with the mode, and the store's two stages
/// must each give it: the wall row pins the store at both, and only the store's own row reaches the power.
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
        assert!((power - WORK).abs() < 1e-9 * WORK, "{intervals}: {power}");
        assert!((held - WALL).abs() < 1e-9 * WALL, "{intervals}: {held}");
    }
}

/// A drive that relaxes inside the beat makes the rate vary. A backward Euler ring advanced the phase by the interval
/// times the rate at each point, so the anchor's mean of those points closed the phase on its wrap, and main admits
/// this clock. Radau's interval advances it by the stage weights' integral, which no sampled mean pins, so the wrap the
/// guard needs once a beat falls off the grid and no start stands. The arena's rate barely moves within a beat, and it
/// admits; a clock like this one needs the cycle's time mean on both sides of rule 17.
#[test]
fn a_varying_clock_anchored_by_its_sampled_mean_finds_no_start() {
    let pacemaker = pacemaker().expect("a body");
    let rows = [RowKind::Memory(name("phase").into())];
    assert!(pacemaker.start(8, [1.0, 1.0], &rows).is_ok());
    assert!(pacemaker.start(8, [1.5, 1.0], &rows).is_err());
}

/// A reset phase, a closed store, a regulated total read through a law on the pool, and a fit, at every slot off the
/// guess.
#[test]
fn the_lifted_ring_is_the_dense_solve_of_the_differenced_rows_through_a_reset() {
    let pacemaker = pacemaker().expect("a body");
    for intervals in [2, 4] {
        let lens = pacemaker.lens(intervals);
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
    let lens = pacemaker.lens(intervals);
    let (system, mut workspace) = pacemaker.swept(intervals, &lens);
    let held = workspace.unknowns_mut().to_vec();
    let rows = [Space::Elastic, Space::Thermal].map(|space| pacemaker.row(pacemaker.store(space)).index());
    let thermal = rows[1];
    let summed = |workspace: &SolveWorkspace| -> f64 {
        let residuals = system.evaluate(workspace).expect("every row");
        let ends = (0..system.points()).flat_map(|point| rows.map(|row| system.endpoint(point).start + row));
        ends.map(|slot| residuals[slot]).sum()
    };
    let before = summed(&workspace);
    let h = 1.0 / f64::from(intervals);
    for (stage, weight) in [(0, 0.75), (1, 0.25)] {
        let mut moved = system.workspace();
        moved.unknowns_mut().copy_from_slice(&held);
        let mut heat = 0.0;
        for point in 0..system.points() {
            let slots = [system.interior(point), system.endpoint(point)];
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
