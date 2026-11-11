use crate::law::Brand;
use core::marker::PhantomData;

pub struct GastrointestinalPorts<'b> {
    brand: Brand<'b>,
}
impl GastrointestinalPorts<'_> {
    pub(super) fn issued() -> Self {
        Self { brand: PhantomData }
    }
}
