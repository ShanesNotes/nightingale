use super::process_route;
use crate::law::Brand;
use crate::{AdmissionError, BoundaryProcess, BoundaryRoute, Hepatic};
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
    pub fn boundary<P: BoundaryProcess<Card = Hepatic>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Hepatic>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
