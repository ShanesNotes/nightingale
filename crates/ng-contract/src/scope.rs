use crate::anatomy::Class;
use crate::compile::compile;
use crate::identity::{Canonical, Seal};
use crate::law::{Brand, Family, FragmentData};
use crate::{
    AdmissionError, Anatomy, Boundary, BoundaryWater, CapillaryBed, Cardiovascular, CompiledBody, EdgeRoute,
    Fluids, Fragment, FragmentBuilder, Measure, Nervous, Owner, PartialVolume, SiteId, SiteRoute, Water,
};
use core::marker::PhantomData;

/// The anatomy and the composition data laws never see: where water crosses the boundary, the
/// partial-volume coefficient and the spaces this body leaves out.
pub struct Schema {
    pub(crate) anatomy: Anatomy,
    pub(crate) boundaries: Vec<WaterBoundary>,
    pub(crate) partial_volume: Option<Measure<PartialVolume<Water>>>,
    pub(crate) left_out: Vec<(Space, &'static str)>,
}
/// A named route by which water crosses the body's boundary at one pool, in order of name.
#[derive(Clone, Copy)]
pub(crate) struct WaterBoundary {
    pub(crate) name: &'static str,
    pub(crate) direction: BoundaryDirection,
    pub(crate) pool: PoolRef,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryDirection {
    IntoBody,
    OutOfBody,
}
impl Schema {
    pub fn new(anatomy: Anatomy) -> Result<Self, AdmissionError> {
        Ok(Self {
            anatomy,
            boundaries: Vec::new(),
            partial_volume: None,
            left_out: Vec::new(),
        })
    }

    /// Names where water crosses the boundary: composition data may name a site, a law never does.
    /// Its law is owed whether or not a boundary scope is issued. Refuses a name declared twice.
    pub fn water_boundary(
        &mut self,
        name: &'static str,
        direction: BoundaryDirection,
        site: SiteId,
        space: Space,
    ) -> Result<(), AdmissionError> {
        match self
            .boundaries
            .binary_search_by_key(&name, |boundary| boundary.name)
        {
            Ok(_) => Err(AdmissionError::BoundaryNamedTwice { name }),
            Err(at) => {
                let pool = PoolRef { site, space };
                self.boundaries.insert(
                    at,
                    WaterBoundary {
                        name,
                        direction,
                        pool,
                    },
                );
                Ok(())
            }
        }
    }

    /// The volume per amount of water, a fixed structural coefficient `Kernel::volume` reads. Refuses
    /// a coefficient that is not positive, and a second one that differs from the first.
    pub fn partial_volume(
        &mut self,
        coefficient: Measure<PartialVolume<Water>>,
    ) -> Result<(), AdmissionError> {
        if coefficient.value() <= 0.0 {
            return Err(AdmissionError::NonPositivePartialVolume);
        }
        if self
            .partial_volume
            .as_ref()
            .is_some_and(|held| held.bits() != coefficient.bits())
        {
            return Err(AdmissionError::PartialVolumeTwice);
        }
        self.partial_volume = Some(coefficient);
        Ok(())
    }

    /// States that this body does not model the space, and why: its pools are not amount rows, and a
    /// law that moves or reads one is refused. Every other pool is an amount some transfer must move.
    pub fn leave_out(&mut self, space: Space, reason: &'static str) {
        self.left_out.push((space, reason));
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
        out.number(self.boundaries.len() as u64);
        for boundary in &self.boundaries {
            out.text(boundary.name);
            out.number(boundary.direction as u64);
            boundary.pool.site.seal(out);
            out.number(boundary.pool.space as u64);
        }
        self.partial_volume.as_ref().map(Measure::bits).seal(out);
        let mut left_out = self.left_out.clone();
        left_out.sort_unstable();
        left_out.dedup();
        out.number(left_out.len() as u64);
        for (space, reason) in left_out {
            out.number(space as u64);
            out.text(reason);
        }
    }
}

/// The registry's handle on one pool. Private: no law can name or hold a pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PoolRef {
    pub(crate) site: SiteId,
    pub(crate) space: Space,
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
    pub fn cardiovascular(&mut self) -> Result<CardiovascularScope<'b>, AdmissionError> {
        Ok(CardiovascularScope {
            laws: self.issue()?,
            ports: CardiovascularPorts { brand: PhantomData },
        })
    }
    pub fn fluids(&mut self) -> Result<FluidsScope<'b>, AdmissionError> {
        Ok(FluidsScope {
            laws: self.issue()?,
            ports: FluidsPorts { brand: PhantomData },
        })
    }
    pub fn boundary(&mut self) -> Result<BoundaryScope<'b>, AdmissionError> {
        Ok(BoundaryScope {
            laws: self.issue()?,
            ports: BoundaryPorts {
                names: self
                    .schema
                    .boundaries
                    .iter()
                    .map(|boundary| boundary.name)
                    .collect(),
                brand: PhantomData,
            },
        })
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
scope!(BoundaryScope, Boundary, BoundaryPorts);
scope!(CardiovascularScope, Cardiovascular, CardiovascularPorts);
scope!(FluidsScope, Fluids, FluidsPorts);
scope!(NervousScope, Nervous, NervousPorts);

pub struct BoundaryPorts<'b> {
    names: Vec<&'static str>,
    brand: Brand<'b>,
}
impl<'b> BoundaryPorts<'b> {
    /// The schema boundary's pair and drive. Refuses a name the schema does not declare.
    pub fn water(&self, name: &'static str) -> Result<BoundaryWater<'b>, AdmissionError> {
        match self.names.binary_search(&name) {
            Ok(_) => Ok(BoundaryWater::named(name)),
            Err(_) => Err(AdmissionError::UnknownBoundary { name }),
        }
    }
}
pub struct CardiovascularPorts<'b> {
    brand: Brand<'b>,
}
impl<'b> CardiovascularPorts<'b> {
    /// Records a law owed for every vascular edge before returning the route, so dropping the route
    /// never drops what the fragment owes.
    pub fn circulation(
        &self,
        laws: &mut FragmentBuilder<'b, Cardiovascular>,
    ) -> EdgeRoute<'b, Cardiovascular, Water> {
        laws.request(Family::Circulation);
        EdgeRoute::requested(Family::Circulation)
    }
}
pub struct FluidsPorts<'b> {
    brand: Brand<'b>,
}
impl<'b> FluidsPorts<'b> {
    /// Records a law owed for every capillary bed before returning the route.
    pub fn starling(
        &self,
        laws: &mut FragmentBuilder<'b, Fluids>,
    ) -> SiteRoute<'b, Fluids, CapillaryBed, Water> {
        laws.request(Family::Starling);
        SiteRoute::requested(Family::Starling)
    }
}
pub struct NervousPorts<'b> {
    brand: Brand<'b>,
}
