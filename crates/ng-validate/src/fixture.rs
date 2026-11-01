use crate::ReplayCase;
use ng_contract::*;
use ng_record::{Running, Session};

const SECOND: u64 = 1_000_000_000;
const NANOSECONDS_PER_SECOND: f64 = 1e9;

const RELAXATION_SECONDS: u64 = 1000;
const CONDUCTANCE: f64 = 0.001;
const START: [f64; 2] = [500_500.0, 499_500.0];
const EXTERIOR: &str = "exterior";

fn seconds(ns: u64) -> f64 {
    ns as f64 / NANOSECONDS_PER_SECOND
}

/// `amount` of water delivered evenly over `[from, to]`.
pub struct Delivery {
    pub id: &'static [u8],
    pub from: SimTime,
    pub to: SimTime,
    pub amount: Measure<Water>,
}
impl Delivery {
    fn rate(&self) -> f64 {
        let Self { from, to, amount, .. } = self;
        number(amount) / seconds(to.nanoseconds() - from.nanoseconds())
    }
}

fn number<U>(value: &Measure<U>) -> f64 {
    f64::from_bits(Compensated::of(value).bits()[0])
}

fn at_origin() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

pub(crate) struct Exact {
    pub(crate) amounts: [f64; 2],
    pub(crate) circulated: f64,
    pub(crate) received: f64,
}

/// Two vessel segments `a -> b` joined by one conductance, with at most one water boundary, at `a`. With
/// `k = g e v`, `q = k (a - b)`, `a' = -q + u` and `b' = q`, so `s = a + b` and `d = a - b` have a closed
/// form on every constant stretch of `u`.
pub struct TwoReservoirs {
    body: CompiledBody,
    a: SiteId,
    b: SiteId,
    direction: Option<BoundaryDirection>,
    start: [f64; 2],
    deliveries: Vec<Delivery>,
    end: SimTime,
}
impl TwoReservoirs {
    /// `1 / k` is 1000 s; start `(500500, 499500)`; 20,000 units delivered over 2,000 to 6,000 s; the case
    /// ends at 12,000 s, long after the difference has settled again.
    pub fn stepped_inflow() -> Result<Self, AdmissionError> {
        let delivery = Delivery {
            id: b"stepped inflow",
            from: SimTime::from_ns(2_000 * SECOND),
            to: SimTime::from_ns(6_000 * SECOND),
            amount: Measure::new(20_000.0)?,
        };
        let mut case = Self::declared(Some(BoundaryDirection::IntoBody), None, START, vec![delivery])?;
        case.end = SimTime::from_ns(12_000 * SECOND);
        Ok(case)
    }
    /// No boundary at all: inventory cannot move, so drift is the ledger's own rounding.
    pub fn closed() -> Result<Self, AdmissionError> {
        Self::declared(None, None, START, Vec::new())
    }
    /// The open body with no delivery, the circulation law trusted only inside `range`.
    pub fn trusted_within(range: ValidityRange<Rate<Water>>) -> Result<Self, AdmissionError> {
        Self::declared(Some(BoundaryDirection::IntoBody), Some(range), START, Vec::new())
    }
    /// The open body from `start`, with these deliveries into `a`.
    pub fn delivering(start: [Measure<Water>; 2], deliveries: Vec<Delivery>) -> Result<Self, AdmissionError> {
        let start = start.map(|amount| number(&amount));
        Self::declared(Some(BoundaryDirection::IntoBody), None, start, deliveries)
    }
    /// The same body with the boundary running out of `a`: these deliveries leave it.
    pub fn draining(start: [Measure<Water>; 2], deliveries: Vec<Delivery>) -> Result<Self, AdmissionError> {
        let start = start.map(|amount| number(&amount));
        Self::declared(Some(BoundaryDirection::OutOfBody), None, start, deliveries)
    }
    fn declared(
        boundary: Option<BoundaryDirection>,
        trusted: Option<ValidityRange<Rate<Water>>>,
        start: [f64; 2],
        deliveries: Vec<Delivery>,
    ) -> Result<Self, AdmissionError> {
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", at_origin()?);
        let a = map.site::<VesselSegment>("a", at_origin()?);
        let b = map.site::<VesselSegment>("b", at_origin()?);
        map.contain(root.id(), a.id());
        map.contain(root.id(), b.id());
        map.connect(Network::Vascular, a.id(), b.id());
        let mut schema = Schema::new(map.finish()?)?;
        if let Some(direction) = boundary {
            schema.water_boundary(EXTERIOR, direction, a.id(), Space::Lumen)?;
        }
        schema.partial_volume(Measure::new(1.0)?)?;
        let body = assemble(schema, |mut body| {
            let (mut laws, ports) = body.cardiovascular()?.parts();
            let route = ports.circulation(&mut laws);
            let e = laws.end_parameter::<Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
            let g = laws
                .edge_parameter::<Conductance<Water>>(ParameterKey::new("cv.g"), Admissible::nonnegative());
            let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
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
                k.finish(flow, laws.every_mode(), wave)
            });
            if let Some(range) = trusted {
                laws.validity(&flow, range);
            }
            laws.transfer_along(&route, flow);
            body.add(laws.finish());
            if boundary.is_some() {
                let (mut laws, ports) = body.boundary()?.parts();
                let BoundaryWater { route: pair, drive } = ports.water(EXTERIOR)?;
                let rate = laws.kernel(|mut k| {
                    let value = k.read(&drive);
                    k.finish(value, laws.every_mode(), wave)
                });
                laws.transfer(Transfer::new(&pair, rate));
                body.add(laws.finish());
            }
            body.compile()
        })?;
        Ok(Self {
            body,
            a: a.id(),
            b: b.id(),
            direction: boundary,
            start,
            deliveries,
            end: SimTime::from_ns(48 * 3600 * SECOND),
        })
    }
    pub fn until(self, end: SimTime) -> Self {
        Self { end, ..self }
    }
    /// `1 / k`. A forward Euler step past this makes the difference between the reservoirs grow.
    pub fn relaxation_ns(&self) -> u64 {
        RELAXATION_SECONDS * SECOND
    }
    /// The instant of the first delivery, and that event's id.
    pub fn delivery(&self) -> Option<(SimTime, InputId)> {
        let delivery = self.deliveries.first()?;
        Some((delivery.from, input_of(delivery.id)))
    }
    /// Both amount rows, `a` first.
    pub fn rows(&self) -> [RowId; 2] {
        let row = |site| {
            let lumen = RowKind::Amount {
                site,
                space: Space::Lumen,
            };
            (self.body.rows().iter())
                .find(|row| row.kind() == lumen)
                .expect("the body declares both lumens")
                .id()
        };
        [row(self.a), row(self.b)]
    }
    fn column(&self, context: Context) -> Option<ColumnId> {
        (self.body.instances().iter())
            .filter(|instance| instance.context() == context)
            .find_map(|instance| match instance.target() {
                Target::Column(column) => Some(column),
                Target::Row(_) => None,
            })
    }
    pub(crate) fn circulation(&self) -> Option<ColumnId> {
        self.column(Context::Edge {
            from: self.a,
            to: self.b,
        })
    }
    /// The boundary's column; `None` for a closed body.
    pub fn exterior(&self) -> Option<ColumnId> {
        self.column(Context::Body)
    }
    fn packet(&self, conductance: f64) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        let default = Binding::Network(Network::Vascular);
        let mut packet = ParameterPacket::new();
        let (e, g) = (ParameterKey::new("cv.e"), ParameterKey::new("cv.g"));
        packet.synthetic(e, default, Measure::<Elastance>::new(1.0)?, "synthetic")?;
        packet.synthetic(
            g,
            default,
            Measure::<Conductance<Water>>::new(conductance)?,
            "synthetic",
        )?;
        for (site, amount) in [self.a, self.b].into_iter().zip(self.start) {
            let row = RowKind::Amount {
                site,
                space: Space::Lumen,
            };
            packet.start(row, Measure::<Water>::new(amount)?, "synthetic")?;
        }
        Ok(packet)
    }
    /// The same packet with a doubled conductance: what a core reads if it evaluates on other values.
    pub fn other_values(&self) -> Result<BoundParameters, AdmissionError> {
        self.body.bind(self.packet(2.0 * CONDUCTANCE)?)
    }
    /// Everything that makes this case this case, so two cases on one body never share an identity.
    pub fn digest(&self) -> Digest {
        let Self {
            body,
            a: _,
            b: _,
            direction: _,
            start,
            deliveries,
            end,
        } = self;
        let mut bytes = body.model_hash().digest().as_bytes().to_vec();
        for amount in start {
            bytes.extend_from_slice(&amount.to_bits().to_be_bytes());
        }
        bytes.extend_from_slice(&end.nanoseconds().to_be_bytes());
        for Delivery { id, from, to, amount } in deliveries {
            bytes.extend_from_slice(&id.len().to_be_bytes());
            bytes.extend_from_slice(id);
            bytes.extend_from_slice(&from.nanoseconds().to_be_bytes());
            bytes.extend_from_slice(&to.nanoseconds().to_be_bytes());
            bytes.extend_from_slice(&number(amount).to_bits().to_be_bytes());
        }
        Digest::of_canonical(&bytes)
    }
    pub(crate) fn exact(&self, at: SimTime) -> Exact {
        let k = CONDUCTANCE;
        let (mut sum, mut difference) = (self.start[0] + self.start[1], self.start[0] - self.start[1]);
        let (mut circulated, mut received) = (0.0, 0.0);
        let sign = match self.direction {
            Some(BoundaryDirection::OutOfBody) => -1.0,
            _ => 1.0,
        };
        let mut applied: Vec<&Delivery> = Vec::with_capacity(self.deliveries.len());
        for delivery in &self.deliveries {
            if !applied.iter().any(|known| known.id == delivery.id) {
                applied.push(delivery);
            }
        }
        let mut edges: Vec<SimTime> = (applied.iter())
            .flat_map(|delivery| [delivery.from, delivery.to])
            .collect();
        edges.push(at);
        edges.sort_unstable();
        let mut reached = SimTime::from_ns(0);
        for edge in edges.into_iter().filter(|edge| *edge <= at) {
            if edge <= reached {
                continue;
            }
            let span = seconds(edge.nanoseconds() - reached.nanoseconds());
            let gross: f64 = (applied.iter())
                .filter(|delivery| delivery.from <= reached && reached < delivery.to)
                .map(|delivery| delivery.rate())
                .sum();
            let inflow = sign * gross;
            let decay = exp(-2.0 * k * span);
            let fading = (1.0 - decay) / (2.0 * k);
            circulated += k * (difference * fading + inflow / (2.0 * k) * (span - fading));
            sum += inflow * span;
            difference = difference * decay + inflow * fading;
            received += gross * span;
            reached = edge;
        }
        Exact {
            amounts: [(sum + difference) / 2.0, (sum - difference) / 2.0],
            circulated,
            received,
        }
    }
}

impl ReplayCase for TwoReservoirs {
    fn body(&self) -> &CompiledBody {
        &self.body
    }
    fn end(&self) -> SimTime {
        self.end
    }
    fn events(&self) -> Result<Vec<BoundaryEvent>, AdmissionError> {
        events_of(&self.deliveries)
    }
    fn admit<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        let boundary = BoundaryPort::new(self.events()?)?;
        admit_fixture(&self.body, self.packet(CONDUCTANCE)?, boundary, core)
    }
    /// The timeline is not part of the run identity, so this is the same run.
    fn admit_withheld<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        let boundary = BoundaryPort::new(Vec::new())?;
        admit_fixture(&self.body, self.packet(CONDUCTANCE)?, boundary, core)
    }
}

fn input_of(name: &[u8]) -> InputId {
    InputId::new(Digest::of_canonical(name))
}

fn events_of(deliveries: &[Delivery]) -> Result<Vec<BoundaryEvent>, AdmissionError> {
    let mut events = Vec::with_capacity(deliveries.len());
    for delivery in deliveries {
        let water = WaterDelivery {
            boundary: EXTERIOR,
            amount: Measure::new(number(&delivery.amount))?,
            ends: delivery.to,
        };
        events.push(BoundaryEvent::new(
            input_of(delivery.id),
            delivery.from,
            BoundaryInput::Water(water),
        ));
    }
    Ok(events)
}

const KNOT_PERIOD_SECONDS: f64 = 10.0;
const HAZARD_AMPLITUDE: f64 = 0.5;
const ECTOPY_PER_SECOND: f64 = 0.05;
const RETURN_PER_SECOND: f64 = 0.2;
const BYSTANDER_PER_SECOND: f64 = 1e-6;
const OWNER: &str = "CV";
const RHYTHM: &str = "rhythm";
const BYSTANDER: &str = "bystander";
const TONE: &str = "tone";
const ONSET_INTEGRAL: &str = "onset-hazard";
const RETURN_INTEGRAL: &str = "return-hazard";
const SECOND_INTEGRAL: &str = "second-hazard";
const THIRD_INTEGRAL: &str = "third-hazard";

fn named(local: &'static str) -> LawName {
    LawName { owner: OWNER, local }
}

fn hazard_rate<'b>(
    laws: &FragmentBuilder<'b, Cardiovascular>,
    pace: &Parameter<'b, Cardiovascular, Rate<Dimensionless>>,
    tone: &StateSlot<'b, Cardiovascular, Dimensionless>,
    wobble: &Forcing<'b, Cardiovascular>,
    holds: ModeSet<'b, Cardiovascular>,
    class: RateClass,
) -> PureKernel<'b, Cardiovascular, Rate<Dimensionless>> {
    laws.kernel(|mut k| {
        let pace = k.parameter(pace);
        let tone = k.state(tone);
        let toned = k.scale(pace, tone);
        let varied = k.forcing(wobble);
        let rate = k.scale(toned, varied);
        k.finish(rate, holds, class)
    })
}

fn relaxing_rate<'b>(
    laws: &FragmentBuilder<'b, Cardiovascular>,
    pace: &Parameter<'b, Cardiovascular, Rate<Dimensionless>>,
    integral: &StateSlot<'b, Cardiovascular, Dimensionless>,
    tone: &StateSlot<'b, Cardiovascular, Dimensionless>,
    wobble: &Forcing<'b, Cardiovascular>,
    holds: ModeSet<'b, Cardiovascular>,
    class: RateClass,
) -> PureKernel<'b, Cardiovascular, Rate<Dimensionless>> {
    laws.kernel(|mut k| {
        let tone = k.state(tone);
        let varied = k.forcing(wobble);
        let target = k.scale(tone, varied);
        let current = k.state(integral);
        let speed = k.parameter(pace);
        let rate = k.relax(target, current, speed);
        k.finish(rate, holds, class)
    })
}

fn flow<'b>(
    laws: &FragmentBuilder<'b, Cardiovascular>,
    route: &EdgeRoute<'b, Cardiovascular, Water>,
    e: &EndParameter<'b, Cardiovascular, Elastance>,
    g: &EdgeParameter<'b, Cardiovascular, Conductance<Water>>,
    holds: ModeSet<'b, Cardiovascular>,
    class: RateClass,
) -> PureKernel<'b, Cardiovascular, Rate<Water>, VascularEdge> {
    laws.edge_kernel(|mut k| {
        let upstream = k.at_edge(&route.from_amount());
        let downstream = k.at_edge(&route.to_amount());
        let upstream = k.volume(upstream);
        let downstream = k.volume(downstream);
        let upstream_elastance = k.upstream_parameter(e);
        let downstream_elastance = k.downstream_parameter(e);
        let upstream = k.multiply(upstream_elastance, upstream);
        let downstream = k.multiply(downstream_elastance, downstream);
        let upstream = k.as_dimension::<_, Pressure>(upstream);
        let downstream = k.as_dimension::<_, Pressure>(downstream);
        let head = k.subtract(upstream, downstream);
        let conductance = k.edge_parameter(g);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, holds, class)
    })
}

/// A forcing varies the hazard rates, which read a memory row, so a core can reach a rate only through a row
/// the scales hold. It carries a delivery, so the late-admission gate has an input to withhold.
pub struct ChanceCase {
    body: CompiledBody,
    a: SiteId,
    b: SiteId,
    deliveries: Vec<Delivery>,
    seed: Seed,
    variant: Variant,
    amplitude: f64,
    end: SimTime,
}
#[derive(Clone, Copy, PartialEq)]
enum Variant {
    Plain,
    Bystander,
    Racing,
    Relaxing,
    TwoRhythms,
    Echoing,
}
impl ChanceCase {
    pub fn ectopy() -> Result<Self, AdmissionError> {
        Self::declared(Variant::Plain)
    }
    /// The same case plus one more stream, on a hazard and a mode variable no other law reads, too slow to fire
    /// inside the case, so no bit of the first streams' events may move.
    pub fn ectopy_and_bystander() -> Result<Self, AdmissionError> {
        Self::declared(Variant::Bystander)
    }
    /// A second hazard leaves sinus for ectopic too, so two guards can cross in one landing and race for one mode
    /// variable. Its rate holds in every mode, so it accrues while its guard is not armed.
    pub fn racing() -> Result<Self, AdmissionError> {
        Self::declared(Variant::Racing)
    }
    /// A second hazard leaves ectopic for sinus. Its rate holds in every mode, so it can stand past its draw when
    /// the first guard's firing arms it, and it sorts after that guard, so the two meet inside one walk.
    pub fn echoing() -> Result<Self, AdmissionError> {
        Self::declared(Variant::Echoing)
    }
    /// The walk's relaxing law on the ectopy rate, which goes negative.
    pub fn relaxing() -> Result<Self, AdmissionError> {
        Self::declared(Variant::Relaxing)
    }
    pub fn relaxing_within(range: ValidityRange<Rate<Dimensionless>>) -> Result<Self, AdmissionError> {
        Self::declared_within(Variant::Relaxing, Some(range))
    }
    /// A second rhythm of its own, on its own mode variable and streams, so two events can fall a floor apart and
    /// neither has anything to do with the other.
    pub fn two_rhythms() -> Result<Self, AdmissionError> {
        Self::declared(Variant::TwoRhythms)
    }
    fn declared(variant: Variant) -> Result<Self, AdmissionError> {
        Self::declared_within(variant, None)
    }
    fn declared_within(
        variant: Variant,
        trusted: Option<ValidityRange<Rate<Dimensionless>>>,
    ) -> Result<Self, AdmissionError> {
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", at_origin()?);
        let a = map.site::<VesselSegment>("a", at_origin()?);
        let b = map.site::<VesselSegment>("b", at_origin()?);
        map.contain(root.id(), a.id());
        map.contain(root.id(), b.id());
        map.connect(Network::Vascular, a.id(), b.id());
        let mut schema = Schema::new(map.finish()?)?;
        schema.water_boundary(EXTERIOR, BoundaryDirection::IntoBody, a.id(), Space::Lumen)?;
        schema.partial_volume(Measure::new(1.0)?)?;
        let body = assemble(schema, |mut body| {
            let (mut laws, ports) = body.cardiovascular()?.parts();
            let route = ports.circulation(&mut laws);
            let e = laws.end_parameter::<Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
            let g = laws
                .edge_parameter::<Conductance<Water>>(ParameterKey::new("cv.g"), Admissible::nonnegative());
            let slowed = laws.edge_parameter::<Conductance<Water>>(
                ParameterKey::new("cv.g-ectopic"),
                Admissible::nonnegative(),
            );
            let target = laws.parameter(ParameterKey::new("cv.tone-target"), Admissible::finite());
            let settle = laws.parameter(ParameterKey::new("cv.tone-rate"), Admissible::nonnegative());
            let tone = laws.state::<Dimensionless>(TONE, Admissible::finite());
            let onset = laws.parameter(ParameterKey::new("cv.ectopy-rate"), Admissible::nonnegative());
            let recovery = laws.parameter(ParameterKey::new("cv.return-rate"), Admissible::nonnegative());
            let dice = laws.stream("ectopic-beat");
            let back = laws.stream("sinus-return");
            let wobble = laws.forcing(
                "hazard-variability",
                ParameterKey::new("cv.knot-period"),
                ParameterKey::new("cv.hazard-amplitude"),
            );
            let onset_hazard = laws.state::<Dimensionless>(ONSET_INTEGRAL, Admissible::nonnegative());
            let return_hazard = laws.state::<Dimensionless>(RETURN_INTEGRAL, Admissible::nonnegative());
            let [sinus, ectopic] = laws.modes(RHYTHM, ["sinus", "ectopic"]);
            let (wave, beat) = (
                RateClass::new(RateBand::Wave, 1_000_000)?,
                RateClass::new(RateBand::Beat, 100_000_000)?,
            );
            laws.transfer_along(&route, flow(&laws, &route, &e, &g, sinus.only(), wave));
            laws.transfer_along(&route, flow(&laws, &route, &e, &slowed, ectopic.only(), wave));
            let settling = laws.kernel(|mut k| {
                let target = k.parameter(&target);
                let current = k.state(&tone);
                let speed = k.parameter(&settle);
                let rate = k.relax(target, current, speed);
                k.finish(rate, laws.every_mode(), beat)
            });
            laws.memory_rate(&tone, settling);
            let onset_rate = if variant == Variant::Relaxing {
                relaxing_rate(&laws, &onset, &onset_hazard, &tone, &wobble, sinus.only(), beat)
            } else {
                hazard_rate(&laws, &onset, &tone, &wobble, sinus.only(), beat)
            };
            if let Some(range) = trusted {
                laws.validity(&onset_rate, range);
            }
            let still = |holds| {
                laws.kernel(|mut k| {
                    let rate = k.zero::<Rate<Dimensionless>>();
                    k.finish(rate, holds, beat)
                })
            };
            let (idle_onset, idle_return) = (still(ectopic.only()), still(sinus.only()));
            let return_rate = hazard_rate(&laws, &recovery, &tone, &wobble, ectopic.only(), beat);
            laws.memory_rate(&onset_hazard, onset_rate);
            laws.memory_rate(&onset_hazard, idle_onset);
            laws.memory_rate(&return_hazard, return_rate);
            laws.memory_rate(&return_hazard, idle_return);
            laws.hazard(HazardGuard::new(&onset_hazard, &dice, sinus.to(&ectopic)?));
            laws.hazard(HazardGuard::new(&return_hazard, &back, ectopic.to(&sinus)?));
            match variant {
                Variant::Bystander => {
                    let pace =
                        laws.parameter(ParameterKey::new("cv.bystander-rate"), Admissible::nonnegative());
                    let other = laws.stream("bystander-stream");
                    let second = laws.state::<Dimensionless>(SECOND_INTEGRAL, Admissible::nonnegative());
                    let [quiet, stirred] = laws.modes(BYSTANDER, ["quiet", "stirred"]);
                    let rate = hazard_rate(&laws, &pace, &tone, &wobble, laws.every_mode(), beat);
                    laws.memory_rate(&second, rate);
                    laws.hazard(HazardGuard::new(&second, &other, quiet.to(&stirred)?));
                }
                Variant::Racing | Variant::Echoing => {
                    let other = laws.stream("second-stream");
                    let second = laws.state::<Dimensionless>(SECOND_INTEGRAL, Admissible::nonnegative());
                    let rate = hazard_rate(&laws, &onset, &tone, &wobble, laws.every_mode(), beat);
                    laws.memory_rate(&second, rate);
                    let move_to = if variant == Variant::Racing {
                        sinus.to(&ectopic)?
                    } else {
                        ectopic.to(&sinus)?
                    };
                    laws.hazard(HazardGuard::new(&second, &other, move_to));
                }
                Variant::TwoRhythms => {
                    let there = laws.stream("second-there");
                    let home = laws.stream("second-back");
                    let out = laws.state::<Dimensionless>(SECOND_INTEGRAL, Admissible::nonnegative());
                    let back_again = laws.state::<Dimensionless>(THIRD_INTEGRAL, Admissible::nonnegative());
                    let [quiet, stirred] = laws.modes(BYSTANDER, ["quiet", "stirred"]);
                    let (going, coming) = (
                        hazard_rate(&laws, &onset, &tone, &wobble, quiet.only(), beat),
                        hazard_rate(&laws, &recovery, &tone, &wobble, stirred.only(), beat),
                    );
                    let still = |holds| {
                        laws.kernel(|mut k| {
                            let rate = k.zero::<Rate<Dimensionless>>();
                            k.finish(rate, holds, beat)
                        })
                    };
                    let (idle_out, idle_back) = (still(stirred.only()), still(quiet.only()));
                    laws.memory_rate(&out, going);
                    laws.memory_rate(&out, idle_out);
                    laws.memory_rate(&back_again, coming);
                    laws.memory_rate(&back_again, idle_back);
                    laws.hazard(HazardGuard::new(&out, &there, quiet.to(&stirred)?));
                    laws.hazard(HazardGuard::new(&back_again, &home, stirred.to(&quiet)?));
                }
                Variant::Plain | Variant::Relaxing => {}
            }
            body.add(laws.finish());
            let (mut laws, ports) = body.boundary()?.parts();
            let BoundaryWater { route: pair, drive } = ports.water(EXTERIOR)?;
            let rate = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, laws.every_mode(), wave)
            });
            laws.transfer(Transfer::new(&pair, rate));
            body.add(laws.finish());
            body.compile()
        })?;
        let delivery = Delivery {
            id: b"chance case inflow",
            from: SimTime::from_ns(30 * SECOND),
            to: SimTime::from_ns(60 * SECOND),
            amount: Measure::new(100.0)?,
        };
        Ok(Self {
            body,
            a: a.id(),
            b: b.id(),
            deliveries: vec![delivery],
            seed: Seed::new([1, 2, 3, 4]),
            variant,
            amplitude: HAZARD_AMPLITUDE,
            end: SimTime::from_ns(240 * SECOND),
        })
    }
    pub fn until(self, end: SimTime) -> Self {
        Self { end, ..self }
    }
    pub fn delivering(self, deliveries: Vec<Delivery>) -> Self {
        Self { deliveries, ..self }
    }
    pub fn seeded(self, seed: Seed) -> Self {
        Self { seed, ..self }
    }
    /// A flat forcing, so a hazard rate is constant and a first crossing is its threshold over that rate.
    pub fn steady(self) -> Self {
        Self {
            amplitude: 0.0,
            ..self
        }
    }
    pub fn memory(&self, local: &'static str) -> Option<RowId> {
        let kind = RowKind::Memory(named(local));
        (self.body.rows().iter())
            .find(|row| row.kind() == kind)
            .map(Row::id)
    }
    pub fn tone_slot(&self) -> usize {
        self.memory(TONE).expect("the body declares it").index()
    }
    pub fn rows(&self) -> [RowId; 2] {
        [self.a, self.b].map(|site| {
            let lumen = RowKind::Amount {
                site,
                space: Space::Lumen,
            };
            (self.body.rows().iter())
                .find(|row| row.kind() == lumen)
                .expect("the body declares both lumens")
                .id()
        })
    }
    fn packet(&self) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        let default = Binding::Network(Network::Vascular);
        let mut packet = ParameterPacket::new();
        let note = "synthetic";
        let body = Binding::Body;
        let zero = || Measure::<Dimensionless>::new(0.0);
        let rate = |value| Measure::<Rate<Dimensionless>>::new(value);
        let e = Measure::<Elastance>::new(1.0)?;
        packet.synthetic(ParameterKey::new("cv.e"), default, e, note)?;
        let conductance = Measure::<Conductance<Water>>::new(CONDUCTANCE)?;
        packet.synthetic(ParameterKey::new("cv.g"), default, conductance, note)?;
        let slowed = Measure::<Conductance<Water>>::new(CONDUCTANCE / 2.0)?;
        packet.synthetic(ParameterKey::new("cv.g-ectopic"), default, slowed, note)?;
        let one = Measure::<Dimensionless>::new(1.0)?;
        packet.synthetic(ParameterKey::new("cv.tone-target"), body, one, note)?;
        packet.synthetic(ParameterKey::new("cv.tone-rate"), body, rate(0.01)?, note)?;
        let onset = rate(ECTOPY_PER_SECOND)?;
        packet.synthetic(ParameterKey::new("cv.ectopy-rate"), body, onset, note)?;
        let recovery = rate(RETURN_PER_SECOND)?;
        packet.synthetic(ParameterKey::new("cv.return-rate"), body, recovery, note)?;
        let period = Measure::<Time>::new(KNOT_PERIOD_SECONDS)?;
        packet.synthetic(ParameterKey::new("cv.knot-period"), body, period, note)?;
        let amplitude = Measure::<Dimensionless>::new(self.amplitude)?;
        packet.synthetic(ParameterKey::new("cv.hazard-amplitude"), body, amplitude, note)?;
        let memory = |local| RowKind::Memory(named(local));
        packet.start(memory(TONE), Measure::<Dimensionless>::new(1.0)?, note)?;
        packet.start(memory(ONSET_INTEGRAL), zero()?, note)?;
        packet.start(memory(RETURN_INTEGRAL), zero()?, note)?;
        packet.start_mode(named(RHYTHM), "sinus", note)?;
        match self.variant {
            Variant::Bystander => {
                let pace = rate(BYSTANDER_PER_SECOND)?;
                packet.synthetic(ParameterKey::new("cv.bystander-rate"), body, pace, note)?;
                packet.start(memory(SECOND_INTEGRAL), zero()?, note)?;
                packet.start_mode(named(BYSTANDER), "quiet", note)?;
            }
            Variant::Racing | Variant::Echoing => packet.start(memory(SECOND_INTEGRAL), zero()?, note)?,
            Variant::TwoRhythms => {
                packet.start(memory(SECOND_INTEGRAL), zero()?, note)?;
                packet.start(memory(THIRD_INTEGRAL), zero()?, note)?;
                packet.start_mode(named(BYSTANDER), "quiet", note)?;
            }
            Variant::Plain | Variant::Relaxing => {}
        }
        for (site, amount) in [self.a, self.b].into_iter().zip(START) {
            let row = RowKind::Amount {
                site,
                space: Space::Lumen,
            };
            packet.start(row, Measure::<Water>::new(amount)?, note)?;
        }
        Ok(packet)
    }
}
impl ReplayCase for ChanceCase {
    fn body(&self) -> &CompiledBody {
        &self.body
    }
    fn end(&self) -> SimTime {
        self.end
    }
    fn events(&self) -> Result<Vec<BoundaryEvent>, AdmissionError> {
        events_of(&self.deliveries)
    }
    fn admit<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        let boundary = BoundaryPort::new(self.events()?)?;
        admit_seeded(&self.body, self.packet()?, boundary, core, self.seed)
    }
    fn admit_withheld<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        let boundary = BoundaryPort::new(Vec::new())?;
        admit_seeded(&self.body, self.packet()?, boundary, core, self.seed)
    }
}

/// Binds a fixture packet and admits a session on the packet's own values: an initializer that returns its guess,
/// so a packet they do not solve is refused with the row named. `admit_solved` solves one. Only a synthetic fixture
/// enters, so validation is nonclinical by type. A core's declared policy tags its continuation.
pub fn admit_fixture<'m>(
    body: &'m CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    core: &'m dyn CoupledSolver,
) -> Result<Session<'m, Running>, InitializationFailure> {
    admit_seeded(body, packet, boundary, core, Seed::new([1, 2, 3, 4]))
}

/// Admission reads no seed, so only the identity and the course move with it.
pub fn admit_seeded<'m>(
    body: &'m CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    core: &'m dyn CoupledSolver,
    seed: Seed,
) -> Result<Session<'m, Running>, InitializationFailure> {
    let origin = SimTime::from_ns(0);
    let parameters = body.bind(packet)?;
    let request = InitializationRequest::at(origin);
    let system = body.initialization(parameters, boundary, request)?;
    let solved = system.workspace();
    let resolution = core.resolution();
    let start = MethodHistory::new(resolution.policy(), origin, Vec::new());
    let origin = system.admit(solved, start)?;
    let identity = RunIdentity::new(body.model_hash(), origin.packet_hash(), seed, resolution);
    Ok(Session::admit(body, core, origin, identity)?)
}
