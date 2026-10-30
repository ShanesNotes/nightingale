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
    pub fn body(&self) -> &CompiledBody {
        &self.body
    }
    /// `1 / k`. A forward Euler step past this makes the difference between the reservoirs grow.
    pub fn relaxation_ns(&self) -> u64 {
        RELAXATION_SECONDS * SECOND
    }
    pub fn end(&self) -> SimTime {
        self.end
    }
    /// The instant of the first delivery, and that event's id.
    pub fn delivery(&self) -> Option<(SimTime, InputId)> {
        let delivery = self.deliveries.first()?;
        Some((delivery.from, Self::input(delivery.id)))
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
    fn input(name: &[u8]) -> InputId {
        InputId::new(Digest::of_canonical(name))
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
    pub fn events(&self) -> Result<Vec<BoundaryEvent>, AdmissionError> {
        let mut events = Vec::with_capacity(self.deliveries.len());
        for delivery in &self.deliveries {
            let water = WaterDelivery {
                boundary: EXTERIOR,
                amount: Measure::new(number(&delivery.amount))?,
                ends: delivery.to,
            };
            events.push(BoundaryEvent::new(
                Self::input(delivery.id),
                delivery.from,
                BoundaryInput::Water(water),
            ));
        }
        Ok(events)
    }
    /// The same packet with a doubled conductance: what a core reads if it evaluates on other values.
    pub fn other_values(&self) -> Result<BoundParameters, AdmissionError> {
        self.body.bind(self.packet(2.0 * CONDUCTANCE)?)
    }
    pub fn admit<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        let boundary = BoundaryPort::new(self.events()?)?;
        admit_fixture(&self.body, self.packet(CONDUCTANCE)?, boundary, core)
    }
    /// The same run with no timeline input known at admission, so each must come through
    /// `Session::enqueue`. The timeline is not part of the run identity, so this is the same run.
    pub fn admit_withheld<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        let boundary = BoundaryPort::new(Vec::new())?;
        admit_fixture(&self.body, self.packet(CONDUCTANCE)?, boundary, core)
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

/// Binds a fixture packet, initializes once and admits a session: the path every composition root writes.
/// Only a synthetic fixture enters, so validation is nonclinical by type. A core's declared policy tags its
/// continuation.
pub fn admit_fixture<'m>(
    body: &'m CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    boundary: BoundaryPort,
    core: &'m dyn CoupledSolver,
) -> Result<Session<'m, Running>, InitializationFailure> {
    let origin = SimTime::from_ns(0);
    let parameters = body.bind(packet)?;
    let request = InitializationRequest::consistent_instant(origin);
    let system = body.initialization(parameters, boundary, request)?;
    let solved = system.workspace();
    let resolution = core.resolution();
    let start = MethodHistory::new(resolution.policy(), origin, Vec::new());
    let origin = system.admit(solved, start)?;
    let identity = RunIdentity::new(
        body.model_hash(),
        origin.packet_hash(),
        Seed::new([1, 2, 3, 4]),
        resolution,
    );
    Ok(Session::admit(body, core, origin, identity)?)
}
