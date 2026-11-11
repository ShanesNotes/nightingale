use crate::law::Brand;
use core::marker::PhantomData;

/// Blood chemistry's ports move nothing: its card states fields at sites.
/// ```compile_fail,E0599
/// use ng_contract::*;
/// fn moved<'b>(bc: &mut FragmentBuilder<'b, BloodChemistry>, ports: &BloodChemistryPorts<'b>) {
///     let _ = ports.exchange::<Oxygen>(bc);
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn stated<'b>(bc: &mut FragmentBuilder<'b, BloodChemistry>)
///     -> SiteField<'b, BloodChemistry, CapillaryBed, Dimensionless> {
///     bc.site_field("saturation", Admissible::unit_interval())
/// }
/// ```
pub struct BloodChemistryPorts<'b> {
    brand: Brand<'b>,
}
impl BloodChemistryPorts<'_> {
    pub(super) fn issued() -> Self {
        Self { brand: PhantomData }
    }
}
