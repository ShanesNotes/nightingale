//! The declaration language: capabilities, closed expression kernels and fragments.
//!
//! A kernel is a closed expression graph, NOT a Rust callback. Each kernel is built inside
//! [`FragmentBuilder::kernel`], whose closure receives a fresh invariant lifetime `'k`, so an
//! [`Expr`] cannot leave its graph or enter another one. Inputs declare dependencies; structural
//! sparsity is generated from exactly those inputs. Operators come from a pinned deterministic
//! instruction set with dimension signatures. An [`Expr`] carries no value, so a kernel cannot call
//! host math, whose precision Rust leaves unspecified; transcendental operators such as
//! [`Kernel::exp`] are the contract's own pinned routines.
use super::catalog::{Coupling, Defines, Reads};
use super::{
    Admissible, AdmissionError, Brand, Conductance, Digest, Dimensionless, Energy, NonMaterial,
    ParameterKey, PerSite, PoolRef, Pressure, Product, Quotient, RateClass, Rate, Reference,
    Representation, SameDimension, Sensed, SiteClass, ValidityRange, Volume,
};
use core::marker::PhantomData;

/// Port endpoint capability: refers to a pool in the one registry, never to a pool name or amount.
/// The owner brand describes the LAW allowed to use it, not ownership of storage.
pub struct Endpoint<'b, O, Q> {
    _pool: PoolRef,
    _brand: Brand<'b>,
    _kind: PhantomData<(O, Q)>,
}
/// Same species and dimension at both ends, including an explicitly named exterior endpoint.
pub struct Pair<'b, O, Q> {
    _from: Endpoint<'b, O, Q>,
    _to: Endpoint<'b, O, Q>,
}
impl<'b, O, Q> Pair<'b, O, Q> {
    /// Neither raw indices nor optional destinations are accepted.
    pub fn new(_from: Endpoint<'b, O, Q>, _to: Endpoint<'b, O, Q>) -> Self {
        todo!()
    }
}
/// Published read capability; it cannot address arbitrary organ state.
pub struct ReadPort<'b, O, U> {
    _slot: u32,
    _brand: Brand<'b>,
    _kind: PhantomData<(O, U)>,
}
pub struct StateSlot<'b, O, U: NonMaterial> {
    _brand: Brand<'b>,
    _kind: PhantomData<(O, U)>,
}
pub struct AlgebraicSlot<'b, O, U> {
    _brand: Brand<'b>,
    _kind: PhantomData<(O, U)>,
}
/// One field of a published coupling payload; its owner defines it exactly once.
pub struct Output<'b, O, U> {
    _slot: u32,
    _brand: Brand<'b>,
    _kind: PhantomData<(O, U)>,
}
/// A declared parameter slot. Its value comes from the bound packet, never from the fragment.
pub struct Parameter<'b, O, U> {
    _key: ParameterKey,
    _domain: Admissible<U>,
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}

pub(super) struct Instruction {
    _opcode: u16,
    _operands: Vec<u32>,
}
pub struct Dependency {
    _source_slot: u32,
    _dimension: Digest,
}
/// The context of a kernel declared once for the whole body. A kernel declared for a site class
/// `C` (`FragmentBuilder::site_kernel`) is instantiated at every site of that class instead.
pub enum Body {}
/// One kernel's graph while it is being built. `'k` is fresh per kernel; `S` is its site context.
pub struct Kernel<'b, 'k, O, S = Body> {
    _graph: Vec<Instruction>,
    _brand: Brand<'b>,
    _kernel: Brand<'k>,
    _owner: PhantomData<(O, S)>,
}
/// A node of exactly one kernel graph.
pub struct Expr<'b, 'k, O, U> {
    _node: u32,
    _brand: Brand<'b>,
    _kernel: Brand<'k>,
    _kind: PhantomData<(O, U)>,
}
/// A completed graph, still inside its kernel's scope.
pub struct Finished<'b, 'k, O, U, S = Body> {
    _graph: Vec<Instruction>,
    _metadata: KernelMetadata<'b, O, U>,
    _kernel: Brand<'k>,
    _site: PhantomData<S>,
}
/// Materialized from graph construction, not a second manually synchronized declaration.
pub struct KernelMetadata<'b, O, U> {
    _dependencies: Vec<Dependency>,
    _output: PhantomData<U>,
    _rate: RateClass,
    _modes: ModeSet<'b, O>,
}
/// A registered closed kernel: dependency set, rate class and modes all from one graph. A site
/// kernel (`S` a site class) carries one instance per site of its class.
pub struct PureKernel<'b, O, U, S = Body> {
    _graph: Vec<Instruction>,
    _metadata: KernelMetadata<'b, O, U>,
    _brand: Brand<'b>,
    _kind: PhantomData<(O, U, S)>,
}
/// Reads only valid inside a kernel declared for site class `C`: its own site's entry.
impl<'b, 'k, O, C: SiteClass> Kernel<'b, 'k, O, C> {
    pub fn at_site<U>(&mut self, _port: &ReadPort<'b, O, PerSite<C, U>>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    /// The parameter's value at this site: the packet binds one per site, or one for the class.
    pub fn site_parameter<U>(&mut self, _parameter: &SiteParameter<'b, O, C, U>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
}
impl<'b, 'k, O, S> Kernel<'b, 'k, O, S> {
    /// Declares a dependency. There is no read-record or read-by-name operation.
    pub fn read<U>(&mut self, _port: &ReadPort<'b, O, U>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    pub fn parameter<U>(&mut self, _parameter: &Parameter<'b, O, U>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    pub fn state<U: NonMaterial>(&mut self, _slot: &StateSlot<'b, O, U>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    pub fn algebraic<U>(&mut self, _slot: &AlgebraicSlot<'b, O, U>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    pub fn add<U>(&mut self, _a: Expr<'b, 'k, O, U>, _b: Expr<'b, 'k, O, U>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    pub fn subtract<U>(&mut self, _a: Expr<'b, 'k, O, U>, _b: Expr<'b, 'k, O, U>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    /// Generic constitutive multiplication with a checked dimensional signature.
    pub fn conductance_flow<Q>(
        &mut self,
        _conductance: Expr<'b, 'k, O, Conductance<Q>>,
        _head: Expr<'b, 'k, O, Pressure>,
    ) -> Expr<'b, 'k, O, Rate<Q>> {
        todo!()
    }
    /// A typed rate primitive; scaling never receives a solver dt.
    pub fn relax<U>(
        &mut self,
        _target: Expr<'b, 'k, O, U>,
        _current: Expr<'b, 'k, O, U>,
        _inverse_time: Expr<'b, 'k, O, Rate<Dimensionless>>,
    ) -> Expr<'b, 'k, O, Rate<U>> {
        todo!()
    }
    /// The contract's own pinned exponential, identical on native and WASM; never host libm.
    pub fn exp(&mut self, _x: Expr<'b, 'k, O, Dimensionless>) -> Expr<'b, 'k, O, Dimensionless> {
        todo!()
    }
    // Instruction set v1. Each operator is built from correctly rounded IEEE operations and the
    // contract's own transcendental routines, so it replays bit for bit in every profile.
    pub fn multiply<A, B>(&mut self, _a: Expr<'b, 'k, O, A>, _b: Expr<'b, 'k, O, B>) -> Expr<'b, 'k, O, Product<A, B>> {
        todo!()
    }
    /// The divisor's positivity is a listed refusal source (`CompiledBody::refusal_sources`).
    pub fn divide<A, B>(&mut self, _a: Expr<'b, 'k, O, A>, _b: Expr<'b, 'k, O, B>) -> Expr<'b, 'k, O, Quotient<A, B>> {
        todo!()
    }
    /// Names a product or quotient through the pinned identity table; there is no free cast.
    pub fn as_dimension<T: SameDimension<U>, U>(&mut self, _x: Expr<'b, 'k, O, T>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    /// Scales a value by a dimensionless factor (a gain, an occupancy effect).
    pub fn scale<U>(&mut self, _x: Expr<'b, 'k, O, U>, _by: Expr<'b, 'k, O, Dimensionless>) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    /// `0.5 (a + b + sqrt((a - b)^2 + w^2))`: a limit that keeps the Jacobian continuous. `w` is a
    /// declared parameter, never a solver tolerance.
    pub fn smooth_max<U>(
        &mut self,
        _a: Expr<'b, 'k, O, U>,
        _b: Expr<'b, 'k, O, U>,
        _width: Expr<'b, 'k, O, U>,
    ) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    pub fn smooth_min<U>(
        &mut self,
        _a: Expr<'b, 'k, O, U>,
        _b: Expr<'b, 'k, O, U>,
        _width: Expr<'b, 'k, O, U>,
    ) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    /// `1 / (1 + exp(-x))` from the pinned `exp`: saturating responses such as the baroreflex.
    pub fn logistic(&mut self, _x: Expr<'b, 'k, O, Dimensionless>) -> Expr<'b, 'k, O, Dimensionless> {
        todo!()
    }
    /// The argument's positivity is a listed refusal source.
    pub fn ln(&mut self, _x: Expr<'b, 'k, O, Dimensionless>) -> Expr<'b, 'k, O, Dimensionless> {
        todo!()
    }
    /// `measured - reference`: the only way a set point meets the variable it guards.
    pub fn deviation<U: Sensed>(
        &mut self,
        _measured: Expr<'b, 'k, O, U>,
        _reference: Expr<'b, 'k, O, Reference<U>>,
    ) -> Expr<'b, 'k, O, U> {
        todo!()
    }
    /// The stream's value for its current event ordinal, uniform on (0, 1). It stays constant
    /// until a guard drawing on the stream fires, which advances the ordinal on publication.
    pub fn draw(&mut self, _stream: &Stream<'b, O>) -> Expr<'b, 'k, O, Dimensionless> {
        todo!()
    }
    /// The forcing's value at the stage time: a cubic B-spline over knot values that are a pure
    /// function of the run seed, the forcing's key and the knot index. Knots are not breakpoints.
    pub fn forcing(&mut self, _forcing: &Forcing<'b, O>) -> Expr<'b, 'k, O, Dimensionless> {
        todo!()
    }
    /// Closes the graph with its validity modes and rate class. The output must be finite and
    /// takes no admissible set: refusal sets attach to slots, never to a computed expression, so
    /// `pressure - reference >= 0` cannot become a refusal rule. Operator preconditions (a
    /// logarithm's positive argument) are the instruction set's and are listed by
    /// `CompiledBody::refusal_sources`.
    pub fn finish<U>(
        self,
        _output: Expr<'b, 'k, O, U>,
        _modes: ModeSet<'b, O>,
        _rate: RateClass,
    ) -> Finished<'b, 'k, O, U, S> {
        todo!()
    }
}

pub struct Transfer<'b, O, Q> {
    _pair: Pair<'b, O, Q>,
    _rate: PureKernel<'b, O, Rate<Q>>,
}
impl<'b, O, Q> Transfer<'b, O, Q> {
    pub fn new(_pair: Pair<'b, O, Q>, _rate: PureKernel<'b, O, Rate<Q>>) -> Self {
        todo!()
    }
}
/// All species and heat advection on one fluid route expand from the schema, never a second list.
pub struct FluidRoute<'b, O> {
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}
/// A physical energy exchange: two energy endpoints and a unique work claim.
pub struct WorkPair<'b, O> {
    _pair: Pair<'b, O, Energy>,
    _work: PhysicalProcess,
}
pub struct PhysicalProcess {
    _catalog_key: Digest,
}
/// The modes a kernel holds in: every mode, or one value of one mode variable.
pub struct ModeSet<'b, O> {
    _values: Vec<(u32, u32)>,
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}
/// One value of an owner's discrete mode variable (a valve shut or open). The candidate's mode
/// vector holds the current value; only a guard's transition changes it.
pub struct Mode<'b, O> {
    _variable: u32,
    _value: u32,
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}
impl<'b, O> Mode<'b, O> {
    /// A kernel finished with this set holds only while its variable has this value.
    pub fn only(&self) -> ModeSet<'b, O> {
        todo!()
    }
    /// The change a guard fires, from this value to `target`. The same value, or a value of
    /// another variable, is refused.
    pub fn to(&self, _target: &Mode<'b, O>) -> Result<ModeTransition<'b, O>, AdmissionError> {
        todo!()
    }
}
pub struct ModeTransition<'b, O> {
    _variable: u32,
    _from: u32,
    _to: u32,
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}
pub enum Crossing {
    Rising,
    Falling,
    Either,
}
/// A zero function, the direction of crossing that counts and the transition it fires. The
/// zero kernel keeps its own dimension; compilation requires it to hold in the mode it leaves.
pub struct Guard<'b, O, U> {
    _zero: PureKernel<'b, O, U>,
    _direction: Crossing,
    _transition: ModeTransition<'b, O>,
    _advances: Option<&'static str>,
}
impl<'b, O, U: Sensed> Guard<'b, O, U> {
    /// The zero kernel's dimension must be sensed: a guard never fires on a set point itself.
    pub fn new(
        _zero: PureKernel<'b, O, U>,
        _direction: Crossing,
        _transition: ModeTransition<'b, O>,
    ) -> Self {
        todo!()
    }
    /// Per-event variability (a beat, a breath): firing advances the stream's ordinal when the
    /// image publishes, so `draw(stream)` takes its next value. One advancing guard per stream.
    pub fn advances(self, _stream: &Stream<'b, O>) -> Self {
        todo!()
    }
}
/// Seeded chance with a rate physiology sets: the guard fires when the integrated hazard (memory
/// whose rate the owner declares) reaches the stream's exponential draw, `-ln(u)`. On publication
/// the contract resets the integral, advances the stream and applies the transition together; a
/// refusal discards all three. Better care lowers the rate, so the same dice give a later event
/// or none.
pub struct HazardGuard<'b, O> {
    _integral: u32,
    _stream: Stream<'b, O>,
    _transition: ModeTransition<'b, O>,
}
impl<'b, O> HazardGuard<'b, O> {
    /// The integral's rate is still declared once with `memory_rate`.
    pub fn new(
        _integral: &StateSlot<'b, O, Dimensionless>,
        _stream: Stream<'b, O>,
        _transition: ModeTransition<'b, O>,
    ) -> Self {
        todo!()
    }
}
/// A random stream minted under a stable key (owner plus name, like a `ParameterKey`), which
/// enters the model hash, so adding a stream renumbers no other. Values are a pure function of
/// the run seed, the key and a physiological event ordinal, never of steps or attempts.
pub struct Stream<'b, O> {
    _key: &'static str,
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}
/// Smooth variability tied to simulated time: knots every `period_ns` with seeded values,
/// interpolated by a cubic B-spline. A pure function of seed, key and knot index, so replay at any
/// pace and step policy is exact; admission holds it at its mean.
pub struct Forcing<'b, O> {
    _key: &'static str,
    _period_ns: u64,
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}
/// A parameter declared once for a site class; the packet binds a value per site or one for the
/// class, by `key` qualified with the site's key.
pub struct SiteParameter<'b, O, C, U> {
    _key: ParameterKey,
    _brand: Brand<'b>,
    _kind: PhantomData<(O, C, U)>,
}
/// A paired route present at every site of class `C` (plasma to interstitium at every capillary
/// bed, release at every junction). One declaration, one column per site.
pub struct SiteRoute<'b, O, C, Q> {
    _route: u32,
    _brand: Brand<'b>,
    _kind: PhantomData<(O, C, Q)>,
}
/// Minted by a scope only after exact element and charge accounting and energy-route checks.
pub struct BalancedReaction<'b, O> {
    _stoichiometry: Vec<(u32, i64)>,
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}

/// Consumed declarations enforce single use; the model compiler checks global coverage and modes.
pub struct FragmentBuilder<'b, O> {
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}
impl<'b, O> FragmentBuilder<'b, O> {
    /// Builds one kernel. The closure's `'k` is fresh, so expressions from another kernel,
    /// open or finished, do not type-check here.
    pub fn kernel<U, F>(&self, _build: F) -> PureKernel<'b, O, U>
    where
        F: for<'k> FnOnce(Kernel<'b, 'k, O>) -> Finished<'b, 'k, O, U>,
    {
        todo!()
    }
    /// Builds one kernel for every site of class `C` the owner's scope grants: declared once, the
    /// compiler instantiates it per site. Its graph may read `at_site` ports and site parameters.
    pub fn site_kernel<C: SiteClass, U, F>(&self, _build: F) -> PureKernel<'b, O, U, C>
    where
        F: for<'k> FnOnce(Kernel<'b, 'k, O, C>) -> Finished<'b, 'k, O, U, C>,
    {
        todo!()
    }
    pub fn site_parameter<C: SiteClass, U>(&mut self, _key: ParameterKey, _domain: Admissible<U>) -> SiteParameter<'b, O, C, U> {
        todo!()
    }
    /// One transfer law for every site of the route's class.
    pub fn transfer_at<C: SiteClass, Q>(&mut self, _route: SiteRoute<'b, O, C, Q>, _rate: PureKernel<'b, O, Rate<Q>, C>) {
        todo!()
    }
    /// Slow memory in the guarded variable's unit (`Reference<U>`), moved by `memory_rate` like any
    /// memory. Disease and history shift set points; nothing writes the guarded variable.
    pub fn set_point<U: Sensed>(&mut self, _local_name: &str, _domain: Admissible<Reference<U>>) -> StateSlot<'b, O, Reference<U>> {
        todo!()
    }
    pub fn stream(&mut self, _name: &'static str) -> Stream<'b, O> {
        todo!()
    }
    pub fn forcing(&mut self, _name: &'static str, _period_ns: u64) -> Result<Forcing<'b, O>, AdmissionError> {
        todo!()
    }
    /// Declares where a kernel's output is trusted. Leaving the range is reported as a
    /// `ValidityExcursion` on the receipt; it never refuses a step or faults the session.
    pub fn validity<U>(&mut self, _kernel: &PureKernel<'b, O, U>, _range: ValidityRange<U>) {
        todo!()
    }
    /// Every mode of this owner: the kernel is valid throughout.
    pub fn every_mode(&self) -> ModeSet<'b, O> {
        todo!()
    }
    /// Declares one discrete mode variable and returns its values in order. Initialization picks
    /// the starting value consistent with the guards; compilation checks law coverage per value.
    pub fn modes<const N: usize>(&mut self, _local_name: &str, _values: [&str; N]) -> [Mode<'b, O>; N] {
        todo!()
    }
    pub fn parameter<U>(&mut self, _key: ParameterKey, _domain: Admissible<U>) -> Parameter<'b, O, U> {
        todo!()
    }
    /// A non-material memory slot and the closed set it must stay in (`Admissible`).
    pub fn state<U: NonMaterial>(&mut self, _local_name: &str, _domain: Admissible<U>) -> StateSlot<'b, O, U> {
        todo!()
    }
    /// Declares a memory slot's rate kernel. The contract integrates it; no organ steps its own state.
    pub fn memory_rate<U: NonMaterial>(&mut self, _slot: StateSlot<'b, O, U>, _rate: PureKernel<'b, O, Rate<U>>) {
        todo!()
    }
    /// Declares local non-authoritative mechanical or chemical closure coordinates.
    pub fn algebraic<U>(&mut self, _local_name: &str, _domain: Admissible<U>) -> AlgebraicSlot<'b, O, U> {
        todo!()
    }
    pub fn residual<U>(&mut self, _slot: AlgebraicSlot<'b, O, U>, _residual: PureKernel<'b, O, U>) {
        todo!()
    }
    /// Defines one published coupling field.
    pub fn define<U>(&mut self, _output: Output<'b, O, U>, _value: PureKernel<'b, O, U>) {
        todo!()
    }
    /// Issues the definition capabilities of a coupling-map edge this owner emits.
    pub fn emit<P>(&mut self) -> P
    where
        P: Coupling<From = O, Side = Defines<'b, O>>,
    {
        todo!()
    }
    /// Issues read capabilities for a coupling-map edge drawn TO this owner. Nothing else is readable
    /// across owners, so every system-to-system signal is a line of the map.
    pub fn receive<P>(&mut self) -> P
    where
        P: Coupling<To = O, Side = Reads<'b, O>>,
    {
        todo!()
    }
    pub fn transfer<Q>(&mut self, _transfer: Transfer<'b, O, Q>) {
        todo!()
    }
    pub fn advect(&mut self, _route: FluidRoute<'b, O>, _flow: PureKernel<'b, O, Rate<Volume>>) {
        todo!()
    }
    /// Energy is a book: power moves joules between two energy endpoints under one work claim.
    pub fn work(&mut self, _pair: WorkPair<'b, O>, _power: PureKernel<'b, O, Rate<Energy>>) {
        todo!()
    }
    pub fn reaction(&mut self, _reaction: BalancedReaction<'b, O>, _rate: PureKernel<'b, O, Rate<Dimensionless>>) {
        todo!()
    }
    pub fn event<U>(&mut self, _guard: Guard<'b, O, U>) {
        todo!()
    }
    pub fn hazard(&mut self, _guard: HazardGuard<'b, O>) {
        todo!()
    }
    /// Claims stay attached to the physical pathway and are checked by compilation.
    pub fn kinetic_claim(&mut self, _claim: KineticClaim) {
        todo!()
    }
    pub fn finish(self) -> Fragment<'b, O> {
        todo!()
    }
}
pub struct Fragment<'b, O> {
    _data: FragmentData,
    _brand: Brand<'b>,
    _owner: PhantomData<O>,
}
pub(super) struct FragmentData {
    _declarations: Vec<LawDeclaration>,
    _parameters: Vec<ParameterKey>,
    _kernel_programs: Vec<Vec<Instruction>>,
    _source_identity: Digest,
}
struct LawDeclaration {
    _process: PhysicalProcess,
    _kernel: u32,
    _rows: Vec<u32>,
    _ports: Vec<Dependency>,
}

pub enum KineticRole {
    Absorption,
    Distribution,
    Binding,
    Transformation,
    Excretion,
}
pub struct KineticClaim {
    _pathway: PhysicalProcess,
    _role: KineticRole,
    _representation: Representation,
}
/// No separate drug store. Duplicate (site, ligand, role, pathway) is refused regardless of representation.
pub struct PharmacokineticsRegistry {
    _claims: Vec<KineticClaim>,
}
