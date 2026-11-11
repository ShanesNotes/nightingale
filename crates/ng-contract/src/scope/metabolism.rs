use crate::law::Brand;
use core::marker::PhantomData;

pub struct MetabolismPorts<'b> {
    brand: Brand<'b>,
}
impl MetabolismPorts<'_> {
    pub(super) fn issued() -> Self {
        Self { brand: PhantomData }
    }
}
