use ng_contract::{
    AcceptedStateView, AdmissibleSet, ColumnId, Compensated, CompiledBody, ConservedBasis, Digest, Interval,
    QuantityId, SimTime, VerifiedStep,
};

pub(crate) fn push_bits(bytes: &mut Vec<u8>, value: Compensated) {
    for limb in value.bits() {
        bytes.extend_from_slice(&limb.to_be_bytes());
    }
}

pub(crate) struct LedgerChunk {
    interval: Interval,
    quantities: Vec<QuantityStep>,
    columns: Vec<ColumnStep>,
}
struct QuantityStep {
    inventory: [Compensated; 2],
    inflow: [Compensated; 2],
    outflow: [Compensated; 2],
}
struct ColumnStep {
    forward: [Compensated; 2],
    reverse: [Compensated; 2],
}
impl LedgerChunk {
    pub(crate) fn after(history: &History, body: &CompiledBody, step: &VerifiedStep) -> Self {
        let previous = history.chunks.last();
        let columns: Vec<ColumnStep> = (step.gross().enumerate())
            .map(|(column, (forward, reverse))| {
                let (from_forward, from_reverse) = previous
                    .map_or((Compensated::ZERO, Compensated::ZERO), |chunk| {
                        (chunk.columns[column].forward[1], chunk.columns[column].reverse[1])
                    });
                ColumnStep {
                    forward: [from_forward, from_forward.plus(forward)],
                    reverse: [from_reverse, from_reverse.plus(reverse)],
                }
            })
            .collect();
        let basis = body.conserved_basis();
        let quantities = (basis.combinations().iter().enumerate())
            .map(|(at, combination)| {
                let inventory = (combination.terms().iter())
                    .fold(Compensated::ZERO, |sum, &(row, weight)| {
                        sum.plus(step.value(row).scaled(weight))
                    });
                let (start, inflow, outflow) = previous.map_or(
                    (history.opening[at], Compensated::ZERO, Compensated::ZERO),
                    |chunk| {
                        let last = &chunk.quantities[at];
                        (last.inventory[1], last.inflow[1], last.outflow[1])
                    },
                );
                let (mut into, mut out_of) = (inflow, outflow);
                for ((forward, reverse), &net) in step.gross().zip(&history.incidence[at]) {
                    if net > 0 {
                        into = into.plus(forward.scaled(net));
                        out_of = out_of.plus(reverse.scaled(net));
                    } else if net < 0 {
                        out_of = out_of.plus(forward.scaled(-net));
                        into = into.plus(reverse.scaled(-net));
                    }
                }
                QuantityStep {
                    inventory: [start, inventory],
                    inflow: [inflow, into],
                    outflow: [outflow, out_of],
                }
            })
            .collect();
        Self {
            interval: step.ticket().interval(),
            quantities,
            columns,
        }
    }
    /// Whether every value fits the ledger. A sum of finite amounts can leave the finite range.
    pub(crate) fn is_finite(&self) -> bool {
        let Self {
            interval: _,
            quantities,
            columns,
        } = self;
        let totals = quantities.iter().flat_map(|quantity| {
            let QuantityStep {
                inventory,
                inflow,
                outflow,
            } = quantity;
            inventory.iter().chain(inflow).chain(outflow)
        });
        let exchanges = columns.iter().flat_map(|column| {
            let ColumnStep { forward, reverse } = column;
            forward.iter().chain(reverse)
        });
        totals
            .chain(exchanges)
            .all(|&value| AdmissibleSet::Finite.holds(value))
    }
    pub(crate) fn digest(&self) -> Digest {
        let Self {
            interval,
            quantities,
            columns,
        } = self;
        let mut bytes = Vec::new();
        for instant in [interval.start(), interval.end()] {
            bytes.extend_from_slice(&instant.nanoseconds().to_be_bytes());
        }
        for quantity in quantities {
            let QuantityStep {
                inventory,
                inflow,
                outflow,
            } = quantity;
            for value in inventory.iter().chain(inflow).chain(outflow) {
                push_bits(&mut bytes, *value);
            }
        }
        for column in columns {
            let ColumnStep { forward, reverse } = column;
            for value in forward.iter().chain(reverse) {
                push_bits(&mut bytes, *value);
            }
        }
        Digest::of_canonical(&bytes)
    }
}

pub(crate) struct History {
    opening: Vec<Compensated>,
    incidence: Vec<Vec<i64>>,
    chunks: Vec<LedgerChunk>,
}
impl History {
    pub(crate) fn opening(body: &CompiledBody, accepted: AcceptedStateView<'_>) -> Self {
        let combinations = body.conserved_basis().combinations();
        let opening = (combinations.iter())
            .map(|combination| {
                (combination.terms().iter()).fold(Compensated::ZERO, |sum, &(row, weight)| {
                    sum.plus(accepted.value(row).scaled(weight))
                })
            })
            .collect();
        let incidence = (combinations.iter())
            .map(|combination| {
                let weight = |wanted| {
                    (combination.terms().iter())
                        .find(|term| term.0 == wanted)
                        .map_or(0, |term| term.1)
                };
                (body.columns().iter())
                    .map(|column| {
                        (column.terms().iter())
                            .map(|&(row, coefficient)| coefficient * weight(row))
                            .sum()
                    })
                    .collect()
            })
            .collect();
        Self {
            opening,
            incidence,
            chunks: Vec::new(),
        }
    }
    pub(crate) fn push(&mut self, chunk: LedgerChunk) {
        self.chunks.push(chunk);
    }
    pub(crate) fn digest(&self) -> Digest {
        let Self {
            opening,
            incidence,
            chunks,
        } = self;
        let mut bytes = Vec::new();
        for value in opening {
            push_bits(&mut bytes, *value);
        }
        for coefficient in incidence.iter().flatten() {
            bytes.extend_from_slice(&coefficient.to_be_bytes());
        }
        for chunk in chunks {
            bytes.extend_from_slice(chunk.digest().as_bytes());
        }
        Digest::of_canonical(&bytes)
    }
}

/// Run identity, branch and the accepted base this view was lent at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HistorySeal {
    pub run: Digest,
    pub branch: Digest,
    pub base: Digest,
}

/// One immutable view of one record's history over one conserved basis.
pub struct ConservationView<'r> {
    pub(crate) history: &'r History,
    pub(crate) basis: &'r ConservedBasis,
    pub(crate) seal: HistorySeal,
}
impl<'r> ConservationView<'r> {
    pub fn seal(&self) -> HistorySeal {
        self.seal
    }
    pub fn basis(&self) -> &'r ConservedBasis {
        self.basis
    }
    /// Admission to the accepted head; `None` before the first accepted step.
    pub fn span(&self) -> Option<Interval> {
        let chunks = &self.history.chunks;
        Interval::new(chunks.first()?.interval.start(), chunks.last()?.interval.end())
    }
    /// Every accepted segment in time order, each ending where the next starts.
    pub fn segments(&self) -> Segments<'r> {
        Segments {
            chunks: self.history.chunks.iter(),
        }
    }
}
pub struct Segments<'r> {
    chunks: core::slice::Iter<'r, LedgerChunk>,
}
impl<'r> Iterator for Segments<'r> {
    type Item = HistorySegment<'r>;
    fn next(&mut self) -> Option<HistorySegment<'r>> {
        self.chunks.next().map(|chunk| HistorySegment { chunk })
    }
}
impl<'r> DoubleEndedIterator for Segments<'r> {
    fn next_back(&mut self) -> Option<HistorySegment<'r>> {
        self.chunks.next_back().map(|chunk| HistorySegment { chunk })
    }
}
#[derive(Clone, Copy)]
pub struct HistorySegment<'r> {
    chunk: &'r LedgerChunk,
}
impl<'r> HistorySegment<'r> {
    pub fn interval(&self) -> Interval {
        self.chunk.interval
    }
    pub fn inventory(&self, quantity: QuantityId) -> Curve<'r> {
        self.curve(&self.chunk.quantities[quantity.index()].inventory)
    }
    /// Cumulative gross inflow since admission.
    pub fn inflow(&self, quantity: QuantityId) -> Curve<'r> {
        self.curve(&self.chunk.quantities[quantity.index()].inflow)
    }
    /// Cumulative gross outflow since admission.
    pub fn outflow(&self, quantity: QuantityId) -> Curve<'r> {
        self.curve(&self.chunk.quantities[quantity.index()].outflow)
    }
    /// One column's cumulative gross forward and reverse extents since admission.
    pub fn exchange(&self, column: ColumnId) -> (Curve<'r>, Curve<'r>) {
        let step = &self.chunk.columns[column.index()];
        (self.curve(&step.forward), self.curve(&step.reverse))
    }
    fn curve(&self, coefficients: &'r [Compensated; 2]) -> Curve<'r> {
        Curve {
            interval: self.chunk.interval,
            coefficients,
        }
    }
}
/// One quantity's curve over one segment, compensated, in Bernstein form.
pub struct Curve<'r> {
    interval: Interval,
    coefficients: &'r [Compensated; 2],
}
impl Curve<'_> {
    /// The Bernstein coefficients of this curve restricted to `piece`; `None` outside the segment. Same
    /// degree, so two curves on one piece subtract coefficient by coefficient, and the hull of the result
    /// bounds the difference.
    pub fn on(&self, piece: Interval) -> Option<Vec<Compensated>> {
        let inside = self.interval.start() <= piece.start() && piece.end() <= self.interval.end();
        inside.then(|| vec![self.at(piece.start()), self.at(piece.end())])
    }
    fn at(&self, instant: SimTime) -> Compensated {
        let reached = instant.nanoseconds() - self.interval.start().nanoseconds();
        let [start, end] = *self.coefficients;
        Compensated::between(start, end, reached, self.interval.length_ns())
    }
}
