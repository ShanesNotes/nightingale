use crate::law::{Brand, Family};
use crate::{
    Beds, Bone, Dimensionless, FragmentBuilder, Junction, Mineral, Musculoskeletal, PerSite, ReadPort,
    SiteRoute, SiteSet, Substance, Wall,
};
use core::marker::PhantomData;

pub struct MusculoskeletalPorts<'b> {
    brand: Brand<'b>,
}
impl<'b> MusculoskeletalPorts<'b> {
    pub(super) fn issued(_: Vec<&'static str>) -> Self {
        Self { brand: PhantomData }
    }
    /// A mineral across the cell membrane of the bone bed.
    /// ```compile_fail,E0277
    /// use ng_contract::*;
    /// fn crossed<'b>(ms: &mut FragmentBuilder<'b, Musculoskeletal>, ports: &MusculoskeletalPorts<'b>)
    ///     -> SiteRoute<'b, Musculoskeletal, Beds<Bone>, Sodium> {
    ///     ports.bone::<Sodium>(ms)
    /// }
    /// ```
    /// ```
    /// use ng_contract::*;
    /// fn crossed<'b>(ms: &mut FragmentBuilder<'b, Musculoskeletal>, ports: &MusculoskeletalPorts<'b>)
    ///     -> SiteRoute<'b, Musculoskeletal, Beds<Bone>, Calcium> {
    ///     ports.bone::<Calcium>(ms)
    /// }
    /// ```
    pub fn bone<Q: Mineral>(
        &self,
        laws: &mut FragmentBuilder<'b, Musculoskeletal>,
    ) -> SiteRoute<'b, Musculoskeletal, Beds<Bone>, Q> {
        let family = Family::Across(Wall::Cell, <Beds<Bone> as SiteSet>::SET, Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// A receptor's effect at its junction, the only way a transmitter changes a musculoskeletal law.
    /// ```compile_fail,E0308
    /// use ng_contract::*;
    /// fn read<'b>(ports: &MusculoskeletalPorts<'b>) -> ReadPort<'b, Musculoskeletal, PerSite<Blood, Dimensionless>> {
    ///     ports.effect::<Acetylcholine>()
    /// }
    /// ```
    /// ```
    /// use ng_contract::*;
    /// fn read<'b>(ports: &MusculoskeletalPorts<'b>) -> ReadPort<'b, Musculoskeletal, PerSite<Junction, Dimensionless>> {
    ///     ports.effect::<Acetylcholine>()
    /// }
    /// ```
    pub fn effect<Q: Substance>(&self) -> ReadPort<'b, Musculoskeletal, PerSite<Junction, Dimensionless>> {
        super::effect::<_, _, Q>()
    }
}
