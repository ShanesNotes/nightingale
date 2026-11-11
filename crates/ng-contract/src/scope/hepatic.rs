use super::{BLOOD_PRESSURE, process_route};
use crate::law::{Brand, Family};
use crate::{
    AdmissionError, Beds, Blood, BoundaryProcess, BoundaryRoute, Cardiovascular, CellFuel, EdgeRoute,
    FragmentBuilder, Hepatic, LawName, Liver, Owner, PerSite, Pressure, ReadPort, SiteRoute, SiteSet, Wall,
    Water,
};
use core::marker::PhantomData;

pub struct HepaticPorts<'b> {
    processes: Vec<&'static str>,
    brand: Brand<'b>,
}
impl<'b> HepaticPorts<'b> {
    pub(super) fn issued(processes: Vec<&'static str>) -> Self {
        Self {
            processes,
            brand: PhantomData,
        }
    }
    /// A fuel across the cell membrane of each liver bed, interstitium to cells.
    pub fn fuel_into_cells<Q: CellFuel>(
        &self,
        laws: &mut FragmentBuilder<'b, Hepatic>,
    ) -> SiteRoute<'b, Hepatic, Beds<Liver>, Q> {
        let family = Family::Across(Wall::Cell, <Beds<Liver> as SiteSet>::SET, Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// Every plain vascular edge with an end at a liver bed.
    pub fn sinusoids(&self, laws: &mut FragmentBuilder<'b, Hepatic>) -> EdgeRoute<'b, Hepatic, Water> {
        laws.request(Family::Sinusoids);
        EdgeRoute::requested(Family::Sinusoids)
    }
    pub fn blood_pressure(&self) -> ReadPort<'b, Hepatic, PerSite<Blood, Pressure>> {
        ReadPort::field(LawName {
            owner: Cardiovascular::CARD,
            local: BLOOD_PRESSURE,
        })
    }
    pub fn boundary<P: BoundaryProcess<Card = Hepatic>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Hepatic>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
