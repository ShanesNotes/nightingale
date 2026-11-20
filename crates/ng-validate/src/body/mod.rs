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
}
pub type BodyCase = AssemblyCase<WholeBody>;

fn exits(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
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
    body.add(laws.finish());
    Ok(())
}

/// The composition root of the whole body: the slice's systems, and every other card held.
pub fn whole_body() -> Result<CompiledBody, AdmissionError> {
    assemble(body_schema()?, |mut body| {
        holds::hold_every_card(&mut body)?;
        exits(&mut body)?;
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
