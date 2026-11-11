use crate::law::Brand;
use core::marker::PhantomData;

pub struct ImmunePorts<'b> {
    brand: Brand<'b>,
}
impl ImmunePorts<'_> {
    pub(super) fn issued(_: Vec<&'static str>) -> Self {
        Self { brand: PhantomData }
    }
}
