use crate::catalog::{self, Coupling, Defines, Reads};
use crate::identity::{Canonical, Seal};
use crate::quantity::dimension;
use crate::scope::ENCLOSURE_PRESSURE;
use crate::{
    Admissible, AdmissibleSet, AdmissionError, Basis, Blood, Boundary, CapillaryBed, Conductance, Deviation,
    Dimension, Dimensionless, Landmark, Length, Network, NonMaterial, Owner, ParameterKey, PerEdge, PerSite,
    Pressure, Product, Quotient, Rate, Reference, SameDimension, Sensed, SiteSet, Sites, Space, Species,
    Substance, Time, Urinary, ValidityRange, Volume, Water,
};
use core::cell::Cell;
use core::marker::PhantomData;

/// Invariant in `'a`, so values branded by two assemblies or two kernels never unify.
pub(crate) type Brand<'a> = PhantomData<fn(&'a ()) -> &'a ()>;

/// A law's name: its owner's card name and the slot, mode variable, route family or boundary it
/// serves. A name exists before any law is attached, so a refusal can name an omitted law.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LawName {
    pub owner: &'static str,
    pub local: &'static str,
}
/// One value of one mode variable of the owner the surrounding name gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModeValue {
    pub variable: &'static str,
    pub value: &'static str,
}
/// Where a law holds: in every mode of its owner, or while one mode variable has one value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Predicate {
    EveryMode,
    Only(ModeValue),
}

/// A declared scheduling constraint. It never makes an organ lane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RateClass {
    band: RateBand,
    maximum_ns: u64,
}
impl RateClass {
    /// A block no law moves: a held pool, which tolerates any interval.
    pub(crate) const UNMOVED: Self = Self {
        band: RateBand::Hours,
        maximum_ns: u64::MAX,
    };
    /// Refuses a zero interval.
    pub fn new(band: RateBand, maximum_ns: u64) -> Result<Self, AdmissionError> {
        if maximum_ns == 0 {
            return Err(AdmissionError::ZeroRateInterval);
        }
        Ok(Self { band, maximum_ns })
    }
    pub fn band(&self) -> RateBand {
        self.band
    }
    /// The longest interval of simulated time, in nanoseconds, the process tolerates.
    pub fn maximum_ns(&self) -> u64 {
        self.maximum_ns
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RateBand {
    Wave,
    Beat,
    Reflex,
    Minutes,
    Hours,
}

/// The context of a kernel declared once for the whole body.
pub enum Body {}
mod edges {
    pub trait Sealed {}
}
/// The context of a kernel declared once for every edge of one route: what each end holds and whose value an
/// unbound edge parameter falls back to.
pub trait EdgeKind: edges::Sealed {
    type From: SiteSet;
    type To: SiteSet;
    const NETWORK: Network;
}
macro_rules! edge_kinds {
    ($($(#[$doc:meta])* $name:ident: $from:ty => $to:ty, $network:ident;)*) => {$(
        $(#[$doc])*
        pub enum $name {}
        impl edges::Sealed for $name {}
        impl EdgeKind for $name {
            type From = $from;
            type To = $to;
            const NETWORK: Network = Network::$network;
        }
    )*};
}
edge_kinds! {
    /// Every vascular edge with no valve: its flow is an algebraic row.
    VascularEdge: Blood => Blood, Vascular;
    /// Every vascular edge with a valve: its flow is memory and its laws hold by mode.
    ValveEdge: Blood => Blood, Vascular;
    /// Every lymph edge: a bed's interstitium to a vessel's lumen.
    LymphEdge: CapillaryBed => Blood, Lymph;
    /// Every urinary edge out of a bed: its plasma to a lumen.
    FilterEdge: Blood => Urinary, Urinary;
    /// Every urinary edge between two lumens.
    TubuleEdge: Urinary => Urinary, Urinary;
    /// Every reclaim edge: a lumen back to a bed's plasma.
    ReclaimEdge: Urinary => Blood, Reclaim;
}

/// One kind of transfer the map implies. A fluid route's water rate is a row and it carries every species both ends
/// hold; any other moves its one species. A route stands wherever its two pools exist, so the schema's pools say
/// where a law reaches and no law names a site.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Family {
    Circulation,
    Valves,
    Starling,
    Lymph,
    Filtration,
    Tubular,
    Reclaim(Species),
    Release(Species),
    Reuptake(Species),
    Binding(Species),
    Exchange(Species),
    StrokeWork,
    Dissipation,
}
impl Family {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Family::Circulation => "circulation",
            Family::Valves => "valves",
            Family::Starling => "starling",
            Family::Lymph => "lymph",
            Family::Filtration => "filtration",
            Family::Tubular => "tubular flow",
            Family::Reclaim(species) => species.names()[0],
            Family::Release(species) => species.names()[1],
            Family::Reuptake(species) => species.names()[2],
            Family::Binding(species) => species.names()[3],
            Family::Exchange(species) => species.names()[5],
            Family::StrokeWork => "stroke work",
            Family::Dissipation => "dissipation",
        }
    }
    /// Whether a row of this name is a route's own flow.
    pub(crate) fn flows(local: &str) -> bool {
        ["circulation", "starling", "lymph", "filtration", "tubular flow"].contains(&local)
    }
    /// Whether this route moves water that can carry what both ends hold. In a body that holds more than water its law
    /// defines one flow row, and the contract writes every column from it. A valve's flow is memory in any body.
    pub(crate) fn fluid(self) -> bool {
        matches!(
            self,
            Family::Circulation
                | Family::Valves
                | Family::Starling
                | Family::Lymph
                | Family::Filtration
                | Family::Tubular
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum End {
    From,
    To,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Input(Input),
    Zero(String),
    Add([usize; 2]),
    Subtract([usize; 2]),
    Multiply([usize; 2]),
    Divide([usize; 2]),
    AsDimension([usize; 1]),
    Scale([usize; 2]),
    SmoothMax([usize; 3]),
    SmoothMin([usize; 3]),
    Exp([usize; 1]),
    Logistic([usize; 1]),
    Ln([usize; 1]),
    Deviation([usize; 2]),
    Relax([usize; 3]),
    ConductanceFlow([usize; 2]),
    Volume([usize; 1]),
    /// The species a water flow carries: `q s_from / w_from` for a forward flow, `q s_to / w_to` for a reverse one.
    Upwind([usize; 5]),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Parameter(ParameterKey),
    SiteParameter(ParameterKey),
    EdgeParameter(ParameterKey),
    EndParameter(ParameterKey, End),
    Memory(&'static str),
    Algebraic(&'static str),
    Read(Source, Where),
    Draw(&'static str),
    Forcing(&'static str),
    /// A site's height in the body frame under the head's posture.
    Height(Where),
    /// The flow row of the one urinary edge into this edge's upstream site.
    Inflow,
}
/// Where a read lands, from the kernel's own place. The compiler resolves it against the map, so a law says what it
/// reads and never where that is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Where {
    Here,
    Up,
    Down,
    /// The junction that supplies this site, or the site an edge runs into.
    Junction,
    /// The one site this bed drains to.
    Drain,
    /// The site a landmark's rows stand at.
    Mark(Landmark),
    /// The site whose height a landmark sits at.
    Sensor(Landmark),
}
impl Op {
    pub(crate) fn operands(&self) -> &[usize] {
        match self {
            Op::Input(_) | Op::Zero(_) => &[],
            Op::Add(operands)
            | Op::Subtract(operands)
            | Op::Multiply(operands)
            | Op::Divide(operands)
            | Op::Scale(operands)
            | Op::Deviation(operands)
            | Op::ConductanceFlow(operands) => operands,
            Op::SmoothMax(operands) | Op::SmoothMin(operands) | Op::Relax(operands) => operands,
            Op::AsDimension(operands)
            | Op::Exp(operands)
            | Op::Logistic(operands)
            | Op::Ln(operands)
            | Op::Volume(operands) => operands,
            Op::Upwind(operands) => operands,
        }
    }
    pub(crate) fn map(&self, f: impl FnMut(usize) -> usize) -> Op {
        match self {
            Op::Input(input) => Op::Input(*input),
            Op::Zero(dimension) => Op::Zero(dimension.clone()),
            Op::Add(operands) => Op::Add(operands.map(f)),
            Op::Subtract(operands) => Op::Subtract(operands.map(f)),
            Op::Multiply(operands) => Op::Multiply(operands.map(f)),
            Op::Divide(operands) => Op::Divide(operands.map(f)),
            Op::AsDimension(operands) => Op::AsDimension(operands.map(f)),
            Op::Scale(operands) => Op::Scale(operands.map(f)),
            Op::SmoothMax(operands) => Op::SmoothMax(operands.map(f)),
            Op::SmoothMin(operands) => Op::SmoothMin(operands.map(f)),
            Op::Exp(operands) => Op::Exp(operands.map(f)),
            Op::Logistic(operands) => Op::Logistic(operands.map(f)),
            Op::Ln(operands) => Op::Ln(operands.map(f)),
            Op::Deviation(operands) => Op::Deviation(operands.map(f)),
            Op::Relax(operands) => Op::Relax(operands.map(f)),
            Op::ConductanceFlow(operands) => Op::ConductanceFlow(operands.map(f)),
            Op::Volume(operands) => Op::Volume(operands.map(f)),
            Op::Upwind(operands) => Op::Upwind(operands.map(f)),
        }
    }
}
impl Seal for Op {
    fn seal(&self, out: &mut Canonical) {
        let (tag, operands): (u64, &[usize]) = match self {
            Op::Input(input) => {
                out.number(0);
                return input.seal(out);
            }
            Op::Zero(dimension) => {
                out.number(1);
                return out.text(dimension);
            }
            Op::Add(operands) => (2, operands),
            Op::Subtract(operands) => (3, operands),
            Op::Multiply(operands) => (4, operands),
            Op::Divide(operands) => (5, operands),
            Op::AsDimension(operands) => (6, operands),
            Op::Scale(operands) => (7, operands),
            Op::SmoothMax(operands) => (8, operands),
            Op::SmoothMin(operands) => (9, operands),
            Op::Exp(operands) => (10, operands),
            Op::Logistic(operands) => (11, operands),
            Op::Ln(operands) => (12, operands),
            Op::Deviation(operands) => (13, operands),
            Op::Relax(operands) => (14, operands),
            Op::ConductanceFlow(operands) => (15, operands),
            Op::Volume(operands) => (16, operands),
            Op::Upwind(operands) => (17, operands),
        };
        out.number(tag);
        for &operand in operands {
            out.number(operand as u64);
        }
    }
}
impl Seal for Input {
    fn seal(&self, out: &mut Canonical) {
        match *self {
            Input::Parameter(key) => {
                out.number(0);
                out.text(key.key());
            }
            Input::SiteParameter(key) => {
                out.number(1);
                out.text(key.key());
            }
            Input::EdgeParameter(key) => {
                out.number(2);
                out.text(key.key());
            }
            Input::EndParameter(key, end) => {
                out.number(3);
                out.text(key.key());
                out.number(end as u64);
            }
            Input::Memory(name) => {
                out.number(4);
                out.text(name);
            }
            Input::Algebraic(name) => {
                out.number(5);
                out.text(name);
            }
            Input::Read(source, place) => {
                out.number(6);
                source.seal(out);
                place.seal(out);
            }
            Input::Draw(stream) => {
                out.number(8);
                out.text(stream);
            }
            Input::Forcing(forcing) => {
                out.number(9);
                out.text(forcing);
            }
            Input::Height(place) => {
                out.number(12);
                place.seal(out);
            }
            Input::Inflow => out.number(13),
        }
    }
}
impl Seal for Where {
    fn seal(&self, out: &mut Canonical) {
        match *self {
            Where::Here => out.number(0),
            Where::Up => out.number(1),
            Where::Down => out.number(2),
            Where::Junction => out.number(3),
            Where::Drain => out.number(4),
            Where::Mark(landmark) => {
                out.number(5);
                out.number(landmark as u64);
            }
            Where::Sensor(landmark) => {
                out.number(6);
                out.number(landmark as u64);
            }
        }
    }
}
impl Seal for Source {
    fn seal(&self, out: &mut Canonical) {
        match *self {
            Source::Amount(family, end, species) => {
                out.number(0);
                out.text(family.name());
                out.number(end as u64);
                out.number(species as u64);
            }
            Source::Pool(species) => {
                out.number(1);
                out.number(species as u64);
            }
            Source::RoutePool(route, species) => {
                out.number(2);
                out.text(route);
                out.number(species as u64);
            }
            Source::Drive(boundary) => {
                out.number(3);
                out.text(boundary);
            }
            Source::Field(name) => {
                out.number(4);
                name.seal(out);
            }
            Source::Flow(family) => {
                out.number(5);
                out.text(family.name());
            }
            Source::Extravascular => out.number(6),
        }
    }
}

fn reachable(code: &[Op], result: usize) -> Vec<Op> {
    fn place(code: &[Op], at: usize, placed: &mut [Option<usize>], out: &mut Vec<Op>) -> usize {
        if let Some(done) = placed[at] {
            return done;
        }
        let op = code[at].map(|operand| place(code, operand, placed, out));
        out.push(op);
        placed[at] = Some(out.len() - 1);
        out.len() - 1
    }
    let mut out = Vec::new();
    place(code, result, &mut vec![None; code.len()], &mut out);
    out
}

/// One kernel's graph while it is built. `'k` is fresh for every kernel, so an expression belongs to
/// one graph; `S` is what the kernel is declared for: the body, a site class or a vascular edge.
pub struct Kernel<'b, 'k, O, S = Body> {
    code: Vec<Op>,
    brand: Brand<'b>,
    kernel: Brand<'k>,
    kind: PhantomData<(O, S)>,
}
pub struct Expr<'b, 'k, O, U> {
    at: usize,
    brand: Brand<'b>,
    kernel: Brand<'k>,
    kind: PhantomData<(O, U)>,
}
/// One node read twice is one value in the program, so a law that shares a term computes it once.
impl<O, U> Clone for Expr<'_, '_, O, U> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<O, U> Copy for Expr<'_, '_, O, U> {}
pub struct Finished<'b, 'k, O, U, S = Body> {
    code: Vec<Op>,
    result: usize,
    dimension: String,
    modes: ModeSet<'b, O>,
    rate: RateClass,
    kernel: Brand<'k>,
    kind: PhantomData<(U, S)>,
}
/// A closed kernel: its graph, the modes it holds in and its rate class. A site or edge kernel
/// compiles to one program applied at every site or edge of its route.
pub struct PureKernel<'b, O, U, S = Body> {
    data: KernelData,
    brand: Brand<'b>,
    kind: PhantomData<(O, U, S)>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KernelData {
    pub(crate) id: u32,
    pub(crate) code: Vec<Op>,
    pub(crate) dimension: String,
    pub(crate) modes: Selector,
    pub(crate) rate: RateClass,
    pub(crate) defines: bool,
}
impl KernelData {
    /// `value`, negated, so one body row's laws add to `row - sum`.
    fn summing(mut self) -> Self {
        let value = self.code.len() - 1;
        self.code.push(Op::Zero(self.dimension.clone()));
        self.code.push(Op::Subtract([value + 1, value]));
        self
    }
    /// `row - value`, so a coupling is a plain algebraic row and a loop of couplings a coupled residual.
    pub(crate) fn defining(mut self, row: &'static str) -> Self {
        let value = self.code.len() - 1;
        self.code.push(Op::Input(Input::Algebraic(row)));
        self.code.push(Op::Subtract([value + 1, value]));
        self.defines = true;
        self
    }
}

impl<'b, 'k, O, S> Kernel<'b, 'k, O, S> {
    fn open() -> Self {
        Self {
            code: Vec::new(),
            brand: PhantomData,
            kernel: PhantomData,
            kind: PhantomData,
        }
    }
    fn push<U>(&mut self, op: Op) -> Expr<'b, 'k, O, U> {
        self.code.push(op);
        Expr {
            at: self.code.len() - 1,
            brand: PhantomData,
            kernel: PhantomData,
            kind: PhantomData,
        }
    }
    fn input<U>(&mut self, input: Input) -> Expr<'b, 'k, O, U> {
        self.push(Op::Input(input))
    }
    pub fn parameter<U: Dimension>(&mut self, parameter: &Parameter<'b, O, U>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Parameter(parameter.key))
    }
    pub fn state<U: NonMaterial>(&mut self, slot: &StateSlot<'b, O, U>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Memory(slot.name))
    }
    pub fn algebraic<U: Dimension>(&mut self, slot: &AlgebraicSlot<'b, O, U>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Algebraic(slot.name))
    }
    pub fn read<U: Dimension>(&mut self, port: &ReadPort<'b, O, U>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Read(port.source, Where::Here))
    }
    /// The entry at the junction that supplies this site, or the site this edge runs into. A place no junction
    /// supplies, or a junction with no such entry, reads zero: no nerve, or no receptor, has no effect. A whole-body
    /// kernel has no such place, and compile refuses it.
    pub fn at_junction<U: Dimension>(
        &mut self,
        port: &ReadPort<'b, O, PerSite<crate::Junction, U>>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::Read(port.source, Where::Junction))
    }
    /// The stream's value at its current ordinal, uniform on (0, 1) and constant until a guard on the stream
    /// fires. Admission reads its mean, one half.
    pub fn draw(&mut self, stream: &Stream<'b, O>) -> Expr<'b, 'k, O, Dimensionless> {
        self.input(Input::Draw(stream.name))
    }
    /// A rate's factor at the stage's instant: `1 + a s(t)`, with `s` the cubic B-spline over seeded knots and
    /// `a` the forcing's amplitude. It stays positive, its mean is exactly one and admission reads exactly
    /// one, so `scale` by it needs no literal one and no literal can stand in for it.
    pub fn forcing(&mut self, forcing: &Forcing<'b, O>) -> Expr<'b, 'k, O, Dimensionless> {
        self.input(Input::Forcing(forcing.name))
    }
    /// The amount's volume, through the schema's fixed partial-volume coefficient; a body whose
    /// schema declares none is refused.
    pub fn volume(&mut self, amount: Expr<'b, 'k, O, Water>) -> Expr<'b, 'k, O, Volume> {
        self.push(Op::Volume([amount.at]))
    }
    /// The one literal is the additive identity; a physical value enters as a declared parameter.
    pub fn zero<U: Dimension>(&mut self) -> Expr<'b, 'k, O, U> {
        self.push(Op::Zero(dimension::<U>()))
    }
    pub fn add<U>(&mut self, a: Expr<'b, 'k, O, U>, b: Expr<'b, 'k, O, U>) -> Expr<'b, 'k, O, U> {
        self.push(Op::Add([a.at, b.at]))
    }
    pub fn subtract<U>(&mut self, a: Expr<'b, 'k, O, U>, b: Expr<'b, 'k, O, U>) -> Expr<'b, 'k, O, U> {
        self.push(Op::Subtract([a.at, b.at]))
    }
    pub fn multiply<A, B>(
        &mut self,
        a: Expr<'b, 'k, O, A>,
        b: Expr<'b, 'k, O, B>,
    ) -> Expr<'b, 'k, O, Product<A, B>> {
        self.push(Op::Multiply([a.at, b.at]))
    }
    /// The divisor's positivity is a refusal source.
    pub fn divide<A, B>(
        &mut self,
        a: Expr<'b, 'k, O, A>,
        b: Expr<'b, 'k, O, B>,
    ) -> Expr<'b, 'k, O, Quotient<A, B>> {
        self.push(Op::Divide([a.at, b.at]))
    }
    pub fn as_dimension<T: SameDimension<U>, U>(&mut self, x: Expr<'b, 'k, O, T>) -> Expr<'b, 'k, O, U> {
        self.push(Op::AsDimension([x.at]))
    }
    pub fn scale<U>(
        &mut self,
        x: Expr<'b, 'k, O, U>,
        by: Expr<'b, 'k, O, Dimensionless>,
    ) -> Expr<'b, 'k, O, U> {
        self.push(Op::Scale([x.at, by.at]))
    }
    /// `0.5 (a + b + sqrt((a - b)^2 + w^2))`, a limit with a continuous Jacobian. The width `w` comes from
    /// the law's declared values, never from a solver.
    pub fn smooth_max<U>(
        &mut self,
        a: Expr<'b, 'k, O, U>,
        b: Expr<'b, 'k, O, U>,
        width: Expr<'b, 'k, O, U>,
    ) -> Expr<'b, 'k, O, U> {
        self.push(Op::SmoothMax([a.at, b.at, width.at]))
    }
    /// `0.5 (a + b - sqrt((a - b)^2 + w^2))`.
    pub fn smooth_min<U>(
        &mut self,
        a: Expr<'b, 'k, O, U>,
        b: Expr<'b, 'k, O, U>,
        width: Expr<'b, 'k, O, U>,
    ) -> Expr<'b, 'k, O, U> {
        self.push(Op::SmoothMin([a.at, b.at, width.at]))
    }
    /// The contract's own exponential, identical on every host.
    pub fn exp(&mut self, x: Expr<'b, 'k, O, Dimensionless>) -> Expr<'b, 'k, O, Dimensionless> {
        self.push(Op::Exp([x.at]))
    }
    /// `1 / (1 + exp(-x))`, from the contract's own exponential.
    pub fn logistic(&mut self, x: Expr<'b, 'k, O, Dimensionless>) -> Expr<'b, 'k, O, Dimensionless> {
        self.push(Op::Logistic([x.at]))
    }
    /// The argument's positivity is a refusal source.
    pub fn ln(&mut self, x: Expr<'b, 'k, O, Dimensionless>) -> Expr<'b, 'k, O, Dimensionless> {
        self.push(Op::Ln([x.at]))
    }
    /// `measured - reference`: the one way a set point meets the variable it guards. The result is its own
    /// dimension, so it can never stand for the variable (finding 14).
    pub fn deviation<U: Sensed>(
        &mut self,
        measured: Expr<'b, 'k, O, U>,
        reference: Expr<'b, 'k, O, Reference<U>>,
    ) -> Expr<'b, 'k, O, Deviation<U>> {
        self.push(Op::Deviation([measured.at, reference.at]))
    }
    /// `speed * deviation`: a set point moving toward what it senses, which `relax` cannot write.
    pub fn drift<U: Sensed>(
        &mut self,
        deviation: Expr<'b, 'k, O, Deviation<U>>,
        speed: Expr<'b, 'k, O, Rate<Dimensionless>>,
    ) -> Expr<'b, 'k, O, Rate<Reference<U>>> {
        self.push(Op::Scale([deviation.at, speed.at]))
    }
    /// `speed (target - current)`: relaxation as a rate, which never sees a solver's interval.
    pub fn relax<U>(
        &mut self,
        target: Expr<'b, 'k, O, U>,
        current: Expr<'b, 'k, O, U>,
        speed: Expr<'b, 'k, O, Rate<Dimensionless>>,
    ) -> Expr<'b, 'k, O, Rate<U>> {
        self.push(Op::Relax([target.at, current.at, speed.at]))
    }
    pub fn conductance_flow<Q>(
        &mut self,
        conductance: Expr<'b, 'k, O, Conductance<Q>>,
        head: Expr<'b, 'k, O, Pressure>,
    ) -> Expr<'b, 'k, O, Rate<Q>> {
        self.push(Op::ConductanceFlow([conductance.at, head.at]))
    }
    pub fn finish<U: Dimension>(
        self,
        value: Expr<'b, 'k, O, U>,
        modes: ModeSet<'b, O>,
        rate: RateClass,
    ) -> Finished<'b, 'k, O, U, S> {
        Finished {
            code: self.code,
            result: value.at,
            dimension: dimension::<U>(),
            modes,
            rate,
            kernel: PhantomData,
            kind: PhantomData,
        }
    }
}

impl<'b, 'k, O> Kernel<'b, 'k, O> {
    /// A per-site value at a landmark. Compile refuses a landmark the map does not place, or one placed off the port.
    pub fn at_landmark<C: SiteSet, U: Dimension>(
        &mut self,
        landmark: Landmark,
        port: &ReadPort<'b, O, PerSite<C, U>>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::Read(port.source, Where::Mark(landmark)))
    }
    /// How far a landmark sits above the site its rows stand at, under the head's posture.
    pub fn rise(&mut self, landmark: Landmark) -> Expr<'b, 'k, O, Length> {
        let sensor = self.input(Input::Height(Where::Sensor(landmark)));
        let site = self.input(Input::Height(Where::Mark(landmark)));
        self.subtract(sensor, site)
    }
}
impl<'b, 'k, O> Kernel<'b, 'k, O, CapillaryBed> {
    /// This bed's entry of a value every blood site has.
    pub fn in_blood<U: Dimension>(
        &mut self,
        port: &ReadPort<'b, O, PerSite<Blood, U>>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::Read(port.source, Where::Here))
    }
    /// The entry at the one site this bed drains to. Compile refuses a bed with more or fewer.
    pub fn drained<U: Dimension>(&mut self, port: &ReadPort<'b, O, PerSite<Blood, U>>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Read(port.source, Where::Drain))
    }
}
impl<'b, 'k, O, C: SiteSet> Kernel<'b, 'k, O, C> {
    pub fn at_site<U: Dimension>(&mut self, port: &ReadPort<'b, O, PerSite<C, U>>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Read(port.source, Where::Here))
    }
    /// This site's height in the body frame under the head's posture.
    pub fn height(&mut self) -> Expr<'b, 'k, O, Length> {
        self.input(Input::Height(Where::Here))
    }
    /// The value bound at this site, or else at its class.
    pub fn site_parameter<U: Dimension>(
        &mut self,
        parameter: &SiteParameter<'b, O, C, U>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::SiteParameter(parameter.key))
    }
}
impl<'b, 'k, O, E: EdgeKind> Kernel<'b, 'k, O, E> {
    pub fn at_edge<U: Dimension>(&mut self, port: &ReadPort<'b, O, PerEdge<U, E>>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Read(port.source, Where::Here))
    }
    /// The entry at this edge's upstream site.
    pub fn upstream<U: Dimension>(
        &mut self,
        port: &ReadPort<'b, O, PerSite<E::From, U>>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::Read(port.source, Where::Up))
    }
    /// The entry at this edge's downstream site.
    pub fn downstream<U: Dimension>(
        &mut self,
        port: &ReadPort<'b, O, PerSite<E::To, U>>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::Read(port.source, Where::Down))
    }
    /// How far this edge falls in the body frame under the head's posture: the upstream height less the downstream.
    pub fn fall(&mut self) -> Expr<'b, 'k, O, Length> {
        let up = self.input(Input::Height(Where::Up));
        let down = self.input(Input::Height(Where::Down));
        self.subtract(up, down)
    }
    /// The flow row of the one urinary edge into this edge's upstream site. Compile refuses a site with more or fewer.
    pub fn inflow(&mut self) -> Expr<'b, 'k, O, Rate<Water>> {
        self.input(Input::Inflow)
    }
    /// The value bound at this edge, or else at the vascular network.
    pub fn edge_parameter<U: Dimension>(
        &mut self,
        parameter: &EdgeParameter<'b, O, U, E>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::EdgeParameter(parameter.key))
    }
    /// The value bound at this edge's upstream site, or else at the vascular network.
    pub fn upstream_parameter<U: Dimension>(
        &mut self,
        parameter: &EndParameter<'b, O, U>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::EndParameter(parameter.key, End::From))
    }
    /// The value bound at this edge's downstream site, or else at the vascular network.
    pub fn downstream_parameter<U: Dimension>(
        &mut self,
        parameter: &EndParameter<'b, O, U>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::EndParameter(parameter.key, End::To))
    }
}

pub struct ReadPort<'b, O, U> {
    source: Source,
    brand: Brand<'b>,
    kind: PhantomData<(O, U)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    Amount(Family, End, Species),
    /// A species in the blood or lumen pool of a site, not through a route.
    Pool(Species),
    /// A species in the pool a boundary route reaches.
    RoutePool(&'static str, Species),
    Drive(&'static str),
    Field(LawName),
    Flow(Family),
    Extravascular,
}
impl<'b, O, U> ReadPort<'b, O, U> {
    fn of(source: Source) -> Self {
        Self {
            source,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    pub(crate) fn field(name: LawName) -> Self {
        Self::of(Source::Field(name))
    }
    pub(crate) fn pool(species: Species) -> Self {
        Self::of(Source::Pool(species))
    }
}
impl<'b, O, C: SiteSet> ReadPort<'b, O, PerSite<C, Pressure>> {
    pub(crate) fn extravascular() -> Self {
        Self::of(Source::Extravascular)
    }
}

/// One field of a coupling or an enclosure's pressure, which its owner owes a law.
pub struct Output<'b, O, U> {
    name: &'static str,
    brand: Brand<'b>,
    kind: PhantomData<(O, U)>,
}
impl<'b, O, U> Output<'b, O, U> {
    pub(crate) fn named(name: &'static str) -> Self {
        Self {
            name,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Over {
    Body,
    Every(Sites),
    HeldEnclosures,
    /// Every place of one route, as a valve's flow and mode stand.
    Route(Family),
    /// Every site of a class where a pool exists, as a receptor's effect stands where its bound pool does.
    WherePool(Sites, Space, Species),
}
pub(crate) trait Carried {
    fn over() -> Over;
    fn dimension() -> String;
}
impl<U: Dimension> Carried for U {
    fn over() -> Over {
        Over::Body
    }
    fn dimension() -> String {
        dimension::<U>()
    }
}
impl<C: SiteSet, U: Dimension> Carried for PerSite<C, U> {
    fn over() -> Over {
        Over::Every(C::SET)
    }
    fn dimension() -> String {
        dimension::<U>()
    }
}
/// A route family at every site of class `C`: one column per site, from one space to another.
pub struct SiteRoute<'b, O, C, Q> {
    family: Family,
    brand: Brand<'b>,
    kind: PhantomData<(O, C, Q)>,
}
impl<'b, O, C: SiteSet, Q> SiteRoute<'b, O, C, Q> {
    pub(crate) fn requested(family: Family) -> Self {
        Self {
            family,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    /// The amount of any species at this route's upstream end. A read of a pool that does not exist is refused.
    pub fn from_amount<R: Substance>(&self) -> ReadPort<'b, O, PerSite<C, R>> {
        ReadPort::of(Source::Amount(self.family, End::From, R::SPECIES))
    }
    pub fn to_amount<R: Substance>(&self) -> ReadPort<'b, O, PerSite<C, R>> {
        ReadPort::of(Source::Amount(self.family, End::To, R::SPECIES))
    }
}
/// A route family along every edge of one kind: one column per edge and per species carried.
pub struct EdgeRoute<'b, O, Q, E: EdgeKind = VascularEdge> {
    family: Family,
    brand: Brand<'b>,
    kind: PhantomData<(O, Q, E)>,
}
impl<'b, O, Q, E: EdgeKind> EdgeRoute<'b, O, Q, E> {
    pub(crate) fn requested(family: Family) -> Self {
        Self {
            family,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    pub fn from_amount<R: Substance>(&self) -> ReadPort<'b, O, PerEdge<R, E>> {
        ReadPort::of(Source::Amount(self.family, End::From, R::SPECIES))
    }
    pub fn to_amount<R: Substance>(&self) -> ReadPort<'b, O, PerEdge<R, E>> {
        ReadPort::of(Source::Amount(self.family, End::To, R::SPECIES))
    }
    /// The water rate along each edge, a row of its own, so a lens or another law reads a flow as it reads anything.
    pub fn flow(&self) -> ReadPort<'b, O, PerEdge<Rate<Water>, E>> {
        ReadPort::of(Source::Flow(self.family))
    }
}
impl<'b, O> EdgeRoute<'b, O, Water, crate::ValveEdge> {
    /// A valve's flow is memory: its laws attach with `memory_rate`, each in its mode, and a guard resets it.
    pub fn flow_memory(&self) -> StateSlot<'b, O, Rate<Water>> {
        StateSlot {
            name: self.family.name(),
            brand: PhantomData,
            kind: PhantomData,
        }
    }
}

pub struct Endpoint<'b, O, Q> {
    brand: Brand<'b>,
    kind: PhantomData<(O, Q)>,
}
/// Both ends of one transfer, the same species at each, one of them the receipt pool outside the
/// body. The schema names where it reaches into the body.
pub struct Pair<'b, O, Q> {
    serves: Obligation,
    brand: Brand<'b>,
    kind: PhantomData<(O, Q)>,
}
impl<'b, O, Q> Pair<'b, O, Q> {
    pub(crate) fn serving(serves: Obligation) -> Self {
        Self {
            serves,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    /// The amount of any species in the pool this pair leaves or, across the boundary, reaches.
    pub fn amount<R: Substance>(&self) -> ReadPort<'b, O, R> {
        ReadPort::of(Source::RoutePool(self.serves.local(), R::SPECIES))
    }
}
/// A transfer law on a pair: what leaves one end enters the other.
pub struct Transfer<'b, O, Q> {
    serves: Obligation,
    rate: PureKernel<'b, O, Rate<Q>>,
}
impl<'b, O, Q> Transfer<'b, O, Q> {
    pub fn new(pair: &Pair<'b, O, Q>, rate: PureKernel<'b, O, Rate<Q>>) -> Self {
        Self {
            serves: pair.serves,
            rate,
        }
    }
}
/// A schema boundary's pair and the drive the boundary port supplies at each stage.
pub struct BoundaryRoute<'b, Q, O = Boundary> {
    pub route: Pair<'b, O, Q>,
    pub drive: ReadPort<'b, O, Rate<Q>>,
}
impl<'b, Q, O> BoundaryRoute<'b, Q, O> {
    pub(crate) fn named(boundary: &'static str) -> Self {
        Self {
            route: Pair::serving(Obligation::Boundary(boundary)),
            drive: ReadPort::of(Source::Drive(boundary)),
        }
    }
}

/// A whole-body parameter slot. Its value binds later, by key, and stays out of the model hash.
pub struct Parameter<'b, O, U> {
    key: ParameterKey,
    brand: Brand<'b>,
    kind: PhantomData<(O, U)>,
}
pub struct StateSlot<'b, O, U: NonMaterial> {
    name: &'static str,
    brand: Brand<'b>,
    kind: PhantomData<(O, U)>,
}
/// An unknown its residual law closes.
pub struct AlgebraicSlot<'b, O, U> {
    name: &'static str,
    brand: Brand<'b>,
    kind: PhantomData<(O, U)>,
}
/// A parameter of every site of class `C`, bound per site with the class as default.
pub struct SiteParameter<'b, O, C, U> {
    key: ParameterKey,
    brand: Brand<'b>,
    kind: PhantomData<(O, C, U)>,
}
/// A parameter of every edge of one kind, bound per edge with the network as default.
pub struct EdgeParameter<'b, O, U, E: EdgeKind = VascularEdge> {
    key: ParameterKey,
    brand: Brand<'b>,
    kind: PhantomData<(O, U, E)>,
}
/// A parameter of every site at an end of a vascular edge, bound per site with the network as
/// default, so one edge law reads each end's own value without naming a site.
pub struct EndParameter<'b, O, U> {
    key: ParameterKey,
    brand: Brand<'b>,
    kind: PhantomData<(O, U)>,
}

/// Where a kernel holds: every mode of its owner, or one value of one mode variable.
pub struct ModeSet<'b, O> {
    selector: Selector,
    brand: Brand<'b>,
    owner: PhantomData<O>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Selector {
    Every,
    Only { variable: usize, value: usize },
}
/// One value of one of its owner's mode variables, such as a valve shut or open. Only a guard's
/// transition changes the current value.
pub struct Mode<'b, O> {
    variable: usize,
    value: usize,
    name: ModeValue,
    brand: Brand<'b>,
    owner: PhantomData<O>,
}
impl<'b, O> Mode<'b, O> {
    pub fn only(&self) -> ModeSet<'b, O> {
        ModeSet {
            selector: Selector::Only {
                variable: self.variable,
                value: self.value,
            },
            brand: PhantomData,
            owner: PhantomData,
        }
    }
    /// The change a guard fires. Refuses the same value and a value of another variable.
    pub fn to(&self, target: &Mode<'b, O>) -> Result<ModeTransition<'b, O>, AdmissionError> {
        if self.variable != target.variable || self.value == target.value {
            return Err(AdmissionError::InvalidTransition {
                from: self.name,
                to: target.name,
            });
        }
        Ok(ModeTransition {
            variable: self.variable,
            from: self.value,
            to: target.value,
            brand: PhantomData,
            owner: PhantomData,
        })
    }
}
pub struct ModeTransition<'b, O> {
    variable: usize,
    from: usize,
    to: usize,
    brand: Brand<'b>,
    owner: PhantomData<O>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Crossing {
    Rising,
    Falling,
    Either,
}
/// A zero function, the crossing that counts and the transition it fires. The zero kernel keeps its
/// own dimension, and compilation requires it to hold in the value the transition leaves. A guard over a route's
/// mode has one instance per edge.
///
/// It fires when an armed guard's zero has crossed at a step's end, whatever it read before: the gap is the zero with
/// the sign `Crossing` gives it, so a landing reads a plain guard as it reads a hazard.
pub struct Guard<'b, O, U, S = Body> {
    zero: PureKernel<'b, O, U, S>,
    crossing: Crossing,
    transition: ModeTransition<'b, O>,
    resets: Option<&'static str>,
}
impl<'b, O, U: Sensed, S> Guard<'b, O, U, S> {
    pub fn new(zero: PureKernel<'b, O, U, S>, crossing: Crossing, transition: ModeTransition<'b, O>) -> Self {
        Self {
            zero,
            crossing,
            transition,
            resets: None,
        }
    }
    /// The firing sets this memory to zero. `bind` refuses a reset row an algebraic law reads, so a firing leaves every
    /// algebraic value true and no landing needs a solve.
    pub fn resetting<V: NonMaterial>(self, slot: &StateSlot<'b, O, V>) -> Self {
        Self {
            resets: Some(slot.name),
            ..self
        }
    }
}

/// A named sequence of random draws of one owner, under a stable key: owner and name. Its draws are a pure
/// function of the run seed, that key and an event ordinal, so adding a stream changes no other.
pub struct Stream<'b, O> {
    name: &'static str,
    brand: Brand<'b>,
    owner: PhantomData<O>,
}
/// Smooth seeded variability of one owner: knots every declared period, under a stable key. A knot is no
/// breakpoint, so a forcing never cuts a step.
pub struct Forcing<'b, O> {
    name: &'static str,
    brand: Brand<'b>,
    owner: PhantomData<O>,
}
/// Fires when `integral` reaches its stream's exponential draw, `-ln u`, while its mode variable holds the
/// value the transition leaves. The integral's rate stays a law on its slot, attached with `memory_rate`, and
/// the contract integrates that rate's positive part, so an integral never falls and a published value below
/// the draw rules out a crossing at any earlier instant. Firing resets the integral, moves the stream to its
/// next ordinal and applies the transition, together, when the image publishes.
pub struct HazardGuard<'b, O> {
    integral: &'static str,
    stream: &'static str,
    transition: ModeTransition<'b, O>,
}
impl<'b, O> HazardGuard<'b, O> {
    pub fn new(
        integral: &StateSlot<'b, O, Dimensionless>,
        stream: &Stream<'b, O>,
        transition: ModeTransition<'b, O>,
    ) -> Self {
        Self {
            integral: integral.name,
            stream: stream.name,
            transition,
        }
    }
}

/// One owner's declarations while they are made. Declaring a slot records a law the fragment owes;
/// attaching a law borrows the slot, so compilation sees every law on a row together.
pub struct FragmentBuilder<'b, O> {
    data: FragmentData,
    kernels: Cell<u32>,
    brand: Brand<'b>,
    owner: PhantomData<O>,
}
impl<'b, O: Owner> FragmentBuilder<'b, O> {
    pub(crate) fn new() -> Self {
        Self {
            data: FragmentData::empty(O::CARD),
            kernels: Cell::new(0),
            brand: PhantomData,
            owner: PhantomData,
        }
    }
    fn close<U, S>(&self, finished: Finished<'b, '_, O, U, S>) -> PureKernel<'b, O, U, S> {
        let id = self.kernels.get();
        self.kernels.set(id + 1);
        PureKernel {
            data: KernelData {
                id,
                code: reachable(&finished.code, finished.result),
                dimension: finished.dimension,
                modes: finished.modes.selector,
                rate: finished.rate,
                defines: false,
            },
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    pub fn kernel<U: Dimension, F>(&self, build: F) -> PureKernel<'b, O, U>
    where
        F: for<'k> FnOnce(Kernel<'b, 'k, O>) -> Finished<'b, 'k, O, U>,
    {
        self.close(build(Kernel::open()))
    }
    pub fn site_kernel<C: SiteSet, U: Dimension, F>(&self, build: F) -> PureKernel<'b, O, U, C>
    where
        F: for<'k> FnOnce(Kernel<'b, 'k, O, C>) -> Finished<'b, 'k, O, U, C>,
    {
        self.close(build(Kernel::open()))
    }
    pub fn edge_kernel<E: EdgeKind, U: Dimension, F>(&self, build: F) -> PureKernel<'b, O, U, E>
    where
        F: for<'k> FnOnce(Kernel<'b, 'k, O, E>) -> Finished<'b, 'k, O, U, E>,
    {
        self.close(build(Kernel::open()))
    }
    fn declare_parameter<U: Dimension>(
        &mut self,
        key: ParameterKey,
        context: ParameterContext,
        set: AdmissibleSet,
    ) {
        self.data.parameters.push(ParameterDeclaration {
            key,
            context,
            dimension: dimension::<U>(),
            set,
        });
    }
    pub fn parameter<U: Dimension>(
        &mut self,
        key: ParameterKey,
        domain: Admissible<U>,
    ) -> Parameter<'b, O, U> {
        self.declare_parameter::<U>(key, ParameterContext::Body, domain.set());
        Parameter {
            key,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    pub fn site_parameter<C: SiteSet, U: Dimension>(
        &mut self,
        key: ParameterKey,
        domain: Admissible<U>,
    ) -> SiteParameter<'b, O, C, U> {
        self.declare_parameter::<U>(key, ParameterContext::Site(C::SET), domain.set());
        SiteParameter {
            key,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    pub fn edge_parameter<E: EdgeKind, U: Dimension>(
        &mut self,
        key: ParameterKey,
        domain: Admissible<U>,
    ) -> EdgeParameter<'b, O, U, E> {
        self.declare_parameter::<U>(key, ParameterContext::Edge(E::NETWORK), domain.set());
        EdgeParameter {
            key,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    pub fn end_parameter<U: Dimension>(
        &mut self,
        key: ParameterKey,
        domain: Admissible<U>,
    ) -> EndParameter<'b, O, U> {
        self.declare_parameter::<U>(key, ParameterContext::Edge(Network::Vascular), domain.set());
        EndParameter {
            key,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    fn declare_slot<U: Dimension>(&mut self, name: &'static str, kind: SlotKind, set: AdmissibleSet) {
        self.data.slots.push(SlotDeclaration {
            name,
            kind,
            dimension: dimension::<U>(),
            set,
            over: Over::Body,
        });
    }
    pub fn state<U: NonMaterial>(
        &mut self,
        name: &'static str,
        domain: Admissible<U>,
    ) -> StateSlot<'b, O, U> {
        self.declare_slot::<U>(name, SlotKind::Memory, domain.set());
        StateSlot {
            name,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    /// Slow memory in the unit of the variable it guards. History and disease move it; nothing
    /// writes the guarded variable.
    pub fn set_point<U: Sensed>(
        &mut self,
        name: &'static str,
        domain: Admissible<Reference<U>>,
    ) -> StateSlot<'b, O, Reference<U>> {
        self.state(name, domain)
    }
    pub fn algebraic<U: Dimension>(
        &mut self,
        name: &'static str,
        domain: Admissible<U>,
    ) -> AlgebraicSlot<'b, O, U> {
        self.declare_slot::<U>(name, SlotKind::Algebraic, domain.set());
        AlgebraicSlot {
            name,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    /// A row at every site of a set, its law attached with `define_at`, and the read of it. One law, one row per site.
    pub fn site_field<C: SiteSet, U: Dimension>(
        &mut self,
        name: &'static str,
        domain: Admissible<U>,
    ) -> (Output<'b, O, PerSite<C, U>>, ReadPort<'b, O, PerSite<C, U>>) {
        self.data.slots.push(SlotDeclaration {
            name,
            kind: SlotKind::Defined,
            dimension: dimension::<U>(),
            set: domain.set(),
            over: Over::Every(C::SET),
        });
        let owner = O::CARD;
        (
            Output::named(name),
            ReadPort::field(LawName { owner, local: name }),
        )
    }
    /// Declares one mode variable of this owner and returns its values in order.
    pub fn modes<const N: usize>(
        &mut self,
        name: &'static str,
        values: [&'static str; N],
    ) -> [Mode<'b, O>; N] {
        self.declare_modes(name, values, Over::Body)
    }
    /// One mode variable per edge of a route, as a valve is open or shut on its own edge. A kernel reads one only in
    /// that route's own context; compile refuses any other.
    pub fn modes_along<Q, E: EdgeKind, const N: usize>(
        &mut self,
        route: &EdgeRoute<'b, O, Q, E>,
        name: &'static str,
        values: [&'static str; N],
    ) -> [Mode<'b, O>; N] {
        self.declare_modes(name, values, Over::Route(route.family))
    }
    fn declare_modes<const N: usize>(
        &mut self,
        name: &'static str,
        values: [&'static str; N],
        over: Over,
    ) -> [Mode<'b, O>; N] {
        let variable = self.data.modes.len();
        self.data.modes.push(ModeDeclaration {
            name,
            values: values.to_vec(),
            over,
        });
        core::array::from_fn(|value| Mode {
            variable,
            value,
            name: ModeValue {
                variable: name,
                value: values[value],
            },
            brand: PhantomData,
            owner: PhantomData,
        })
    }
    /// Records a guard the fragment owes: a stream no guard draws on is refused at compile.
    pub fn stream(&mut self, name: &'static str) -> Stream<'b, O> {
        self.data.streams.push(name);
        Stream {
            name,
            brand: PhantomData,
            owner: PhantomData,
        }
    }
    /// Records a law the fragment owes: a forcing no law reads is refused at compile. Both numbers are packet
    /// values: the knots' period, positive, and the amplitude, in the unit interval so the factor stays
    /// positive. Declaring them here means neither can be given another set.
    pub fn forcing(
        &mut self,
        name: &'static str,
        period: ParameterKey,
        amplitude: ParameterKey,
    ) -> Forcing<'b, O> {
        self.declare_parameter::<Time>(
            period,
            ParameterContext::Body,
            Admissible::<Time>::positive().set(),
        );
        self.declare_parameter::<Dimensionless>(
            amplitude,
            ParameterContext::Body,
            Admissible::<Dimensionless>::unit_interval().set(),
        );
        self.data.forcings.push(ForcingDeclaration {
            name,
            period,
            amplitude,
        });
        Forcing {
            name,
            brand: PhantomData,
            owner: PhantomData,
        }
    }
    pub fn every_mode(&self) -> ModeSet<'b, O> {
        ModeSet {
            selector: Selector::Every,
            brand: PhantomData,
            owner: PhantomData,
        }
    }
    /// Declares where a kernel is trusted. Leaving the range is reported on the receipt, never refused.
    pub fn validity<U, S>(&mut self, kernel: &PureKernel<'b, O, U, S>, range: ValidityRange<U>) {
        let (lower, upper, basis) = range.parts();
        self.data
            .validity
            .push((kernel.data.id, Validity { lower, upper, basis }));
    }
    fn attach(&mut self, serves: Obligation, kernel: KernelData) {
        self.data.laws.push(LawDeclaration { serves, kernel });
    }
    /// The contract integrates the rate; no law steps its own memory.
    pub fn memory_rate<U: NonMaterial, S>(
        &mut self,
        slot: &StateSlot<'b, O, U>,
        rate: PureKernel<'b, O, Rate<U>, S>,
    ) {
        self.attach(Obligation::Slot(slot.name), rate.data);
    }
    pub fn residual<U: Dimension>(&mut self, slot: &AlgebraicSlot<'b, O, U>, residual: PureKernel<'b, O, U>) {
        self.attach(Obligation::Slot(slot.name), residual.data);
    }
    /// A body row that is one site kernel summed over a set: the row less the sum of `value` at every site.
    pub fn sum_at<C: SiteSet, U: Dimension>(
        &mut self,
        slot: &AlgebraicSlot<'b, O, U>,
        row: PureKernel<'b, O, U>,
        value: PureKernel<'b, O, U, C>,
    ) {
        self.attach(Obligation::Slot(slot.name), row.data);
        self.attach(Obligation::Sum(slot.name, C::SET), value.data.summing());
    }
    /// An unknown no row of its own defines: `residual` is zero by way of an amount `slot`'s rate moves, as a store is
    /// held to a sum by the flow out of it. Compile matches the row to `slot` by this declaration, and a stage's
    /// tangent is what lets the solve reach it, so a body with no cycle refuses it at the rank report.
    pub fn closes<U: Dimension, V: Dimension>(
        &mut self,
        slot: &AlgebraicSlot<'b, O, U>,
        residual: PureKernel<'b, O, V>,
    ) {
        self.attach(Obligation::Slot(slot.name), residual.data);
        self.data.closed.push(slot.name);
    }
    /// Issues the outputs of an arrow the map draws from this owner, and records a law owed on each.
    pub fn emit<P>(&mut self) -> P
    where
        P: Coupling<From = O, Side = Defines<'b, O>>,
    {
        let (payload, fields) = catalog::issued::<P>();
        for field in fields {
            self.data.slots.push(SlotDeclaration {
                name: field.name,
                kind: SlotKind::Defined,
                dimension: field.dimension,
                set: AdmissibleSet::Finite,
                over: field.over,
            });
        }
        payload
    }
    /// The only way to read a coupling field, so every such read is a line of the map.
    pub fn receive<P>(&self) -> P
    where
        P: Coupling<To = O, Side = Reads<'b, O>>,
    {
        catalog::issued::<P>().0
    }
    /// A validity range on `value` judges the row.
    pub fn define<U: Dimension>(&mut self, output: &Output<'b, O, U>, value: PureKernel<'b, O, U>) {
        self.attach(Obligation::Defined(output.name), value.data.defining(output.name));
    }
    /// One kernel for the set, one row per site.
    pub fn define_at<C: SiteSet, U: Dimension>(
        &mut self,
        output: &Output<'b, O, PerSite<C, U>>,
        value: PureKernel<'b, O, U, C>,
    ) {
        self.attach(Obligation::Defined(output.name), value.data.defining(output.name));
    }
    pub(crate) fn owe_enclosure_pressure(&mut self) {
        self.data.slots.push(SlotDeclaration {
            name: ENCLOSURE_PRESSURE,
            kind: SlotKind::Defined,
            dimension: dimension::<Pressure>(),
            set: AdmissibleSet::Finite,
            over: Over::HeldEnclosures,
        });
    }
    pub fn transfer_at<C: SiteSet, Q>(
        &mut self,
        route: &SiteRoute<'b, O, C, Q>,
        rate: PureKernel<'b, O, Rate<Q>, C>,
    ) {
        self.attach(Obligation::Family(route.family), rate.data);
    }
    pub fn transfer_along<Q, E: EdgeKind>(
        &mut self,
        route: &EdgeRoute<'b, O, Q, E>,
        rate: PureKernel<'b, O, Rate<Q>, E>,
    ) {
        self.attach(Obligation::Family(route.family), rate.data);
    }
    pub fn transfer<Q>(&mut self, transfer: Transfer<'b, O, Q>) {
        self.attach(transfer.serves, transfer.rate.data);
    }
    /// How much of species `R` this bed's wall holds back from the water the route moves: `1 - sigma` of it crosses.
    /// A species with no such line crosses freely. Returns the parameter, so a law can read the sigma it depends on.
    pub fn reflect<R: Substance>(
        &mut self,
        route: &SiteRoute<'b, O, CapillaryBed, Water>,
        key: ParameterKey,
    ) -> SiteParameter<'b, O, CapillaryBed, Dimensionless> {
        self.data.reflections.push((route.family, R::SPECIES, key));
        self.site_parameter(key, Admissible::unit_interval())
    }
    pub(crate) fn request(&mut self, family: Family) {
        if self.data.families.contains(&family) {
            return;
        }
        self.data.families.push(family);
    }
    /// A valve's flow memory is owed by requesting its route.
    pub(crate) fn owe_flow_memory(&mut self, family: Family) {
        self.data.slots.push(SlotDeclaration {
            name: family.name(),
            kind: SlotKind::Memory,
            dimension: dimension::<Rate<Water>>(),
            set: AdmissibleSet::Finite,
            over: Over::Route(family),
        });
    }
    pub(crate) fn owe_defined<U: Dimension>(&mut self, name: &'static str, over: Over) {
        self.data.slots.push(SlotDeclaration {
            name,
            kind: SlotKind::Defined,
            dimension: dimension::<U>(),
            set: AdmissibleSet::Finite,
            over,
        });
    }
    pub fn event<U, S>(&mut self, guard: Guard<'b, O, U, S>) {
        self.data.guards.push(GuardDeclaration {
            zero: guard.zero.data,
            crossing: guard.crossing,
            variable: guard.transition.variable,
            leaves: guard.transition.from,
            enters: guard.transition.to,
            resets: guard.resets,
        });
    }
    pub fn hazard(&mut self, guard: HazardGuard<'b, O>) {
        self.data.hazards.push(HazardDeclaration {
            integral: guard.integral,
            stream: guard.stream,
            variable: guard.transition.variable,
            leaves: guard.transition.from,
            enters: guard.transition.to,
        });
    }
    pub fn finish(self) -> Fragment<'b, O> {
        Fragment {
            data: self.data,
            brand: PhantomData,
            owner: PhantomData,
        }
    }
}

/// What a system crate declares, as data the contract compiles. No fragment runs during a solve.
pub struct Fragment<'b, O> {
    pub(crate) data: FragmentData,
    brand: Brand<'b>,
    owner: PhantomData<O>,
}
pub(crate) struct FragmentData {
    pub(crate) owner: &'static str,
    pub(crate) slots: Vec<SlotDeclaration>,
    pub(crate) modes: Vec<ModeDeclaration>,
    pub(crate) parameters: Vec<ParameterDeclaration>,
    pub(crate) families: Vec<Family>,
    pub(crate) laws: Vec<LawDeclaration>,
    pub(crate) guards: Vec<GuardDeclaration>,
    pub(crate) streams: Vec<&'static str>,
    pub(crate) forcings: Vec<ForcingDeclaration>,
    pub(crate) hazards: Vec<HazardDeclaration>,
    pub(crate) validity: Vec<(u32, Validity)>,
    pub(crate) closed: Vec<&'static str>,
    pub(crate) reflections: Vec<(Family, Species, ParameterKey)>,
}
impl FragmentData {
    pub(crate) fn owe_flow(&mut self, family: Family) {
        self.slots.push(SlotDeclaration {
            name: family.name(),
            kind: SlotKind::Defined,
            dimension: dimension::<Rate<Water>>(),
            set: AdmissibleSet::Finite,
            over: Over::Route(family),
        });
    }
    pub(crate) fn empty(owner: &'static str) -> Self {
        Self {
            owner,
            slots: Vec::new(),
            modes: Vec::new(),
            parameters: Vec::new(),
            families: Vec::new(),
            laws: Vec::new(),
            guards: Vec::new(),
            streams: Vec::new(),
            forcings: Vec::new(),
            hazards: Vec::new(),
            validity: Vec::new(),
            closed: Vec::new(),
            reflections: Vec::new(),
        }
    }
}
pub(crate) struct SlotDeclaration {
    pub(crate) name: &'static str,
    pub(crate) kind: SlotKind,
    pub(crate) dimension: String,
    pub(crate) set: AdmissibleSet,
    pub(crate) over: Over,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SlotKind {
    Memory,
    Algebraic,
    /// Only a definition serves it and only a port reads it, so a plain slot of its name does neither.
    Defined,
}
pub(crate) struct ModeDeclaration {
    pub(crate) name: &'static str,
    pub(crate) values: Vec<&'static str>,
    pub(crate) over: Over,
}
pub(crate) struct ParameterDeclaration {
    pub(crate) key: ParameterKey,
    pub(crate) context: ParameterContext,
    pub(crate) dimension: String,
    pub(crate) set: AdmissibleSet,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ParameterContext {
    Body,
    Site(Sites),
    Edge(Network),
}
pub(crate) struct LawDeclaration {
    pub(crate) serves: Obligation,
    pub(crate) kernel: KernelData,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Obligation {
    Slot(&'static str),
    Defined(&'static str),
    Family(Family),
    Boundary(&'static str),
    /// A body row's other laws: one kernel at every site of a set.
    Sum(&'static str, Sites),
}
impl Obligation {
    pub(crate) fn local(self) -> &'static str {
        match self {
            Obligation::Slot(name)
            | Obligation::Defined(name)
            | Obligation::Boundary(name)
            | Obligation::Sum(name, _) => name,
            Obligation::Family(family) => family.name(),
        }
    }
}
pub(crate) struct GuardDeclaration {
    pub(crate) zero: KernelData,
    pub(crate) crossing: Crossing,
    pub(crate) variable: usize,
    pub(crate) leaves: usize,
    pub(crate) enters: usize,
    pub(crate) resets: Option<&'static str>,
}
pub(crate) struct ForcingDeclaration {
    pub(crate) name: &'static str,
    pub(crate) period: ParameterKey,
    pub(crate) amplitude: ParameterKey,
}
pub(crate) struct HazardDeclaration {
    pub(crate) integral: &'static str,
    pub(crate) stream: &'static str,
    pub(crate) variable: usize,
    pub(crate) leaves: usize,
    pub(crate) enters: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Validity {
    pub(crate) lower: u64,
    pub(crate) upper: u64,
    pub(crate) basis: Basis,
}

impl Seal for LawName {
    fn seal(&self, out: &mut Canonical) {
        let Self { owner, local } = self;
        out.text(owner);
        out.text(local);
    }
}
impl Seal for ModeValue {
    fn seal(&self, out: &mut Canonical) {
        let Self { variable, value } = self;
        out.text(variable);
        out.text(value);
    }
}
impl Seal for Predicate {
    fn seal(&self, out: &mut Canonical) {
        match self {
            Predicate::EveryMode => out.number(0),
            Predicate::Only(mode) => {
                out.number(1);
                mode.seal(out);
            }
        }
    }
}
impl Seal for RateClass {
    fn seal(&self, out: &mut Canonical) {
        let Self { band, maximum_ns } = self;
        out.number(*band as u64);
        out.number(*maximum_ns);
    }
}
impl Seal for Crossing {
    fn seal(&self, out: &mut Canonical) {
        out.number(*self as u64);
    }
}
impl Seal for Validity {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            lower,
            upper,
            basis: Basis::Fixture(name),
        } = self;
        out.number(*lower);
        out.number(*upper);
        out.text(name);
    }
}
