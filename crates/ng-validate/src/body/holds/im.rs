use super::FIXED;
use ng_contract::*;

/// Immunity at rest: no route and no pool, only the coupling fields the census holds (01:475-476).
pub(super) fn immune(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Immune>::new(Vec::new(), Vec::new(), FIXED);
    let (laws, _) = body.hold(declaration)?.parts();
    body.add(laws.finish());
    Ok(())
}
