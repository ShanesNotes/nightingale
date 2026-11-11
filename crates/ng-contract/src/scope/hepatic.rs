use super::process_route;
use crate::law::{Brand, Family};
use crate::{
    AdmissionError, Beds, BoundaryProcess, BoundaryRoute, CellFuel, FragmentBuilder, Hepatic, Liver,
    SiteRoute, SiteSet, Wall,
};
use core::marker::PhantomData;

/// Only the hepatic card reacts at the liver bed; the musculoskeletal card reacts at the muscle bed, and the renal card
/// at the peritubular bed.
/// ```compile_fail,E0277
/// use ng_contract::*;
/// fn ungranted<'b>(ms: &mut FragmentBuilder<'b, Musculoskeletal>, store: Proportions<Glucose>)
///     -> Reaction<'b, Musculoskeletal, Beds<Liver>, Glucose> {
///     ms.reaction::<Beds<Liver>, Glucose>("glycogen made", store)
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn liver<'b>(hp: &mut FragmentBuilder<'b, Hepatic>, store: Proportions<Glucose>)
///     -> Reaction<'b, Hepatic, Beds<Liver>, Glucose> {
///     hp.reaction("glycogen made", store)
/// }
/// fn muscle<'b>(ms: &mut FragmentBuilder<'b, Musculoskeletal>, store: Proportions<Glucose>)
///     -> Reaction<'b, Musculoskeletal, Beds<Muscle>, Glucose> {
///     ms.reaction("glycogen made", store)
/// }
/// fn tubule<'b>(rn: &mut FragmentBuilder<'b, Renal>, used: Proportions<AminoAcids>)
///     -> Reaction<'b, Renal, Beds<Tubule>, AminoAcids> {
///     rn.reaction("ammonium made", used)
/// }
/// ```
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
    /// A fuel across the cell membrane of each liver bed, interstitium to cells: the glucose hub's crossings.
    pub fn fuel_into_cells<Q: CellFuel>(
        &self,
        laws: &mut FragmentBuilder<'b, Hepatic>,
    ) -> SiteRoute<'b, Hepatic, Beds<Liver>, Q> {
        let family = Family::Across(Wall::Cell, <Beds<Liver> as SiteSet>::SET, Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    pub fn boundary<P: BoundaryProcess<Card = Hepatic>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Hepatic>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
