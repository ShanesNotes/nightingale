use super::process_route;
use crate::law::Brand;
use crate::{AdmissionError, BoundaryProcess, BoundaryRoute, Gastrointestinal};
use core::marker::PhantomData;

pub struct GastrointestinalPorts<'b> {
    processes: Vec<&'static str>,
    brand: Brand<'b>,
}
impl<'b> GastrointestinalPorts<'b> {
    pub(super) fn issued(processes: Vec<&'static str>) -> Self {
        Self {
            processes,
            brand: PhantomData,
        }
    }
    pub fn boundary<P: BoundaryProcess<Card = Gastrointestinal>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Gastrointestinal>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
