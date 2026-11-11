use crate::law::Brand;
use core::marker::PhantomData;

pub struct HepaticPorts<'b> {
    brand: Brand<'b>,
}
impl HepaticPorts<'_> {
    pub(super) fn issued() -> Self {
        Self { brand: PhantomData }
    }
}
