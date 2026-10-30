use crate::checkpoint::CheckpointError;
use core::cell::Cell;
use ng_contract::{
    AcceptedStateView, AdmissibleSet, ByteReader, ColumnId, Compensated, CompiledBody, ConservedBasis,
    DenseBasis, Digest, Interval, Malformed, QuantityId, SimTime, VerifiedStep,
};

pub(crate) fn push_bits(bytes: &mut Vec<u8>, value: Compensated) {
    for limb in value.bits() {
        bytes.extend_from_slice(&limb.to_be_bytes());
    }
}

#[derive(Clone)]
pub(crate) struct LedgerChunk {
    interval: Interval,
    quantities: Vec<QuantityStep>,
    columns: Vec<ColumnStep>,
    root: Digest,
}
#[derive(Clone)]
struct QuantityStep {
    inventory: [Compensated; 2],
    inflow: [Compensated; 2],
    outflow: [Compensated; 2],
}
#[derive(Clone)]
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
        let mut chunk = Self {
            interval: step.ticket().interval(),
            quantities,
            columns,
            root: history.root(),
        };
        chunk.root = rolled(history.root(), chunk.digest());
        chunk
    }
    pub(crate) fn root(&self) -> Digest {
        self.root
    }
    pub(crate) fn is_finite(&self) -> bool {
        let Self {
            interval: _,
            quantities,
            columns,
            root: _,
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
    fn bytes(&self) -> Vec<u8> {
        let Self {
            interval,
            quantities,
            columns,
            root: _,
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
        bytes
    }
    pub(crate) fn digest(&self) -> Digest {
        Digest::of_canonical(&self.bytes())
    }
    /// Reads `bytes`'s output for a body of `quantities` combinations and `columns` columns. Refuses an
    /// empty interval and a value no published step holds.
    fn read(read: &mut ByteReader<'_>, quantities: usize, columns: usize) -> Result<Self, Malformed> {
        let interval = Interval::new(read.time()?, read.time()?).ok_or(Malformed)?;
        let quantities = (0..quantities)
            .map(|_| {
                Ok(QuantityStep {
                    inventory: [read.compensated()?, read.compensated()?],
                    inflow: [read.compensated()?, read.compensated()?],
                    outflow: [read.compensated()?, read.compensated()?],
                })
            })
            .collect::<Result<_, Malformed>>()?;
        let columns = (0..columns)
            .map(|_| {
                Ok(ColumnStep {
                    forward: [read.compensated()?, read.compensated()?],
                    reverse: [read.compensated()?, read.compensated()?],
                })
            })
            .collect::<Result<_, Malformed>>()?;
        let chunk = Self {
            interval,
            quantities,
            columns,
            root: Digest::of_canonical(b"set by the walk"),
        };
        if chunk.is_finite() {
            Ok(chunk)
        } else {
            Err(Malformed)
        }
    }
}

/// One link of the ledger chain: the root it follows and the digest of its chunk.
fn rolled(previous: Digest, chunk: Digest) -> Digest {
    let mut pair = [0u8; 64];
    pair[..32].copy_from_slice(previous.as_bytes());
    pair[32..].copy_from_slice(chunk.as_bytes());
    Digest::of_canonical(&pair)
}

pub(crate) struct History {
    opening: Vec<Compensated>,
    incidence: Vec<Vec<i64>>,
    opening_root: Digest,
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
        Self::from_opening(body, opening)
    }
    fn from_opening(body: &CompiledBody, opening: Vec<Compensated>) -> Self {
        let combinations = body.conserved_basis().combinations();
        let incidence: Vec<Vec<i64>> = (combinations.iter())
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
        let mut bytes = b"ng-record ledger root 1".to_vec();
        for value in &opening {
            push_bits(&mut bytes, *value);
        }
        for coefficient in incidence.iter().flatten() {
            bytes.extend_from_slice(&coefficient.to_be_bytes());
        }
        Self {
            opening,
            incidence,
            opening_root: Digest::of_canonical(&bytes),
            chunks: Vec::new(),
        }
    }
    pub(crate) fn opening_values(&self) -> &[Compensated] {
        &self.opening
    }
    /// The ledger root at the accepted head: one link per accepted step from the opening.
    pub(crate) fn root(&self) -> Digest {
        self.chunks.last().map_or(self.opening_root, |chunk| chunk.root)
    }
    pub(crate) fn push(&mut self, chunk: LedgerChunk) {
        self.chunks.push(chunk);
    }
    /// The stored form of one link: the root before it, then its chunk.
    fn link(&self, at: usize) -> Vec<u8> {
        let previous = at
            .checked_sub(1)
            .map_or(self.opening_root, |before| self.chunks[before].root);
        let mut bytes = previous.as_bytes().to_vec();
        bytes.extend_from_slice(&self.chunks[at].bytes());
        bytes
    }
    /// Every link the store lacks, oldest first, so a store that fails part way still holds a chain with
    /// no gap. `resolve` answers for what the store holds, and a store is asked to keep what it took.
    pub(crate) fn unkept(
        &self,
        resolve: &dyn Fn(&Digest) -> bool,
    ) -> impl Iterator<Item = (Digest, Vec<u8>)> {
        let kept = self.chunks.iter().rposition(|chunk| resolve(&chunk.root));
        let from = kept.map_or(0, |at| at + 1);
        (from..self.chunks.len()).map(|at| (self.chunks[at].root, self.link(at)))
    }
    /// Answers a walk from this record's own chain, newest first, as a store would.
    pub(crate) fn resolver(&self) -> impl Fn(&Digest) -> Option<Vec<u8>> + '_ {
        let end = Cell::new(self.chunks.len());
        move |root| {
            let chunks = &self.chunks[..end.get()];
            let at = chunks.iter().rposition(|chunk| chunk.root == *root)?;
            end.set(at);
            Some(self.link(at))
        }
    }
    /// Rebuilds a history of `count` links from `root` back to the opening. Each link is checked against
    /// the root that asked for it before a byte of it is parsed, so a store is trusted for nothing.
    pub(crate) fn walked(
        body: &CompiledBody,
        opening: Vec<Compensated>,
        root: Digest,
        count: u64,
        resolve: &dyn Fn(&Digest) -> Option<Vec<u8>>,
    ) -> Result<Self, CheckpointError> {
        let mut history = Self::from_opening(body, opening);
        let quantities = body.conserved_basis().combinations().len();
        let mut chunks = Vec::new();
        let mut at = root;
        for _ in 0..count {
            let altered = CheckpointError::HistoryAltered { root: at };
            let link = resolve(&at).ok_or(CheckpointError::HistoryMissing { root: at })?;
            let (previous, bytes) = link.split_at_checked(32).ok_or(altered)?;
            let previous = *previous.first_chunk().ok_or(altered)?;
            let previous = Digest::from_bytes(previous);
            if rolled(previous, Digest::of_canonical(bytes)) != at {
                return Err(altered);
            }
            let mut read = ByteReader::new(bytes);
            let mut chunk =
                LedgerChunk::read(&mut read, quantities, body.columns().len()).map_err(|_| altered)?;
            read.finish().map_err(|_| altered)?;
            chunk.root = at;
            chunks.push(chunk);
            at = previous;
        }
        if at != history.opening_root {
            return Err(CheckpointError::HistoryAltered { root: at });
        }
        chunks.reverse();
        history.chunks = chunks;
        Ok(history)
    }
    pub(crate) fn digest(&self) -> Digest {
        let Self {
            opening,
            incidence,
            opening_root: _,
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
    pub fn basis(&self) -> DenseBasis {
        DenseBasis::CHORD
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
