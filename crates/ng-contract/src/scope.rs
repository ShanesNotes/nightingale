use crate::anatomy::Class;
use crate::compile::compile;
use crate::identity::{Canonical, Seal};
use crate::law::{Brand, FragmentData};
use crate::{AdmissionError, Anatomy, CompiledBody, Fragment, FragmentBuilder, Nervous, Owner, SiteId};
use core::marker::PhantomData;

pub struct Schema {
    pub(crate) anatomy: Anatomy,
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
impl Seal for Schema {
    fn seal(&self, out: &mut Canonical) {
        self.anatomy.digest().seal(out);
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

/// One composition: the scopes it issued and the fragments added to it, under a brand no other
/// assembly shares, so declarations from two assemblies never mix.
pub struct Assembly<'b> {
    schema: Schema,
    issued: Vec<&'static str>,
    fragments: Vec<FragmentData>,
    brand: Brand<'b>,
}
impl<'b> Assembly<'b> {
    fn issue<O: Owner>(&mut self) -> Result<FragmentBuilder<'b, O>, AdmissionError> {
        if self.issued.contains(&O::CARD) {
            return Err(AdmissionError::ScopeIssuedTwice { owner: O::CARD });
        }
        self.issued.push(O::CARD);
        Ok(FragmentBuilder::new())
    }
    pub fn nervous(&mut self) -> Result<NervousScope<'b>, AdmissionError> {
        Ok(NervousScope {
            laws: self.issue()?,
            ports: NervousPorts { brand: PhantomData },
        })
    }
    pub fn add<O: Owner>(&mut self, fragment: Fragment<'b, O>) {
        self.fragments.push(fragment.data);
    }
    /// Checks the schema and every fragment into one hashed body. A scope issued here whose fragment
    /// was never added is refused, so dropping a builder never drops what it owed.
    pub fn compile(self) -> Result<CompiledBody, AdmissionError> {
        compile(self.schema, self.issued, self.fragments)
    }
}

/// Runs `declare` on a fresh assembly of `schema`. The assembly's brand ends with the closure.
pub fn assemble(
    schema: Schema,
    declare: impl for<'b> FnOnce(Assembly<'b>) -> Result<CompiledBody, AdmissionError>,
) -> Result<CompiledBody, AdmissionError> {
    declare(Assembly {
        schema,
        issued: Vec::new(),
        fragments: Vec::new(),
        brand: PhantomData,
    })
}

macro_rules! scope {
    ($scope:ident, $owner:ident, $ports:ident) => {
        /// Issued once per assembly.
        pub struct $scope<'b> {
            laws: FragmentBuilder<'b, $owner>,
            ports: $ports<'b>,
        }
        impl<'b> $scope<'b> {
            pub fn parts(self) -> (FragmentBuilder<'b, $owner>, $ports<'b>) {
                (self.laws, self.ports)
            }
        }
    };
}
scope!(NervousScope, Nervous, NervousPorts);

pub struct NervousPorts<'b> {
    brand: Brand<'b>,
}
