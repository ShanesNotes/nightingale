use crate::anatomy::Class;
use crate::{AdmissionError, Anatomy, SiteId};

pub struct Schema {
    anatomy: Anatomy,
}
impl Schema {
    pub fn new(anatomy: Anatomy) -> Result<Self, AdmissionError> {
        Ok(Self { anatomy })
    }

    /// One water pool per space of every site, sites in key order and each class's spaces in the
    /// order [`Space`] declares them.
    pub fn pools(&self) -> impl Iterator<Item = (SiteId, Space)> + '_ {
        let sites = self.anatomy.sites();
        sites.flat_map(|site| Space::of(site.class()).iter().map(move |&space| (site, space)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Space {
    Plasma,
    Interstitium,
    Cells,
    Lumen,
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
