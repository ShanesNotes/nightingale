use super::process_route;
use crate::law::{Brand, Family};
use crate::{
    AdmissionError, BoundaryProcess, BoundaryRoute, CapillaryBed, FragmentBuilder, Hematology, Iron,
    SiteRoute, SiteSet, Species, Wall,
};
use core::marker::PhantomData;

pub struct HematologyPorts<'b> {
    processes: Vec<&'static str>,
    brand: Brand<'b>,
}
impl<'b> HematologyPorts<'b> {
    pub(super) fn issued(processes: Vec<&'static str>) -> Self {
        Self {
            processes,
            brand: PhantomData,
        }
    }
    /// Iron across the cell membrane of each bed whose interstitium and cells hold iron: the liver's store and the
    /// gut's cells, where body 01's schema puts iron.
    pub fn iron_into_cells(
        &self,
        laws: &mut FragmentBuilder<'b, Hematology>,
    ) -> SiteRoute<'b, Hematology, CapillaryBed, Iron> {
        let family = Family::Across(Wall::Cell, <CapillaryBed as SiteSet>::SET, Species::Iron);
        laws.request(family);
        SiteRoute::requested(family)
    }
    pub fn boundary<P: BoundaryProcess<Card = Hematology>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Hematology>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
