use ng_contract::catalog::{Defines, PumpForce};
use ng_contract::*;

/// The pump force on line 95 is `P_mus = PImax E` for the acetylcholine receptor's effect `E` at the pump's junction
/// (research 11's pump row), where the nervous card's pump drive arrives as release.
pub fn pump<'b>(
    laws: &mut FragmentBuilder<'b, Musculoskeletal>,
    ports: &MusculoskeletalPorts<'b>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Beat, 1_000_000_000)?;
    let effect = ports.effect::<Acetylcholine>();
    let most = laws.parameter::<Pressure>(ParameterKey::new("ms.pump-pressure"), Admissible::nonnegative());
    let line: PumpForce<Defines<'b, Musculoskeletal>> = laws.emit();
    let force = laws.kernel(|mut k| {
        let (most, effect) = (
            k.parameter(&most),
            k.at_landmark(Landmark::RespiratoryPump, &effect),
        );
        let value = k.scale(most, effect);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define(&line.pump_force, force);
    Ok(())
}
