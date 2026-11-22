use super::{CORE_TEMPERATURE, TISSUE_TEMPERATURE, process_route};
use crate::catalog::{CoreTemperature, Defines};
use crate::law::{Brand, Family, Over, StoreHeat, StoreTemperature};
use crate::{
    AdmissionError, Besides, BoundaryProcess, BoundaryRoute, CapillaryBed, CellFuel, Energy, FatStore,
    FragmentBuilder, HeatRoute, LawName, Liver, Metabolism, Output, Owner, PerSite, ReadPort, Respired,
    SiteField, SiteRoute, SiteSet, Species, Temperature, TissueSite, Triglyceride, Wall,
};
use core::marker::PhantomData;

/// MT's tissue heat stores: the core store's temperature as arrow 91's scalar, every other store's at its site, and
/// the store heats MT defines them from. Each is declared once, so a fragment that declares one again is refused.
///
/// A store's temperature is a temperature: a law that gives it the store's heat is refused.
/// ```compile_fail,E0308
/// use ng_contract::*;
/// fn wrong<'b>(
///     t: &TissueTemperatures<'b>,
///     mt: &mut FragmentBuilder<'b, Metabolism>,
///     v: PureKernel<'b, Metabolism, Energy, TissueSite>,
/// ) {
///     mt.define_at(&t.other.0, v);
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn defined<'b>(
///     t: &TissueTemperatures<'b>,
///     mt: &mut FragmentBuilder<'b, Metabolism>,
///     v: PureKernel<'b, Metabolism, Temperature, TissueSite>,
/// ) {
///     mt.define_at(&t.other.0, v);
/// }
/// ```
#[non_exhaustive]
pub struct TissueTemperatures<'b> {
    pub core: (
        Output<'b, Metabolism, Temperature>,
        ReadPort<'b, Metabolism, Temperature>,
    ),
    pub other: SiteField<'b, Metabolism, TissueSite, Temperature>,
    pub core_heat: ReadPort<'b, Metabolism, Energy>,
    pub tissue_heat: ReadPort<'b, Metabolism, PerSite<TissueSite, Energy>>,
}
impl<'b> TissueTemperatures<'b> {
    /// The temperature of the store a bed sits in: the core store's where no other store holds the bed.
    pub fn at_bed(&self) -> ReadPort<'b, Metabolism, PerSite<CapillaryBed, Temperature>> {
        ReadPort::temperature(StoreTemperature::OwnBed)
    }
    /// The temperature of the store at an MT heat exit's inside thermal end. Compile refuses an exit at any other
    /// pool. The exit is MT's own and moves energy.
    /// ```compile_fail,E0308
    /// use ng_contract::*;
    /// fn wrong<'b>(t: &TissueTemperatures<'b>, w: &BoundaryRoute<'b, Water, Metabolism>) {
    ///     let _ = t.at_exit(w);
    /// }
    /// ```
    /// ```compile_fail,E0308
    /// use ng_contract::*;
    /// fn wrong<'b>(t: &TissueTemperatures<'b>, r: &BoundaryRoute<'b, Energy, Respiratory>) {
    ///     let _ = t.at_exit(r);
    /// }
    /// ```
    /// ```
    /// use ng_contract::*;
    /// fn read<'b>(t: &TissueTemperatures<'b>, e: &BoundaryRoute<'b, Energy, Metabolism>) {
    ///     let _ = t.at_exit(e);
    /// }
    /// ```
    pub fn at_exit(
        &self,
        exit: &BoundaryRoute<'b, Energy, Metabolism>,
    ) -> ReadPort<'b, Metabolism, Temperature> {
        ReadPort::temperature(StoreTemperature::OwnExit(exit.name()))
    }
}

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
    /// ```compile_fail,E0308
    /// use ng_contract::*;
    /// fn crossed<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>)
    ///     -> SiteRoute<'b, Metabolism, Beds<Liver>, Glucose> {
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
    /// Declares the core store's temperature through arrow 91 and every other tissue heat store's at its site, each
    /// owing MT's law, with the store heats that law reads. The store sites come from the schema's pools, whatever
    /// transfer laws the fragment asks for. Only MT declares them.
    /// ```compile_fail,E0599
    /// use ng_contract::*;
    /// fn wrong<'b>(p: &NervousPorts<'b>, ns: &mut FragmentBuilder<'b, Nervous>) {
    ///     let _ = p.temperatures(ns);
    /// }
    /// ```
    /// ```
    /// use ng_contract::*;
    /// fn declared<'b>(p: &MetabolismPorts<'b>, mt: &mut FragmentBuilder<'b, Metabolism>) -> TissueTemperatures<'b> {
    ///     p.temperatures(mt)
    /// }
    /// ```
    pub fn temperatures(&self, laws: &mut FragmentBuilder<'b, Metabolism>) -> TissueTemperatures<'b> {
        let core: CoreTemperature<Defines<'b, Metabolism>> = laws.emit();
        laws.owe_defined::<Temperature>(TISSUE_TEMPERATURE, Over::OtherTissueHeatStores);
        let owner = Metabolism::CARD;
        TissueTemperatures {
            core: (
                core.core_temperature,
                ReadPort::field(LawName {
                    owner,
                    local: CORE_TEMPERATURE,
                }),
            ),
            other: (
                Output::named(TISSUE_TEMPERATURE),
                ReadPort::field(LawName {
                    owner,
                    local: TISSUE_TEMPERATURE,
                }),
            ),
            core_heat: ReadPort::heat(StoreHeat::Root),
            tissue_heat: ReadPort::heat(StoreHeat::Here),
        }
    }
    pub fn boundary<P: BoundaryProcess<Card = Metabolism>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Metabolism>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
