use crate::catalog::{self, Coupling, Defines, Reads};
use crate::identity::{Canonical, Seal};
use crate::quantity::dimension;
use crate::scope::ENCLOSURE_PRESSURE;
use crate::{
    Admissible, AdmissibleSet, AdmissionError, Basis, Boundary, Class, Conductance, Dimension, Dimensionless,
    NonMaterial, Owner, ParameterKey, PerEdge, PerSite, Pressure, Product, Quotient, Rate, Reference,
    SameDimension, Sensed, SiteClass, Time, ValidityRange, Volume, Water,
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
/// The context of a kernel declared once for every vascular edge.
pub enum VascularEdge {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Family {
    Circulation,
    Starling,
}
impl Family {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Family::Circulation => "circulation",
            Family::Starling => "starling",
        }
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
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Parameter(ParameterKey),
    SiteParameter(ParameterKey),
    EdgeParameter(ParameterKey),
    EndParameter(ParameterKey, End),
    Memory(&'static str),
    Algebraic(&'static str),
    Amount(Family, End),
    Drive(&'static str),
    Draw(&'static str),
    Forcing(&'static str),
    Field(LawName),
    Extravascular,
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
            Input::Amount(family, end) => {
                out.number(6);
                out.text(family.name());
                out.number(end as u64);
            }
            Input::Drive(boundary) => {
                out.number(7);
                out.text(boundary);
            }
            Input::Draw(stream) => {
                out.number(8);
                out.text(stream);
            }
            Input::Forcing(forcing) => {
                out.number(9);
                out.text(forcing);
            }
            Input::Field(name) => {
                out.number(10);
                name.seal(out);
            }
            Input::Extravascular => out.number(11),
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
    /// `row - value`, so a coupling is a plain algebraic row and a loop of couplings a coupled residual.
    fn defining(mut self, row: &'static str) -> Self {
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
        self.input(port.source.input())
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
    /// `measured - reference`: the one way a set point meets the variable it guards.
    pub fn deviation<U: Sensed>(
        &mut self,
        measured: Expr<'b, 'k, O, U>,
        reference: Expr<'b, 'k, O, Reference<U>>,
    ) -> Expr<'b, 'k, O, U> {
        self.push(Op::Deviation([measured.at, reference.at]))
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

impl<'b, 'k, O, C: SiteClass> Kernel<'b, 'k, O, C> {
    pub fn at_site<U: Dimension>(&mut self, port: &ReadPort<'b, O, PerSite<C, U>>) -> Expr<'b, 'k, O, U> {
        self.input(port.source.input())
    }
    /// The value bound at this site, or else at its class.
    pub fn site_parameter<U: Dimension>(
        &mut self,
        parameter: &SiteParameter<'b, O, C, U>,
    ) -> Expr<'b, 'k, O, U> {
        self.input(Input::SiteParameter(parameter.key))
    }
}
impl<'b, 'k, O> Kernel<'b, 'k, O, VascularEdge> {
    pub fn at_edge<U: Dimension>(&mut self, port: &ReadPort<'b, O, PerEdge<U>>) -> Expr<'b, 'k, O, U> {
        self.input(port.source.input())
    }
    /// The value bound at this edge, or else at the vascular network.
    pub fn edge_parameter<U: Dimension>(
        &mut self,
        parameter: &EdgeParameter<'b, O, U>,
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
enum Source {
    Amount(Family, End),
    Drive(&'static str),
    Field(LawName),
    Extravascular,
}
impl Source {
    fn input(self) -> Input {
        match self {
            Source::Amount(family, end) => Input::Amount(family, end),
            Source::Drive(boundary) => Input::Drive(boundary),
            Source::Field(name) => Input::Field(name),
            Source::Extravascular => Input::Extravascular,
        }
    }
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
}
impl<'b, O> ReadPort<'b, O, PerSite<crate::CapillaryBed, Pressure>> {
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
    Every(Class),
    HeldEnclosures,
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
impl<C: SiteClass, U: Dimension> Carried for PerSite<C, U> {
    fn over() -> Over {
        Over::Every(C::CLASS)
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
impl<'b, O, C: SiteClass, Q> SiteRoute<'b, O, C, Q> {
    pub(crate) fn requested(family: Family) -> Self {
        Self {
            family,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    pub fn from_amount(&self) -> ReadPort<'b, O, PerSite<C, Q>> {
        ReadPort::of(Source::Amount(self.family, End::From))
    }
    pub fn to_amount(&self) -> ReadPort<'b, O, PerSite<C, Q>> {
        ReadPort::of(Source::Amount(self.family, End::To))
    }
}
/// A route family along every vascular edge: one column per edge.
pub struct EdgeRoute<'b, O, Q> {
    family: Family,
    brand: Brand<'b>,
    kind: PhantomData<(O, Q)>,
}
impl<'b, O, Q> EdgeRoute<'b, O, Q> {
    pub(crate) fn requested(family: Family) -> Self {
        Self {
            family,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    pub fn from_amount(&self) -> ReadPort<'b, O, PerEdge<Q>> {
        ReadPort::of(Source::Amount(self.family, End::From))
    }
    pub fn to_amount(&self) -> ReadPort<'b, O, PerEdge<Q>> {
        ReadPort::of(Source::Amount(self.family, End::To))
    }
}

pub struct Endpoint<'b, O, Q> {
    brand: Brand<'b>,
    kind: PhantomData<(O, Q)>,
}
/// Both ends of one transfer, the same species at each, one of them the receipt pool outside the
/// body. The schema names where it reaches into the body.
pub struct Pair<'b, O, Q> {
    boundary: &'static str,
    brand: Brand<'b>,
    kind: PhantomData<(O, Q)>,
}
/// A transfer law on a pair: what leaves one end enters the other.
pub struct Transfer<'b, O, Q> {
    boundary: &'static str,
    rate: PureKernel<'b, O, Rate<Q>>,
}
impl<'b, O, Q> Transfer<'b, O, Q> {
    pub fn new(pair: &Pair<'b, O, Q>, rate: PureKernel<'b, O, Rate<Q>>) -> Self {
        Self {
            boundary: pair.boundary,
            rate,
        }
    }
}
/// A schema boundary's pair and the drive the boundary port supplies at each stage.
pub struct BoundaryWater<'b, O = Boundary> {
    pub route: Pair<'b, O, Water>,
    pub drive: ReadPort<'b, O, Rate<Water>>,
}
impl<'b, O> BoundaryWater<'b, O> {
    pub(crate) fn named(boundary: &'static str) -> Self {
        Self {
            route: Pair {
                boundary,
                brand: PhantomData,
                kind: PhantomData,
            },
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
/// A parameter of every vascular edge, bound per edge with the network as default.
pub struct EdgeParameter<'b, O, U> {
    key: ParameterKey,
    brand: Brand<'b>,
    kind: PhantomData<(O, U)>,
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
/// own dimension, and compilation requires it to hold in the value the transition leaves.
pub struct Guard<'b, O, U> {
    zero: PureKernel<'b, O, U>,
    crossing: Crossing,
    transition: ModeTransition<'b, O>,
}
impl<'b, O, U: Sensed> Guard<'b, O, U> {
    pub fn new(zero: PureKernel<'b, O, U>, crossing: Crossing, transition: ModeTransition<'b, O>) -> Self {
        Self {
            zero,
            crossing,
            transition,
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
    pub fn site_kernel<C: SiteClass, U: Dimension, F>(&self, build: F) -> PureKernel<'b, O, U, C>
    where
        F: for<'k> FnOnce(Kernel<'b, 'k, O, C>) -> Finished<'b, 'k, O, U, C>,
    {
        self.close(build(Kernel::open()))
    }
    pub fn edge_kernel<U: Dimension, F>(&self, build: F) -> PureKernel<'b, O, U, VascularEdge>
    where
        F: for<'k> FnOnce(Kernel<'b, 'k, O, VascularEdge>) -> Finished<'b, 'k, O, U, VascularEdge>,
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
    pub fn site_parameter<C: SiteClass, U: Dimension>(
        &mut self,
        key: ParameterKey,
        domain: Admissible<U>,
    ) -> SiteParameter<'b, O, C, U> {
        self.declare_parameter::<U>(key, ParameterContext::Site(C::CLASS), domain.set());
        SiteParameter {
            key,
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    pub fn edge_parameter<U: Dimension>(
        &mut self,
        key: ParameterKey,
        domain: Admissible<U>,
    ) -> EdgeParameter<'b, O, U> {
        self.declare_parameter::<U>(key, ParameterContext::Edge, domain.set());
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
        self.declare_parameter::<U>(key, ParameterContext::End, domain.set());
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
    /// Declares one mode variable of this owner and returns its values in order.
    pub fn modes<const N: usize>(
        &mut self,
        name: &'static str,
        values: [&'static str; N],
    ) -> [Mode<'b, O>; N] {
        let variable = self.data.modes.len();
        self.data.modes.push(ModeDeclaration {
            name,
            values: values.to_vec(),
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
    pub fn memory_rate<U: NonMaterial>(
        &mut self,
        slot: &StateSlot<'b, O, U>,
        rate: PureKernel<'b, O, Rate<U>>,
    ) {
        self.attach(Obligation::Slot(slot.name), rate.data);
    }
    pub fn residual<U: Dimension>(&mut self, slot: &AlgebraicSlot<'b, O, U>, residual: PureKernel<'b, O, U>) {
        self.attach(Obligation::Slot(slot.name), residual.data);
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
    /// One kernel for the class, one row per site.
    pub fn define_at<C: SiteClass, U: Dimension>(
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
    pub fn transfer_at<C: SiteClass, Q>(
        &mut self,
        route: &SiteRoute<'b, O, C, Q>,
        rate: PureKernel<'b, O, Rate<Q>, C>,
    ) {
        self.attach(Obligation::Family(route.family), rate.data);
    }
    pub fn transfer_along<Q>(
        &mut self,
        route: &EdgeRoute<'b, O, Q>,
        rate: PureKernel<'b, O, Rate<Q>, VascularEdge>,
    ) {
        self.attach(Obligation::Family(route.family), rate.data);
    }
    pub fn transfer<Q>(&mut self, transfer: Transfer<'b, O, Q>) {
        self.attach(Obligation::Boundary(transfer.boundary), transfer.rate.data);
    }
    pub(crate) fn request(&mut self, family: Family) {
        if !self.data.families.contains(&family) {
            self.data.families.push(family);
        }
    }
    pub fn event<U>(&mut self, guard: Guard<'b, O, U>) {
        self.data.guards.push(GuardDeclaration {
            zero: guard.zero.data,
            crossing: guard.crossing,
            variable: guard.transition.variable,
            leaves: guard.transition.from,
            enters: guard.transition.to,
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
}
impl FragmentData {
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
    Site(Class),
    Edge,
    End,
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
}
impl Obligation {
    pub(crate) fn local(self) -> &'static str {
        match self {
            Obligation::Slot(name) | Obligation::Defined(name) | Obligation::Boundary(name) => name,
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
