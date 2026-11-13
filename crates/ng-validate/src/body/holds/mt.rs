use super::FIXED;
use ng_contract::*;

constant_routes! {Metabolism;
    SkinEvaporation => "mt.skin-evaporation",
    Sweat<Water> => "mt.sweat-water",
    Sweat<Sodium> => "mt.sweat-sodium",
    Sweat<Chloride> => "mt.sweat-chloride",
    DryHeatLoss => "mt.dry-heat-loss",
    SkinEvaporativeHeat => "mt.skin-evaporative-heat",
}

pub(super) fn metabolism(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Metabolism>::new(CONSTANT.to_vec(), Vec::new(), FIXED)
        .pool::<Oxygen, CapillaryBed>(Space::Interstitium)
        .pool::<CarbonDioxide, CapillaryBed>(Space::Interstitium)
        .pool::<Glucose, CapillaryBed>(Space::Interstitium)
        .pool::<FattyAcids, CapillaryBed>(Space::Interstitium)
        .pool::<Lactate, CapillaryBed>(Space::Interstitium)
        .pool::<Oxygen, CapillaryBed>(Space::Cells)
        .pool::<CarbonDioxide, CapillaryBed>(Space::Cells)
        .pool::<Glucose, Besides<Liver>>(Space::Cells)
        .pool::<FattyAcids, Besides<Liver>>(Space::Cells)
        .pool::<Lactate, Besides<Liver>>(Space::Cells)
        .pool::<Triglyceride, Beds<Fat>>(Space::Cells)
        .pool::<Glycerol, Beds<Fat>>(Space::Cells);
    let (mut laws, ports) = body.hold(declaration)?.parts();
    constants(&mut laws, &ports)?;
    body.add(laws.finish());
    Ok(())
}
