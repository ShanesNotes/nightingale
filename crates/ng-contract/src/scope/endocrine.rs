use crate::law::Brand;
use core::marker::PhantomData;

pub struct EndocrinePorts<'b> {
    brand: Brand<'b>,
}
impl EndocrinePorts<'_> {
    pub(super) fn issued() -> Self {
        Self { brand: PhantomData }
    }
}
