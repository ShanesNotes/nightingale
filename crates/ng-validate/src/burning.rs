use crate::{CertificatePolicy, ExpectedDrift, QuantityPolicy, ReplayCase, SlidingWindows, admit_fixture};
use ng_contract::*;
use ng_record::{HistoryStore, Running, Session};

const SECOND: u64 = 1_000_000_000;
const BURN: &str = "burn";
const SYNTHETIC: &str = "bench: synthetic";
const GLUCOSE_ENERGY: f64 = 1_000.0;
const CLOSURE_FLOOR: f64 = 1e-9;
const CLOSURE_RELATIVE: f64 = 1e-12;
const SYNTHETIC_DRIFT: f64 = 1e4;

/// A bed and the boundaries it alone uses.
struct Bed {
    site: &'static str,
    glucose: &'static str,
    oxygen: &'static str,
    carbon_dioxide: &'static str,
    water: &'static str,
    heat: &'static str,
}
const BEDS: [Bed; 3] = [
    Bed {
        site: "bed a",
        glucose: "bed a glucose",
        oxygen: "bed a oxygen",
        carbon_dioxide: "bed a carbon dioxide",
        water: "bed a water",
        heat: "bed a heat",
    },
    Bed {
        site: "bed b",
        glucose: "bed b glucose",
        oxygen: "bed b oxygen",
        carbon_dioxide: "bed b carbon dioxide",
        water: "bed b water",
        heat: "bed b heat",
    },
    Bed {
        site: "bed c",
        glucose: "bed c glucose",
        oxygen: "bed c oxygen",
        carbon_dioxide: "bed c carbon dioxide",
        water: "bed c water",
        heat: "bed c heat",
    },
];

/// Values a packet binds, by key: the burn's at the capillary beds, the boundaries' for the body.
const BURN_SPEED: (&str, f64) = ("mt.burn-speed", 0.001);
const OXYGEN_HALF: (&str, f64) = ("mt.oxygen-half", 5.0);
const GLUCOSE_SUPPLY: (&str, f64) = ("bench.glucose-supply", 0.01);
const OXYGEN_SUPPLY: (&str, f64) = ("bench.oxygen-supply", 0.06);
const CARBON_DIOXIDE_CLEARANCE: (&str, f64) = ("bench.carbon-dioxide-clearance", 0.01);
const WATER_CLEARANCE: (&str, f64) = ("bench.water-clearance", 6e-5);
const HEAT_LOSS: (&str, f64) = ("bench.heat-loss", 0.01);

fn at_origin() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

/// Capillary beds on a vascular loop, each burning glucose with oxygen in its interstitium into a heat store of its
/// own. Each bed takes glucose and oxygen in and lets carbon dioxide, water and heat out through boundaries of its
/// own, so no column joins two beds. Every number is synthetic, glucose's energy content included.
pub struct BurningBeds {
    body: CompiledBody,
    beds: Vec<SiteId>,
    burns: bool,
}
impl BurningBeds {
    /// Each bed burns one glucose with six oxygen into six carbon dioxide and six water, its energy into the bed's heat.
    pub fn burning() -> Result<Self, AdmissionError> {
        Self::declared(true, Some(burn()))
    }
    /// The same beds and boundaries with no reaction and none of its parameters, the twin a cost is measured against.
    pub fn idle() -> Result<Self, AdmissionError> {
        Self::declared(true, None)
    }
    /// The beds burning by other proportions.
    pub fn with(proportions: Proportions<Glucose>) -> Result<Self, AdmissionError> {
        Self::declared(true, Some(proportions))
    }
    /// The beds burning with no energy content stated for glucose.
    pub fn unstated() -> Result<Self, AdmissionError> {
        Self::declared(false, Some(burn()))
    }
    fn declared(stated: bool, proportions: Option<Proportions<Glucose>>) -> Result<Self, AdmissionError> {
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", at_origin()?);
        let vessel = map.site::<VesselSegment>("vessel", at_origin()?);
        map.contain(root.id(), vessel.id());
        let mut beds = Vec::with_capacity(BEDS.len());
        for bed in &BEDS {
            let site = map.site::<CapillaryBed>(bed.site, at_origin()?).id();
            map.contain(root.id(), site);
            map.connect(Network::Vascular, vessel.id(), site);
            map.connect(Network::Vascular, site, vessel.id());
            beds.push(site);
        }
        let mut schema = Schema::new(map.finish()?)?;
        schema.leave_out(Space::Plasma, "the burn draws on the interstitium alone");
        schema.leave_out(Space::Cells, "the burn draws on the interstitium alone");
        schema.leave_out(Space::Lumen, "no blood moves in this bench");
        schema.carry::<Glucose, CapillaryBed>(Space::Interstitium);
        schema.carry::<Oxygen, CapillaryBed>(Space::Interstitium);
        schema.carry::<CarbonDioxide, CapillaryBed>(Space::Interstitium);
        schema.carry::<Energy, CapillaryBed>(Space::Thermal);
        if stated {
            schema.energy_content::<Glucose>(Measure::new(GLUCOSE_ENERGY)?)?;
        }
        let (into, out) = (BoundaryDirection::IntoBody, BoundaryDirection::OutOfBody);
        for (bed, &site) in BEDS.iter().zip(&beds) {
            schema.boundary::<Glucose>(bed.glucose, into, site, Space::Interstitium)?;
            schema.boundary::<Oxygen>(bed.oxygen, into, site, Space::Interstitium)?;
            schema.boundary::<CarbonDioxide>(bed.carbon_dioxide, out, site, Space::Interstitium)?;
            schema.boundary::<Water>(bed.water, out, site, Space::Interstitium)?;
            schema.boundary::<Energy>(bed.heat, out, site, Space::Thermal)?;
        }
        let class = RateClass::new(RateBand::Minutes, 10 * SECOND)?;
        let burns = proportions.is_some();
        let body = assemble(schema, |mut body| {
            if let Some(proportions) = proportions {
                let held = HeldDeclaration::<Metabolism>::new(Vec::new(), Vec::new(), SYNTHETIC);
                let (mut laws, _) = body.hold(held)?.parts();
                burned(&mut laws, proportions, class);
                body.add(laws.finish());
            }
            let (mut laws, ports) = body.boundary()?.parts();
            let key = |entry: (&'static str, f64)| ParameterKey::new(entry.0);
            let glucose = laws.parameter::<Rate<Glucose>>(key(GLUCOSE_SUPPLY), Admissible::nonnegative());
            let oxygen = laws.parameter::<Rate<Oxygen>>(key(OXYGEN_SUPPLY), Admissible::nonnegative());
            let [carbon_dioxide, water, heat] = [CARBON_DIOXIDE_CLEARANCE, WATER_CLEARANCE, HEAT_LOSS]
                .map(|entry| laws.parameter::<Rate<Dimensionless>>(key(entry), Admissible::nonnegative()));
            for bed in &BEDS {
                supplied(&mut laws, &ports, bed.glucose, &glucose, class)?;
                supplied(&mut laws, &ports, bed.oxygen, &oxygen, class)?;
                cleared::<CarbonDioxide>(&mut laws, &ports, bed.carbon_dioxide, &carbon_dioxide, class)?;
                cleared::<Water>(&mut laws, &ports, bed.water, &water, class)?;
                cleared::<Energy>(&mut laws, &ports, bed.heat, &heat, class)?;
            }
            body.add(laws.finish());
            body.compile()
        })?;
        Ok(Self { body, beds, burns })
    }
    /// Each bed's reaction column, in column order.
    pub fn burns(&self) -> Vec<ColumnId> {
        let burn = LawName {
            owner: Metabolism::CARD,
            local: BURN,
        };
        let mut burns: Vec<ColumnId> = (self.body.instances().iter())
            .filter_map(|instance| match instance.target() {
                Target::Column(column) if self.body.columns()[column.index()].origin() == burn => {
                    Some(column)
                }
                Target::Column(_) | Target::Row(_) => None,
            })
            .collect();
        burns.dedup();
        burns
    }
    /// Each bed starts a little above the one before it, so no two beds' quantities hold one value.
    pub fn packet(&self) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        let mut packet = ParameterPacket::new();
        let key = |entry: (&'static str, f64)| ParameterKey::new(entry.0);
        let value = |entry: (&'static str, f64)| entry.1;
        let body = Binding::Body;
        packet.synthetic(
            key(GLUCOSE_SUPPLY),
            body,
            Measure::<Rate<Glucose>>::new(value(GLUCOSE_SUPPLY))?,
            SYNTHETIC,
        )?;
        packet.synthetic(
            key(OXYGEN_SUPPLY),
            body,
            Measure::<Rate<Oxygen>>::new(value(OXYGEN_SUPPLY))?,
            SYNTHETIC,
        )?;
        for speed in [CARBON_DIOXIDE_CLEARANCE, WATER_CLEARANCE, HEAT_LOSS] {
            let measure = Measure::<Rate<Dimensionless>>::new(value(speed))?;
            packet.synthetic(key(speed), body, measure, SYNTHETIC)?;
        }
        if self.burns {
            let beds = Binding::Class(Class::CapillaryBed);
            let speed = Measure::<Rate<Dimensionless>>::new(value(BURN_SPEED))?;
            packet.synthetic(key(BURN_SPEED), beds, speed, SYNTHETIC)?;
            let half = Measure::<Oxygen>::new(value(OXYGEN_HALF))?;
            packet.synthetic(key(OXYGEN_HALF), beds, half, SYNTHETIC)?;
        }
        for (raise, &site) in (0_u32..).zip(&self.beds) {
            let raise = f64::from(raise);
            let amount = |space, species| RowKind::Amount { site, space, species };
            let interstitial = |species| amount(Space::Interstitium, species);
            let water = Measure::<Water>::new(1_000.0 + 50.0 * raise)?;
            packet.start(interstitial(Species::Water), water, SYNTHETIC)?;
            let glucose = Measure::<Glucose>::new(10.0 + raise)?;
            packet.start(interstitial(Species::Glucose), glucose, SYNTHETIC)?;
            let oxygen = Measure::<Oxygen>::new(50.0 + 5.0 * raise)?;
            packet.start(interstitial(Species::Oxygen), oxygen, SYNTHETIC)?;
            let carbon_dioxide = Measure::<CarbonDioxide>::new(6.0 + raise)?;
            packet.start(interstitial(Species::CarbonDioxide), carbon_dioxide, SYNTHETIC)?;
            let heat = Measure::<Energy>::new(1_000.0 + 100.0 * raise)?;
            packet.start(amount(Space::Thermal, Species::Energy), heat, SYNTHETIC)?;
        }
        Ok(packet)
    }
    /// Every quantity closes to rounding over each window and drifts within a synthetic bound.
    pub fn certificate(&self, windows: SlidingWindows) -> Result<CertificatePolicy, AdmissionError> {
        fn limits<U: Substance>(quantity: QuantityId) -> Result<QuantityPolicy, AdmissionError> {
            let floor = Measure::<U>::new(CLOSURE_FLOOR)?;
            let drift = Measure::<U>::new(SYNTHETIC_DRIFT)?;
            QuantityPolicy::new(quantity, floor, Measure::new(CLOSURE_RELATIVE)?, drift)
        }
        let basis = self.body.conserved_basis();
        let quantities = basis.ids().map(|quantity| match quantity.book() {
            Book::Element(_) => QuantityPolicy::element(
                quantity,
                Measure::new(CLOSURE_FLOOR)?,
                Measure::new(CLOSURE_RELATIVE)?,
                Measure::new(SYNTHETIC_DRIFT)?,
            ),
            Book::Energy | Book::Of(Species::Energy) => limits::<Energy>(quantity),
            Book::Of(Species::Water) => limits::<Water>(quantity),
            Book::Of(Species::Glucose) => limits::<Glucose>(quantity),
            Book::Of(Species::Oxygen) => limits::<Oxygen>(quantity),
            Book::Of(Species::CarbonDioxide) => limits::<CarbonDioxide>(quantity),
            Book::Of(_) | Book::Charge => Err(AdmissionError::QuantityDimension { quantity }),
        });
        let quantities = quantities.collect::<Result<Vec<_>, _>>()?;
        CertificatePolicy::new(basis, quantities, ExpectedDrift::Zero, windows)
    }
}

/// One glucose with six oxygen into six carbon dioxide and six water in a bed's interstitium, its energy into the bed's
/// heat: definitional chemistry.
fn burn() -> Proportions<Glucose> {
    Proportions::<Glucose>::using(Space::Interstitium)
        .uses::<Oxygen, 6>(Space::Interstitium)
        .makes::<CarbonDioxide, 6>(Space::Interstitium)
        .makes::<Water, 6>(Space::Interstitium)
        .energy_into(Space::Thermal)
}

/// The burn at each bed, in proportion to its glucose and slowed as its oxygen runs short.
fn burned(laws: &mut FragmentBuilder<'_, Metabolism>, proportions: Proportions<Glucose>, class: RateClass) {
    let burn = laws.reaction::<CapillaryBed, Glucose>(BURN, proportions);
    let speed = laws.site_parameter::<CapillaryBed, Rate<Dimensionless>>(
        ParameterKey::new(BURN_SPEED.0),
        Admissible::nonnegative(),
    );
    let half =
        laws.site_parameter::<CapillaryBed, Oxygen>(ParameterKey::new(OXYGEN_HALF.0), Admissible::positive());
    let (fuel, oxygen) = (burn.amount::<Glucose>(), burn.amount::<Oxygen>());
    let rate = laws.site_kernel(|mut k| {
        let (fuel, oxygen) = (k.at_site(&fuel), k.at_site(&oxygen));
        let (zero, speed, half) = (k.zero(), k.site_parameter(&speed), k.site_parameter(&half));
        let paced = k.relax(fuel, zero, speed);
        let reach = k.add(oxygen, half);
        let share = k.divide(oxygen, reach);
        let share = k.as_dimension::<_, Dimensionless>(share);
        let rate = k.scale(paced, share);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.react_at(&burn, rate);
}

/// A boundary that brings `Q` in at a fixed rate.
fn supplied<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    name: &'static str,
    supply: &Parameter<'b, Boundary, Rate<Q>>,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let BoundaryRoute { route, .. } = ports.route::<Q>(name)?;
    let rate = laws.kernel(|mut k| {
        let rate = k.parameter(supply);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
    Ok(())
}

/// A boundary that lets `Q` out in proportion to what its pool holds.
fn cleared<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    name: &'static str,
    speed: &Parameter<'b, Boundary, Rate<Dimensionless>>,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let BoundaryRoute { route, .. } = ports.route::<Q>(name)?;
    let held = route.amount::<Q>();
    let rate = laws.kernel(|mut k| {
        let (held, zero, speed) = (k.read(&held), k.zero(), k.parameter(speed));
        let rate = k.relax(held, zero, speed);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
    Ok(())
}

impl ReplayCase for BurningBeds {
    fn body(&self) -> &CompiledBody {
        &self.body
    }
    fn end(&self) -> SimTime {
        SimTime::from_ns(600 * SECOND)
    }
    fn events(&self) -> Result<Vec<BoundaryEvent>, AdmissionError> {
        Ok(Vec::new())
    }
    fn admit<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
        store: &'m dyn HistoryStore,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        admit_fixture(
            &self.body,
            self.packet()?,
            BoundaryPort::new(Vec::new())?,
            core,
            store,
        )
    }
    fn admit_withheld<'m>(
        &'m self,
        core: &'m dyn CoupledSolver,
        store: &'m dyn HistoryStore,
    ) -> Result<Session<'m, Running>, InitializationFailure> {
        self.admit(core, store)
    }
}
