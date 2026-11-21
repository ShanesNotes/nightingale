mod anatomy;
mod holds;
mod keep_back;
mod packet;
mod schema;
mod table;

pub use anatomy::body_anatomy;
pub use keep_back::{FluidRoute, KEEP_BACKS, KeepBack};
pub use packet::{
    BodyBreath, body_breath, body_certificate, body_lens, body_packet, body_packet_at_viscosity_exponent,
    body_packet_with_the_drive_scale_held, body_start_guess,
};
pub use schema::body_schema;
pub use table::{HOLD_TABLE, Held, Hold, Release};

use crate::{AssemblyCase, CaseAssembly};
use ng_contract::*;
use schema::{CARBON_LEAK, NITROGEN_EXCHANGE, NITROGEN_LEAK, OXYGEN_LEAK, VASOPRESSIN_LEAK};

pub struct WholeBody;
impl CaseAssembly for WholeBody {
    fn compiled() -> Result<CompiledBody, AdmissionError> {
        whole_body()
    }
    fn packet(body: &CompiledBody) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        body_packet(body)
    }
    fn lens(body: &CompiledBody) -> Result<ObservationLens, AdmissionError> {
        body_lens(body)
    }
    fn anchored(at: SimTime) -> Vec<BoundaryEvent> {
        vec![body_awake(at)]
    }
}
pub type BodyCase = AssemblyCase<WholeBody>;

/// The body reads sleep, so every admission of it states sleep at the anchor, or admission refuses it as unstated.
pub fn body_awake(at: SimTime) -> BoundaryEvent {
    let id = InputId::new(Digest::of_canonical(b"awake at the anchor"));
    BoundaryEvent::new(id, at, BoundaryInput::Sleep(Sleep::Awake))
}

fn exits(body: &mut Assembly<'_>, leaking: bool) -> Result<(), AdmissionError> {
    let (mut laws, ports) = body.boundary()?.parts();
    let urine = ports.route::<Water>("urine")?.route;
    let rest = laws.parameter::<Water>(ParameterKey::new("bd.bladder-rest"), Admissible::nonnegative());
    let voiding =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.bladder-speed"), Admissible::positive());
    let width = laws.parameter::<Rate<Water>>(ParameterKey::new("bd.bladder-width"), Admissible::positive());
    let heat = ports.route::<Energy>("heat")?.route;
    let loss =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.heat-speed"), Admissible::positive());
    let class = RateClass::new(RateBand::Hours, 3_600_000_000_000)?;
    let rate = laws.kernel(|mut k| {
        let (held, floor, pace) = (
            k.read(&urine.amount::<Water>()),
            k.parameter(&rest),
            k.parameter(&voiding),
        );
        let draining = k.relax(held, floor, pace);
        let (zero, soft) = (k.zero(), k.parameter(&width));
        let rate = k.smooth_max(draining, zero, soft);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&urine, rate));
    let rate = laws.kernel(|mut k| {
        let (held, zero, pace) = (k.read(&heat.amount::<Energy>()), k.zero(), k.parameter(&loss));
        let rate = k.relax(held, zero, pace);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&heat, rate));
    if leaking {
        leak::<Oxygen>(&mut laws, &ports, (OXYGEN_LEAK, "bd.oxygen-leak"), class)?;
        leak::<CarbonDioxide>(&mut laws, &ports, (CARBON_LEAK, "bd.carbon-dioxide-leak"), class)?;
        leak::<NitrogenGas>(&mut laws, &ports, (NITROGEN_LEAK, "bd.nitrogen-gas-leak"), class)?;
        leak::<Vasopressin>(
            &mut laws,
            &ports,
            (VASOPRESSIN_LEAK, "bd.vasopressin-leak"),
            class,
        )?;
        nitrogen_gas_exchange(&mut laws, &ports, class)?;
    }
    body.add(laws.finish());
    Ok(())
}

fn leak<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    (route, key): (&'static str, &'static str),
    class: RateClass,
) -> Result<(), AdmissionError> {
    let route = ports.route::<Q>(route)?.route;
    let rate = laws.parameter::<Rate<Q>>(ParameterKey::new(key), Admissible::finite());
    let kernel = laws.kernel(|mut k| {
        let (rate, open) = (k.parameter(&rate), k.room_humidity());
        let value = k.scale(rate, open);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, kernel));
    Ok(())
}

/// The fat bed's gas holds nitrogen gas for its leak alone. An exchange slow beside the leak holds it at its rest, so
/// admission solves its level.
fn nitrogen_gas_exchange<'b>(
    laws: &mut FragmentBuilder<'b, Boundary>,
    ports: &BoundaryPorts<'b>,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let route = ports.route::<NitrogenGas>(NITROGEN_EXCHANGE)?.route;
    let rest =
        laws.parameter::<NitrogenGas>(ParameterKey::new("bd.nitrogen-gas-rest"), Admissible::positive());
    let pace = laws
        .parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.nitrogen-gas-pace"), Admissible::positive());
    let kernel = laws.kernel(|mut k| {
        let (rest, held, pace) = (
            k.parameter(&rest),
            k.read(&route.amount::<NitrogenGas>()),
            k.parameter(&pace),
        );
        let rate = k.relax(rest, held, pace);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, kernel));
    Ok(())
}

/// The composition root of the whole body: the slice's systems, and every other card held.
pub fn whole_body() -> Result<CompiledBody, AdmissionError> {
    assembled(body_schema()?, false)
}

/// The whole body with four steady leaks for its certificate's tests, each into the body at the signed rate its
/// parameter states times the room's humidity, so a room of humidity zero shuts them and one of humidity one opens
/// them: `bd.oxygen-leak` and `bd.carbon-dioxide-leak` into the arteries' blood, `bd.nitrogen-gas-leak` into the fat
/// bed's gas, which holds nitrogen gas for it alone at `bd.nitrogen-gas-rest`, and `bd.vasopressin-leak` into the
/// central veins. Nothing else reads the room.
pub fn whole_body_leaking() -> Result<CompiledBody, AdmissionError> {
    let mut schema = body_schema()?;
    schema::leaks(&mut schema)?;
    assembled(schema, true)
}

fn assembled(schema: Schema, leaking: bool) -> Result<CompiledBody, AdmissionError> {
    assemble(schema, |mut body| {
        holds::hold_every_card(&mut body)?;
        exits(&mut body, leaking)?;
        let fragment = ng_cardiovascular::declare_in_the_whole_body(body.cardiovascular()?)?;
        body.add(fragment);
        let fragment = ng_fluids::declare(body.fluids()?)?;
        body.add(fragment);
        let fragment = ng_renal::declare(body.renal()?)?;
        body.add(fragment);
        let fragment = ng_nervous::declare_in_the_whole_body(body.nervous()?)?;
        body.add(fragment);
        let fragment = ng_receptor::declare(body.receptor()?)?;
        body.add(fragment);
        body.compile()
    })
}
