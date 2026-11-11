use super::process_route;
use crate::law::Brand;
use crate::{AdmissionError, BoundaryProcess, BoundaryRoute, Hematology};
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
    pub fn boundary<P: BoundaryProcess<Card = Hematology>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Hematology>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
