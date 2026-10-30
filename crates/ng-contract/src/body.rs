use crate::identity::{Canonical, Seal};
use crate::law::{Op, Validity};
use crate::{
    AdmissibleSet, Class, ConservedBasis, Crossing, Digest, LawName, Network, ParameterKey, Predicate,
    RateClass, Schema, SiteId, Space,
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
                self.index.seal(out);
            }
        }
    )*};
}
index!(
    /// A row of the compiled body: an amount's balance, a memory's rate or an algebraic residual.
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
    layout: Layout,
}
pub(crate) struct Layout {
    pub(crate) schema: Schema,
    pub(crate) rows: Vec<Row>,
    pub(crate) columns: Vec<Column>,
    pub(crate) programs: Vec<Program>,
    pub(crate) instances: Vec<Instance>,
    pub(crate) parameters: Vec<ParameterSlot>,
    pub(crate) sparsity: Sparsity,
    pub(crate) blocks: Vec<Block>,
    pub(crate) modes: Vec<ModeLayout>,
    pub(crate) guards: Vec<GuardLayout>,
    pub(crate) basis: ConservedBasis,
}
impl CompiledBody {
    pub(crate) fn seal(layout: Layout) -> Self {
        let mut out = Canonical::new("ng-contract model 1");
        layout.seal(&mut out);
        Self {
            hash: ModelHash { digest: out.digest() },
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
            blocks,
            modes,
            guards,
            basis,
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
        basis.seal(out);
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
    pub fn unknown(&self) -> UnknownId {
        self.unknown
    }
    pub fn kind(&self) -> RowKind {
        self.kind
    }
    /// The set the row's unknown is refused outside of.
    pub fn admissible(&self) -> AdmissibleSet {
        self.admissible
    }
}
impl Seal for Row {
    fn seal(&self, out: &mut Canonical) {
        self.unknown.seal(out);
        self.kind.seal(out);
        out.number(self.admissible as u64);
        self.dimension.seal(out);
    }
}
/// What a row balances. An amount's row sums the columns that move it, so no law owns an amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowKind {
    Amount { site: SiteId, space: Space },
    Memory(LawName),
    Algebraic(LawName),
}
impl Seal for RowKind {
    fn seal(&self, out: &mut Canonical) {
        match self {
            RowKind::Amount { site, space } => {
                out.number(0);
                site.seal(out);
                out.number(*space as u64);
            }
            RowKind::Memory(name) => {
                out.number(1);
                name.seal(out);
            }
            RowKind::Algebraic(name) => {
                out.number(2);
                name.seal(out);
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
        self.terms.seal(out);
        self.origin.seal(out);
        self.context.seal(out);
        self.receipt.seal(out);
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
    /// The schema boundary it receives for.
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
        self.boundary.seal(out);
        self.coefficient.seal(out);
    }
}

/// Where a program instance applies.
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
        self.id.seal(out);
        self.law.seal(out);
        self.code.seal(out);
    }
}
#[derive(Debug, PartialEq)]
pub(crate) struct Code {
    pub(crate) ops: Vec<Op>,
    pub(crate) dimension: String,
    pub(crate) predicate: Predicate,
    pub(crate) rate: RateClass,
    pub(crate) validity: Vec<Validity>,
}
impl Seal for Code {
    fn seal(&self, out: &mut Canonical) {
        self.ops.seal(out);
        self.dimension.seal(out);
        self.predicate.seal(out);
        self.rate.seal(out);
        self.validity.seal(out);
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

/// A program applied in one context, with what each of its inputs reads there.
#[derive(Debug, PartialEq)]
pub struct Instance {
    pub(crate) program: ProgramId,
    pub(crate) context: Context,
    pub(crate) target: Target,
    pub(crate) parameters: Vec<ParameterId>,
    pub(crate) inputs: Vec<Bound>,
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
}
impl Seal for Instance {
    fn seal(&self, out: &mut Canonical) {
        self.program.seal(out);
        self.context.seal(out);
        self.target.seal(out);
        self.parameters.seal(out);
        self.inputs.seal(out);
    }
}
/// What one input instruction of a program reads at one instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bound {
    Parameter(ParameterId),
    Unknown(UnknownId),
    /// The drive of the schema's boundary at this position in name order.
    Boundary(usize),
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
        self.key.key().seal(out);
        self.binding.seal(out);
        self.default.seal(out);
        out.number(self.admissible as u64);
        self.dimension.seal(out);
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
        self.offsets.seal(out);
        self.unknowns.seal(out);
    }
}

/// A strongly connected part of the matched equation graph, with the most restrictive rate class of
/// the laws in it. Structure only, never permission to lag a dependency.
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
        self.rows.seal(out);
        self.unknowns.seal(out);
        self.rate.seal(out);
    }
}

pub(crate) struct ModeLayout {
    pub(crate) variable: LawName,
    pub(crate) values: Vec<&'static str>,
}
impl Seal for ModeLayout {
    fn seal(&self, out: &mut Canonical) {
        self.variable.seal(out);
        out.number(self.values.len() as u64);
        for value in &self.values {
            value.seal(out);
        }
    }
}
pub(crate) struct GuardLayout {
    pub(crate) variable: LawName,
    pub(crate) leaves: &'static str,
    pub(crate) enters: &'static str,
    pub(crate) crossing: Crossing,
    pub(crate) code: Code,
    pub(crate) inputs: Vec<Bound>,
}
impl Seal for GuardLayout {
    fn seal(&self, out: &mut Canonical) {
        self.variable.seal(out);
        self.leaves.seal(out);
        self.enters.seal(out);
        self.crossing.seal(out);
        self.code.seal(out);
        self.inputs.seal(out);
    }
}
