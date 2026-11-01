use crate::anatomy::Class;
use crate::compile::compile;
use crate::identity::{Canonical, Seal};
use crate::law::{Brand, Family, FragmentData};
use crate::{
    AdmissionError, Anatomy, Boundary, BoundaryWater, CapillaryBed, Cardiovascular, CompiledBody, EdgeRoute,
    EnclosedSpace, Fluids, Fragment, FragmentBuilder, Measure, Nervous, Output, OutsideSlice, Owner,
    PartialVolume, PerSite, Pressure, ReadPort, SiteId, SiteRoute, Water,
};
use core::marker::PhantomData;

/// The local name of the row each enclosure a card holds carries its pressure on.
pub(crate) const ENCLOSURE_PRESSURE: &str = "enclosure_pressure";

/// The anatomy and the composition data laws never see: where water crosses the boundary, the
/// partial-volume coefficient and the spaces this body leaves out.
pub struct Schema {
    pub(crate) anatomy: Anatomy,
    pub(crate) boundaries: Vec<WaterBoundary>,
    pub(crate) partial_volume: Option<Measure<PartialVolume<Water>>>,
    pub(crate) left_out: Vec<(Space, &'static str)>,
}
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
        let Self {
            anatomy,
            boundaries,
            partial_volume,
            left_out,
        } = self;
        anatomy.digest().seal(out);
        out.length(boundaries.len());
        for WaterBoundary {
            name,
            direction,
            pool: PoolRef { site, space },
        } in boundaries
        {
            out.text(name);
            out.number(*direction as u64);
            site.seal(out);
            out.number(*space as u64);
        }
        partial_volume.as_ref().map(Measure::bits).seal(out);
        let mut left_out = left_out.clone();
        left_out.sort_unstable();
        left_out.dedup();
        out.length(left_out.len());
        for (space, reason) in left_out {
            out.number(space as u64);
            out.text(reason);
        }
    }
}

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

pub struct Assembly<'b> {
    schema: Schema,
    issued: Vec<&'static str>,
    held: Vec<HeldCard>,
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
            ports: FluidsPorts {
                extravascular_pressure: ReadPort::extravascular(),
                brand: PhantomData,
            },
        })
    }
    fn boundary_names(&self) -> Vec<&'static str> {
        self.schema
            .boundaries
            .iter()
            .map(|boundary| boundary.name)
            .collect()
    }
    pub fn boundary(&mut self) -> Result<BoundaryScope<'b>, AdmissionError> {
        Ok(BoundaryScope {
            laws: self.issue()?,
            ports: BoundaryPorts {
                names: self.boundary_names(),
                brand: PhantomData,
            },
        })
    }
    /// Scope for a card outside the slice, held by a declaration of what it supplies. Refuses a route the
    /// schema does not name, and one another card declared.
    pub fn hold<S: OutsideSlice>(
        &mut self,
        declaration: HeldDeclaration<S>,
    ) -> Result<HeldScope<'b, S>, AdmissionError> {
        let names = self.boundary_names();
        let HeldDeclaration {
            mut supplies,
            mut holds,
            validity,
            card: _,
        } = declaration;
        supplies.sort_unstable();
        supplies.dedup();
        holds.sort_unstable();
        holds.dedup();
        for &name in &supplies {
            if names.binary_search(&name).is_err() {
                return Err(AdmissionError::UnknownBoundary { name });
            }
            if self.held.iter().any(|held| held.supplies.contains(&name)) {
                return Err(AdmissionError::BoundaryNamedTwice { name });
            }
        }
        let mut laws: FragmentBuilder<'b, S> = self.issue()?;
        if self
            .schema
            .anatomy
            .held_enclosures()
            .any(|(_, card)| card == S::CARD)
        {
            laws.owe_enclosure_pressure();
        }
        self.held.push(HeldCard {
            card: S::CARD,
            supplies,
            holds,
            validity,
        });
        Ok(HeldScope {
            laws,
            ports: HeldPorts {
                enclosure_pressure: Output::named(ENCLOSURE_PRESSURE),
                names,
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
        compile(self.schema, self.issued, self.held, self.fragments)
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
        held: Vec::new(),
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
    /// Each bed's pressure from outside: its enclosure's, or zero gauge for a bed nothing encloses.
    pub extravascular_pressure: ReadPort<'b, Fluids, PerSite<CapillaryBed, Pressure>>,
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

/// What a held stand-in answers for. The rest of what it does shows in its fragment: a field it emits, an
/// enclosure the map says it holds, a row it reads.
pub struct HeldDeclaration<S> {
    supplies: Vec<&'static str>,
    holds: Vec<Space>,
    validity: &'static str,
    card: PhantomData<S>,
}
impl<S: OutsideSlice> HeldDeclaration<S> {
    /// `supplies` names every schema boundary this card moves water on: compile refuses a law on any other,
    /// and a named route no law serves. `holds` names the spaces whose pools this card stands behind: they
    /// stay amounts, no law moves them, and a held enclosure's own fluid needs no entry. `validity` says
    /// where the stand-in is meant to hold.
    pub fn new(supplies: Vec<&'static str>, holds: Vec<Space>, validity: &'static str) -> Self {
        Self {
            supplies,
            holds,
            validity,
            card: PhantomData,
        }
    }
}
/// A declaration with its card, normalized so its order reaches nothing.
pub(crate) struct HeldCard {
    pub(crate) card: &'static str,
    pub(crate) supplies: Vec<&'static str>,
    pub(crate) holds: Vec<Space>,
    pub(crate) validity: &'static str,
}
impl Seal for HeldCard {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            card,
            supplies,
            holds,
            validity,
        } = self;
        out.text(card);
        out.length(supplies.len());
        for route in supplies {
            out.text(route);
        }
        out.length(holds.len());
        for space in holds {
            out.number(*space as u64);
        }
        out.text(validity);
    }
}

/// Issued once per assembly, for a card outside the slice.
pub struct HeldScope<'b, S> {
    laws: FragmentBuilder<'b, S>,
    ports: HeldPorts<'b, S>,
}
impl<'b, S: OutsideSlice> HeldScope<'b, S> {
    pub fn parts(self) -> (FragmentBuilder<'b, S>, HeldPorts<'b, S>) {
        (self.laws, self.ports)
    }
}
pub struct HeldPorts<'b, S> {
    /// The pressure of each enclosure the map says this card holds, defined once for all of them.
    pub enclosure_pressure: Output<'b, S, PerSite<EnclosedSpace, Pressure>>,
    names: Vec<&'static str>,
    brand: Brand<'b>,
}
impl<'b, S> HeldPorts<'b, S> {
    /// The schema boundary's pair and drive. Refuses a name the schema does not declare. A law on a route
    /// the declaration omits is refused at compile.
    pub fn water(&self, name: &'static str) -> Result<BoundaryWater<'b, S>, AdmissionError> {
        match self.names.binary_search(&name) {
            Ok(_) => Ok(BoundaryWater::named(name)),
            Err(_) => Err(AdmissionError::UnknownBoundary { name }),
        }
    }
}
