use crate::law::Brand;
use core::marker::PhantomData;

pub struct HematologyPorts<'b> {
    brand: Brand<'b>,
}
impl HematologyPorts<'_> {
    pub(super) fn issued() -> Self {
        Self { brand: PhantomData }
    }
}
