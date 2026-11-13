use super::FIXED;
use ng_contract::*;

pub(super) fn immune(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Immune>::new(Vec::new(), Vec::new(), FIXED);
    let (laws, _) = body.hold(declaration)?.parts();
    body.add(laws.finish());
    Ok(())
}
