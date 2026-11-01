use crate::anatomy::Class;
use crate::compile::compile;
use crate::identity::{Canonical, Seal};
use crate::law::{Brand, Family, FragmentData, Over};
use crate::{
    AdmissionError, Anatomy, Blood, Boundary, BoundaryRoute, CapillaryBed, Cardiovascular, CompiledBody,
    Dimensionless, EdgeRoute, EnclosedSpace, Energy, FilterEdge, Fluids, Fragment, FragmentBuilder, Junction,
    LawName, LymphEdge, Measure, Nervous, Output, OutsideSlice, Owner, Pair, PartialVolume, PerSite,
    Pressure, ReadPort, Receptor, ReclaimEdge, Renal, SiteId, SiteRoute, SiteSet, Sites, Species, Substance,
    TubuleEdge, Urinary, ValveEdge, Water,
};
use core::marker::PhantomData;

pub(crate) const ENCLOSURE_PRESSURE: &str = "enclosure_pressure";
const BLOOD_PRESSURE: &str = "blood pressure";

/// The anatomy and the composition data laws never see: which species live in which spaces, where a route crosses
/// the boundary, the partial-volume coefficient and the spaces this body leaves out.
pub struct Schema {
    pub(crate) anatomy: Anatomy,
    pub(crate) carried: Vec<(Sites, Space, Species)>,
    pub(crate) placed: Vec<PoolRef>,
    pub(crate) boundaries: Vec<RouteBoundary>,
    pub(crate) partial_volume: Option<Measure<PartialVolume<Water>>>,
    pub(crate) left_out: Vec<(Space, &'static str)>,
}
#[derive(Clone, Copy)]
pub(crate) struct RouteBoundary {
    pub(crate) name: &'static str,
    pub(crate) direction: BoundaryDirection,
    pub(crate) pool: PoolRef,
    /// A drain: its law sets the water rate, a row, and its columns carry what the pool holds.
    pub(crate) drains: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundaryDirection {
    IntoBody,
    OutOfBody,
}
impl Schema {
    /// Water lives in every watery space of every site; anything else lives where `carry` or `pool` puts it.
    pub fn new(anatomy: Anatomy) -> Result<Self, AdmissionError> {
        Ok(Self {
            anatomy,
            carried: Vec::new(),
            placed: Vec::new(),
            boundaries: Vec::new(),
            partial_volume: None,
            left_out: Vec::new(),
        })
    }

    /// `Q` lives in this space of every site of a set, so blood and urine can hold different things in a lumen.
    pub fn carry<Q: Substance, C: SiteSet>(&mut self, space: Space) {
        self.carried.push((C::SET, space, Q::SPECIES));
    }

    /// `Q` lives in this space of one site, the composition data that says which junction holds which receptor.
    /// Refuses a space the site's class does not have.
    pub fn pool<Q: Substance>(&mut self, site: SiteId, space: Space) -> Result<(), AdmissionError> {
        if !Space::of(site.class()).contains(&space) {
            return Err(AdmissionError::SpaceOfAnotherClass { site, space });
        }
        self.placed.push(PoolRef {
            site,
            space,
            species: Q::SPECIES,
        });
        Ok(())
    }

    fn named(&mut self, boundary: RouteBoundary) -> Result<(), AdmissionError> {
        let name = boundary.name;
        match self.boundaries.binary_search_by_key(&name, |known| known.name) {
            Ok(_) => Err(AdmissionError::BoundaryNamedTwice { name }),
            Err(at) => {
                self.boundaries.insert(at, boundary);
                Ok(())
            }
        }
    }

    /// Names where `Q` crosses the boundary: composition data may name a site, a law never does. Its law is owed
    /// whether or not a boundary scope is issued. Refuses a name declared twice.
    pub fn boundary<Q: Substance>(
        &mut self,
        name: &'static str,
        direction: BoundaryDirection,
        site: SiteId,
        space: Space,
    ) -> Result<(), AdmissionError> {
        let species = Q::SPECIES;
        let pool = PoolRef { site, space, species };
        let drains = false;
        self.named(RouteBoundary {
            name,
            direction,
            pool,
            drains,
        })
    }

    /// Names a way out of the body for a space's water and everything in it, such as the urine. Its law sets the water
    /// rate, a row, and it carries every species the pool holds, so no list of them exists to keep in step.
    pub fn drain(&mut self, name: &'static str, site: SiteId, space: Space) -> Result<(), AdmissionError> {
        let species = Species::Water;
        let pool = PoolRef { site, space, species };
        self.named(RouteBoundary {
            name,
            direction: BoundaryDirection::OutOfBody,
            pool,
            drains: true,
        })
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

    /// Every pool in key order: by site, then space, then species.
    pub fn pools(&self) -> impl Iterator<Item = (SiteId, Space, Species)> + '_ {
        let sites = self.anatomy.sites();
        let classed = sites.flat_map(|site| {
            let spaces = Space::of(site.class()).iter();
            spaces.flat_map(move |&space| {
                let water = space.holds_water().then_some(Species::Water);
                let more = (self.carried.iter())
                    .filter(move |carried| {
                        carried.1 == space && self.anatomy.in_set(carried.0).contains(&site)
                    })
                    .map(|carried| carried.2);
                water
                    .into_iter()
                    .chain(more)
                    .map(move |species| (site, space, species))
            })
        });
        let named = self
            .placed
            .iter()
            .map(|pool| (pool.site, pool.space, pool.species));
        let mut pools: Vec<(SiteId, Space, Species)> = classed.chain(named).collect();
        pools.sort_unstable();
        pools.dedup();
        pools.into_iter()
    }
}
impl Seal for Schema {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            anatomy,
            carried,
            placed,
            boundaries,
            partial_volume,
            left_out,
        } = self;
        anatomy.digest().seal(out);
        let mut pools: Vec<(SiteId, Space, Species)> = self.pools().collect();
        pools.sort_unstable();
        let _ = (carried, placed);
        out.length(pools.len());
        for (site, space, species) in pools {
            site.seal(out);
            out.number(space as u64);
            out.number(species as u64);
        }
        out.length(boundaries.len());
        for RouteBoundary {
            name,
            direction,
            pool: PoolRef { site, space, species },
            drains,
        } in boundaries
        {
            out.text(name);
            out.number(*direction as u64);
            site.seal(out);
            out.number(*space as u64);
            out.number(*species as u64);
            out.number(u64::from(*drains));
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
    pub(crate) species: Species,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Space {
    Plasma,
    Interstitium,
    Cells,
    Lumen,
    Fluid,
    /// A nerve ending's store of transmitter.
    Vesicle,
    /// The gap a transmitter is released into, where its receptors sit.
    Cleft,
    /// Ligand held on its receptors, at a junction or a bed.
    Bound,
    /// The mechanical energy held in stretched walls: one store, on the tissue site that stands for the body.
    Elastic,
    /// Heat: one store, on the same site.
    Thermal,
}
impl Space {
    fn of(class: Class) -> &'static [Space] {
        match class {
            Class::CapillaryBed => &[Space::Plasma, Space::Interstitium, Space::Cells, Space::Bound],
            Class::VesselSegment => &[Space::Lumen],
            Class::EnclosedSpace => &[Space::Fluid],
            Class::Junction => &[Space::Vesicle, Space::Cleft, Space::Bound],
            Class::TissueSite => &[Space::Elastic, Space::Thermal],
            Class::NerveSegment => &[],
        }
    }
    fn holds_water(self) -> bool {
        matches!(
            self,
            Space::Plasma | Space::Interstitium | Space::Cells | Space::Lumen | Space::Fluid
        )
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
    fn boundary_names(&self) -> Vec<(&'static str, Species)> {
        self.schema
            .boundaries
            .iter()
            .map(|boundary| (boundary.name, boundary.pool.species))
            .collect()
    }
    pub fn renal(&mut self) -> Result<RenalScope<'b>, AdmissionError> {
        Ok(RenalScope {
            laws: self.issue()?,
            ports: RenalPorts { brand: PhantomData },
        })
    }
    pub fn receptor(&mut self) -> Result<ReceptorScope<'b>, AdmissionError> {
        Ok(ReceptorScope {
            laws: self.issue()?,
            ports: ReceptorPorts { brand: PhantomData },
        })
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
    /// Refuses a route the schema does not name, and one another card declared.
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
            if !names.iter().any(|known| known.0 == name) {
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
scope!(RenalScope, Renal, RenalPorts);
scope!(ReceptorScope, Receptor, ReceptorPorts);

fn named_route<'b, Q: Substance, O>(
    names: &[(&'static str, Species)],
    name: &'static str,
) -> Result<BoundaryRoute<'b, Q, O>, AdmissionError> {
    match names.iter().find(|known| known.0 == name) {
        Some(known) if known.1 == Q::SPECIES => Ok(BoundaryRoute::named(name)),
        _ => Err(AdmissionError::UnknownBoundary { name }),
    }
}
/// What every system reads of the receptor layer: the effect of a species at the junction or bed that holds it.
fn effect<'b, O, C: SiteSet, Q: Substance>() -> ReadPort<'b, O, PerSite<C, Dimensionless>> {
    ReadPort::field(LawName {
        owner: Receptor::CARD,
        local: Q::SPECIES.names()[4],
    })
}

pub struct BoundaryPorts<'b> {
    names: Vec<(&'static str, Species)>,
    brand: Brand<'b>,
}
impl<'b> BoundaryPorts<'b> {
    /// The schema boundary's pair and drive. Refuses a name the schema does not declare for `Q`.
    pub fn route<Q: Substance>(&self, name: &'static str) -> Result<BoundaryRoute<'b, Q>, AdmissionError> {
        named_route(&self.names, name)
    }
}
pub struct CardiovascularPorts<'b> {
    brand: Brand<'b>,
}
impl<'b> CardiovascularPorts<'b> {
    /// Records a law owed for every plain vascular edge before returning the route, so dropping the route
    /// never drops what the fragment owes.
    pub fn circulation(
        &self,
        laws: &mut FragmentBuilder<'b, Cardiovascular>,
    ) -> EdgeRoute<'b, Cardiovascular, Water> {
        laws.request(Family::Circulation);
        EdgeRoute::requested(Family::Circulation)
    }
    /// Every valved edge. Its flow is memory, owed here with its route.
    pub fn valves(
        &self,
        laws: &mut FragmentBuilder<'b, Cardiovascular>,
    ) -> EdgeRoute<'b, Cardiovascular, Water, ValveEdge> {
        laws.request(Family::Valves);
        laws.owe_flow_memory(Family::Valves);
        EdgeRoute::requested(Family::Valves)
    }
    /// The pressure row of every blood site: the one law CV owes, and the read of it. FL reads it as line 80's fields.
    #[allow(clippy::type_complexity)]
    pub fn blood_pressure(
        &self,
        laws: &mut FragmentBuilder<'b, Cardiovascular>,
    ) -> (
        Output<'b, Cardiovascular, PerSite<Blood, Pressure>>,
        ReadPort<'b, Cardiovascular, PerSite<Blood, Pressure>>,
    ) {
        laws.site_field(BLOOD_PRESSURE, crate::Admissible::finite())
    }
    /// What each blood site feels from outside: its enclosure's pressure, or zero gauge.
    pub fn extravascular_pressure(&self) -> ReadPort<'b, Cardiovascular, PerSite<Blood, Pressure>> {
        ReadPort::extravascular()
    }
    /// `Q` in each blood site's own pool.
    pub fn amount<Q: Substance>(&self) -> ReadPort<'b, Cardiovascular, PerSite<Blood, Q>> {
        ReadPort::pool(Q::SPECIES)
    }
    /// A receptor's effect at its junction, the only way a transmitter changes a CV law.
    pub fn effect<Q: Substance>(&self) -> ReadPort<'b, Cardiovascular, PerSite<Junction, Dimensionless>> {
        effect::<_, _, Q>()
    }
    /// Work into the elastic store at each blood site. The far end is a receipt: the chemical energy MT stands behind.
    pub fn stroke_work(
        &self,
        laws: &mut FragmentBuilder<'b, Cardiovascular>,
    ) -> SiteRoute<'b, Cardiovascular, Blood, Energy> {
        laws.request(Family::StrokeWork);
        SiteRoute::requested(Family::StrokeWork)
    }
    /// The elastic store to the thermal store: one transfer.
    pub fn dissipation(
        &self,
        laws: &mut FragmentBuilder<'b, Cardiovascular>,
    ) -> Pair<'b, Cardiovascular, Energy> {
        laws.request(Family::Dissipation);
        Pair::serving(crate::law::Obligation::Family(Family::Dissipation))
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
    /// `Q` across the wall of every capillary bed, apart from what the water carries.
    pub fn exchange<Q: Substance>(
        &self,
        laws: &mut FragmentBuilder<'b, Fluids>,
    ) -> SiteRoute<'b, Fluids, CapillaryBed, Q> {
        let family = Family::Exchange(Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// Every lymph edge.
    pub fn lymph(&self, laws: &mut FragmentBuilder<'b, Fluids>) -> EdgeRoute<'b, Fluids, Water, LymphEdge> {
        laws.request(Family::Lymph);
        EdgeRoute::requested(Family::Lymph)
    }
}
pub struct NervousPorts<'b> {
    brand: Brand<'b>,
}
impl<'b> NervousPorts<'b> {
    /// Vesicle to cleft at every junction that holds `Q`.
    pub fn release<Q: Substance>(
        &self,
        laws: &mut FragmentBuilder<'b, Nervous>,
    ) -> SiteRoute<'b, Nervous, Junction, Q> {
        let family = Family::Release(Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// Cleft to vesicle at every junction that holds `Q`.
    pub fn reuptake<Q: Substance>(
        &self,
        laws: &mut FragmentBuilder<'b, Nervous>,
    ) -> SiteRoute<'b, Nervous, Junction, Q> {
        let family = Family::Reuptake(Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
}
pub struct RenalPorts<'b> {
    brand: Brand<'b>,
}
impl<'b> RenalPorts<'b> {
    /// Every urinary edge out of a bed. It carries what plasma and lumen both hold, so a space with no albumin keeps
    /// albumin back by having none.
    pub fn filtration(
        &self,
        laws: &mut FragmentBuilder<'b, Renal>,
    ) -> EdgeRoute<'b, Renal, Water, FilterEdge> {
        laws.request(Family::Filtration);
        EdgeRoute::requested(Family::Filtration)
    }
    /// Every urinary edge between two lumens.
    pub fn tubular(&self, laws: &mut FragmentBuilder<'b, Renal>) -> EdgeRoute<'b, Renal, Water, TubuleEdge> {
        laws.request(Family::Tubular);
        EdgeRoute::requested(Family::Tubular)
    }
    /// `Q` back from a lumen to a bed along every reclaim edge. One species, so each has its own rate.
    pub fn reclaim<Q: Substance>(
        &self,
        laws: &mut FragmentBuilder<'b, Renal>,
    ) -> EdgeRoute<'b, Renal, Q, ReclaimEdge> {
        let family = Family::Reclaim(Q::SPECIES);
        laws.request(family);
        EdgeRoute::requested(family)
    }
    /// `Q` in each urinary segment's own pool.
    pub fn lumen<Q: Substance>(&self) -> ReadPort<'b, Renal, PerSite<Urinary, Q>> {
        ReadPort::pool(Q::SPECIES)
    }
    /// The pressure row of every blood site, which a bed's filtration reads.
    pub fn blood_pressure(&self) -> ReadPort<'b, Renal, PerSite<Blood, Pressure>> {
        ReadPort::field(LawName {
            owner: Cardiovascular::CARD,
            local: BLOOD_PRESSURE,
        })
    }
    /// A receptor's effect where the bed holds it.
    pub fn effect<Q: Substance>(&self) -> ReadPort<'b, Renal, PerSite<Blood, Dimensionless>> {
        effect::<_, _, Q>()
    }
}
pub struct ReceptorPorts<'b> {
    brand: Brand<'b>,
}
impl<'b> ReceptorPorts<'b> {
    /// Free ligand to bound at every site of `C` that holds `Q` bound: a two-ended transfer, so the ligand's moiety
    /// closes by pairing and no reaction type is needed. A receptor is a site parameter, not a pool.
    pub fn binding<C: SiteSet, Q: Substance>(
        &self,
        laws: &mut FragmentBuilder<'b, Receptor>,
    ) -> SiteRoute<'b, Receptor, C, Q> {
        let family = Family::Binding(Q::SPECIES);
        laws.request(family);
        SiteRoute::requested(family)
    }
    /// The effect of `Q` at every site of `C` that holds it bound. One row per such site, and every law reads it through
    /// its own scope's `effect`, so no law says which junction.
    pub fn effect<C: SiteSet, Q: Substance>(
        &self,
        laws: &mut FragmentBuilder<'b, Receptor>,
    ) -> Output<'b, Receptor, PerSite<C, Dimensionless>> {
        let name = Q::SPECIES.names()[4];
        laws.owe_defined::<Dimensionless>(name, Over::WherePool(C::SET, Space::Bound, Q::SPECIES));
        Output::named(name)
    }
}

/// What a held stand-in answers for. The rest of what it does shows in its fragment.
pub struct HeldDeclaration<S> {
    supplies: Vec<&'static str>,
    holds: Vec<Space>,
    validity: &'static str,
    card: PhantomData<S>,
}
impl<S: OutsideSlice> HeldDeclaration<S> {
    /// `supplies` names every route this card moves water on. `holds` names the spaces whose pools it stands
    /// behind: no law moves them, and a held enclosure's own fluid needs no entry.
    pub fn new(supplies: Vec<&'static str>, holds: Vec<Space>, validity: &'static str) -> Self {
        Self {
            supplies,
            holds,
            validity,
            card: PhantomData,
        }
    }
}
/// Normalized, so a declaration's order never reaches the model hash.
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
    names: Vec<(&'static str, Species)>,
    brand: Brand<'b>,
}
impl<'b, S> HeldPorts<'b, S> {
    /// Refuses a name the schema does not declare for `Q`. Compile refuses a law on a route the declaration omits.
    pub fn route<Q: Substance>(&self, name: &'static str) -> Result<BoundaryRoute<'b, Q, S>, AdmissionError> {
        named_route(&self.names, name)
    }
}
