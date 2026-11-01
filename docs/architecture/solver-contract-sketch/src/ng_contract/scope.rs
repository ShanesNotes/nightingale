//! Composition: the schema's one pool registry, the assembly that issues each owner's scope once,
//! and the per-owner capability bundles. Ports for coupling-map arrows come from the generated
//! catalog through `FragmentBuilder::emit`/`receive`; the bundles below grant only routes and reads
//! that are not map arrows (pools at the owner's own edges, shared substrates, boundary routes).
use super::law::FragmentData;
use super::*;

pub struct PoolRegistry {
    _pools: Vec<PoolRef>,
}
/// The anatomy plus the pools it generates: each site class's spaces, the species and carrier
/// media in each, and the conserved books they make. Organs are regions of the anatomy,
/// so the registry has no organ in it.
pub struct Schema {
    _anatomy: Anatomy,
    _registry: PoolRegistry,
}
impl Schema {
    /// Derives the pool registry from the anatomy's site classes; a species with no carrier is
    /// refused.
    pub fn new(_anatomy: Anatomy) -> Result<Self, AdmissionError> {
        todo!()
    }
    /// States that this body does not model the space, and why: its pools are not amounts, and a law
    /// that moves or reads one is refused. Every other pool is an amount some transfer must move.
    pub fn leave_out(&mut self, _space: Space, _reason: &'static str) {
        todo!()
    }
}
/// One part of a site that holds pools: a capillary bed's plasma, interstitium and cells, a vessel
/// segment's lumen and an enclosed space's fluid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Space {
    Plasma,
    Interstitium,
    Cells,
    Lumen,
    Fluid,
}
pub struct Assembly<'b> {
    _schema: Schema,
    _fragments: Vec<FragmentData>,
    _brand: Brand<'b>,
}
impl<'b> Assembly<'b> {
    /// A scope can be issued once; a second request fails admission, never duplicates ownership.
    pub fn renal(&mut self) -> Result<RenalScope<'b>, AdmissionError> {
        todo!()
    }
    pub fn cardiovascular(&mut self) -> Result<CardiovascularScope<'b>, AdmissionError> {
        todo!()
    }
    pub fn fluids(&mut self) -> Result<FluidsScope<'b>, AdmissionError> {
        todo!()
    }
    pub fn nervous(&mut self) -> Result<NervousScope<'b>, AdmissionError> {
        todo!()
    }
    pub fn receptor(&mut self) -> Result<ReceptorScope<'b>, AdmissionError> {
        todo!()
    }
    pub fn boundary(&mut self) -> Result<BoundaryScope<'b>, AdmissionError> {
        todo!()
    }
    /// Scope for a card outside the slice, held by a declaration of what it supplies. Refuses a route
    /// the schema does not name, and one another card declared.
    pub fn hold<S: OutsideSlice>(&mut self, _declaration: HeldDeclaration<S>) -> Result<HeldScope<'b, S>, AdmissionError> {
        todo!()
    }
    pub fn add<O>(&mut self, _fragment: Fragment<'b, O>) {
        todo!()
    }
    /// Resolves ports, dimensions, mode coverage, ownership, PK claims, incidence, structural
    /// matching, Jacobian sparsity, conserved basis, gauges and stable order, then hashes. Refuses a
    /// held stand-in whose declared supplies are not exactly its fragment's exterior columns, an
    /// orphan pool (one the schema does not leave out that no transfer moves and no declaration
    /// holds), a law that moves a pool a declaration holds, and a law that moves or reads a pool the
    /// schema left out.
    /// Only here is the lifetime brand erased. Missing contributors are errors, not zeroes.
    pub fn compile(self) -> Result<CompiledBody, AdmissionError> {
        todo!()
    }
}
/// Prevents handles from one assembly escaping into another, per type-system-discipline.
pub fn assemble(
    _schema: Schema,
    _declare: impl for<'b> FnOnce(Assembly<'b>) -> Result<CompiledBody, AdmissionError>,
) -> Result<CompiledBody, AdmissionError> {
    todo!()
}

pub struct RenalScope<'b> {
    _builder: FragmentBuilder<'b, Renal>,
}
pub struct CardiovascularScope<'b> {
    _builder: FragmentBuilder<'b, Cardiovascular>,
}
pub struct FluidsScope<'b> {
    _builder: FragmentBuilder<'b, Fluids>,
}
pub struct NervousScope<'b> {
    _builder: FragmentBuilder<'b, Nervous>,
}
pub struct ReceptorScope<'b> {
    _builder: FragmentBuilder<'b, Receptor>,
}
pub struct BoundaryScope<'b> {
    _builder: FragmentBuilder<'b, Boundary>,
}
/// A held card's builder is plain: it emits, receives and defines like any owner, on the first
/// ticket that reads a row. The composition root may add fixture laws on shared substrates (for
/// EN, surrogate ligand secretion and clearance) and must route every material or energy export
/// to `S` into a receipt pool listed in the coverage manifest.
pub struct HeldScope<'b, S> {
    _builder: FragmentBuilder<'b, S>,
}
/// What a held stand-in answers for: every route on which it supplies or removes material or
/// energy, the spaces whose pools it stands behind, and where it is meant to hold. What it
/// prescribes shows in the rows it defines, and the catalog rows it does not answer are the sink
/// rows it does not read. It enters the model hash and the coverage manifest, each supply becomes
/// a `HeldRoute` whose exchange the history keeps apart, and the certificate reports each route's
/// flow per window, so a run that stays stable only through held support shows it.
pub struct HeldDeclaration<S> {
    _supplies: Vec<&'static str>,
    _holds: Vec<Space>,
    _validity: &'static str,
    _card: PhantomData<S>,
}
impl<S: OutsideSlice> HeldDeclaration<S> {
    /// `supplies` names every schema route this card moves material on: compile refuses a law on
    /// any other, and a named route no law serves. `holds` names the spaces whose pools this card
    /// stands behind: they stay amounts, no law moves them, and a held enclosure's own fluid needs
    /// no entry.
    pub fn new(_supplies: Vec<&'static str>, _holds: Vec<Space>, _validity: &'static str) -> Self {
        todo!()
    }
    /// An intensive value held as if from an external reservoir of `book`, so its implicit support
    /// is named. Compile refuses one whose store is inside the slice. It lands with the second book.
    pub fn reservoir(self, _field: &'static str, _book: Book) -> Self {
        todo!()
    }
}

/// Renal law routes and the pressures at the renal edges' own endpoints. No arbitrary pool lookup.
pub struct RenalPorts<'b> {
    /// The registry's glomerular-water route.
    pub filtration: Pair<'b, Renal, Water>,
    pub filtration_inputs: FiltrationInputs<'b>,
    /// Tubule to bladder to exterior; every carried species expands from the schema.
    pub urine: FluidRoute<'b, Renal>,
}
pub struct FiltrationInputs<'b> {
    pub glomerular_pressure: ReadPort<'b, Renal, Pressure>,
    pub tubular_pressure: ReadPort<'b, Renal, Pressure>,
    pub oncotic_pressure: ReadPort<'b, Renal, Pressure>,
    pub modes: ModeSet<'b, Renal>,
}
pub struct CardiovascularPorts<'b> {
    pub circulation: FluidRoute<'b, Cardiovascular>,
    /// Every vessel segment's hydrostatic head, derived once from site positions and posture.
    pub hydrostatic_head: ReadPort<'b, Cardiovascular, PerSite<VesselSegment, Pressure>>,
    pub chamber_pressure: AlgebraicSlot<'b, Cardiovascular, Pressure>,
    /// Receptor occupancy at the heart and vessels: the only way a transmitter changes a CV law.
    pub receptor_effect: ReadPort<'b, Cardiovascular, Dimensionless>,
    /// Pressure-volume work between the heart's elastic store and the circulation.
    pub stroke_work: WorkPair<'b, Cardiovascular>,
}
/// Starling exchange at every capillary bed, the pleural space and the legs alike.
pub struct FluidsPorts<'b> {
    /// Each bed's extravascular pressure, from the space that encloses it (or zero gauge when
    /// none does). Pleural and abdominal pressure reach the beds this way, not as wires.
    pub extravascular_pressure: ReadPort<'b, Fluids, PerSite<CapillaryBed, Pressure>>,
    pub lymph: FluidRoute<'b, Fluids>,
}
impl<'b> FluidsPorts<'b> {
    /// Plasma to interstitium at every capillary bed: one law, one column per bed. Requesting the
    /// route records the law the fragment owes, so dropping the route never drops the debt.
    pub fn starling(&self, _laws: &mut FragmentBuilder<'b, Fluids>) -> SiteRoute<'b, Fluids, CapillaryBed, Water> {
        todo!()
    }
}
/// Paired transmitter routes at every junction; NS writes no heart rate or resistance. Each request
/// records the law the fragment owes.
pub struct NervousPorts<'b> {
    _brand: Brand<'b>,
}
impl<'b> NervousPorts<'b> {
    pub fn sympathetic_release(&self, _laws: &mut FragmentBuilder<'b, Nervous>) -> SiteRoute<'b, Nervous, Junction, Norepinephrine> {
        todo!()
    }
    pub fn sympathetic_reuptake(&self, _laws: &mut FragmentBuilder<'b, Nervous>) -> SiteRoute<'b, Nervous, Junction, Norepinephrine> {
        todo!()
    }
    pub fn vagal_release(&self, _laws: &mut FragmentBuilder<'b, Nervous>) -> SiteRoute<'b, Nervous, Junction, Acetylcholine> {
        todo!()
    }
}
pub struct ReceptorPorts<'b> {
    /// Ligand + free receptor to complex, per receptor site (beta-1, alpha-1, M2, V2).
    pub bindings: Vec<BalancedReaction<'b, Receptor>>,
    /// Transmitter clearance into named product pools.
    pub clearance: Vec<BalancedReaction<'b, Receptor>>,
    pub ligand_activity: ReadPort<'b, Receptor, Dimensionless>,
}
pub struct BoundaryPorts<'b> {
    pub intake_water: Pair<'b, Boundary, Water>,
    pub evaporative_loss: Pair<'b, Boundary, Water>,
    /// Heat to and from the room: the energy book's exterior route.
    pub heat_exchange: Pair<'b, Boundary, Energy>,
}
/// Routes a held stand-in may use: the pressure of each enclosure the map says it holds, receipts
/// for exports, and secretion and clearance pairs for the surrogate ligands it releases into the
/// body's shared ligand pools.
pub struct HeldPorts<'b, S> {
    /// Defined once for all of this card's enclosures, one row per enclosure.
    pub enclosure_pressure: Output<'b, S, PerSite<EnclosedSpace, Pressure>>,
    pub receipts: FluidRoute<'b, S>,
    pub ligand_secretion: Vec<Pair<'b, S, SurrogateLigand>>,
    pub ligand_clearance: Vec<Pair<'b, S, SurrogateLigand>>,
}
impl<'b, S> HeldPorts<'b, S> {
    /// A schema route's pair. Refuses a name the schema does not declare. A law on a route the
    /// declaration omits is refused at compile.
    pub fn water(&self, _name: &'static str) -> Result<Pair<'b, S, Water>, AdmissionError> {
        todo!()
    }
}
impl<'b, S: OutsideSlice> HeldScope<'b, S> {
    pub fn parts(self) -> (FragmentBuilder<'b, S>, HeldPorts<'b, S>) {
        todo!()
    }
}
impl<'b> RenalScope<'b> {
    pub fn parts(self) -> (FragmentBuilder<'b, Renal>, RenalPorts<'b>) {
        todo!()
    }
}
impl<'b> CardiovascularScope<'b> {
    pub fn parts(self) -> (FragmentBuilder<'b, Cardiovascular>, CardiovascularPorts<'b>) {
        todo!()
    }
}
impl<'b> FluidsScope<'b> {
    pub fn parts(self) -> (FragmentBuilder<'b, Fluids>, FluidsPorts<'b>) {
        todo!()
    }
}
impl<'b> NervousScope<'b> {
    pub fn parts(self) -> (FragmentBuilder<'b, Nervous>, NervousPorts<'b>) {
        todo!()
    }
}
impl<'b> ReceptorScope<'b> {
    pub fn parts(self) -> (FragmentBuilder<'b, Receptor>, ReceptorPorts<'b>) {
        todo!()
    }
}
impl<'b> BoundaryScope<'b> {
    pub fn parts(self) -> (FragmentBuilder<'b, Boundary>, BoundaryPorts<'b>) {
        todo!()
    }
}
