//! The pump drive reaches the respiratory pump as acetylcholine released at the pump's junction: the nervous card's
//! breath and release, the receptor card's binding and effect, and the musculoskeletal card's force, on one junction
//! with no body admission. A stated vagal traffic stands in for the baroreflex, so a law that lets it reach the pump
//! shows.

use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::{Running, Session};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const NOTE: &str = "pump junction fixture: synthetic";
const JUNCTION: &str = "respiratory pump junction";
const RELEASE: f64 = 0.05;
const REUPTAKE: f64 = 50.0;
const ON: f64 = 10.0;
const OFF: f64 = 50.0;
const SITES: f64 = 1.0;
const STORE: f64 = 100.0;
const PIMAX: f64 = 60.0;
const VAGAL: f64 = 0.3;
const REST_DRIVE: f64 = 0.4;
const PEAK: f64 = 1.0;

#[derive(Clone, Copy)]
struct Case {
    breath_start: f64,
    rise: f64,
}

const HELD: Case = Case {
    breath_start: 0.0,
    rise: 1.5,
};
/// The breath starts within a millisecond of admission and the drive meets its peak within 25 ms.
const STEPPED: Case = Case {
    breath_start: 1e4,
    rise: 1e3,
};

/// Vesicle, cleft and bound at a held release drive `d`: `C = rho V` with `rho = q d / r`,
/// `B = R k_on C / (k_off + k_on C)` and `V + C + B = T`, so the cleft is the positive root of
/// `alpha k_on C^2 + (alpha k_off + (R - T) k_on) C - T k_off = 0` with `alpha = (1 + rho) / rho`. The effect at
/// efficacy one is `B / R`.
fn steady(drive: f64) -> [f64; 3] {
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
    let junction = map.site::<Junction>(JUNCTION, origin()?).id();
    map.contain(root, junction);
    map.landmark(Landmark::RespiratoryPump, junction, junction);
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    for space in [Space::Vesicle, Space::Cleft, Space::Bound] {
        schema.pool::<Acetylcholine>(junction, space)?;
    }
    assemble(schema, |mut body| {
        let (mut ns, ports) = body.nervous()?.parts();
        let class = RateClass::new(RateBand::Reflex, SECOND)?;
        let vagal = ns.algebraic::<Dimensionless>("vagal traffic", Admissible::unit_interval());
        let stated =
            ns.parameter::<Dimensionless>(ParameterKey::new("fixture.vagal"), Admissible::unit_interval());
        let residual = ns.kernel(|mut k| {
            let (held, value) = (k.algebraic(&vagal), k.parameter(&stated));
            let residual = k.subtract(held, value);
            k.finish(residual, ns.every_mode(), class)
        });
        ns.residual(&vagal, residual);
        ng_nervous::breathing(&mut ns, &ports, &vagal)?;
        body.add(ns.finish());
        let fragment = ng_receptor::declare(body.receptor()?)?;
        body.add(fragment);
        let (mut ms, ports) = body.musculoskeletal(None)?.parts();
        ng_musculoskeletal::pump(&mut ms, &ports)?;
        body.add(ms.finish());
        body.compile()
    })
}

fn packet(body: &CompiledBody, case: Case) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    fn set<U: Dimension>(
        packet: &mut ParameterPacket<SyntheticFixture>,
        key: &'static str,
        binding: Binding,
        value: f64,
    ) -> Result<(), AdmissionError> {
        packet.synthetic(ParameterKey::new(key), binding, Measure::<U>::new(value)?, NOTE)
    }
    let mut packet = ParameterPacket::new();
    let p = &mut packet;
    let (all, junctions) = (Binding::Body, Binding::Class(Class::Junction));
    set::<Dimensionless>(p, "fixture.vagal", all, VAGAL)?;
    for (key, value) in [
        ("ns.breath-rate", 0.2),
        ("ns.drive-rise", case.rise),
        ("ns.drive-fall", 2.0),
        ("ns.rest-speed", 1.0),
        ("ns.share-fall", 1.0),
        ("ns.breath-start", case.breath_start),
    ] {
        set::<Rate<Dimensionless>>(p, key, all, value)?;
    }
    for (key, value) in [
        ("ns.inspiration-length", 0.4),
        ("ns.expiration-length", 0.6),
        ("ns.drive-peak", PEAK),
        ("ns.rest-drive", REST_DRIVE),
        ("ns.rest-share", 1.0),
    ] {
        set::<Dimensionless>(p, key, all, value)?;
    }
    set::<Dimensionless>(p, "ns.vagal-share", junctions, 0.0)?;
    set::<Dimensionless>(p, "ns.motor-share", junctions, 1.0)?;
    set::<Rate<Dimensionless>>(p, "ns.release-ach", junctions, RELEASE)?;
    set::<Rate<Dimensionless>>(p, "ns.reuptake-ach", junctions, REUPTAKE)?;
    set::<Acetylcholine>(p, "rc.ach-sites", junctions, SITES)?;
    set::<Affinity<Acetylcholine>>(p, "rc.ach-on", junctions, ON)?;
    set::<Rate<Dimensionless>>(p, "rc.ach-off", junctions, OFF)?;
    set::<Dimensionless>(p, "rc.ach-efficacy", junctions, 1.0)?;
    set::<Pressure>(p, "ms.pump-pressure", all, PIMAX)?;
    let [vesicle, cleft, bound] = steady(REST_DRIVE);
    let guess = |row: RowKind| match row {
        RowKind::Amount {
            space: Space::Vesicle,
            ..
        } => vesicle,
        RowKind::Amount {
            space: Space::Cleft, ..
        } => cleft,
        RowKind::Amount { .. } => bound,
        RowKind::Memory(slot) if slot.law.local == "pump drive" => REST_DRIVE,
        RowKind::Memory(slot) if slot.law.local == "mixing share" => 1.0,
        RowKind::Algebraic(slot) if slot.law.local == "vagal traffic" => VAGAL,
        RowKind::Algebraic(slot) if slot.law.local == "pump_force" => PIMAX * effect(REST_DRIVE),
        RowKind::Algebraic(slot) if slot.law.local == "mixing_share" => 1.0,
        RowKind::Algebraic(slot) if slot.law.owner == Receptor::CARD => effect(REST_DRIVE),
        RowKind::Memory(_) | RowKind::Algebraic(_) => 0.0,
    };
    let breath = LawName {
        owner: Nervous::CARD,
        local: "breath",
    };
    packet.start_mode(breath, "rest", NOTE)?;
    packet.start_rest(body, guess, NOTE)?;
    packet.within(ErrorScales::uniform(Measure::new(1e-12)?, Measure::new(1e-13)?)?);
    Ok(packet)
}

fn run<T>(
    case: Case,
    then: impl FnOnce(&CompiledBody, Session<'_, Running>) -> T,
) -> Result<T, InitializationFailure> {
    let body = compiled()?;
    let core = ng_solver::Core::new(SECOND / 1_000);
    let store = MemoryStore::default();
    let lens = body.lens(Vec::new(), NonZeroU64::MIN.saturating_add(SECOND - 1))?;
    let port = BoundaryPort::new(Vec::new())?;
    let session = admit_solved(&body, packet(&body, case)?, port, &core, &lens, &store)?.session;
    Ok(then(&body, session))
}

fn algebraic(body: &CompiledBody, owner: &str, local: Option<&str>) -> RowId {
    (body.rows().iter())
        .find(|row| {
            matches!(row.kind(), RowKind::Algebraic(slot)
                if slot.law.owner == owner && local.is_none_or(|local| slot.law.local == local))
        })
        .unwrap_or_else(|| panic!("no algebraic row of {owner}"))
        .id()
}

fn now(session: &Session<'_, Running>, row: RowId) -> f64 {
    f64::from_bits(session.record().accepted().value(row).bits()[0])
}

fn until<'b>(session: Session<'b, Running>, seconds: f64) -> Session<'b, Running> {
    let at = SimTime::from_ns((seconds * SECOND as f64) as u64);
    run_until(session, at).unwrap_or_else(|stopped| panic!("{:?}", stopped.fault()))
}

fn close(measured: f64, expected: f64, relative: f64) -> bool {
    (measured - expected).abs() <= relative * expected.abs()
}

/// At a held drive the junction sits where release from the vesicle meets reuptake and binding meets unbinding, so
/// the effect is the closed form's at the pump drive alone, whatever the vagal traffic, and the force is `PImax E`.
#[test]
fn at_a_held_drive_the_pumps_effect_and_force_meet_the_junctions_closed_form()
-> Result<(), InitializationFailure> {
    let expected = effect(REST_DRIVE);
    let readings = run(HELD, |body, session| {
        let (effect_row, force_row) = (
            algebraic(body, Receptor::CARD, None),
            algebraic(body, Musculoskeletal::CARD, Some("pump_force")),
        );
        let first = (0.0, now(&session, effect_row), now(&session, force_row));
        let session = until(session, 1.0);
        [first, (1.0, now(&session, effect_row), now(&session, force_row))]
    })?;
    for (at, measured, force) in readings {
        assert!(
            close(measured, expected, 1e-9),
            "at {at} s the effect is {measured}, the closed form at the held drive {expected}"
        );
        assert!(
            close(force, PIMAX * expected, 1e-9),
            "at {at} s the pump's force is {force}, PImax times the closed form {}",
            PIMAX * expected
        );
    }
    Ok(())
}
/// The breath starts at once and the drive steps to its peak, and the effect follows within a tenth of a second: the
/// junction's slowest mode is about 31 ms, so 0.1 s leaves about 4% of the step. Late in inspiration the drive holds
/// at its peak and the effect meets the closed form there.
#[test]
fn the_pumps_effect_follows_a_step_in_the_drive_within_a_tenth_of_a_second()
-> Result<(), InitializationFailure> {
    let (low, high) = (effect(REST_DRIVE), effect(PEAK));
    let (stepped, late) = run(STEPPED, |body, session| {
        let effect_row = algebraic(body, Receptor::CARD, None);
        let session = until(session, 0.1);
        let stepped = now(&session, effect_row);
        let session = until(session, 1.5);
        (stepped, now(&session, effect_row))
    })?;
    let gap = (stepped - high).abs();
    assert!(
        gap <= 0.1 * (high - low),
        "0.1 s after the step the effect is {gap} from the closed form at the peak, of a step of {}",
        high - low
    );
    assert!(
        close(late, high, 1e-9),
        "late in inspiration the effect is {late}, the closed form at the peak {high}"
    );
    Ok(())
}
