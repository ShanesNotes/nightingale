use super::{ENCLOSURE_PRESSURE, process_route};
use crate::law::Brand;
use crate::{
    AdmissionError, BoundaryProcess, BoundaryRoute, EnclosedSpace, Output, PerSite, Pressure, Respiratory,
};
use core::marker::PhantomData;

/// Only the respiratory card's ports carry an enclosure's pressure.
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
    /// The pressure of each enclosure the map names the respiratory card on: the pleural space's, from the breath.
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
    /// The route of a boundary process the respiratory card owns. Refuses one the schema does not place, or one its
    /// held declaration leaves to its stand-ins.
    pub fn boundary<P: BoundaryProcess<Card = Respiratory>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Respiratory>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
