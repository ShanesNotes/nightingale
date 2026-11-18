//! A held use and a held production at fixed rates stand for the blood, so the alveolar gas equation and the
//! alveolar ventilation equation are checked against closed forms with no body admission.

use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::{Running, Session};
use ng_respiratory::{Plasma, Ventilation, exchange};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const NOTE: &str = "alveolar fixture: synthetic";
const LUNG: &str = "lung bed";
const AIRWAY: &str = "conducting airway";
const ROOM: f64 = 700.0;
const VAPOUR: f64 = 50.0;
const DRY: f64 = ROOM - VAPOUR;
/// Oxygen, carbon dioxide and nitrogen gas, in that order throughout.
const INSPIRED: [f64; 3] = [0.21, 0.0, 0.79];
const CONDUCTANCE: f64 = 1_000.0;
const MIXING: f64 = 5_000.0;
/// The lung's and the chest wall's elastances and relaxed volumes, apart so their balance is no plain mean.
const ELASTANCES: [f64; 2] = [1e-3, 3e-3];
const RESTS: [f64; 2] = [4_000.0, 6_000.0];
const ZERO_HEAD: f64 =
    (ELASTANCES[0] * RESTS[0] + ELASTANCES[1] * RESTS[1]) / (ELASTANCES[0] + ELASTANCES[1]);
const WIDTH: f64 = 1e-6;
const DIFFUSING: [f64; 2] = [25.0, 100.0];
const PER_AMOUNT: f64 = 0.1;
/// The run's length in seconds: the slowest of the unit's settling speeds is near one a second.
const SETTLED: u64 = 60;
/// The run settles to about 5e-8 of each value.
const TOLERANCE: f64 = 1e-6;

#[derive(Clone, Copy)]
struct Case {
    oxygen_use: f64,
    carbon_dioxide_made: f64,
    ventilation: f64,
    molar_volume: [f64; 3],
}

const AT_REST: Case = Case {
    oxygen_use: 250.0,
    carbon_dioxide_made: 200.0,
    ventilation: 1.0,
    molar_volume: [1.0; 3],
};

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
    let airway = map.site::<TissueSite>(AIRWAY, origin()?).id();
    let lung_site = map.site::<CapillaryBed>(LUNG, origin()?);
    let lung = lung_site.id();
    let vessel = map.site::<VesselSegment>("vessel", origin()?).id();
    map.serves(&lung_site, Tissue::Lung);
    map.contain(root, airway);
    map.contain(root, lung);
    map.contain(root, vessel);
    map.connect(Network::Vascular, vessel, lung);
    map.connect(Network::Vascular, lung, vessel);
    map.connect(Network::Airway, airway, lung);
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "one alveolar unit and its plasma");
    }
    schema.carry::<Oxygen, Airway>(Space::Gas);
    schema.carry::<CarbonDioxide, Airway>(Space::Gas);
    schema.carry::<NitrogenGas, Airway>(Space::Gas);
    schema.pool::<Oxygen>(lung, Space::Plasma)?;
    schema.pool::<CarbonDioxide>(lung, Space::Plasma)?;
    for process in [
        Schema::process::<Inspired<Oxygen>>,
        Schema::process::<Inspired<CarbonDioxide>>,
        Schema::process::<Inspired<NitrogenGas>>,
        Schema::process::<Expired<Oxygen>>,
        Schema::process::<Expired<CarbonDioxide>>,
        Schema::process::<Expired<NitrogenGas>>,
    ] {
        process(&mut schema, airway, Space::Gas)?;
    }
    let (into, out) = (BoundaryDirection::IntoBody, BoundaryDirection::OutOfBody);
    schema.boundary::<Oxygen>("oxygen use", out, lung, Space::Plasma)?;
    schema.boundary::<CarbonDioxide>("carbon dioxide made", into, lung, Space::Plasma)?;
    assemble(schema, |mut body| {
        let held = HeldDeclaration::<BloodChemistry>::new(Vec::new(), Vec::new(), NOTE)
            .pool::<Water, CapillaryBed>(Space::Plasma);
        let (laws, _) = body.hold(held)?.parts();
        body.add(laws.finish());

        let class = RateClass::new(RateBand::Beat, SECOND)?;
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryRoute { route: used, .. } = ports.route::<Oxygen>("oxygen use")?;
        let BoundaryRoute { route: made, .. } = ports.route::<CarbonDioxide>("carbon dioxide made")?;
        let key = ParameterKey::new;
        let use_rate = laws.parameter::<Rate<Oxygen>>(key("fixture.oxygen-use"), Admissible::nonnegative());
        let made_rate = laws
            .parameter::<Rate<CarbonDioxide>>(key("fixture.carbon-dioxide-made"), Admissible::nonnegative());
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&use_rate);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&used, rate));
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&made_rate);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&made, rate));
        body.add(laws.finish());

        let (mut laws, ports) = body.respiratory(None)?.parts();
        let ventilation = Ventilation::declare(&mut laws, &ports)?;
        let plasma = Dissolved {
            oxygen: laws.parameter(key("fixture.oxygen-per-amount"), Admissible::positive()),
            carbon_dioxide: laws.parameter(key("fixture.carbon-dioxide-per-amount"), Admissible::positive()),
        };
        exchange(&mut laws, &ports, &ventilation, &plasma)?;
        body.add(laws.finish());
        body.compile()
    })
}

/// Every gas crosses the room's side and the airway edge at the same two coefficients, `A = G h+ + M` in and
/// `B = G h- + M` out, and the net `G h` at each is the net uptake, so with `U_i` each gas's uptake into the plasma
/// `f_D = (A F_I - U) / B` in the dead space and `f_A = (A f_D - U) / B` in the alveolar unit.
fn closed_form(unit: Case) -> ([f64; 3], [f64; 3]) {
    let (g, m) = (CONDUCTANCE * unit.ventilation, MIXING * unit.ventilation);
    let net = unit.oxygen_use - unit.carbon_dioxide_made;
    let head = net / g;
    let smooth = |a: f64| 0.5 * (a + (a * a + WIDTH * WIDTH).sqrt());
    let (inward, outward) = (g * smooth(head) + m, g * smooth(-head) + m);
    let uptake = [unit.oxygen_use, -unit.carbon_dioxide_made, 0.0];
    let dead = [0, 1, 2].map(|i| (inward * INSPIRED[i] - uptake[i]) / outward);
    let alveolar = [0, 1, 2].map(|i| (inward * dead[i] - uptake[i]) / outward);
    (dead, alveolar)
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
    set::<Rate<Oxygen>>(p, "fixture.oxygen-use", unit.oxygen_use)?;
    set::<Rate<CarbonDioxide>>(p, "fixture.carbon-dioxide-made", unit.carbon_dioxide_made)?;
    set::<Quotient<Pressure, Oxygen>>(p, "fixture.oxygen-per-amount", PER_AMOUNT)?;
    set::<Quotient<Pressure, CarbonDioxide>>(p, "fixture.carbon-dioxide-per-amount", PER_AMOUNT)?;
    set::<Pressure>(p, "rs.room-pressure", ROOM)?;
    set::<Pressure>(p, "rs.water-vapour-pressure", VAPOUR)?;
    set::<Pressure>(p, "rs.head-width", WIDTH)?;
    set::<Conductance<Oxygen>>(p, "rs.oxygen-diffusing", DIFFUSING[0])?;
    set::<Conductance<CarbonDioxide>>(p, "rs.carbon-dioxide-diffusing", DIFFUSING[1])?;
    set::<Elastance>(p, "rs.lung-elastance", ELASTANCES[0])?;
    set::<Elastance>(p, "rs.wall-elastance", ELASTANCES[1])?;
    set::<Volume>(p, "rs.lung-rest-volume", RESTS[0])?;
    set::<Volume>(p, "rs.wall-rest-volume", RESTS[1])?;
    set::<Elastance>(p, "rs.dead-space-elastance", 1e-2)?;
    set::<Volume>(p, "rs.dead-space-volume", 500.0)?;
    let (g, m) = (CONDUCTANCE * unit.ventilation, MIXING * unit.ventilation);
    let [vo, vc, vn] = unit.molar_volume;
    set::<Quotient<Volume, Oxygen>>(p, "rs.oxygen-volume", vo)?;
    set::<Quotient<Volume, CarbonDioxide>>(p, "rs.carbon-dioxide-volume", vc)?;
    set::<Quotient<Volume, NitrogenGas>>(p, "rs.nitrogen-volume", vn)?;
    set::<Conductance<Volume>>(p, "rs.airway-conductance", g)?;
    set::<Rate<Volume>>(p, "rs.airway-mixing", m)?;
    for (gas, key) in [
        (0, "rs.oxygen-inspired"),
        (1, "rs.carbon-dioxide-inspired"),
        (2, "rs.nitrogen-inspired"),
    ] {
        set::<Dimensionless>(p, key, INSPIRED[gas])?;
    }
    let (dead, alveolar) = closed_form(unit);
    let guess = |row: RowKind| match row {
        RowKind::Amount { site, species, space } => {
            let (shares, volume) = if site.key() == LUNG {
                (alveolar, ZERO_HEAD)
            } else {
                (dead, 500.0)
            };
            match (species, space) {
                (Species::Oxygen, Space::Gas) => shares[0] * volume / vo,
                (Species::CarbonDioxide, Space::Gas) => shares[1] * volume / vc,
                (Species::NitrogenGas, Space::Gas) => shares[2] * volume / vn,
                (Species::Oxygen, Space::Plasma) => DRY * alveolar[0] / PER_AMOUNT,
                (Species::CarbonDioxide, Space::Plasma) => DRY * alveolar[1] / PER_AMOUNT,
                _ => 100.0,
            }
        }
        RowKind::Memory(_) | RowKind::Algebraic(_) => 0.0,
    };
    packet.start_rest(body, guess, NOTE)?;
    packet.within(ErrorScales::uniform(Measure::new(1e-9)?, Measure::new(1e-10)?)?);
    Ok(packet)
}

struct Settled {
    alveolar_volume: f64,
    alveolar: [f64; 3],
    dead: [f64; 3],
    plasma: [f64; 2],
}

fn settled(unit: Case) -> Result<Settled, InitializationFailure> {
    let body = compiled()?;
    let core = ng_solver::Core::new(SECOND / 100);
    let store = MemoryStore::default();
    let lens = body.lens(Vec::new(), NonZeroU64::MIN.saturating_add(SECOND - 1))?;
    let port = BoundaryPort::new(Vec::new())?;
    let session = admit_solved(&body, packet(&body, unit)?, port, &core, &lens, &store)?.session;
    let session = run_until(session, SimTime::from_ns(SETTLED * SECOND))
        .unwrap_or_else(|stopped| panic!("{:?}", stopped.fault()));
    let amount = |session: &Session<'_, Running>, key, species, space| -> Result<f64, AdmissionError> {
        let site = if key == LUNG {
            site::<CapillaryBed>(LUNG)?
        } else {
            site::<TissueSite>(AIRWAY)?
        };
        let kind = RowKind::Amount { site, species, space };
        let row = (body.rows().iter())
            .find(|row| row.kind() == kind)
            .unwrap_or_else(|| panic!("no row {kind:?}"));
        Ok(f64::from_bits(
            session.record().accepted().value(row.id()).bits()[0],
        ))
    };
    let gas_space = |key| -> Result<(f64, [f64; 3]), AdmissionError> {
        let gas = [Species::Oxygen, Species::CarbonDioxide, Species::NitrogenGas];
        let held = gas.map(|species| amount(&session, key, species, Space::Gas));
        let [vo, vc, vn] = unit.molar_volume;
        let parts = [held[0]? * vo, held[1]? * vc, held[2]? * vn];
        let total: f64 = parts.iter().sum();
        Ok((total, parts.map(|part| part / total)))
    };
    let (alveolar_volume, alveolar) = gas_space(LUNG)?;
    let (_, dead) = gas_space(AIRWAY)?;
    Ok(Settled {
        alveolar_volume,
        alveolar,
        dead,
        plasma: [
            PER_AMOUNT * amount(&session, LUNG, Species::Oxygen, Space::Plasma)?,
            PER_AMOUNT * amount(&session, LUNG, Species::CarbonDioxide, Space::Plasma)?,
        ],
    })
}

fn close(measured: f64, expected: f64, relative: f64) -> bool {
    (measured - expected).abs() <= relative * expected.abs()
}

#[test]
fn the_alveolar_gas_equation_holds_at_a_steady_state() -> Result<(), InitializationFailure> {
    let unit = AT_REST;
    let settled = settled(unit)?;
    let [oxygen, carbon_dioxide] = [DRY * settled.alveolar[0], DRY * settled.alveolar[1]];
    let quotient = unit.carbon_dioxide_made / unit.oxygen_use;
    let inspired = INSPIRED[0];
    let expected = DRY * inspired - carbon_dioxide * (inspired + (1.0 - inspired) / quotient);
    assert!(
        close(oxygen, expected, TOLERANCE),
        "alveolar oxygen {oxygen}, the gas equation's {expected} at alveolar carbon dioxide {carbon_dioxide}"
    );
    Ok(())
}

#[test]
fn mixing_moves_no_net_volume_when_the_membrane_trades_equal_volumes() -> Result<(), InitializationFailure> {
    let unit = Case {
        molar_volume: [1.0, 1.25, 0.8],
        ..AT_REST
    };
    assert_eq!(
        unit.oxygen_use * unit.molar_volume[0],
        unit.carbon_dioxide_made * unit.molar_volume[1]
    );
    let settled = settled(unit)?;
    let zero_head_volume = ZERO_HEAD;
    assert!(
        close(settled.alveolar_volume, zero_head_volume, TOLERANCE),
        "alveolar volume {} with no net volume uptake, expected {zero_head_volume}",
        settled.alveolar_volume
    );
    Ok(())
}

#[test]
fn the_airway_carries_each_ends_share_upwind_into_the_dead_space_and_the_alveolar_unit()
-> Result<(), InitializationFailure> {
    let settled = settled(AT_REST)?;
    let (dead, alveolar) = closed_form(AT_REST);
    for gas in 0..3 {
        assert!(
            close(settled.dead[gas], dead[gas], TOLERANCE),
            "gas {gas}: the dead space's share {}, the closed form's {}",
            settled.dead[gas],
            dead[gas]
        );
        assert!(
            close(settled.alveolar[gas], alveolar[gas], TOLERANCE),
            "gas {gas}: the alveolar share {}, the closed form's {}",
            settled.alveolar[gas],
            alveolar[gas]
        );
    }
    Ok(())
}

#[test]
fn the_membrane_moves_each_gas_down_its_own_head_from_the_dry_alveolar_pressure()
-> Result<(), InitializationFailure> {
    let unit = AT_REST;
    let settled = settled(unit)?;
    let alveolar = [DRY * settled.alveolar[0], DRY * settled.alveolar[1]];
    let crossing = [unit.oxygen_use, -unit.carbon_dioxide_made];
    for gas in 0..2 {
        let expected = alveolar[gas] - crossing[gas] / DIFFUSING[gas];
        assert!(
            close(settled.plasma[gas], expected, TOLERANCE),
            "gas {gas}: the plasma's partial pressure {}, Fick's {expected} from the alveolar {}",
            settled.plasma[gas],
            alveolar[gas]
        );
    }
    Ok(())
}

#[test]
fn doubling_ventilation_halves_alveolar_carbon_dioxide_where_use_equals_production()
-> Result<(), InitializationFailure> {
    let even = Case {
        oxygen_use: 200.0,
        ..AT_REST
    };
    let doubled = Case {
        ventilation: 2.0,
        ..even
    };
    let [once, twice] = [settled(even)?, settled(doubled)?].map(|settled| DRY * settled.alveolar[1]);
    assert!(
        close(twice, once / 2.0, TOLERANCE),
        "alveolar carbon dioxide {once}, then {twice} at twice the ventilation"
    );
    Ok(())
}

/// With a net uptake the two exchanges in series do not halve: the miss is the net uptake's share of the
/// ventilation, and the closed form carries it.
#[test]
fn doubling_ventilation_at_resting_use_meets_the_closed_form_of_the_two_exchanges_in_series()
-> Result<(), InitializationFailure> {
    let twice = Case {
        ventilation: 2.0,
        ..AT_REST
    };
    let (_, expected) = closed_form(twice);
    let settled = settled(twice)?;
    assert!(
        close(settled.alveolar[1], expected[1], TOLERANCE),
        "alveolar carbon dioxide's share {} at twice the ventilation, the closed form's {}",
        settled.alveolar[1],
        expected[1]
    );
    Ok(())
}
