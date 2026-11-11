use super::process_route;
use crate::law::Brand;
use crate::{AdmissionError, BoundaryProcess, BoundaryRoute, Metabolism};
use core::marker::PhantomData;

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
    pub fn boundary<P: BoundaryProcess<Card = Metabolism>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Metabolism>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
