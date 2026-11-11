use super::process_route;
use crate::law::Brand;
use crate::{AdmissionError, BoundaryProcess, BoundaryRoute, Endocrine};
use core::marker::PhantomData;

pub struct EndocrinePorts<'b> {
    processes: Vec<&'static str>,
    brand: Brand<'b>,
}
impl<'b> EndocrinePorts<'b> {
    pub(super) fn issued(processes: Vec<&'static str>) -> Self {
        Self {
            processes,
            brand: PhantomData,
        }
    }
    pub fn boundary<P: BoundaryProcess<Card = Endocrine>>(
        &self,
    ) -> Result<BoundaryRoute<'b, P::Quantity, Endocrine>, AdmissionError> {
        process_route::<P>(&self.processes)
    }
}
