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
    let (mut laws, ports) = body.hold(declaration)?.parts();
    constants(&mut laws, &ports)?;
    body.add(laws.finish());
    Ok(())
}
