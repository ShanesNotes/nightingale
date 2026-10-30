use crate::{AnatomyDefect, LawName, ModeValue, ParameterKey, Predicate};

/// A refusal at a construction call, at `AnatomyBuilder::finish`, or at `Assembly::compile`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    NonFinite,
    Anatomy(AnatomyDefect),
    Compile(CompileDefect),
    /// A second scope for an owner this assembly already issued one to.
    ScopeIssuedTwice {
        owner: &'static str,
    },
    /// `Mode::to` with the same value, or with a value of another mode variable.
    InvalidTransition {
        from: ModeValue,
        to: ModeValue,
    },
    ZeroRateInterval,
    /// A validity range whose lower bound is above its upper bound.
    ReversedValidityRange,
}
impl From<AnatomyDefect> for AdmissionError {
    fn from(defect: AnatomyDefect) -> Self {
        Self::Anatomy(defect)
    }
}
impl From<CompileDefect> for AdmissionError {
    fn from(defect: CompileDefect) -> Self {
        Self::Compile(defect)
    }
}

/// Each names the first offender in canonical order: owners by card, laws by name, pools by site key
/// and then space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompileDefect {
    /// A declared slot, a requested route family or a declared boundary that no law defines. An
    /// omitted law is not zero.
    OmittedLaw { law: LawName },
    /// Two laws on one row that hold in one mode.
    SecondOwner {
        law: LawName,
        first: Predicate,
        second: Predicate,
    },
    /// A value of the row's mode variable that no law on the row holds in.
    UncoveredMode { law: LawName, mode: ModeValue },
    /// A guard whose zero kernel does not hold in the value its transition leaves.
    GuardOutsideMode { variable: LawName, leaves: &'static str },
    /// A scope this assembly issued whose fragment was never added.
    MissingFragment { owner: &'static str },
    /// Two slots, mode variables or route families under one name in one owner.
    NamedTwice { law: LawName },
    /// One parameter key declared with two dimensions, contexts or admissible sets.
    ParameterConflict { key: ParameterKey },
    /// A mode variable with no values, or with one value twice.
    BadModeVariable { variable: LawName },
    /// An algebraic row that no structural matching pairs with an algebraic unknown it reads.
    StructuralSingularity { law: LawName },
}
