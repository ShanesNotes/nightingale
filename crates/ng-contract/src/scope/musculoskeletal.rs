use crate::law::{Brand, Family};
use crate::{Beds, Bone, FragmentBuilder, Mineral, Musculoskeletal, SiteRoute, SiteSet, Wall};
use core::marker::PhantomData;

pub struct MusculoskeletalPorts<'b> {
    brand: Brand<'b>,
}
impl<'b> MusculoskeletalPorts<'b> {
    pub(super) fn issued(_: Vec<&'static str>) -> Self {
        Self { brand: PhantomData }
    }
    /// A mineral across the cell membrane of the bone bed.
    pub fn bone<Q: Mineral>(
        &self,
        laws: &mut FragmentBuilder<'b, Musculoskeletal>,
    ) -> SiteRoute<'b, Musculoskeletal, Beds<Bone>, Q> {
        let family = Family::Across(Wall::Cell, <Beds<Bone> as SiteSet>::SET, Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
}
