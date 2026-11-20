//! One alveolar unit breathes under the chemoreflex. Blood chemistry is staged at the lung bed and a brain bed, a held
//! production feeds the plasma and the void gate stands for the fixture's switch, so each sensor's response, the
//! loop's gain on a step in production, a held breath's return and the exact hold of the drive scale at one are
//! checked against closed forms with no body admission.

use core::f64::consts::LOG10_E;
use core::num::NonZeroU64;
use ng_contract::catalog::{Defines, PumpForce};
use ng_contract::*;
use ng_record::{Running, Session};
use ng_respiratory::{Plasma, Ventilation, exchange};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const NOTE: &str = "chemoreflex fixture: synthetic";
const LUNG: &str = "lung bed";
const BRAIN: &str = "brain bed";
const AIRWAY: &str = "conducting airway";
const PLEURA: &str = "pleural space";
const JUNCTION: &str = "respiratory pump junction";
const ROOM: f64 = 700.0;
const VAPOUR: f64 = 50.0;
const DRY: f64 = ROOM - VAPOUR;
const INSPIRED: [f64; 3] = [0.21, 0.0, 0.79];
const CONDUCTANCE: f64 = 1e5;
const ELASTANCES: [f64; 2] = [1e-3, 3e-3];
const RESTS: [f64; 2] = [11_000.0, 19_000.0];
const HEAD_WIDTH: f64 = 1e-6;
const DIFFUSING: [f64; 2] = [25.0, 100.0];
const PER_AMOUNT: f64 = 0.1;
const DEAD_SPACE: f64 = 1_200.0;
const DEAD_SPACE_ELASTANCE: f64 = 1e-2;
const WATER_VOLUME: f64 = 10.0;
const HUMIDITY: f64 = 0.5;
const ROOM_VAPOUR: f64 = 20.0;
const INTERSTITIUM_WATER_SET: f64 = 1_000.0;
const UPRIGHT_WALL_REST_RISE: f64 = 2_000.0;
const CAROTID_ABOVE_VESSEL: f64 = 0.3;
const WORK_MEAN: f64 = 1.0;
const USE: f64 = 20.0;
const MADE: f64 = 20.0;
const PUMP: f64 = 364.0;

const RATE: f64 = 0.2;
const PHASES: [f64; 2] = [0.4, 0.6];
/// The drive rises and falls at twenty a second, so it meets its peak and zero inside each phase at any breath length
/// near the packet's, and the tidal volume does not depend on the breath's length.
const DRIVE: [f64; 3] = [20.0, 20.0, 1.0];
const RELEASE: f64 = 0.05;
const REUPTAKE: f64 = 50.0;
const ON: f64 = 10.0;
const OFF: f64 = 50.0;
const SITES: f64 = 1.0;
const STORE: f64 = 100.0;

const SATURATION: f64 = 0.95;
const ACIDITY: f64 = 7.4;
const BICARBONATE: f64 = 24.0;
const GAINS: [f64; 2] = [2.0 / 0.778, 1.0 / 0.778];
const CENTRAL_WIDTH: f64 = LOG10_E;
const OFFSET: f64 = 40.0;
const SMOOTHING: f64 = 0.5;
const SET_POINT_SPEED: f64 = 1e-6;
const REST: f64 = 1.0;

#[derive(Clone, Copy)]
struct Steps {
    saturation: f64,
    acidity: f64,
    production: f64,
    held: f64,
}

const STILL: Steps = Steps {
    saturation: 0.0,
    acidity: 0.0,
    production: 0.0,
    held: 0.0,
};

#[derive(Clone, Copy)]
struct Case {
    offset: f64,
    gains: [f64; 2],
    steps: Steps,
    opens: u64,
    shuts: Option<u64>,
    seconds: u64,
}

const SWITCH: u64 = 3;

const SENSED: Case = Case {
    offset: OFFSET,
    gains: GAINS,
    steps: STILL,
    opens: SWITCH,
    shuts: None,
    seconds: 6,
};

/// The breath's alveolar ventilation at a drive scale of one: a breath of `V_T` through a mixed dead space `V_D`
/// returns `V_D tanh(V_T / 2 V_D)` of it, and `V_T` is the pump's swing over the elastances at the junction's steady
/// effect at the drive's peak.
fn ventilation() -> f64 {
    let tidal = PUMP * effect(DRIVE[2]) / (ELASTANCES[0] + ELASTANCES[1]);
    let x = tidal / DEAD_SPACE;
    RATE * (tidal - DEAD_SPACE * (1.0 - exp(-x)) / (1.0 + exp(-x)))
}

/// The mixing exchange at rest carries the breath's alveolar ventilation: `f_A = F_I - 2U / M` where the heads are
/// zero, so `M` is twice it, and the rest the fixture admits at is the breathing one.
fn mixing() -> f64 {
    2.0 * ventilation()
}

/// The rest's gas shares in the dead space and the alveolar unit, `f_D = (A F_I - U) / B` and `f_A = (A f_D - U) / B`,
/// with `A = B` because use equals production at equal molar volumes.
fn rest_shares() -> ([f64; 3], [f64; 3]) {
    let each = CONDUCTANCE * HEAD_WIDTH / 2.0 + mixing();
    let uptake = [USE, -MADE, 0.0];
    let dead = [0, 1, 2].map(|i| (each * INSPIRED[i] - uptake[i]) / each);
    let alveolar = [0, 1, 2].map(|i| (each * dead[i] - uptake[i]) / each);
    (dead, alveolar)
}

fn rest_plasma() -> [f64; 2] {
    let (_, alveolar) = rest_shares();
    [
        DRY * alveolar[0] - USE / DIFFUSING[0],
        DRY * alveolar[1] + MADE / DIFFUSING[1],
    ]
}

/// What a unit relative change in carbon dioxide moves the peripheral sensor by at the rest, as the body's packet
/// states it.
fn peripheral_width() -> f64 {
    (1.0 - SATURATION) * rest_plasma()[1]
}

fn smooth_max(a: f64, b: f64, width: f64) -> f64 {
    0.5 * (a + b + ((a - b) * (a - b) + width * width).sqrt())
}

/// The pump junction's vesicle, cleft and bound amounts at a held drive: `C = rho V` with `rho = q a / r`,
/// `B = R k_on C / (k_off + k_on C)` and `V + C + B = T`, so the cleft is the positive root of
/// `alpha k_on C^2 + (alpha k_off + (R - T) k_on) C - T k_off = 0` with `alpha = (1 + rho) / rho`.
fn steady(drive: f64) -> [f64; 3] {
    if drive <= 0.0 {
        return [STORE, 0.0, 0.0];
    }
    let rho = RELEASE * drive / REUPTAKE;
    let alpha = (1.0 + rho) / rho;
    let (a, b, c) = (alpha * ON, alpha * OFF + (SITES - STORE) * ON, STORE * OFF);
    let cleft = 2.0 * c / (b + (b * b + 4.0 * a * c).sqrt());
    let bound = SITES * ON * cleft / (OFF + ON * cleft);
    [cleft / rho, cleft, bound]
}

fn effect(drive: f64) -> f64 {
    steady(drive)[2] / SITES
}

fn site<C: SiteClass>(key: &'static str) -> Result<SiteId, AdmissionError> {
    let origin = Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?);
    Ok(AnatomyBuilder::new().site::<C>(key, origin).id())
}

struct Dissolved<'b> {
    oxygen: Parameter<'b, Respiratory, Quotient<Pressure, Oxygen>>,
    carbon_dioxide: Parameter<'b, Respiratory, Quotient<Pressure, CarbonDioxide>>,
}
type AtLung<'b, 'k> = Kernel<'b, 'k, Respiratory, Beds<Lung>>;
impl<'b> Plasma<'b> for Dissolved<'b> {
    fn oxygen<'k>(
        &self,
        k: &mut AtLung<'b, 'k>,
        plasma: &ReadPort<'b, Respiratory, PerSite<Beds<Lung>, Oxygen>>,
    ) -> Expr<'b, 'k, Respiratory, Pressure> {
        let (per, held) = (k.parameter(&self.oxygen), k.at_site(plasma));
        let pressure = k.multiply(per, held);
        k.as_dimension(pressure)
    }
    fn carbon_dioxide<'k>(
        &self,
        k: &mut AtLung<'b, 'k>,
        plasma: &ReadPort<'b, Respiratory, PerSite<Beds<Lung>, CarbonDioxide>>,
    ) -> Expr<'b, 'k, Respiratory, Pressure> {
        let (per, held) = (k.parameter(&self.carbon_dioxide), k.at_site(plasma));
        let pressure = k.multiply(per, held);
        k.as_dimension(pressure)
    }
}

/// Each gas's pressure is its plasma amount times the same factor the respiratory card's membrane reads, so the
/// peripheral sensor and the membrane see one carbon dioxide pressure.
fn stage<'b>(
    laws: &mut FragmentBuilder<'b, BloodChemistry>,
    ports: &BloodChemistryPorts<'b>,
    class: RateClass,
) {
    let key = ParameterKey::new;
    let stated = ports.speciation(laws);
    let (oxygen, carbon_dioxide) = (ports.blood::<Oxygen>(), ports.blood::<CarbonDioxide>());
    let oxygen_per = laws
        .parameter::<Quotient<Pressure, Oxygen>>(key("fixture.bc-oxygen-per-amount"), Admissible::positive());
    let carbon_dioxide_per = laws.parameter::<Quotient<Pressure, CarbonDioxide>>(
        key("fixture.bc-carbon-dioxide-per-amount"),
        Admissible::positive(),
    );
    let saturation = laws
        .site_parameter::<Speciated, Dimensionless>(key("fixture.saturation"), Admissible::unit_interval());
    let acidity =
        laws.site_parameter::<Speciated, Dimensionless>(key("fixture.acidity"), Admissible::finite());
    let bicarbonate = laws.site_parameter::<Speciated, Concentration<CarbonDioxide>>(
        key("fixture.bicarbonate"),
        Admissible::positive(),
    );
    let steps = [
        laws.parameter::<Dimensionless>(key("fixture.saturation-step"), Admissible::nonnegative()),
        laws.parameter::<Dimensionless>(key("fixture.acidity-step"), Admissible::nonnegative()),
    ];
    let value = laws.site_kernel(|mut k| {
        let (per, held) = (k.parameter(&oxygen_per), k.at_site(&oxygen));
        let pressure = k.multiply(per, held);
        let value = k.as_dimension(pressure);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&stated.oxygen_partial_pressure.0, value);
    let moved = [
        (&stated.oxygen_saturation.0, &saturation, &steps[0]),
        (&stated.acidity.0, &acidity, &steps[1]),
    ];
    for (output, at_rest, step) in moved {
        let value = laws.site_kernel(|mut k| {
            let (at_rest, step, open) = (k.site_parameter(at_rest), k.parameter(step), k.voiding());
            let moved = k.scale(step, open);
            let value = k.subtract(at_rest, moved);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define_at(output, value);
    }
    let value = laws.site_kernel(|mut k| {
        let value = k.site_parameter(&bicarbonate);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&stated.bicarbonate.0, value);
    laws.express_at(&stated.carbon_dioxide_partial_pressure.0, |mut k| {
        let (per, held) = (k.parameter(&carbon_dioxide_per), k.at_site(&carbon_dioxide));
        let pressure = k.multiply(per, held);
        let value = k.as_dimension(pressure);
        k.finish_term(value)
    });
}

fn compiled() -> Result<CompiledBody, AdmissionError> {
    let origin = || -> Result<Position, AdmissionError> {
        Ok(Position::new(
            Measure::new(0.0)?,
            Measure::new(0.0)?,
            Measure::new(0.0)?,
        ))
    };
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin()?).id();
    let raised = Position::new(
        Measure::new(CAROTID_ABOVE_VESSEL)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    );
    let airway = map.site::<TissueSite>(AIRWAY, raised).id();
    let lung_site = map.site::<CapillaryBed>(LUNG, origin()?);
    let lung = lung_site.id();
    let brain_site = map.site::<CapillaryBed>(BRAIN, origin()?);
    let brain = brain_site.id();
    let vessel = map.site::<VesselSegment>("vessel", origin()?).id();
    map.serves(&lung_site, Tissue::Lung);
    map.serves(&brain_site, Tissue::Brain);
    for site in [airway, lung, brain, vessel] {
        map.contain(root, site);
    }
    for bed in [lung, brain] {
        map.connect(Network::Vascular, vessel, bed);
        map.connect(Network::Vascular, bed, vessel);
    }
    map.connect(Network::Airway, airway, lung);
    map.landmark(Landmark::CarotidSinus, vessel, airway);
    map.landmark(Landmark::CentralChemoreceptor, brain, brain);
    map.landmark(Landmark::PeripheralChemoreceptor, lung, lung);
    let junction = map.site::<Junction>(JUNCTION, origin()?).id();
    map.contain(root, junction);
    map.landmark(Landmark::RespiratoryPump, junction, junction);
    let pleura = map.site::<EnclosedSpace>(PLEURA, origin()?);
    map.contain(root, pleura.id());
    map.enclose(
        &pleura,
        &[lung],
        Compliance::Held {
            card: Respiratory::CARD,
        },
    );
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Cells, Space::Lumen, Space::Fluid] {
        schema.leave_out(space, "one alveolar unit, a brain bed and their plasma");
    }
    schema.carry::<Oxygen, Airway>(Space::Gas);
    schema.carry::<CarbonDioxide, Airway>(Space::Gas);
    schema.carry::<NitrogenGas, Airway>(Space::Gas);
    schema.carry::<Water, Airway>(Space::Gas);
    for bed in [lung, brain] {
        schema.pool::<Oxygen>(bed, Space::Plasma)?;
        schema.pool::<CarbonDioxide>(bed, Space::Plasma)?;
    }
    schema.partial_volume(Measure::new(1.0)?)?;
    for space in [Space::Vesicle, Space::Cleft, Space::Bound] {
        schema.pool::<Acetylcholine>(junction, space)?;
    }
    for process in [
        Schema::process::<Inspired<Oxygen>>,
        Schema::process::<Inspired<CarbonDioxide>>,
        Schema::process::<Inspired<NitrogenGas>>,
        Schema::process::<Expired<Oxygen>>,
        Schema::process::<Expired<CarbonDioxide>>,
        Schema::process::<Expired<NitrogenGas>>,
        Schema::process::<Expired<Water>>,
    ] {
        process(&mut schema, airway, Space::Gas)?;
    }
    let (into, out) = (BoundaryDirection::IntoBody, BoundaryDirection::OutOfBody);
    schema.boundary::<Oxygen>("oxygen use", out, lung, Space::Plasma)?;
    schema.boundary::<CarbonDioxide>("carbon dioxide made", into, lung, Space::Plasma)?;
    schema.boundary::<Water>("water held", into, lung, Space::Interstitium)?;
    assemble(schema, |mut body| {
        let class = RateClass::new(RateBand::Beat, SECOND)?;
        let key = ParameterKey::new;
        let held = HeldDeclaration::<BloodChemistry>::new(Vec::new(), Vec::new(), NOTE)
            .pool::<Water, CapillaryBed>(Space::Plasma)
            .pool::<Oxygen, Beds<Brain>>(Space::Plasma)
            .pool::<CarbonDioxide, Beds<Brain>>(Space::Plasma)
            .pool::<Water, Beds<Brain>>(Space::Interstitium);
        let held = body.hold(held)?;
        let (mut laws, ports) = body.blood_chemistry(Some(held))?.parts();
        stage(&mut laws, &ports, class);
        body.add(laws.finish());

        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryRoute { route: used, .. } = ports.route::<Oxygen>("oxygen use")?;
        let BoundaryRoute { route: made, .. } = ports.route::<CarbonDioxide>("carbon dioxide made")?;
        let use_rate = laws.parameter::<Rate<Oxygen>>(key("fixture.oxygen-use"), Admissible::nonnegative());
        let made_rate = laws
            .parameter::<Rate<CarbonDioxide>>(key("fixture.carbon-dioxide-made"), Admissible::nonnegative());
        let more = laws.parameter::<Dimensionless>(key("fixture.production-step"), Admissible::nonnegative());
        let rate = laws.kernel(|mut k| {
            let (used, more, open) = (k.parameter(&use_rate), k.parameter(&more), k.voiding());
            let share = k.scale(more, open);
            let added = k.scale(used, share);
            let value = k.add(used, added);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&used, rate));
        let rate = laws.kernel(|mut k| {
            let (made, more, open) = (k.parameter(&made_rate), k.parameter(&more), k.voiding());
            let share = k.scale(more, open);
            let added = k.scale(made, share);
            let value = k.add(made, added);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&made, rate));
        let BoundaryRoute { route: wetting, .. } = ports.route::<Water>("water held")?;
        let held = wetting.amount::<Water>();
        let set = laws.parameter::<Water>(key("fixture.water-set"), Admissible::positive());
        let speed = laws.parameter::<Rate<Dimensionless>>(key("fixture.water-speed"), Admissible::positive());
        let rate = laws.kernel(|mut k| {
            let (set, held, speed) = (k.parameter(&set), k.read(&held), k.parameter(&speed));
            let value = k.relax(set, held, speed);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&wetting, rate));
        body.add(laws.finish());

        let (mut laws, ports) = body.nervous()?.parts();
        let vagal = laws.algebraic::<Dimensionless>("vagal traffic", Admissible::unit_interval());
        let residual = laws.kernel(|mut k| {
            let held = k.algebraic(&vagal);
            k.finish(held, laws.every_mode(), class)
        });
        laws.residual(&vagal, residual);
        let scale = ng_nervous::chemoreflex(&mut laws)?;
        ng_nervous::breathing(&mut laws, &ports, (&vagal, &scale))?;
        body.add(laws.finish());
        let fragment = ng_receptor::declare(body.receptor()?)?;
        body.add(fragment);

        let (mut laws, ports) = body.musculoskeletal(None)?.parts();
        let effect = ports.effect::<Acetylcholine>();
        let most = laws.parameter::<Pressure>(key("ms.pump-pressure"), Admissible::nonnegative());
        let hold = laws.parameter::<Dimensionless>(key("fixture.breath-held"), Admissible::nonnegative());
        let speed = laws.parameter::<Rate<Dimensionless>>(key("fixture.hold-speed"), Admissible::positive());
        // The hold follows the switch at the drive's speed: the pump's pressure cut in an instant through the stiff
        // airway left the core no admissible step.
        let held = laws.state::<Dimensionless>("pump hold", Admissible::finite());
        let follow = laws.kernel(|mut k| {
            let (hold, open, now, speed) = (
                k.parameter(&hold),
                k.voiding(),
                k.state(&held),
                k.parameter(&speed),
            );
            let target = k.scale(hold, open);
            let value = k.relax(target, now, speed);
            k.finish(value, laws.every_mode(), class)
        });
        laws.memory_rate(&held, follow);
        let line: PumpForce<Defines<'_, Musculoskeletal>> = laws.emit();
        let force = laws.kernel(|mut k| {
            let (most, effect) = (
                k.parameter(&most),
                k.at_landmark(Landmark::RespiratoryPump, &effect),
            );
            let pulled = k.scale(most, effect);
            let share = k.state(&held);
            let lost = k.scale(pulled, share);
            let value = k.subtract(pulled, lost);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define(&line.pump_force, force);
        body.add(laws.finish());

        let (mut laws, ports) = body.respiratory(None)?.parts();
        let ventilation = Ventilation::declare(&mut laws, &ports)?;
        let plasma = Dissolved {
            oxygen: laws.parameter(key("fixture.oxygen-per-amount"), Admissible::positive()),
            carbon_dioxide: laws.parameter(key("fixture.carbon-dioxide-per-amount"), Admissible::positive()),
        };
        exchange(&mut laws, &ports, &ventilation, &plasma)?;
        ventilation.pleural(&mut laws, &ports)?;
        ventilation.work(&mut laws, &ports)?;
        ventilation.water(&mut laws, &ports)?;
        body.add(laws.finish());
        body.compile()
    })
}

fn packet(body: &CompiledBody, unit: Case) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    fn set<U: Dimension>(
        packet: &mut ParameterPacket<SyntheticFixture>,
        key: &'static str,
        value: f64,
    ) -> Result<(), AdmissionError> {
        packet.synthetic(
            ParameterKey::new(key),
            Binding::Body,
            Measure::<U>::new(value)?,
            NOTE,
        )
    }
    let mut packet = ParameterPacket::new();
    let p = &mut packet;
    set::<Rate<Oxygen>>(p, "fixture.oxygen-use", USE)?;
    set::<Rate<CarbonDioxide>>(p, "fixture.carbon-dioxide-made", MADE)?;
    set::<Quotient<Pressure, Oxygen>>(p, "fixture.oxygen-per-amount", PER_AMOUNT)?;
    set::<Quotient<Pressure, CarbonDioxide>>(p, "fixture.carbon-dioxide-per-amount", PER_AMOUNT)?;
    set::<Quotient<Pressure, Oxygen>>(p, "fixture.bc-oxygen-per-amount", PER_AMOUNT)?;
    set::<Quotient<Pressure, CarbonDioxide>>(p, "fixture.bc-carbon-dioxide-per-amount", PER_AMOUNT)?;
    let Steps {
        saturation,
        acidity,
        production,
        held,
    } = unit.steps;
    for (key, value) in [
        ("fixture.saturation-step", saturation),
        ("fixture.acidity-step", acidity),
        ("fixture.production-step", production),
        ("fixture.breath-held", held),
    ] {
        set::<Dimensionless>(p, key, value)?;
    }
    for bed in [site::<CapillaryBed>(LUNG)?, site::<CapillaryBed>(BRAIN)?] {
        let at = Binding::Site(bed);
        p.synthetic(
            ParameterKey::new("fixture.saturation"),
            at,
            Measure::<Dimensionless>::new(SATURATION)?,
            NOTE,
        )?;
        p.synthetic(
            ParameterKey::new("fixture.acidity"),
            at,
            Measure::<Dimensionless>::new(ACIDITY)?,
            NOTE,
        )?;
        p.synthetic(
            ParameterKey::new("fixture.bicarbonate"),
            at,
            Measure::<Concentration<CarbonDioxide>>::new(BICARBONATE)?,
            NOTE,
        )?;
    }
    set::<Dimensionless>(p, "ns.central-gain", unit.gains[0])?;
    set::<Dimensionless>(p, "ns.peripheral-gain", unit.gains[1])?;
    set::<Deviation<Dimensionless>>(p, "ns.central-width", CENTRAL_WIDTH)?;
    set::<Deviation<Pressure>>(p, "ns.peripheral-width", peripheral_width())?;
    set::<Pressure>(p, "ns.peripheral-offset", unit.offset)?;
    set::<Pressure>(p, "ns.peripheral-smoothing", SMOOTHING)?;
    set::<Rate<Dimensionless>>(p, "ns.chemo-set-point-speed", SET_POINT_SPEED)?;
    set::<Dimensionless>(p, "ns.drive-scale-rest", REST)?;
    set::<Pressure>(p, "rs.room-pressure", ROOM)?;
    set::<Pressure>(p, "rs.water-vapour-pressure", VAPOUR)?;
    set::<Pressure>(p, "rs.head-width", HEAD_WIDTH)?;
    set::<Conductance<Oxygen>>(p, "rs.oxygen-diffusing", DIFFUSING[0])?;
    set::<Conductance<CarbonDioxide>>(p, "rs.carbon-dioxide-diffusing", DIFFUSING[1])?;
    set::<Elastance>(p, "rs.lung-elastance", ELASTANCES[0])?;
    set::<Elastance>(p, "rs.wall-elastance", ELASTANCES[1])?;
    set::<Volume>(p, "rs.lung-rest-volume", RESTS[0])?;
    set::<Volume>(p, "rs.wall-rest-volume", RESTS[1])?;
    set::<Volume>(p, "rs.wall-rest-rise", UPRIGHT_WALL_REST_RISE)?;
    set::<Length>(p, "rs.trunk-length", CAROTID_ABOVE_VESSEL)?;
    set::<Pressure>(p, "ms.pump-pressure", PUMP)?;
    set::<Rate<Dimensionless>>(p, "fixture.hold-speed", DRIVE[1])?;
    set::<Rate<Energy>>(p, "rs.work-mean", WORK_MEAN)?;
    set::<Rate<Dimensionless>>(p, "rs.work-speed", 0.05)?;
    for (key, value) in [
        ("ns.breath-rate", RATE),
        ("ns.drive-rise", DRIVE[0]),
        ("ns.drive-fall", DRIVE[1]),
        ("ns.rest-speed", 1.0),
        ("ns.share-fall", 1.0),
        ("ns.breath-start", 1e3),
    ] {
        set::<Rate<Dimensionless>>(p, key, value)?;
    }
    for (key, value) in [
        ("ns.inspiration-length", PHASES[0]),
        ("ns.expiration-length", PHASES[1]),
        ("ns.drive-peak", DRIVE[2]),
        ("ns.rest-drive", 0.0),
        ("ns.rest-share", 1.0),
    ] {
        set::<Dimensionless>(p, key, value)?;
    }
    set::<Elastance>(p, "rs.dead-space-elastance", DEAD_SPACE_ELASTANCE)?;
    let junctions = Binding::Class(Class::Junction);
    for (key, value) in [
        ("ns.vagal-share", 0.0),
        ("ns.motor-share", 1.0),
        ("rc.ach-efficacy", 1.0),
    ] {
        p.synthetic(
            ParameterKey::new(key),
            junctions,
            Measure::<Dimensionless>::new(value)?,
            NOTE,
        )?;
    }
    for (key, value) in [
        ("ns.release-ach", RELEASE),
        ("ns.reuptake-ach", REUPTAKE),
        ("rc.ach-off", OFF),
    ] {
        p.synthetic(
            ParameterKey::new(key),
            junctions,
            Measure::<Rate<Dimensionless>>::new(value)?,
            NOTE,
        )?;
    }
    p.synthetic(
        ParameterKey::new("rc.ach-sites"),
        junctions,
        Measure::<Acetylcholine>::new(SITES)?,
        NOTE,
    )?;
    let on = Measure::<Affinity<Acetylcholine>>::new(ON)?;
    p.synthetic(ParameterKey::new("rc.ach-on"), junctions, on, NOTE)?;
    set::<Volume>(p, "rs.dead-space-volume", DEAD_SPACE)?;
    let m = mixing();
    set::<Quotient<Volume, Oxygen>>(p, "rs.oxygen-volume", 1.0)?;
    set::<Quotient<Volume, CarbonDioxide>>(p, "rs.carbon-dioxide-volume", 1.0)?;
    set::<Quotient<Volume, NitrogenGas>>(p, "rs.nitrogen-volume", 1.0)?;
    set::<Conductance<Volume>>(p, "rs.airway-conductance", CONDUCTANCE)?;
    set::<Rate<Volume>>(p, "rs.airway-mixing", m)?;
    set::<Quotient<Volume, Water>>(p, "rs.water-volume", WATER_VOLUME)?;
    set::<Rate<Volume>>(p, "rs.alveolar-ventilation", m / 2.0)?;
    set::<Dimensionless>(p, "rs.room-humidity", HUMIDITY)?;
    set::<Pressure>(p, "rs.room-vapour-pressure", ROOM_VAPOUR)?;
    set::<Water>(p, "fixture.water-set", INTERSTITIUM_WATER_SET)?;
    set::<Rate<Dimensionless>>(p, "fixture.water-speed", 1.0)?;
    for (gas, key) in [(0, "rs.oxygen-inspired"), (1, "rs.carbon-dioxide-inspired")] {
        set::<Dimensionless>(p, key, INSPIRED[gas])?;
    }
    let (dead, alveolar) = rest_shares();
    let [oxygen, carbon_dioxide] = rest_plasma();
    let measure = (1.0 - SATURATION) * smooth_max(carbon_dioxide - unit.offset, 0.0, SMOOTHING);
    let lung = ELASTANCES[0] * RESTS[0] + ELASTANCES[1] * RESTS[1];
    let lung = lung / (ELASTANCES[0] + ELASTANCES[1]);
    let guess = |row: RowKind| match row {
        RowKind::Amount { site, species, space } => {
            let (shares, volume) = if site.key() == AIRWAY {
                (dead, DEAD_SPACE)
            } else {
                (alveolar, lung)
            };
            match (species, space) {
                (Species::Acetylcholine, Space::Vesicle) => STORE,
                (Species::Acetylcholine, _) => 0.0,
                (Species::Oxygen, Space::Gas) => shares[0] * volume,
                (Species::CarbonDioxide, Space::Gas) => shares[1] * volume,
                (Species::NitrogenGas, Space::Gas) => shares[2] * volume,
                (Species::Water, Space::Gas) => VAPOUR / DRY * volume / WATER_VOLUME,
                (Species::Water, Space::Interstitium) => INTERSTITIUM_WATER_SET,
                (Species::Oxygen, Space::Plasma) => oxygen / PER_AMOUNT,
                (Species::CarbonDioxide, Space::Plasma) => carbon_dioxide / PER_AMOUNT,
                _ => 100.0,
            }
        }
        RowKind::Algebraic(Slot { law, .. }) if law.owner == BloodChemistry::CARD => match law.local {
            "oxygen partial pressure" => oxygen,
            "oxygen saturation" => SATURATION,
            "bicarbonate" => BICARBONATE,
            _ => ACIDITY,
        },
        RowKind::Memory(slot) if slot.law.local == "central set point" => ACIDITY,
        RowKind::Memory(slot) if slot.law.local == "peripheral set point" => measure,
        RowKind::Memory(slot) if slot.law.local == "mixing share" => 1.0,
        RowKind::Memory(slot) if slot.law.local == "breath work memory" => WORK_MEAN,
        RowKind::Algebraic(slot) if slot.law.local == "chest wall rest volume" => RESTS[1],
        RowKind::Algebraic(slot) if slot.law.local == "mixing_share" => 1.0,
        RowKind::Algebraic(slot) if slot.law.local == "work_of_breathing" => WORK_MEAN,
        RowKind::Algebraic(slot) if slot.law.local == "drive scale" => 1.0,
        RowKind::Memory(_) | RowKind::Algebraic(_) => 0.0,
    };
    let breath = LawName {
        owner: Nervous::CARD,
        local: "breath",
    };
    packet.start_mode(breath, "rest", NOTE)?;
    packet.start_rest(body, guess, NOTE)?;
    packet.within(ErrorScales::uniform(Measure::new(1e-9)?, Measure::new(1e-10)?)?);
    Ok(packet)
}

fn switched(unit: Case) -> Vec<BoundaryEvent> {
    let gate = |id: &[u8], seconds: u64, gate: Gate| {
        BoundaryEvent::new(
            InputId::new(Digest::of_canonical(id)),
            SimTime::from_ns(seconds * SECOND),
            BoundaryInput::Void(gate),
        )
    };
    let mut events = vec![
        gate(b"shut", 0, Gate::Shut),
        gate(b"open", unit.opens, Gate::Open),
    ];
    if let Some(shuts) = unit.shuts {
        events.push(gate(b"shut again", shuts, Gate::Shut));
    }
    events
}

fn row(body: &CompiledBody, kind: RowKind) -> RowId {
    (body.rows().iter())
        .find(|row| row.kind() == kind)
        .unwrap_or_else(|| panic!("no row {kind:?}"))
        .id()
}

fn memory(body: &CompiledBody, local: &'static str) -> RowId {
    (body.rows().iter())
        .find(|row| matches!(row.kind(), RowKind::Memory(slot) if slot.law.local == local))
        .unwrap_or_else(|| panic!("no memory {local}"))
        .id()
}

fn algebraic(body: &CompiledBody, local: &'static str) -> RowId {
    (body.rows().iter())
        .find(|row| matches!(row.kind(), RowKind::Algebraic(slot) if slot.law.local == local))
        .unwrap_or_else(|| panic!("no algebraic row {local}"))
        .id()
}

fn now(session: &Session<'_, Running>, row: RowId) -> f64 {
    f64::from_bits(session.record().accepted().value(row).bits()[0])
}

#[derive(Clone, Copy)]
struct Sample {
    at: f64,
    scale: f64,
    phase: f64,
    pressure: f64,
    saturation: f64,
    acidity: f64,
    set_points: [f64; 2],
}

struct Rows {
    scale: RowId,
    phase: RowId,
    carbon_dioxide: RowId,
    saturation: RowId,
    acidity: RowId,
    set_points: [RowId; 2],
}

impl Rows {
    fn of(body: &CompiledBody) -> Result<Self, AdmissionError> {
        let speciated = |local: &'static str, key: &'static str| -> Result<RowId, AdmissionError> {
            let law = LawName {
                owner: BloodChemistry::CARD,
                local,
            };
            let at = Context::Site(site::<CapillaryBed>(key)?);
            Ok(row(body, RowKind::Algebraic(Slot { law, at })))
        };
        let lung = site::<CapillaryBed>(LUNG)?;
        Ok(Self {
            scale: algebraic(body, "drive scale"),
            phase: memory(body, "breath phase"),
            carbon_dioxide: row(
                body,
                RowKind::Amount {
                    site: lung,
                    species: Species::CarbonDioxide,
                    space: Space::Plasma,
                },
            ),
            saturation: speciated("oxygen saturation", LUNG)?,
            acidity: speciated("acidity", BRAIN)?,
            set_points: [
                memory(body, "central set point"),
                memory(body, "peripheral set point"),
            ],
        })
    }

    fn sample(&self, session: &Session<'_, Running>) -> Sample {
        let at = session.record().accepted().time().nanoseconds();
        Sample {
            at: at as f64 / SECOND as f64,
            scale: now(session, self.scale),
            phase: now(session, self.phase),
            pressure: PER_AMOUNT * now(session, self.carbon_dioxide),
            saturation: now(session, self.saturation),
            acidity: now(session, self.acidity),
            set_points: self.set_points.map(|row| now(session, row)),
        }
    }
}

fn trace(unit: Case, core: &dyn CoupledSolver) -> Result<Vec<Sample>, InitializationFailure> {
    let body = compiled()?;
    let store = MemoryStore::default();
    let lens = body.lens(Vec::new(), NonZeroU64::MIN.saturating_add(SECOND - 1))?;
    let port = BoundaryPort::new(switched(unit))?;
    let mut session = admit_solved(&body, packet(&body, unit)?, port, core, &lens, &store)?.session;
    let rows = Rows::of(&body)?;
    let mut samples = vec![rows.sample(&session)];
    while session.record().accepted().time().nanoseconds() < unit.seconds * SECOND {
        let at = session.record().accepted().time().nanoseconds();
        session = run_until(session, SimTime::from_ns(at + 1))
            .unwrap_or_else(|stopped| panic!("{:?}", stopped.fault()));
        samples.push(rows.sample(&session));
    }
    Ok(samples)
}

fn peripheral(unit: Case, sample: &Sample) -> f64 {
    (1.0 - sample.saturation) * smooth_max(sample.pressure - unit.offset, 0.0, SMOOTHING)
}

fn scale_form(unit: Case, sample: &Sample) -> f64 {
    let [central, peripheral_gain] = unit.gains;
    let [central_set, peripheral_set] = sample.set_points;
    let rising = peripheral_gain * (peripheral(unit, sample) - peripheral_set) / peripheral_width();
    let falling = central * (sample.acidity - central_set) / CENTRAL_WIDTH;
    REST + rising - falling
}

fn edges(samples: &[Sample]) -> Vec<usize> {
    (1..samples.len())
        .filter(|&at| samples[at].phase < samples[at - 1].phase)
        .collect()
}

fn last_breaths(samples: &[Sample], by: f64) -> &[Sample] {
    let ends: Vec<usize> = (edges(samples).into_iter())
        .filter(|&at| samples[at].at <= by)
        .collect();
    &samples[ends[ends.len() - 1 - MEANS]..=ends[ends.len() - 1]]
}

fn mean(breaths: &[Sample], of: impl Fn(&Sample) -> f64) -> f64 {
    let area: f64 = (breaths.windows(2))
        .map(|pair| 0.5 * (of(&pair[0]) + of(&pair[1])) * (pair[1].at - pair[0].at))
        .sum();
    area / (breaths[breaths.len() - 1].at - breaths[0].at)
}

fn length(breaths: &[Sample]) -> f64 {
    (breaths[breaths.len() - 1].at - breaths[0].at) / MEANS as f64
}

fn assert_on_form(unit: Case, samples: &[Sample]) {
    for sample in samples {
        let (row, form) = (sample.scale, scale_form(unit, sample));
        assert!(
            (row - form).abs() <= 1e-12,
            "at {} s the drive scale is {row}, its form {form}",
            sample.at
        );
    }
}

fn either_side(samples: &[Sample], at: u64) -> (Sample, Sample) {
    let at = at as f64;
    let before = (samples.iter()).rev().find(|sample| sample.at < at);
    let after = (samples.iter()).find(|sample| sample.at > at);
    (
        *before.expect("a state before the switch"),
        *after.expect("a state after the switch"),
    )
}

/// A held breath carries the carbon dioxide pressure from three widths under the offset to well over it. The runs use
/// the reference integrator, whose Newton stops at rounding: the core's stops at a tenth of its step's estimate, which
/// left the solved scale 2.4e-12 off its form.
#[test]
fn each_sensor_moves_the_drive_scale_by_its_form_above_near_and_below_the_peripheral_offset()
-> Result<(), InitializationFailure> {
    let central = Case {
        steps: Steps {
            acidity: 0.01,
            ..STILL
        },
        ..SENSED
    };
    let saturation = Case {
        steps: Steps {
            saturation: 0.01,
            ..STILL
        },
        ..SENSED
    };
    let swept = Case {
        offset: rest_plasma()[1] + 3.0 * SMOOTHING,
        steps: Steps {
            saturation: 0.01,
            acidity: 0.01,
            held: 1.0,
            ..STILL
        },
        shuts: Some(SWITCH + 12),
        seconds: 20,
        ..SENSED
    };
    let floor = NonZeroU64::MIN.saturating_add(SECOND / 100_000 - 1);
    let reference = ReferenceIntegrator::new(SECOND / 1_000).with_floor(floor);
    let mut classes = [0_usize; 3];
    for unit in [central, saturation, swept] {
        let samples = trace(unit, &reference)?;
        let admitted = samples[0];
        let measured = peripheral(unit, &admitted);
        let [central_set, peripheral_set] = admitted.set_points;
        assert!(
            (central_set - admitted.acidity).abs() <= 1e-12 * admitted.acidity.abs()
                && (peripheral_set - measured).abs() <= 1e-12 * measured
                && (admitted.scale - REST).abs() <= 1e-12,
            "at admission the set points are {central_set} and {peripheral_set} against the sensors {} and \
             {measured}, and the scale {}",
            admitted.acidity,
            admitted.scale
        );
        assert_on_form(unit, &samples);
        let (before, after) = either_side(&samples, unit.opens);
        assert!(
            after.scale > before.scale,
            "the switch moved the scale from {} to {}",
            before.scale,
            after.scale
        );
        for sample in samples.iter().filter(|sample| sample.at > unit.opens as f64) {
            let above = (sample.pressure - unit.offset) / SMOOTHING;
            if above < -2.0 {
                classes[0] += 1;
            } else if above.abs() <= 1.0 {
                classes[1] += 1;
            } else if above > 2.0 {
                classes[2] += 1;
            }
        }
    }
    assert!(
        classes.iter().all(|&count| count >= 10),
        "states after the switch below, near and above the offset: {classes:?}"
    );
    Ok(())
}

/// The drive scale is one in the admitted beat's mean, and with its sensors steady the fixture's beat is its instant.
/// An instant packet solves no memory, so each set point admits where the packet states it: the central one at the
/// staged acidity's bits, and the peripheral one at the sensor's closed form on the staged plasma, 2 ulps over the
/// form at the admitted rows. That rounding leaves the scale 2 ulps under its rest.
#[test]
fn with_its_sensors_steady_the_drive_scale_admits_at_its_rest() -> Result<(), InitializationFailure> {
    let samples = trace(Case { seconds: 0, ..SENSED }, &ng_solver::Core::new(SECOND / 100))?;
    let admitted = samples[0];
    let [central, peripheral_set] = admitted.set_points;
    let from = |x: f64| (x.to_bits() as i64 - REST.to_bits() as i64).abs();
    assert!(
        central.to_bits() == admitted.acidity.to_bits() && from(admitted.scale) <= 2,
        "the drive scale admits at {}, its set points at {central} and {peripheral_set} against the sensors {} and \
         {}",
        admitted.scale,
        admitted.acidity,
        peripheral(SENSED, &admitted)
    );
    Ok(())
}

/// The step's time and half the run: the closed loop's slowest mode is near fifty seconds (inferred from the drift of
/// the breath means), so 900 s leaves under 1e-7 of a start or a step.
const SETTLED: u64 = 900;
const MEANS: usize = 10;

const STEPPED: Case = Case {
    steps: Steps {
        production: 0.1,
        ..STILL
    },
    opens: SETTLED,
    seconds: 2 * SETTLED,
    ..SENSED
};

/// The steady carbon dioxide pressure is `P = P_A + U / D`, with `P_A = a U / C` by the alveolar ventilation equation,
/// since the breath's rate is the scale's and its tidal volume is not, and `C` the scale's form at `P` with the central
/// arm at its set point. Linear about the measured `P`, a step `x` in `U` moves `P` by `x / (1 + L)` of itself, with
/// `L = P_A C' / C` and `C' = G_p (1 - S) smooth_max'(P - B_p) / w_p`. A tenth is not small: the relation's
/// second-order term, `L x^2 / (1 + L)^3`, is 2.5% of the change at this `L`, so the linear form is held to 3%, and the
/// relation solved whole, with each window's set points, to 2e-3. What that leaves is the plant's: the drive, the
/// junction and the membrane keep their own speeds while the breath shortens 5%, which moves the steady alveolar
/// pressure 1e-4 off `U T`, a thousandth of the change through the loop. Use steps with production, since the 2 a
/// second of net gas that production alone sends out by the airway ventilates beyond the forms, 5% of the change. The
/// breath's rate is the scale's to where each breath edge lands inside the core's 0.1 ms floor.
#[test]
fn a_tenth_more_carbon_dioxide_made_raises_its_pressure_by_a_tenth_over_one_and_the_loops_gain()
-> Result<(), InitializationFailure> {
    let unit = STEPPED;
    let samples = trace(unit, &ng_solver::Core::new(SECOND / 100))?;
    let [first, last] = [SETTLED, unit.seconds].map(|by| last_breaths(&samples, by as f64));
    let [before, after] = [first, last].map(|breaths| mean(breaths, |sample| sample.pressure));
    let [long, short] = [first, last].map(length);
    let scale = |breaths: &[Sample], pressure: f64| {
        let at = Sample {
            pressure,
            saturation: SATURATION,
            acidity: ACIDITY,
            ..breaths[breaths.len() - 1]
        };
        scale_form(unit, &at)
    };
    let head = MADE / DIFFUSING[1];
    let above = before - unit.offset;
    let slope = 0.5 * (1.0 + above / (above * above + SMOOTHING * SMOOTHING).sqrt());
    let rising = unit.gains[1] * (1.0 - SATURATION) * slope / peripheral_width();
    let gain = (before - head) * rising / scale(first, before);
    let step = unit.steps.production;
    let linear = step / (1.0 + gain);
    let (mut low, mut high) = (before, (1.0 + step) * before);
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        let steady = (1.0 + step) * ((before - head) * scale(first, before) / scale(last, middle) + head);
        if middle < steady {
            low = middle;
        } else {
            high = middle;
        }
    }
    let nonlinear = 0.5 * (low + high) / before - 1.0;
    let measured = after / before - 1.0;
    assert!(
        (measured - linear).abs() <= 3e-2 * linear,
        "a tenth more carbon dioxide made moved its pressure by {measured}, the loop's {linear} at L = {gain}"
    );
    assert!(
        (measured - nonlinear).abs() <= 2e-3 * nonlinear,
        "a tenth more carbon dioxide made moved its pressure by {measured}, the steady forms' {nonlinear}; the \
         alveolar ventilation equation on the breath's length misses by {:e}",
        (after - (1.0 + step) * head) / ((before - head) * (1.0 + step) * short / long) - 1.0
    );
    let rate = (long / short) / (scale(last, after) / scale(first, before)) - 1.0;
    assert!(
        rate.abs() <= 1e-5,
        "breaths of {long} s and {short} s, against the scale's {} and {}",
        scale(first, before),
        scale(last, after)
    );
    Ok(())
}

const HELD: Case = Case {
    steps: Steps { held: 1.0, ..STILL },
    opens: SETTLED,
    shuts: Some(SETTLED + 10),
    seconds: 2 * SETTLED,
    ..SENSED
};

/// While the pump is held the alveolar unit sits at its relaxed volume and nothing ventilates it, so what is made over
/// the hold raises the pressure by `U t / (V_A / P_dry + 1 / k)`, over the gas's and the plasma's capacities, to 1%
/// for the hold's edges. The set points follow at 1e-6 a second, so they keep about a millionth of the hold, and the
/// breath means return to 1e-5.
#[test]
fn after_a_held_breath_the_breaths_carbon_dioxide_returns_to_where_it_was()
-> Result<(), InitializationFailure> {
    let unit = HELD;
    let samples = trace(unit, &ng_solver::Core::new(SECOND / 100))?;
    let [first, last] = [SETTLED, unit.seconds].map(|by| last_breaths(&samples, by as f64));
    let [before, after] = [first, last].map(|breaths| mean(breaths, |sample| sample.pressure));
    let lung = (ELASTANCES[0] * RESTS[0] + ELASTANCES[1] * RESTS[1]) / (ELASTANCES[0] + ELASTANCES[1]);
    let shuts = unit.shuts.expect("the hold ends");
    let closed = MADE * (shuts - unit.opens) as f64 / (lung / DRY + 1.0 / PER_AMOUNT);
    let [(start, _), (end, _)] = [unit.opens, shuts].map(|at| either_side(&samples, at));
    let rise = end.pressure - start.pressure;
    assert!(
        (rise - closed).abs() <= 1e-2 * closed,
        "over the held breath the pressure rose {rise}, the closed form {closed}"
    );
    assert!(
        (after - before).abs() <= 1e-5 * before,
        "after the held breath the pressure's breath mean is {after}, against {before} before it"
    );
    Ok(())
}

/// Each breath edge lands past its crossing inside the core's 0.1 ms floor.
#[test]
fn gains_of_zero_hold_the_drive_scale_at_exactly_one_and_the_breath_at_the_packets_period()
-> Result<(), InitializationFailure> {
    let unit = Case {
        gains: [0.0, 0.0],
        steps: Steps {
            saturation: 0.01,
            acidity: 0.01,
            production: 0.1,
            held: 0.0,
        },
        opens: 10,
        seconds: 60,
        ..SENSED
    };
    let samples = trace(unit, &ng_solver::Core::new(SECOND / 100))?;
    let one = 1.0f64.to_bits();
    let off: Vec<f64> = (samples.iter())
        .filter(|sample| sample.scale.to_bits() != one)
        .map(|sample| sample.at)
        .collect();
    assert!(off.is_empty(), "the scale is off one at {off:?} s");
    let last = samples[samples.len() - 1];
    let [central_set, peripheral_set] = last.set_points;
    let moved = [
        last.acidity - central_set,
        peripheral(unit, &last) - peripheral_set,
    ];
    assert!(
        moved[0] < -1e-3 && moved[1] > 1e-3 * peripheral_set,
        "the sensors moved by {moved:?} from their set points"
    );
    let period = (PHASES[0] + PHASES[1]) / RATE;
    let ends: Vec<f64> = (edges(&samples).into_iter()).map(|at| samples[at].at).collect();
    let miss = (ends.windows(2))
        .map(|pair| (pair[1] - pair[0] - period).abs())
        .fold(0.0, |most, x| if x > most { x } else { most });
    assert!(
        ends.len() >= 10 && miss <= 1e-4,
        "breaths of {period} s end at {ends:?}"
    );
    Ok(())
}
