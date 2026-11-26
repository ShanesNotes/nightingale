use crate::{ReplayCase, admit_fixture};
use core::f64::consts::TAU;
use ng_contract::*;
use ng_record::{HistoryStore, Running, Session};

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const NANOSECONDS_PER_SECOND: f64 = 1e9;

const PERIOD_SECONDS: f64 = 60.0;
const GROWTH_PER_SECOND: f64 = 0.01;
const PULL_PER_SECOND: f64 = 0.01;
const START_AMPLITUDE: f64 = 0.5;
const AMPLITUDE_TOLERANCE: f64 = 0.1;
const PASSING_REFERENCE_STEP_NS: u64 = 500 * MILLISECOND;
const END_SECONDS: u64 = 1_200;
const WATER_START: f64 = 500_000.0;
const WATER_EXCHANGE_PER_SECOND: f64 = 1.0;

const OWNER: &str = "CV";
const X: &str = "oscillation-x";
const Y: &str = "oscillation-y";

fn at_origin() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn named(local: &'static str) -> RowKind {
    RowKind::Memory(Slot {
        law: LawName { owner: OWNER, local },
        at: Context::Body,
    })
}

/// Two memory rows `x` and `y` on a closed two-vessel water loop:
/// `x' = (g - p (x^2 + y^2)) x + w y`, `y' = (g - p (x^2 + y^2)) y - w x`, with `w = 2 pi / T`,
/// driving water exchange `q = c x` between the two lumens so total water closes identically.
/// Its amplitude `r = sqrt(x^2 + y^2)` obeys `r' = (g - p r^2) r` with closed form
/// `sqrt(g / (p + (g / r_0^2 - p) exp(-2 g t)))` toward limit-cycle amplitude `sqrt(g / p) = 1`.
///
/// The halved backward Euler step damps the oscillation at roughly `w^2 h / 4` per second, so a
/// core with step past `4 g / w^2` (about 3.65 s here) decays the oscillation to zero while every
/// step passes `verify` and water closes. Holding the steady-state amplitude within
/// `tolerance = 0.1` of `1` allows damping up to `(2 * 0.1 - 0.1^2) g = 0.19 g` (`0.0019 / s`),
/// giving a necessary steady-state bound of roughly `0.76 g / w^2` (0.693 s at period 60 s).
/// Because the gate also judges the growth transient from `r_0 = 0.5`, the measured passing
/// cutoff for halved/doubled backward Euler sits between `600 ms` (passes, worst error `0.09635`)
/// and `625 ms` (fails at `156.25 s`, error `0.100007`). `ReferenceIntegrator` passes at `0.5 s`
/// (damping `0.00137 / s`, steady amplitude `0.9289`, worst amplitude error `0.0799`).
pub struct Oscillation {
    body: CompiledBody,
    period: f64,
    growth: f64,
    pull: f64,
    start: f64,
    water_start: f64,
    water_exchange: f64,
    tolerance: f64,
    end: SimTime,
}
impl Oscillation {
    pub fn self_sustained() -> Result<Self, AdmissionError> {
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", at_origin()?);
        let a = map.site::<VesselSegment>("a", at_origin()?);
        let b = map.site::<VesselSegment>("b", at_origin()?);
        map.contain(root.id(), a.id());
        map.contain(root.id(), b.id());
        map.connect(Network::Vascular, a.id(), b.id());
        let mut schema = Schema::new(map.finish()?)?;
        schema.partial_volume(Measure::new(1.0)?)?;
        let body = assemble(schema, |mut body| {
            let (mut laws, ports) = body.cardiovascular()?.parts();
            let route = ports.circulation(&mut laws);
            let x = laws.state::<Dimensionless>(X, Admissible::finite());
            let y = laws.state::<Dimensionless>(Y, Admissible::finite());
            let growth = laws.parameter::<Rate<Dimensionless>>(
                ParameterKey::new("cv.oscillation-growth"),
                Admissible::nonnegative(),
            );
            let turn = laws.parameter::<Rate<Dimensionless>>(
                ParameterKey::new("cv.oscillation-turn"),
                Admissible::nonnegative(),
            );
            let pull = laws.parameter::<Rate<Dimensionless>>(
                ParameterKey::new("cv.oscillation-pull"),
                Admissible::nonnegative(),
            );
            let exchange = laws.parameter::<Rate<Water>>(
                ParameterKey::new("cv.oscillation-exchange"),
                Admissible::nonnegative(),
            );
            let class = RateClass::new(RateBand::Minutes, u64::MAX)?;
            for (own, other, clockwise) in [(&x, &y, false), (&y, &x, true)] {
                let rate = laws.kernel(|mut k| {
                    let own = k.state(own);
                    let other = k.state(other);
                    let own_square = k.scale(own, own);
                    let other_square = k.scale(other, other);
                    let squares = k.add(own_square, other_square);
                    let growth = k.parameter(&growth);
                    let pull = k.parameter(&pull);
                    let loss = k.scale(pull, squares);
                    let net = k.subtract(growth, loss);
                    let radial = k.scale(net, own);
                    let turn = k.parameter(&turn);
                    let swing = k.scale(turn, other);
                    let rate = if clockwise {
                        k.subtract(radial, swing)
                    } else {
                        k.add(radial, swing)
                    };
                    k.finish(rate, laws.every_mode(), class)
                });
                laws.memory_rate(own, rate);
            }
            let flow = laws.edge_kernel(|mut k| {
                let amplitude = k.state(&x);
                let scale = k.parameter(&exchange);
                let flow = k.scale(scale, amplitude);
                k.finish(flow, laws.every_mode(), class)
            });
            laws.transfer_along(&route, flow);
            body.add(laws.finish());
            body.compile()
        })?;
        Ok(Self {
            body,
            period: PERIOD_SECONDS,
            growth: GROWTH_PER_SECOND,
            pull: PULL_PER_SECOND,
            start: START_AMPLITUDE,
            water_start: WATER_START,
            water_exchange: WATER_EXCHANGE_PER_SECOND,
            tolerance: AMPLITUDE_TOLERANCE,
            end: SimTime::from_ns(END_SECONDS * SECOND),
        })
    }
    pub fn until(self, end: SimTime) -> Self {
        Self { end, ..self }
    }
    pub fn period(&self) -> Measure<Time> {
        Measure::new(self.period).expect("a positive period")
    }
    pub fn growth(&self) -> Measure<Rate<Dimensionless>> {
        Measure::new(self.growth).expect("a positive growth rate")
    }
    pub fn amplitude(&self) -> Measure<Dimensionless> {
        Measure::new((self.growth / self.pull).sqrt()).expect("a finite limit-cycle amplitude")
    }
    pub fn tolerance(&self) -> Measure<Dimensionless> {
        Measure::new(self.tolerance).expect("a positive tolerance")
    }
    /// `4 * growth / w^2`: the halved-step length past which backward Euler damps the oscillation to zero.
    pub fn damping_step_ns(&self) -> u64 {
        let turn = TAU / self.period;
        (4.0 * self.growth / (turn * turn) * NANOSECONDS_PER_SECOND) as u64
    }
    /// The step at which `ReferenceIntegrator` passes this case (`0.5 s`, damping `0.00137 / s`).
    pub fn reference_step_ns(&self) -> u64 {
        PASSING_REFERENCE_STEP_NS
    }
    pub fn rows(&self) -> [RowId; 2] {
        let find = |local| {
            let kind = named(local);
            (self.body.rows().iter())
                .find(|row| row.kind() == kind)
                .expect("the body declares both oscillation rows")
                .id()
        };
        [find(X), find(Y)]
    }
    pub fn exact_amplitude(&self, at: SimTime) -> Measure<Dimensionless> {
        Measure::new(self.exact_amplitude_f64(at)).expect("the closed form of finite constants is finite")
    }
    pub(crate) fn exact_amplitude_f64(&self, at: SimTime) -> f64 {
        let seconds = at.nanoseconds() as f64 / NANOSECONDS_PER_SECOND;
        let decay = exp(-2.0 * self.growth * seconds);
        let denominator = self.pull + (self.growth / (self.start * self.start) - self.pull) * decay;
        (self.growth / denominator).sqrt()
    }
    pub(crate) fn tolerance_compensated(&self) -> Compensated {
        Compensated::of(&self.tolerance())
    }
    pub fn packet(&self) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        let mut packet = ParameterPacket::new();
        let note = "synthetic";
        let turn = TAU / self.period;
        for (key, value) in [
            ("cv.oscillation-growth", self.growth),
            ("cv.oscillation-turn", turn),
            ("cv.oscillation-pull", self.pull),
        ] {
            packet.synthetic(
                ParameterKey::new(key),
                Binding::Body,
                Measure::<Rate<Dimensionless>>::new(value)?,
                note,
            )?;
        }
        packet.synthetic(
            ParameterKey::new("cv.oscillation-exchange"),
            Binding::Body,
            Measure::<Rate<Water>>::new(self.water_exchange)?,
            note,
        )?;
        let x_kind = named(X);
        packet.start_rest(
            &self.body,
            |row| match row {
                RowKind::Amount { .. } => self.water_start,
                kind if kind == x_kind => self.start,
                _ => 0.0,
            },
            note,
        )?;
        Ok(packet)
    }
    pub fn digest(&self) -> Digest {
        let Self {
            body,
            period,
            growth,
            pull,
            start,
            water_start,
            water_exchange,
            tolerance,
            end,
        } = self;
        let mut bytes = body.model_hash().digest().as_bytes().to_vec();
        for value in [
            period,
            growth,
            pull,
            start,
            water_start,
            water_exchange,
            tolerance,
        ] {
            bytes.extend_from_slice(&value.to_bits().to_be_bytes());
        }
        bytes.extend_from_slice(&end.nanoseconds().to_be_bytes());
        Digest::of_canonical(&bytes)
    }
}

impl ReplayCase for Oscillation {
    fn body(&self) -> &CompiledBody {
        &self.body
    }
    fn end(&self) -> SimTime {
        self.end
    }
    fn events(&self) -> Result<Vec<BoundaryEvent>, AdmissionError> {
        Ok(Vec::new())
    }
    fn admit<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
        store: &'m dyn HistoryStore,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        let boundary = BoundaryPort::new(Vec::new())?;
        admit_fixture(&self.body, self.packet()?, boundary, core, store)
    }
    fn admit_withheld<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
        store: &'m dyn HistoryStore,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        self.admit(core, store)
    }
}
