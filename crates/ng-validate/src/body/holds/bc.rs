use super::FIXED;
use ng_contract::*;

/// Cell water, sodium and potassium until FL's cell membrane laws (15, 17), the cell anion for good, and the acid
/// books' pools until ticket 18: the diet base and the fixed acid outside the liver's cells, and ammonium and its
/// amino acids in the peritubular bed's cells, whose law is RN's, a slice card that cannot hold.
pub(super) fn blood_chemistry(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<BloodChemistry>::new(Vec::new(), Vec::new(), FIXED)
        .pool::<Water, CapillaryBed>(Space::Cells)
        .pool::<Sodium, CapillaryBed>(Space::Cells)
        .pool::<Potassium, Beds<Muscle>>(Space::Cells)
        .pool::<Potassium, Beds<Fat>>(Space::Cells)
        .pool::<Potassium, Beds<Lung>>(Space::Cells)
        .pool::<Potassium, Beds<Gut>>(Space::Cells)
        .pool::<Potassium, Beds<Glomerulus>>(Space::Cells)
        .pool::<Potassium, Beds<Tubule>>(Space::Cells)
        .pool::<Potassium, Beds<Brain>>(Space::Cells)
        .pool::<Potassium, Beds<Heart>>(Space::Cells)
        .pool::<Potassium, Beds<Liver>>(Space::Cells)
        .pool::<Potassium, Beds<Skin>>(Space::Cells)
        .pool::<Potassium, Beds<Bone>>(Space::Cells)
        .pool::<CellAnion, CapillaryBed>(Space::Cells)
        .pool::<DietBase, CapillaryBed>(Space::Cells)
        .pool::<Ammonium, Beds<Tubule>>(Space::Cells)
        .pool::<AminoAcids, Beds<Tubule>>(Space::Cells)
        .pool::<DietBase, CapillaryBed>(Space::Interstitium)
        .pool::<FixedAcid, CapillaryBed>(Space::Interstitium);
    let (laws, _) = body.hold(declaration)?.parts();
    body.add(laws.finish());
    Ok(())
}
