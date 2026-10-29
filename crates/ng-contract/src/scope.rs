use crate::anatomy::Class;
use crate::{AdmissionError, Anatomy, SiteId};

/// A checked anatomy and the pool registry derived from it.
pub struct Schema {
    anatomy: Anatomy,
}
impl Schema {
    /// Every check ran in `finish`, so a checked anatomy always has a registry; the result type is
    /// the signature walk's.
    pub fn new(anatomy: Anatomy) -> Result<Self, AdmissionError> {
        Ok(Self { anatomy })
    }

    /// One water pool per space of every site, sites in key order and each class's spaces in the
    /// order below.
    pub fn pools(&self) -> impl Iterator<Item = (SiteId, Space)> + '_ {
        let sites = self.anatomy.sites();
        sites.flat_map(|site| Space::of(site.class()).iter().map(move |&space| (site, space)))
    }
}

/// One part of a site that holds a pool. A junction, a tissue site and a nerve segment hold none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    Plasma,
    Interstitium,
    Cells,
    /// A vessel segment's blood.
    Lumen,
    /// An enclosed space's own fluid.
    Fluid,
}
impl Space {
    fn of(class: Class) -> &'static [Space] {
        match class {
            Class::CapillaryBed => &[Space::Plasma, Space::Interstitium, Space::Cells],
            Class::VesselSegment => &[Space::Lumen],
            Class::EnclosedSpace => &[Space::Fluid],
            Class::Junction | Class::TissueSite | Class::NerveSegment => &[],
        }
    }
}
