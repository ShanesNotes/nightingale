use super::FIXED;
use ng_contract::*;

constant_routes! {Hematology;
    HemoglobinProduction => "hc.hemoglobin-production",
    HemoglobinDestruction => "hc.hemoglobin-destruction",
}

pub(super) fn hematology(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Hematology>::new(CONSTANT.to_vec(), Vec::new(), FIXED)
        .pool::<Iron, Beds<Liver>>(Space::Cells)
        .pool::<Iron, Beds<Gut>>(Space::Cells)
        .pool::<Iron, Beds<Liver>>(Space::Interstitium)
        .pool::<Iron, Beds<Gut>>(Space::Interstitium);
    let mut held = body.hold(declaration)?;
    let (laws, ports) = held.stand_ins();
    constants(laws, ports)?;
    let production = ports.route::<Hemoglobin>(<HemoglobinProduction as BoundaryProcess>::NAME)?;
    let fragment = ng_hematology::declare(body.hematology(Some(held))?, &production)?;
    body.add(fragment);
    Ok(())
}
