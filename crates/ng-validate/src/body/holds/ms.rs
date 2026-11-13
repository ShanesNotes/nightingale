use super::{FIXED, hours};
use ng_contract::*;

const SKIN_WATER: &str = "skin water";

/// The slice's skin water, which wanders smoothly with time, and its stir, the replay gates' chance fixture
/// (01:651). Muscle and bone's stores and the minerals outside cells wait for tickets 16 and 19.
pub(super) fn musculoskeletal(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Musculoskeletal>::new(vec![SKIN_WATER], Vec::new(), FIXED)
        .pool::<Magnesium, CapillaryBed>(Space::Cells)
        .pool::<Phosphate, CapillaryBed>(Space::Cells)
        .pool::<Calcium, Beds<Bone>>(Space::Cells)
        .pool::<Creatinine, Beds<Muscle>>(Space::Cells)
        .pool::<BodyProtein, Beds<Muscle>>(Space::Cells)
        .pool::<BodyProtein, Beds<Liver>>(Space::Cells)
        .pool::<Glycogen, Beds<Muscle>>(Space::Cells)
        .pool::<AminoAcids, Beds<Muscle>>(Space::Cells)
        .pool::<Creatinine, CapillaryBed>(Space::Interstitium)
        .pool::<Calcium, CapillaryBed>(Space::Interstitium)
        .pool::<Magnesium, CapillaryBed>(Space::Interstitium)
        .pool::<Phosphate, CapillaryBed>(Space::Interstitium);
    let (mut laws, ports) = body.hold(declaration)?.parts();
    let BoundaryRoute { route, .. } = ports.route::<Water>(SKIN_WATER)?;
    let steady = laws.parameter::<Rate<Water>>(ParameterKey::new("ms.skin-water"), Admissible::nonnegative());
    let wandering = laws.forcing(
        "skin water variability",
        ParameterKey::new("ms.skin-water-period"),
        ParameterKey::new("ms.skin-water-amplitude"),
    );
    let class = hours()?;
    let rate = laws.kernel(|mut k| {
        let (value, factor) = (k.parameter(&steady), k.forcing(&wandering));
        let varied = k.scale(value, factor);
        k.finish(varied, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
    let pace =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ms.stir-rate"), Admissible::nonnegative());
    let dice = laws.stream("stir");
    let integral = laws.state::<Dimensionless>("stir hazard", Admissible::nonnegative());
    let [still, stirred] = laws.modes("stirring", ["still", "stirred"]);
    let accruing = laws.kernel(|mut k| {
        let rate = k.parameter(&pace);
        k.finish(rate, still.only(), class)
    });
    let spent = laws.kernel(|mut k| {
        let rate = k.zero();
        k.finish(rate, stirred.only(), class)
    });
    laws.memory_rate(&integral, accruing);
    laws.memory_rate(&integral, spent);
    laws.hazard(HazardGuard::new(&integral, &dice, still.to(&stirred)?));
    body.add(laws.finish());
    Ok(())
}
