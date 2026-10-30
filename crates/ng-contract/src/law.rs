use crate::identity::{Canonical, Seal};
use crate::quantity::dimension;
use crate::{
    Admissible, AdmissibleSet, AdmissionError, Basis, Conductance, Dimension, Dimensionless, NonMaterial,
    Owner, ParameterKey, Pressure, Product, Quotient, Rate, Reference, SameDimension, Sensed, ValidityRange,
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

/// One instruction of instruction set v1. Operands index earlier instructions of the same graph.
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
}
/// What an input instruction reads, by its declared name, so a graph's meaning never depends on
/// declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Input {
    Parameter(ParameterKey),
    Memory(&'static str),
    Algebraic(&'static str),
}
impl Op {
    fn map(&self, f: impl FnMut(usize) -> usize) -> Op {
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
            Input::Memory(name) => {
                out.number(1);
                out.text(name);
            }
            Input::Algebraic(name) => {
                out.number(2);
                out.text(name);
            }
        }
    }
}

/// The graph reachable from `result`, operands before their users and `result` last. Operand order
/// is kept, so `a + b` and `b + a` stay distinct and floating arithmetic is never reassociated.
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
/// An expression of exactly one kernel graph. It carries no number, so no host math can run on it.
pub struct Expr<'b, 'k, O, U> {
    at: usize,
    brand: Brand<'b>,
    kernel: Brand<'k>,
    kind: PhantomData<(O, U)>,
}
/// A finished graph, still inside its kernel's scope.
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
    /// Its value binds later, by key.
    pub fn parameter<U: Dimension>(&mut self, parameter: &Parameter<'b, O, U>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Parameter(parameter.key))
    }
    pub fn state<U: NonMaterial>(&mut self, slot: &StateSlot<'b, O, U>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Memory(slot.name))
    }
    pub fn algebraic<U: Dimension>(&mut self, slot: &AlgebraicSlot<'b, O, U>) -> Expr<'b, 'k, O, U> {
        self.input(Input::Algebraic(slot.name))
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
    /// Names a product or a quotient through the pinned identity table; there is no free cast.
    pub fn as_dimension<T: SameDimension<U>, U>(&mut self, x: Expr<'b, 'k, O, T>) -> Expr<'b, 'k, O, U> {
        self.push(Op::AsDimension([x.at]))
    }
    /// Scales by a dimensionless factor, such as a gain.
    pub fn scale<U>(
        &mut self,
        x: Expr<'b, 'k, O, U>,
        by: Expr<'b, 'k, O, Dimensionless>,
    ) -> Expr<'b, 'k, O, U> {
        self.push(Op::Scale([x.at, by.at]))
    }
    /// `0.5 (a + b + sqrt((a - b)^2 + w^2))`, a limit with a continuous Jacobian. The width `w` is a
    /// declared parameter, never a solver tolerance.
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
    /// Closes the graph. The result takes no admissible set: a refusal set belongs to a slot, never
    /// to a computed expression.
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
    /// The zero kernel's dimension is sensed, so a guard never fires on a set point itself.
    pub fn new(zero: PureKernel<'b, O, U>, crossing: Crossing, transition: ModeTransition<'b, O>) -> Self {
        Self {
            zero,
            crossing,
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
            data: FragmentData {
                owner: O::CARD,
                slots: Vec::new(),
                modes: Vec::new(),
                parameters: Vec::new(),
                laws: Vec::new(),
                guards: Vec::new(),
                validity: Vec::new(),
            },
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
            },
            brand: PhantomData,
            kind: PhantomData,
        }
    }
    /// Builds one whole-body kernel. `'k` is fresh, so an expression from another kernel, open or
    /// finished, does not type-check here.
    pub fn kernel<U: Dimension, F>(&self, build: F) -> PureKernel<'b, O, U>
    where
        F: for<'k> FnOnce(Kernel<'b, 'k, O>) -> Finished<'b, 'k, O, U>,
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
    fn declare_slot<U: Dimension>(&mut self, name: &'static str, kind: SlotKind, set: AdmissibleSet) {
        self.data.slots.push(SlotDeclaration {
            name,
            kind,
            dimension: dimension::<U>(),
            set,
        });
    }
    /// A memory slot and the closed set it stays in.
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
    pub fn event<U>(&mut self, guard: Guard<'b, O, U>) {
        self.data.guards.push(GuardDeclaration {
            zero: guard.zero.data,
            crossing: guard.crossing,
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
    pub(crate) laws: Vec<LawDeclaration>,
    pub(crate) guards: Vec<GuardDeclaration>,
    pub(crate) validity: Vec<(u32, Validity)>,
}
pub(crate) struct SlotDeclaration {
    pub(crate) name: &'static str,
    pub(crate) kind: SlotKind,
    pub(crate) dimension: String,
    pub(crate) set: AdmissibleSet,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum SlotKind {
    Memory,
    Algebraic,
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
}
pub(crate) struct LawDeclaration {
    pub(crate) serves: Obligation,
    pub(crate) kernel: KernelData,
}
/// What a law defines: a slot by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Obligation {
    Slot(&'static str),
}
pub(crate) struct GuardDeclaration {
    pub(crate) zero: KernelData,
    pub(crate) crossing: Crossing,
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
        out.text(self.owner);
        out.text(self.local);
    }
}
impl Seal for ModeValue {
    fn seal(&self, out: &mut Canonical) {
        out.text(self.variable);
        out.text(self.value);
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
        out.number(self.band as u64);
        out.number(self.maximum_ns);
    }
}
impl Seal for Crossing {
    fn seal(&self, out: &mut Canonical) {
        out.number(*self as u64);
    }
}
impl Seal for Validity {
    fn seal(&self, out: &mut Canonical) {
        out.number(self.lower);
        out.number(self.upper);
        let Basis::Fixture(name) = self.basis;
        out.text(name);
    }
}
