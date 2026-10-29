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
    /// Scope for a card outside the slice, held by a labeled stand-in that says what it supplies.
    pub fn hold<S: OutsideSlice>(&mut self, _declaration: HeldDeclaration<S>) -> Result<HeldScope<'b, S>, AdmissionError> {
        todo!()
    }
    pub fn add<O>(&mut self, _fragment: Fragment<'b, O>) {
        todo!()
    }
    /// Resolves ports, dimensions, mode coverage, ownership, PK claims, incidence, structural
    /// matching, Jacobian sparsity, conserved basis, gauges and stable order, then hashes. Refuses a
    /// held stand-in whose declared supplies are not exactly its fragment's exterior columns, a
    /// held intensive field tied to a store inside the slice, and an orphan pool (one no law
    /// touches and no declaration holds).
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
/// A held card's builder arrives pre-filled from the catalog: every `input` row from `S` is a
/// synthetic kernel over fixture parameters and the boundary timeline, every `sink` row to `S` a
/// labeled receiver that consumes no physiology. The composition root may add fixture laws on
/// shared substrates (for EN, surrogate ligand secretion and clearance) and must route every
/// material or energy export to `S` into a receipt pool listed in the coverage manifest.
pub struct HeldScope<'b, S> {
    _builder: FragmentBuilder<'b, S>,
}
/// What a held stand-in prescribes, every route on which it supplies or removes material or
/// energy, the catalog rows whose physiology it does not feed back, and where it is meant to hold.
/// It enters the model hash and the coverage manifest, each supply becomes a `HeldRoute` whose
/// exchange the history keeps apart, and the certificate reports each route's flow per window, so
/// a run that stays stable only through held support shows it.
pub struct HeldDeclaration<S> {
    _prescribes: Vec<HeldField>,
    _supplies: Vec<HeldSupply>,
    _absent_feedback: Vec<u16>,
    _validity: &'static str,
    _card: PhantomData<S>,
}
impl<S: OutsideSlice> HeldDeclaration<S> {
    /// `absent_feedback` lists coupling-map lines (as in `catalog`) this stand-in does not answer.
    pub fn new(
        _prescribes: Vec<HeldField>,
        _supplies: Vec<HeldSupply>,
        _absent_feedback: Vec<u16>,
        _validity: &'static str,
    ) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// A value a held stand-in prescribes. An intensive field (a concentration, a temperature) that
/// stands for a conserved store is either derived from that store inside the slice or declared an
/// external reservoir with its book, so its implicit support is named.
pub struct HeldField {
    _name: &'static str,
    _reservoir: Option<Book>,
}
impl HeldField {
    /// A signal with no conserved store behind it (a gain, a phase, a pressure).
    pub fn signal(_name: &'static str) -> Self {
        todo!()
    }
    /// An intensive value held as if from an external reservoir of `book`.
    pub fn reservoir(_name: &'static str, _book: Book) -> Self {
        todo!()
    }
}
/// One exterior route of a held stand-in and the book it moves.
pub struct HeldSupply {
    _route: &'static str,
    _book: Book,
}
impl HeldSupply {
    pub fn new(_route: &'static str, _book: Book) -> Self {
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
    /// Plasma to interstitium at every capillary bed: one law, one column per bed.
    pub starling: SiteRoute<'b, Fluids, CapillaryBed, Water>,
    /// Each bed's extravascular pressure, from the space that encloses it (or zero gauge when
    /// none does). Pleural and abdominal pressure reach the beds this way, not as wires.
    pub extravascular_pressure: ReadPort<'b, Fluids, PerSite<CapillaryBed, Pressure>>,
    pub lymph: FluidRoute<'b, Fluids>,
}
/// Paired transmitter routes at every junction; NS writes no heart rate or resistance.
pub struct NervousPorts<'b> {
    pub sympathetic_release: SiteRoute<'b, Nervous, Junction, Norepinephrine>,
    pub sympathetic_reuptake: SiteRoute<'b, Nervous, Junction, Norepinephrine>,
    pub vagal_release: SiteRoute<'b, Nervous, Junction, Acetylcholine>,
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
/// Routes a held stand-in may use: receipts for exports, and secretion and clearance pairs for
/// the surrogate ligands it releases into the body's shared ligand pools.
pub struct HeldPorts<'b, S> {
    pub receipts: FluidRoute<'b, S>,
    pub ligand_secretion: Vec<Pair<'b, S, SurrogateLigand>>,
    pub ligand_clearance: Vec<Pair<'b, S, SurrogateLigand>>,
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
