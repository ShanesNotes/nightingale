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

pub(crate) struct LedgerChunk {
    interval: Interval,
    quantities: Vec<QuantityStep>,
    columns: Vec<ColumnStep>,
    image: Digest,
    seal: Digest,
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
            image: Digest::of_canonical(b"set when the chunk is sealed"),
            seal: Digest::of_canonical(b"set when the chunk is sealed"),
        }
    }
    /// Chains this chunk onto `previous` with the digest of the image it ends at, and returns its seal.
    pub(crate) fn seal(&mut self, previous: Digest, revision: u64, image: Digest) -> Digest {
        self.image = image;
        self.seal = linked(previous, revision, image, self.digest());
        self.seal
    }
    pub(crate) fn is_finite(&self) -> bool {
        let Self {
            interval: _,
            quantities,
            columns,
            image: _,
            seal: _,
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
            image: _,
            seal: _,
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
    fn read(
        read: &mut ByteReader<'_>,
        quantities: usize,
        columns: usize,
        image: Digest,
        seal: Digest,
    ) -> Result<Self, Malformed> {
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
            image,
            seal,
        };
        if chunk.is_finite() {
            Ok(chunk)
        } else {
            Err(Malformed)
        }
    }
}

/// One link of the chain every accepted revision seals: the seal before it, the revision, the digest of
/// the image it ends at and the digest of its ledger. It covers the past and nothing that waits.
pub(crate) fn linked(previous: Digest, revision: u64, image: Digest, ledger: Digest) -> Digest {
    let mut bytes = Vec::with_capacity(104);
    bytes.extend_from_slice(previous.as_bytes());
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(image.as_bytes());
    bytes.extend_from_slice(ledger.as_bytes());
    Digest::of_canonical(&bytes)
}

/// What the chain starts from: the digest of the admitted image and the seal of revision 0, which also
/// covers the values bound for the run, because no later revision changes them.
pub(crate) struct Origin {
    pub(crate) image: Digest,
    seal: Digest,
}

pub(crate) struct History {
    opening: Vec<Compensated>,
    incidence: Vec<Vec<i64>>,
    origin: Origin,
    chunks: Vec<LedgerChunk>,
}
impl History {
    pub(crate) fn opening(
        body: &CompiledBody,
        accepted: AcceptedStateView<'_>,
        run: Digest,
        image: Digest,
        parameters: Digest,
    ) -> Self {
        let combinations = body.conserved_basis().combinations();
        let opening = (combinations.iter())
            .map(|combination| {
                (combination.terms().iter()).fold(Compensated::ZERO, |sum, &(row, weight)| {
                    sum.plus(accepted.value(row).scaled(weight))
                })
            })
            .collect();
        Self::from_opening(body, opening, run, image, parameters)
    }
    fn from_opening(
        body: &CompiledBody,
        opening: Vec<Compensated>,
        run: Digest,
        image: Digest,
        parameters: Digest,
    ) -> Self {
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
        let mut bytes = b"ng-record ledger opening 1".to_vec();
        for value in &opening {
            push_bits(&mut bytes, *value);
        }
        for coefficient in incidence.iter().flatten() {
            bytes.extend_from_slice(&coefficient.to_be_bytes());
        }
        bytes.extend_from_slice(parameters.as_bytes());
        let seal = linked(run, 0, image, Digest::of_canonical(&bytes));
        Self {
            opening,
            incidence,
            origin: Origin { image, seal },
            chunks: Vec::new(),
        }
    }
    pub(crate) fn opening_values(&self) -> &[Compensated] {
        &self.opening
    }
    pub(crate) fn origin(&self) -> &Origin {
        &self.origin
    }
    /// The seal of the accepted head.
    pub(crate) fn seal(&self) -> Digest {
        self.chunks.last().map_or(self.origin.seal, |chunk| chunk.seal)
    }
    /// The digest of the image the accepted head holds, as the chain recorded it.
    pub(crate) fn head_image(&self) -> Digest {
        self.chunks.last().map_or(self.origin.image, |chunk| chunk.image)
    }
    pub(crate) fn push(&mut self, chunk: LedgerChunk) {
        self.chunks.push(chunk);
    }
    /// The stored form of one link: the seal before it, the image digest it holds, then its chunk.
    fn link(&self, at: usize) -> Vec<u8> {
        let previous = (at.checked_sub(1)).map_or(self.origin.seal, |before| self.chunks[before].seal);
        let chunk = &self.chunks[at];
        let mut bytes = previous.as_bytes().to_vec();
        bytes.extend_from_slice(chunk.image.as_bytes());
        bytes.extend_from_slice(&chunk.bytes());
        bytes
    }
    /// Every link the store lacks, oldest first, so a store that fails part way still holds a chain with
    /// no gap. `held` answers for what the store holds, and a store is asked to keep what it took.
    pub(crate) fn unkept(&self, held: &dyn Fn(&Digest) -> bool) -> Vec<(Digest, Vec<u8>)> {
        let kept = self.chunks.iter().rposition(|chunk| held(&chunk.seal));
        let from = kept.map_or(0, |at| at + 1);
        (from..self.chunks.len())
            .map(|at| (self.chunks[at].seal, self.link(at)))
            .collect()
    }
    /// Answers a walk from this record's own chain, newest first, as a store would.
    pub(crate) fn resolver(&self) -> impl Fn(&Digest) -> Option<Vec<u8>> + '_ {
        let end = Cell::new(self.chunks.len());
        move |seal| {
            let chunks = &self.chunks[..end.get()];
            let at = chunks.iter().rposition(|chunk| chunk.seal == *seal)?;
            end.set(at);
            Some(self.link(at))
        }
    }
    /// Rebuilds a history of `count` links from `seal` back to the opening. Each link is checked against
    /// the seal that asked for it before a byte of it is parsed, so a store is trusted for nothing.
    pub(crate) fn walked(
        body: &CompiledBody,
        start: Start,
        seal: Digest,
        count: u64,
        resolve: &dyn Fn(&Digest) -> Option<Vec<u8>>,
    ) -> Result<Self, CheckpointError> {
        let Start {
            run,
            opening,
            image,
            parameters,
        } = start;
        let mut history = Self::from_opening(body, opening, run, image, parameters);
        let quantities = body.conserved_basis().combinations().len();
        let columns = body.columns().len();
        let mut chunks = Vec::new();
        let mut at = seal;
        for revision in (1..=count).rev() {
            let altered = CheckpointError::HistoryAltered { seal: at };
            let link = resolve(&at).ok_or(CheckpointError::HistoryMissing { seal: at })?;
            let (previous, rest) = link.split_at_checked(32).ok_or(altered)?;
            let (image, bytes) = rest.split_at_checked(32).ok_or(altered)?;
            let previous = Digest::from_bytes(*previous.first_chunk().ok_or(altered)?);
            let image = Digest::from_bytes(*image.first_chunk().ok_or(altered)?);
            if linked(previous, revision, image, Digest::of_canonical(bytes)) != at {
                return Err(altered);
            }
            let mut read = ByteReader::new(bytes);
            let chunk = LedgerChunk::read(&mut read, quantities, columns, image, at);
            let chunk = chunk.map_err(|_| altered)?;
            read.finish().map_err(|_| altered)?;
            chunks.push(chunk);
            at = previous;
        }
        if at != history.origin.seal {
            return Err(CheckpointError::HistoryAltered { seal: at });
        }
        chunks.reverse();
        history.chunks = chunks;
        Ok(history)
    }
    pub(crate) fn digest(&self) -> Digest {
        let Self {
            opening,
            incidence,
            origin: _,
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

/// What a walk needs to rebuild the start of a chain: the checkpoint holds the rest of the origin's facts.
pub(crate) struct Start {
    pub(crate) run: Digest,
    pub(crate) opening: Vec<Compensated>,
    pub(crate) image: Digest,
    pub(crate) parameters: Digest,
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
