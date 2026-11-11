use super::{ENCLOSURE_PRESSURE, process_route};
use crate::law::{Brand, Family};
use crate::{
    AdmissionError, AirwayEdge, Beds, BoundaryProcess, BoundaryRoute, Breathed, EdgeRoute, EnclosedSpace,
    FragmentBuilder, Lung, Output, Passage, PerSite, Pressure, Respiratory, Respired, SiteRoute, SiteSet,
    Species, Wall, Water,
};
use core::marker::PhantomData;

/// ```compile_fail,E0609
/// use ng_contract::*;
/// fn defined<'b>(
///     gi: &mut FragmentBuilder<'b, Gastrointestinal>,
///     ports: &GastrointestinalPorts<'b>,
///     pressure: PureKernel<'b, Gastrointestinal, Pressure, EnclosedSpace>,
/// ) {
///     gi.define_at(&ports.enclosure_pressure, pressure);
/// }
/// ```
/// ```
/// use ng_contract::*;
/// fn defined<'b>(
///     rs: &mut FragmentBuilder<'b, Respiratory>,
///     ports: &RespiratoryPorts<'b>,
///     pressure: PureKernel<'b, Respiratory, Pressure, EnclosedSpace>,
/// ) {
///     rs.define_at(&ports.enclosure_pressure, pressure);
/// }
/// ```
pub struct RespiratoryPorts<'b> {
    /// The pressure of each enclosure the map names the respiratory card on.
    pub enclosure_pressure: Output<'b, Respiratory, PerSite<EnclosedSpace, Pressure>>,
    processes: Vec<&'static str>,
    brand: Brand<'b>,
}
impl<'b> RespiratoryPorts<'b> {
    pub(super) fn issued(processes: Vec<&'static str>) -> Self {
        Self {
            enclosure_pressure: Output::named(ENCLOSURE_PRESSURE),
            processes,
            brand: PhantomData,
        }
    }
    /// Gas to plasma across the gas-exchange surface of every lung bed; carbon dioxide leaves at a negative rate.
    pub fn alveolar<Q: Respired>(
        &self,
        laws: &mut FragmentBuilder<'b, Respiratory>,
    ) -> SiteRoute<'b, Respiratory, Beds<Lung>, Q> {
        let family = Family::Across(Wall::Alveolar, <Beds<Lung> as SiteSet>::SET, Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// `Q` along every airway edge toward the alveolar gas: a breath in, or out at a negative rate.
    pub fn airway<Q: Breathed>(
        &self,
        laws: &mut FragmentBuilder<'b, Respiratory>,
    ) -> EdgeRoute<'b, Respiratory, Q, AirwayEdge> {
        let family = Family::Along(Passage::Airway, Q::SPECIES);
        laws.request(family);
        EdgeRoute::requested(family)
    }
    /// Water from each lung bed's interstitium into its gas: what humidifies a breath.
    pub fn lining(
        &self,
        laws: &mut FragmentBuilder<'b, Respiratory>,
    ) -> SiteRoute<'b, Respiratory, Beds<Lung>, Water> {
        let family = Family::Across(Wall::Lining, <Beds<Lung> as SiteSet>::SET, Species::Water);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// The route of a boundary process the respiratory card owns. Refuses one the schema does not place, or one its
    /// held declaration leaves to its stand-ins.
    pub fn boundary<P: BoundaryProcess<Card = Respiratory>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Respiratory>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
