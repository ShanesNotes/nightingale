use crate::law::{Brand, ExplicitField, SiteField};
use crate::{
    Admissible, BloodChemistry, CarbonDioxide, Concentration, Dimension, Dimensionless, FragmentBuilder,
    PerSite, Pressure, ReadPort, Speciated, Substance,
};
use core::marker::PhantomData;

mod quantity {
    pub trait Sealed {}
}
/// One quantity speciation states in a speciated site's blood. The list is closed, so a reader's row names only these,
/// in their own units, and blood chemistry states each once.
pub trait Speciation: quantity::Sealed {
    /// The local name of blood chemistry's row or row-free field.
    const NAME: &'static str;
    type Unit: Dimension;
}
macro_rules! speciation {
    ($($(#[$doc:meta])* $name:ident = $local:literal, $unit:ty;)*) => {$(
        $(#[$doc])*
        pub enum $name {}
        impl quantity::Sealed for $name {}
        impl Speciation for $name {
            const NAME: &'static str = $local;
            type Unit = $unit;
        }
    )*};
}
speciation! {
    /// A row.
    OxygenPartialPressure = "oxygen partial pressure", Pressure;
    /// A row, from 0 to 1.
    OxygenSaturation = "oxygen saturation", Dimensionless;
    /// A row.
    Bicarbonate = "bicarbonate", Concentration<CarbonDioxide>;
    /// A row.
    Acidity = "acidity", Dimensionless;
    /// A row-free field, written into each program that reads it.
    CarbonDioxidePartialPressure = "carbon dioxide partial pressure", Pressure;
}

/// What blood chemistry states at every speciated site: four rows and one row-free field, each owed one law, with
/// blood chemistry's own read of each.
pub struct SpeciationOutputs<'b> {
    pub oxygen_partial_pressure: SiteField<'b, BloodChemistry, Speciated, Pressure>,
    pub oxygen_saturation: SiteField<'b, BloodChemistry, Speciated, Dimensionless>,
    pub bicarbonate: SiteField<'b, BloodChemistry, Speciated, Concentration<CarbonDioxide>>,
    pub acidity: SiteField<'b, BloodChemistry, Speciated, Dimensionless>,
    pub carbon_dioxide_partial_pressure: ExplicitField<'b, BloodChemistry, Speciated, Pressure>,
}

/// Blood chemistry reads the blood and states speciation. Its ports move nothing.
/// ```compile_fail,E0599
/// use ng_contract::*;
/// fn moved<'b>(bc: &mut FragmentBuilder<'b, BloodChemistry>, ports: &BloodChemistryPorts<'b>) {
///     let _ = ports.exchange::<Oxygen>(bc);
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn read<'b>(ports: &BloodChemistryPorts<'b>) -> ReadPort<'b, BloodChemistry, PerSite<Speciated, Oxygen>> {
///     ports.blood::<Oxygen>()
/// }
/// ```
/// Only blood chemistry states speciation: no other card's ports issue it.
/// ```compile_fail,E0599
/// use ng_contract::*;
/// fn stated<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>) {
///     let _ = ports.speciation(mt);
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn stated<'b>(bc: &mut FragmentBuilder<'b, BloodChemistry>, ports: &BloodChemistryPorts<'b>) -> SpeciationOutputs<'b> {
///     ports.speciation(bc)
/// }
/// ```
/// And no other card's builder defines what blood chemistry's port issued.
/// ```compile_fail,E0308
/// use ng_contract::*;
/// fn defined<'b>(
///     mt: &mut FragmentBuilder<'b, Metabolism>,
///     outputs: &SpeciationOutputs<'b>,
///     acidity: PureKernel<'b, Metabolism, Dimensionless, Speciated>,
/// ) {
///     mt.define_at(&outputs.acidity.0, acidity);
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn defined<'b>(
///     bc: &mut FragmentBuilder<'b, BloodChemistry>,
///     outputs: &SpeciationOutputs<'b>,
///     acidity: PureKernel<'b, BloodChemistry, Dimensionless, Speciated>,
/// ) {
///     bc.define_at(&outputs.acidity.0, acidity);
/// }
/// ```
pub struct BloodChemistryPorts<'b> {
    brand: Brand<'b>,
}
impl<'b> BloodChemistryPorts<'b> {
    pub(super) fn issued(_: Vec<&'static str>) -> Self {
        Self { brand: PhantomData }
    }
    /// The blood's amount of `Q` at every speciated site: a capillary bed's plasma, or a sampled segment's lumen.
    pub fn blood<Q: Substance>(&self) -> ReadPort<'b, BloodChemistry, PerSite<Speciated, Q>> {
        ReadPort::pool(Q::SPECIES)
    }
    /// Speciation's four rows and its row-free field at every speciated site, under the contract's names. Called
    /// twice, it names each twice, which compile refuses (`NamedTwice`).
    pub fn speciation(&self, laws: &mut FragmentBuilder<'b, BloodChemistry>) -> SpeciationOutputs<'b> {
        SpeciationOutputs {
            oxygen_partial_pressure: row::<OxygenPartialPressure>(laws, Admissible::positive()),
            oxygen_saturation: row::<OxygenSaturation>(laws, Admissible::unit_interval()),
            bicarbonate: row::<Bicarbonate>(laws, Admissible::positive()),
            acidity: row::<Acidity>(laws, Admissible::finite()),
            carbon_dioxide_partial_pressure: laws.explicit_field(CarbonDioxidePartialPressure::NAME),
        }
    }
}

fn row<'b, Q: Speciation>(
    laws: &mut FragmentBuilder<'b, BloodChemistry>,
    domain: Admissible<Q::Unit>,
) -> SiteField<'b, BloodChemistry, Speciated, Q::Unit> {
    laws.site_field(Q::NAME, domain)
}
