use crate::body::Layout;
use crate::evaluate::{OperatorDomain, Undefined};
use crate::law::Op;
use crate::{AdmissibleSet, Compensated, CompiledBody, Context, LawName, RowId, SimTime};

/// A position in `CompiledBody::refusal_sources`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceId {
    index: usize,
}
impl SourceId {
    pub fn index(self) -> usize {
        self.index
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefusalSource {
    /// A row's admissible set: an amount is nonnegative, and a slot takes its declared set.
    Row { row: RowId },
    /// A `divide` whose divisor must stay positive, so a step cannot jump a pole unseen.
    Divisor { law: LawName },
    /// A `ln` whose argument must stay positive.
    LogarithmArgument { law: LawName },
    /// An evaluation at a finite point that came back non-finite, such as an `exp` overflow.
    NonFinite { law: LawName },
}

pub(crate) fn refusal_sources(layout: &Layout) -> Vec<RefusalSource> {
    let mut sources: Vec<RefusalSource> = (layout.rows.iter())
        .map(|row| RefusalSource::Row { row: row.id() })
        .collect();
    let mut laws: Vec<LawName> = layout.programs.iter().map(|program| program.law).collect();
    laws.dedup();
    for law in laws {
        let reads = |wanted: fn(&Op) -> bool| {
            (layout.programs.iter())
                .filter(|program| program.law == law)
                .any(|program| program.code.ops.iter().any(wanted))
        };
        if reads(|op| matches!(op, Op::Divide(_))) {
            sources.push(RefusalSource::Divisor { law });
        }
        if reads(|op| matches!(op, Op::Ln(_))) {
            sources.push(RefusalSource::LogarithmArgument { law });
        }
        sources.push(RefusalSource::NonFinite { law });
    }
    sources
}

impl CompiledBody {
    /// In row order, then in law name order. Derived from the rows and programs, so not hashed apart.
    pub fn refusal_sources(&self) -> &[RefusalSource] {
        &self.sources
    }
    pub fn row_source(&self, row: RowId) -> SourceId {
        self.source_of(RefusalSource::Row { row })
    }
    pub(crate) fn undefined_source(&self, undefined: Undefined) -> SourceId {
        let Undefined { law, domain } = undefined;
        self.source_of(match domain {
            OperatorDomain::Divisor => RefusalSource::Divisor { law },
            OperatorDomain::LogarithmArgument => RefusalSource::LogarithmArgument { law },
            OperatorDomain::NonFinite => RefusalSource::NonFinite { law },
        })
    }
    fn source_of(&self, source: RefusalSource) -> SourceId {
        let index = self.sources.iter().position(|known| *known == source);
        SourceId {
            index: index.expect("each source a body can refuse on is derived from that body"),
        }
    }
}

impl AdmissibleSet {
    /// Whether a compensated value lies in the set, read on the exact pair and not its rounding.
    pub fn holds(self, value: Compensated) -> bool {
        let one = Compensated::exact(1.0);
        value.is_finite()
            && match self {
                AdmissibleSet::Finite => true,
                AdmissibleSet::Nonnegative => Compensated::ZERO.at_most(value),
                AdmissibleSet::Positive => !value.at_most(Compensated::ZERO),
                AdmissibleSet::UnitInterval => Compensated::ZERO.at_most(value) && value.at_most(one),
            }
    }
}

/// An accepted value outside a law's `ValidityRange`: reported on the step, never refused.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValidityExcursion {
    pub(crate) law: LawName,
    pub(crate) context: Context,
    pub(crate) at: SimTime,
    pub(crate) side: ExcursionSide,
    pub(crate) value: Compensated,
}
impl ValidityExcursion {
    pub fn law(&self) -> LawName {
        self.law
    }
    pub fn context(&self) -> Context {
        self.context
    }
    pub fn at(&self) -> SimTime {
        self.at
    }
    pub fn side(&self) -> ExcursionSide {
        self.side
    }
    /// The kernel's value at the published state, in its canonical unit.
    pub fn value(&self) -> Compensated {
        self.value
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExcursionSide {
    Below,
    Above,
}
