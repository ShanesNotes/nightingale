use super::process_route;
use crate::law::{Brand, Family};
use crate::{
    AdmissionError, Besides, BoundaryProcess, BoundaryRoute, CapillaryBed, CellFuel, Energy, FatStore,
    FragmentBuilder, HeatRoute, Liver, Metabolism, Respired, SiteRoute, SiteSet, Species, TissueSite,
    Triglyceride, Wall,
};
use core::marker::PhantomData;

/// The metabolism card crosses fuels at every bed but the liver's, which the hepatic card's ports cross.
/// ```compile_fail,E0308
/// use ng_contract::*;
/// fn crossed<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
///     -> SiteRoute<'b, Metabolism, Beds<Liver>, Glucose> {
///     ports.fuel_into_cells::<Glucose>(mt)
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn crossed<'b>(hp: &mut FragmentBuilder<'b, Hepatic>, ports: &HepaticPorts<'b>)
///     -> SiteRoute<'b, Hepatic, Beds<Liver>, Glucose> {
///     ports.fuel_into_cells::<Glucose>(hp)
/// }
/// ```
pub struct MetabolismPorts<'b> {
    processes: Vec<&'static str>,
    brand: Brand<'b>,
}
impl<'b> MetabolismPorts<'b> {
    pub(super) fn issued(processes: Vec<&'static str>) -> Self {
        Self {
            processes,
            brand: PhantomData,
        }
    }
    /// Oxygen or carbon dioxide across the capillary wall of every bed.
    /// ```compile_fail,E0277
    /// use ng_contract::*;
    /// fn crossed<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
    ///     -> SiteRoute<'b, Metabolism, CapillaryBed, Sodium> {
    ///     ports.gas_exchange::<Sodium>(mt)
    /// }
    /// ```
    /// ```
    /// use ng_contract::*;
    /// fn crossed<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
    ///     -> SiteRoute<'b, Metabolism, CapillaryBed, CarbonDioxide> {
    ///     ports.gas_exchange::<CarbonDioxide>(mt)
    /// }
    /// ```
    pub fn gas_exchange<Q: Respired>(
        &self,
        laws: &mut FragmentBuilder<'b, Metabolism>,
    ) -> SiteRoute<'b, Metabolism, CapillaryBed, Q> {
        let family = Family::Exchange(Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// Oxygen or carbon dioxide across the cell membrane of every bed.
    /// ```compile_fail,E0277
    /// use ng_contract::*;
    /// fn crossed<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
    ///     -> SiteRoute<'b, Metabolism, CapillaryBed, Sodium> {
    ///     ports.gas_into_cells::<Sodium>(mt)
    /// }
    /// ```
    /// ```
    /// use ng_contract::*;
    /// fn crossed<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
    ///     -> SiteRoute<'b, Metabolism, CapillaryBed, Oxygen> {
    ///     ports.gas_into_cells::<Oxygen>(mt)
    /// }
    /// ```
    pub fn gas_into_cells<Q: Respired>(
        &self,
        laws: &mut FragmentBuilder<'b, Metabolism>,
    ) -> SiteRoute<'b, Metabolism, CapillaryBed, Q> {
        let family = Family::Across(Wall::Cell, <CapillaryBed as SiteSet>::SET, Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// A fuel across the cell membrane of every bed but the liver's.
    /// ```compile_fail,E0277
    /// use ng_contract::*;
    /// fn crossed<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
    ///     -> SiteRoute<'b, Metabolism, Besides<Liver>, Sodium> {
    ///     ports.fuel_into_cells::<Sodium>(mt)
    /// }
    /// ```
    /// ```
    /// use ng_contract::*;
    /// fn crossed<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
    ///     -> SiteRoute<'b, Metabolism, Besides<Liver>, Glucose> {
    ///     ports.fuel_into_cells::<Glucose>(mt)
    /// }
    /// ```
    pub fn fuel_into_cells<Q: CellFuel>(
        &self,
        laws: &mut FragmentBuilder<'b, Metabolism>,
    ) -> SiteRoute<'b, Metabolism, Besides<Liver>, Q> {
        let family = Family::Across(Wall::Cell, <Besides<Liver> as SiteSet>::SET, Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// Triglyceride from a store bed's plasma into its cells, past the interstitium.
    /// ```compile_fail,E0277
    /// use ng_contract::*;
    /// fn stored<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
    ///     -> SiteRoute<'b, Metabolism, Beds<Liver>, Triglyceride> {
    ///     ports.fat_into_cells::<Beds<Liver>>(mt)
    /// }
    /// ```
    /// ```
    /// use ng_contract::*;
    /// fn stored<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
    ///     -> SiteRoute<'b, Metabolism, Beds<Fat>, Triglyceride> {
    ///     ports.fat_into_cells::<Beds<Fat>>(mt)
    /// }
    /// ```
    pub fn fat_into_cells<C: FatStore>(
        &self,
        laws: &mut FragmentBuilder<'b, Metabolism>,
    ) -> SiteRoute<'b, Metabolism, C, Triglyceride> {
        let family = Family::Across(Wall::Endothelium, C::SET, Species::Triglyceride);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// Each bed's heat to the core store.
    pub fn bed_heat(
        &self,
        laws: &mut FragmentBuilder<'b, Metabolism>,
    ) -> SiteRoute<'b, Metabolism, CapillaryBed, Energy> {
        let family = Family::Heat(HeatRoute::FromBeds);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// The core store to each other tissue site's store.
    pub fn conduction(
        &self,
        laws: &mut FragmentBuilder<'b, Metabolism>,
    ) -> SiteRoute<'b, Metabolism, TissueSite, Energy> {
        let family = Family::Heat(HeatRoute::Conduction);
        laws.request(family);
        SiteRoute::requested(family)
    }
    pub fn boundary<P: BoundaryProcess<Card = Metabolism>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Metabolism>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
