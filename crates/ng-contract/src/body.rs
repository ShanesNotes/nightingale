use crate::elimination::{Elimination, Factors};
use crate::identity::{Canonical, Seal};
use crate::law::{Op, Validity};
use crate::problem::Jacobian;
use crate::scope::HeldCard;
use crate::sources::{RefusalSource, refusal_sources};
use crate::{
    AdmissibleSet, Class, ConservedBasis, Crossing, Digest, LawName, Network, ParameterKey, Predicate,
    RateClass, Schema, SiteId, Space, Species,
};

macro_rules! index {
    ($($(#[$doc:meta])* $name:ident),*) => {$(
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $name {
            index: usize,
        }
        impl $name {
            pub(crate) fn at(index: usize) -> Self {
                Self { index }
            }
            pub fn index(self) -> usize {
                self.index
            }
        }
        impl Seal for $name {
            fn seal(&self, out: &mut Canonical) {
                let Self { index } = self;
                index.seal(out);
            }
        }
    )*};
}
index!(
    RowId,
    /// An unknown: row `i`'s own coordinate has index `i`.
    UnknownId,
    ColumnId,
    ProgramId,
    ParameterId
);

/// The checked, immutable result of compiling the anatomy and every fragment. Its arrays are in
/// canonical order, so neither the order fragments arrive in nor the order of declarations reaches
/// them or the model hash.
pub struct CompiledBody {
    hash: ModelHash,
    pub(crate) layout: Layout,
    pub(crate) sources: Vec<RefusalSource>,
}
pub(crate) struct Layout {
    pub(crate) schema: Schema,
    pub(crate) rows: Vec<Row>,
    pub(crate) columns: Vec<Column>,
    pub(crate) programs: Vec<Program>,
    pub(crate) instances: Vec<Instance>,
    pub(crate) parameters: Vec<ParameterSlot>,
    pub(crate) sparsity: Sparsity,
    /// Derived from the sparsity and the matching, so the hash covers it through them.
    pub(crate) elimination: Elimination,
    pub(crate) blocks: Vec<Block>,
    pub(crate) modes: Vec<ModeLayout>,
    pub(crate) guards: Vec<GuardLayout>,
    pub(crate) streams: Vec<StreamLayout>,
    pub(crate) forcings: Vec<ForcingLayout>,
    pub(crate) hazards: Vec<HazardLayout>,
    pub(crate) basis: ConservedBasis,
    pub(crate) held: Vec<HeldCard>,
}
impl CompiledBody {
    pub(crate) fn seal(layout: Layout) -> Self {
        let mut out = Canonical::new("ng-contract model 1");
        layout.seal(&mut out);
        Self {
            hash: ModelHash { digest: out.digest() },
            sources: refusal_sources(&layout),
            layout,
        }
    }
    pub fn model_hash(&self) -> ModelHash {
        self.hash
    }
    /// Amounts first, in registry order, then memory and then algebraic rows, each by law name.
    pub fn rows(&self) -> &[Row] {
        &self.layout.rows
    }
    /// One per instance of each transfer, in law-name order and then site or edge key order.
    pub fn columns(&self) -> &[Column] {
        &self.layout.columns
    }
    pub fn programs(&self) -> &[Program] {
        &self.layout.programs
    }
    pub fn instances(&self) -> &[Instance] {
        &self.layout.instances
    }
    pub fn parameters(&self) -> &[ParameterSlot] {
        &self.layout.parameters
    }
    pub fn sparsity(&self) -> &Sparsity {
        &self.layout.sparsity
    }
    /// In dependency order: a block comes after every block it depends on.
    pub fn blocks(&self) -> &[Block] {
        &self.layout.blocks
    }
    pub fn conserved_basis(&self) -> &ConservedBasis {
        &self.layout.basis
    }
    pub fn hazards(&self) -> impl Iterator<Item = LawName> + '_ {
        (self.layout.hazards.iter()).map(|hazard| self.layout.streams[hazard.stream].name)
    }
    /// One Jacobian of this body factored over its elimination, or the row whose pivot it cannot take. A pivot never
    /// moves, so a core that meets one solves `Jacobian::view` its own way.
    pub fn factor(&self, jacobian: &Jacobian) -> Result<Factors<'_>, usize> {
        self.layout.elimination.factor(&jacobian.values)
    }
    /// The cards held, in card order.
    pub fn held(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.layout.held.iter().map(|held| held.card)
    }
    /// The longest step this body's laws tolerate: the tightest class of any block a law moves. `attempt` caps a ticket
    /// here, as a running delivery's end caps one, so no step runs past a law's interval. A held pool's block limits
    /// nothing. A core reads it, so it need not commit to a span the record will cap.
    pub fn longest_step_ns(&self) -> u64 {
        (self.layout.blocks.iter())
            .map(|block| block.rate.maximum_ns())
            .min()
            .unwrap_or(u64::MAX)
    }
}
impl Layout {
    /// One firing, in stable order: each crossed guard still armed on what those before it left moves its mode, so of two
    /// guards on one variable the first wins, and a guard a firing arms waits for the next step. The run and the ring
    /// both fire here.
    pub(crate) fn fire(&self, modes: &mut [usize], crossed: &[bool]) -> Vec<&GuardLayout> {
        let mut fired = Vec::new();
        for (guard, &crossed) in self.guards.iter().zip(crossed) {
            if crossed && guard.armed(modes) {
                modes[guard.variable] = guard.enters;
                fired.push(guard);
            }
        }
        fired
    }
}
impl Seal for Layout {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            schema,
            rows,
            columns,
            programs,
            instances,
            parameters,
            sparsity,
            elimination: _,
            blocks,
            modes,
            guards,
            streams,
            forcings,
            hazards,
            basis,
            held,
        } = self;
        schema.seal(out);
        rows.seal(out);
        columns.seal(out);
        programs.seal(out);
        instances.seal(out);
        parameters.seal(out);
        sparsity.seal(out);
        blocks.seal(out);
        modes.seal(out);
        guards.seal(out);
        streams.seal(out);
        forcings.seal(out);
        hazards.seal(out);
        basis.seal(out);
        held.seal(out);
    }
}

/// The hash of the compiled body, anatomy included. Parameter values are not in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelHash {
    digest: Digest,
}
impl ModelHash {
    pub fn digest(&self) -> Digest {
        self.digest
    }
}

#[derive(Debug, PartialEq)]
pub struct Row {
    pub(crate) unknown: UnknownId,
    pub(crate) kind: RowKind,
    pub(crate) admissible: AdmissibleSet,
    pub(crate) dimension: String,
}
impl Row {
    pub fn id(&self) -> RowId {
        RowId::at(self.unknown.index())
    }
    pub fn unknown(&self) -> UnknownId {
        self.unknown
    }
    pub fn kind(&self) -> RowKind {
        self.kind
    }
    pub fn admissible(&self) -> AdmissibleSet {
        self.admissible
    }
}
impl Seal for Row {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            unknown,
            kind,
            admissible,
            dimension,
        } = self;
        unknown.seal(out);
        kind.seal(out);
        out.number(*admissible as u64);
        dimension.seal(out);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowKind {
    Amount {
        site: SiteId,
        space: Space,
        species: Species,
    },
    Memory(Slot),
    Algebraic(Slot),
}
/// A law's row or mode variable and the place it stands: the body, a site or an edge, so one name serves all three.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slot {
    pub law: LawName,
    pub at: Context,
}
impl From<LawName> for Slot {
    fn from(law: LawName) -> Self {
        Self {
            law,
            at: Context::Body,
        }
    }
}
impl Seal for Slot {
    fn seal(&self, out: &mut Canonical) {
        let Self { law, at } = self;
        law.seal(out);
        at.seal(out);
    }
}
impl Seal for RowKind {
    fn seal(&self, out: &mut Canonical) {
        match self {
            RowKind::Amount { site, space, species } => {
                out.number(0);
                site.seal(out);
                out.number(*space as u64);
                out.number(*species as u64);
            }
            RowKind::Memory(slot) => {
                out.number(1);
                slot.seal(out);
            }
            RowKind::Algebraic(slot) => {
                out.number(2);
                slot.seal(out);
            }
        }
    }
}

/// One instance of a transfer: the amounts it moves, each with its signed coefficient. Its rate comes
/// from whichever law on its route holds in the current mode.
#[derive(Debug, PartialEq)]
pub struct Column {
    pub(crate) terms: Vec<(RowId, i64)>,
    pub(crate) origin: LawName,
    pub(crate) context: Context,
    pub(crate) receipt: Option<Receipt>,
}
impl Column {
    /// In row order. A transfer whose two ends are one pool has none.
    pub fn terms(&self) -> &[(RowId, i64)] {
        &self.terms
    }
    /// The route family's law name, shared by every instance of the route.
    pub fn origin(&self) -> LawName {
        self.origin
    }
    pub fn context(&self) -> Context {
        self.context
    }
    /// The exterior end of a transfer across the boundary; `None` for a transfer inside the body.
    pub fn receipt(&self) -> Option<Receipt> {
        self.receipt
    }
}
impl Seal for Column {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            terms,
            origin,
            context,
            receipt,
        } = self;
        terms.seal(out);
        origin.seal(out);
        context.seal(out);
        receipt.seal(out);
    }
}
/// The receipt pool outside the body that takes what a boundary transfer moves, so the transfer still
/// has two ends and no amount row goes negative for the exterior.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub(crate) boundary: &'static str,
    pub(crate) coefficient: i64,
}
impl Receipt {
    pub fn boundary(&self) -> &'static str {
        self.boundary
    }
    /// Its signed coefficient, the opposite of the body end's.
    pub fn coefficient(&self) -> i64 {
        self.coefficient
    }
}
impl Seal for Receipt {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            boundary,
            coefficient,
        } = self;
        boundary.seal(out);
        coefficient.seal(out);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Context {
    Body,
    Site(SiteId),
    Edge { from: SiteId, to: SiteId },
}
impl Seal for Context {
    fn seal(&self, out: &mut Canonical) {
        match self {
            Context::Body => out.number(0),
            Context::Site(site) => {
                out.number(1);
                site.seal(out);
            }
            Context::Edge { from, to } => {
                out.number(2);
                from.seal(out);
                to.seal(out);
            }
        }
    }
}

/// One compiled kernel. A site or edge law is one program however many sites or edges it applies at.
#[derive(Debug, PartialEq)]
pub struct Program {
    pub(crate) id: ProgramId,
    pub(crate) law: LawName,
    pub(crate) code: Code,
}
impl Program {
    pub fn id(&self) -> ProgramId {
        self.id
    }
}
impl Seal for Program {
    fn seal(&self, out: &mut Canonical) {
        let Self { id, law, code } = self;
        id.seal(out);
        law.seal(out);
        code.seal(out);
    }
}
#[derive(Debug, PartialEq)]
pub(crate) struct Code {
    pub(crate) ops: Vec<Op>,
    pub(crate) dimension: String,
    pub(crate) predicate: Predicate,
    pub(crate) rate: RateClass,
    pub(crate) validity: Vec<Validity>,
    /// The kernel defines its row, so a range judges the row and not the residual.
    pub(crate) defines: bool,
}
impl Seal for Code {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            ops,
            dimension,
            predicate,
            rate,
            validity,
            defines,
        } = self;
        ops.seal(out);
        dimension.seal(out);
        predicate.seal(out);
        rate.seal(out);
        validity.seal(out);
        out.number(u64::from(*defines));
    }
}

/// What a program instance defines: a memory or algebraic row, or a transfer's column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Row(RowId),
    Column(ColumnId),
}
impl Seal for Target {
    fn seal(&self, out: &mut Canonical) {
        match self {
            Target::Row(row) => {
                out.number(0);
                row.seal(out);
            }
            Target::Column(column) => {
                out.number(1);
                column.seal(out);
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Instance {
    pub(crate) program: ProgramId,
    pub(crate) context: Context,
    pub(crate) target: Target,
    pub(crate) parameters: Vec<ParameterId>,
    pub(crate) inputs: Vec<Bound>,
    /// The compiled sparsity is the union over modes, so selecting by mode moves no entry.
    pub(crate) mode: Option<(usize, usize)>,
    pub(crate) rectified: bool,
}
impl Instance {
    pub fn program(&self) -> ProgramId {
        self.program
    }
    pub fn context(&self) -> Context {
        self.context
    }
    pub fn target(&self) -> Target {
        self.target
    }
    /// The parameter slots the program reads here, each once, in the order the program first reads them.
    pub fn parameters(&self) -> &[ParameterId] {
        &self.parameters
    }
    pub(crate) fn holds(&self, modes: &[usize]) -> bool {
        self.mode.is_none_or(|(variable, value)| modes[variable] == value)
    }
}
impl Seal for Instance {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            program,
            context,
            target,
            parameters,
            inputs,
            mode,
            rectified,
        } = self;
        program.seal(out);
        context.seal(out);
        target.seal(out);
        parameters.seal(out);
        inputs.seal(out);
        mode.seal(out);
        out.number(u64::from(*rectified));
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    Parameter(ParameterId),
    Unknown(UnknownId),
    Boundary(&'static str),
    Draw(usize),
    Forcing(usize),
    Gauge,
    Height(u64),
}
impl Bound {
    pub(crate) fn unknown(&self) -> Option<UnknownId> {
        match *self {
            Bound::Unknown(unknown) => Some(unknown),
            Bound::Parameter(_)
            | Bound::Boundary(_)
            | Bound::Draw(_)
            | Bound::Forcing(_)
            | Bound::Gauge
            | Bound::Height(_) => None,
        }
    }
}
impl Seal for Bound {
    fn seal(&self, out: &mut Canonical) {
        match self {
            Bound::Parameter(parameter) => {
                out.number(0);
                parameter.seal(out);
            }
            Bound::Unknown(unknown) => {
                out.number(1);
                unknown.seal(out);
            }
            Bound::Boundary(boundary) => {
                out.number(2);
                boundary.seal(out);
            }
            Bound::Draw(stream) => {
                out.number(3);
                stream.seal(out);
            }
            Bound::Forcing(forcing) => {
                out.number(4);
                forcing.seal(out);
            }
            Bound::Gauge => out.number(5),
            Bound::Height(axial) => {
                out.number(6);
                out.number(*axial);
            }
        }
    }
}

/// Where a parameter's value binds: the body, one site, one edge, or as a default, a site class or a
/// network.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Binding {
    Body,
    Site(SiteId),
    Edge { from: SiteId, to: SiteId },
    Class(Class),
    Network(Network),
}
impl Seal for Binding {
    fn seal(&self, out: &mut Canonical) {
        match self {
            Binding::Body => out.number(0),
            Binding::Site(site) => {
                out.number(1);
                site.seal(out);
            }
            Binding::Edge { from, to } => {
                out.number(2);
                from.seal(out);
                to.seal(out);
            }
            Binding::Class(class) => {
                out.number(3);
                out.number(*class as u64);
            }
            Binding::Network(network) => {
                out.number(4);
                out.number(*network as u64);
            }
        }
    }
}
/// A parameter's slot in the layout. Its value binds later and stays out of the model hash.
#[derive(Debug, PartialEq)]
pub struct ParameterSlot {
    pub(crate) key: ParameterKey,
    pub(crate) binding: Binding,
    pub(crate) default: Option<Binding>,
    pub(crate) admissible: AdmissibleSet,
    pub(crate) dimension: String,
}
impl ParameterSlot {
    pub fn key(&self) -> ParameterKey {
        self.key
    }
    pub fn binding(&self) -> Binding {
        self.binding
    }
    /// Where the value binds when the packet gives none for `binding`; a whole-body parameter has none.
    pub fn default_binding(&self) -> Option<Binding> {
        self.default
    }
    pub fn admissible(&self) -> AdmissibleSet {
        self.admissible
    }
}
impl Seal for ParameterSlot {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            key,
            binding,
            default,
            admissible,
            dimension,
        } = self;
        key.key().seal(out);
        binding.seal(out);
        default.seal(out);
        out.number(*admissible as u64);
        dimension.seal(out);
    }
}

/// The structural sparsity in compressed rows: row `r` reads the unknowns
/// `unknowns()[row_offsets()[r]..row_offsets()[r + 1]]`.
#[derive(Debug, PartialEq)]
pub struct Sparsity {
    pub(crate) offsets: Vec<usize>,
    pub(crate) unknowns: Vec<UnknownId>,
}
impl Sparsity {
    pub fn row_offsets(&self) -> &[usize] {
        &self.offsets
    }
    pub fn unknowns(&self) -> &[UnknownId] {
        &self.unknowns
    }
}
impl Seal for Sparsity {
    fn seal(&self, out: &mut Canonical) {
        let Self { offsets, unknowns } = self;
        offsets.seal(out);
        unknowns.seal(out);
    }
}

/// A strongly connected part of the matched equation graph, with the most restrictive rate class of
/// the laws in it. A held pool's block has no law, so its class limits nothing. Structure only, never
/// permission to lag a dependency.
#[derive(Debug, PartialEq)]
pub struct Block {
    pub(crate) rows: Vec<RowId>,
    pub(crate) unknowns: Vec<UnknownId>,
    pub(crate) rate: RateClass,
}
impl Block {
    pub fn rows(&self) -> &[RowId] {
        &self.rows
    }
    pub fn unknowns(&self) -> &[UnknownId] {
        &self.unknowns
    }
    pub fn rate_class(&self) -> RateClass {
        self.rate
    }
}
impl Seal for Block {
    fn seal(&self, out: &mut Canonical) {
        let Self { rows, unknowns, rate } = self;
        rows.seal(out);
        unknowns.seal(out);
        rate.seal(out);
    }
}

/// One per variable and place, so a valve's mode is one more entry of one `Vec<usize>`.
pub(crate) struct ModeLayout {
    pub(crate) variable: Slot,
    pub(crate) values: Vec<&'static str>,
}
impl Seal for ModeLayout {
    fn seal(&self, out: &mut Canonical) {
        let Self { variable, values } = self;
        variable.seal(out);
        out.length(values.len());
        for value in values {
            value.seal(out);
        }
    }
}
/// One instance per place of its mode variable. `leaves` and `enters` index the variable's sorted values, as a
/// hazard's do, so a firing is the same two writes for both.
pub(crate) struct GuardLayout {
    pub(crate) variable: usize,
    pub(crate) leaves: usize,
    pub(crate) enters: usize,
    pub(crate) crossing: Crossing,
    pub(crate) code: Code,
    pub(crate) inputs: Vec<Bound>,
    pub(crate) resets: Option<RowId>,
}
impl GuardLayout {
    pub(crate) fn armed(&self, modes: &[usize]) -> bool {
        modes[self.variable] == self.leaves
    }
}
impl Seal for GuardLayout {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            variable,
            leaves,
            enters,
            crossing,
            code,
            inputs,
            resets,
        } = self;
        variable.seal(out);
        leaves.seal(out);
        enters.seal(out);
        crossing.seal(out);
        code.seal(out);
        inputs.seal(out);
        resets.seal(out);
    }
}

/// A key's word derives from the name, so the name is what the model hash covers.
pub(crate) struct StreamLayout {
    pub(crate) name: LawName,
    pub(crate) key: u64,
}
impl Seal for StreamLayout {
    fn seal(&self, out: &mut Canonical) {
        let Self { name, key: _ } = self;
        name.seal(out);
    }
}
pub(crate) struct ForcingLayout {
    pub(crate) name: LawName,
    pub(crate) key: u64,
    pub(crate) period: ParameterId,
    pub(crate) amplitude: ParameterId,
}
impl Seal for ForcingLayout {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            name,
            key: _,
            period,
            amplitude,
        } = self;
        name.seal(out);
        period.seal(out);
        amplitude.seal(out);
    }
}
pub(crate) struct HazardLayout {
    pub(crate) integral: RowId,
    pub(crate) stream: usize,
    pub(crate) variable: usize,
    pub(crate) leaves: usize,
    pub(crate) enters: usize,
}
impl HazardLayout {
    pub(crate) fn armed(&self, modes: &[usize]) -> bool {
        modes[self.variable] == self.leaves
    }
}
impl Seal for HazardLayout {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            integral,
            stream,
            variable,
            leaves,
            enters,
        } = self;
        integral.seal(out);
        stream.seal(out);
        variable.seal(out);
        leaves.seal(out);
        enters.seal(out);
    }
}
