use crate::law::Brand;
use core::marker::PhantomData;

pub struct MusculoskeletalPorts<'b> {
    brand: Brand<'b>,
}
impl MusculoskeletalPorts<'_> {
    pub(super) fn issued(_: Vec<&'static str>) -> Self {
        Self { brand: PhantomData }
    }
}
