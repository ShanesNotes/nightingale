use crate::checkpoint::{CheckpointError, HistoryStore, RecordFormat};
use core::num::NonZeroU64;
use ng_contract::{
    AcceptedStateView, AdmissibleSet, ByteReader, ColumnId, Compensated, CompiledBody, ConservedBasis,
    DenseBasis, Digest, Interval, Malformed, QuantityId, RecordHash, SimTime, VerifiedStep,
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
        let previous = history.chain.head().map(|head| &head.chunk);
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
    fn shape(&self) -> (usize, usize) {
        (self.quantities.len(), self.columns.len())
    }
    fn write(&self, bytes: &mut Vec<u8>) {
        let Self {
            interval,
            quantities,
            columns,
        } = self;
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
                push_bits(bytes, *value);
            }
        }
        for column in columns {
            let ColumnStep { forward, reverse } = column;
            for value in forward.iter().chain(reverse) {
                push_bits(bytes, *value);
            }
        }
    }
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
        };
        if chunk.is_finite() {
            Ok(chunk)
        } else {
            Err(Malformed)
        }
    }
}

fn linked(hash: RecordHash, previous: Digest, revision: u64, image: Digest, ledger: &[u8]) -> Digest {
    hash.link(previous, revision, image, Digest::for_record(ledger, hash))
}

fn opening_bytes(opening: &[Compensated], incidence: &[Vec<i64>], parameters: Digest) -> Vec<u8> {
    let mut bytes = b"ng-record ledger opening 1".to_vec();
    for value in opening {
        push_bits(&mut bytes, *value);
    }
    for coefficient in incidence.iter().flatten() {
        bytes.extend_from_slice(&coefficient.to_be_bytes());
    }
    bytes.extend_from_slice(parameters.as_bytes());
    bytes
}

const LINK_HEAD: usize = 2 * size_of::<Digest>();

pub(crate) struct Link {
    chunk: LedgerChunk,
    image: Digest,
    seal: Digest,
    stored: Vec<u8>,
}
impl Link {
    pub(crate) fn seal(&self) -> Digest {
        self.seal
    }
}

/// The seal of revision 0 also covers the values bound for the run, because no later revision changes them.
#[derive(PartialEq)]
pub(crate) struct Origin {
    pub(crate) image: Digest,
    seal: Digest,
}

enum Chain {
    Opening,
    Grown {
        revision: NonZeroU64,
        opened: SimTime,
        head: Head,
    },
}
impl Chain {
    fn head(&self) -> Option<&Head> {
        match self {
            Self::Opening => None,
            Self::Grown { head, .. } => Some(head),
        }
    }
}

struct Head {
    chunk: LedgerChunk,
    image: Digest,
    seal: Digest,
    previous: Digest,
}

pub(crate) struct History {
    format: RecordFormat,
    opening: Vec<Compensated>,
    incidence: Vec<Vec<i64>>,
    origin: Origin,
    chain: Chain,
    unkept: Vec<(Digest, Vec<u8>)>,
}
impl History {
    pub(crate) fn opening(
        format: RecordFormat,
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
        Self::from_opening(format, body, opening, run, image, parameters)
    }
    fn from_opening(
        format: RecordFormat,
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
        let bytes = opening_bytes(&opening, &incidence, parameters);
        let seal = linked(format.hash(), run, 0, image, &bytes);
        Self {
            format,
            opening,
            incidence,
            origin: Origin { image, seal },
            chain: Chain::Opening,
            unkept: Vec::new(),
        }
    }
    pub(crate) fn format(&self) -> RecordFormat {
        self.format
    }
    pub(crate) fn opening_values(&self) -> &[Compensated] {
        &self.opening
    }
    pub(crate) fn origin(&self) -> &Origin {
        &self.origin
    }
    pub(crate) fn revision(&self) -> u64 {
        match &self.chain {
            Chain::Opening => 0,
            Chain::Grown { revision, .. } => revision.get(),
        }
    }
    pub(crate) fn seal(&self) -> Digest {
        self.chain.head().map_or(self.origin.seal, |head| head.seal)
    }
    pub(crate) fn previous(&self) -> Option<Digest> {
        self.chain.head().map(|head| head.previous)
    }
    /// The digest of the ledger bytes the head's seal covers: the opening's at admission, else the head chunk's.
    pub(crate) fn ledger_digest(&self, parameters: Digest) -> Digest {
        let bytes = match self.chain.head() {
            None => opening_bytes(&self.opening, &self.incidence, parameters),
            Some(head) => {
                let mut bytes = Vec::new();
                head.chunk.write(&mut bytes);
                bytes
            }
        };
        Digest::for_record(&bytes, self.format.hash())
    }
    pub(crate) fn head_image(&self) -> Digest {
        self.chain.head().map_or(self.origin.image, |head| head.image)
    }
    pub(crate) fn oldest_unkept(&self) -> Option<Digest> {
        self.unkept.first().map(|(seal, _)| *seal)
    }
    pub(crate) fn linked(&self, chunk: LedgerChunk, image: Digest) -> Link {
        let previous = self.seal();
        let mut stored = previous.as_bytes().to_vec();
        stored.extend_from_slice(image.as_bytes());
        chunk.write(&mut stored);
        let revision = self.revision() + 1;
        let seal = linked(
            self.format.hash(),
            previous,
            revision,
            image,
            &stored[LINK_HEAD..],
        );
        Link {
            chunk,
            image,
            seal,
            stored,
        }
    }
    pub(crate) fn push(&mut self, link: Link, store: &dyn HistoryStore) {
        let Link {
            chunk,
            image,
            seal,
            stored,
        } = link;
        let revision = NonZeroU64::MIN.saturating_add(self.revision());
        let opened = match &self.chain {
            Chain::Opening => chunk.interval.start(),
            Chain::Grown { opened, .. } => *opened,
        };
        let previous = self.seal();
        let head = Head {
            chunk,
            image,
            seal,
            previous,
        };
        self.chain = Chain::Grown {
            revision,
            opened,
            head,
        };
        self.unkept.push((seal, stored));
        let kept = (self.unkept.iter())
            .position(|(seal, link)| store.keep(*seal, link).is_err())
            .unwrap_or(self.unkept.len());
        self.unkept.drain(..kept);
    }
    fn before_head<'s>(&self, store: &'s dyn HistoryStore) -> Option<Walk<'s>> {
        let Chain::Grown { revision, head, .. } = &self.chain else {
            return None;
        };
        Some(Walk {
            store,
            format: self.format,
            shape: head.chunk.shape(),
            origin: self.origin.seal,
            at: head.previous,
            revision: revision.get() - 1,
            done: false,
        })
    }
    pub(crate) fn holds(
        &self,
        seal: Digest,
        revision: u64,
        store: &dyn HistoryStore,
    ) -> Result<bool, CheckpointError> {
        let Some(head) = self.chain.head() else {
            return Ok(revision == 0 && seal == self.origin.seal);
        };
        let below = match self.revision().checked_sub(revision) {
            None => return Ok(false),
            Some(0) => return Ok(seal == head.seal),
            Some(below) => below,
        };
        let mut at = head.previous;
        let between = usize::try_from(below - 1).unwrap_or(usize::MAX);
        for walked in self.before_head(store).into_iter().flatten().take(between) {
            at = walked?.previous;
        }
        Ok(at == seal)
    }
    pub(crate) fn walked(
        body: &CompiledBody,
        start: Start,
        seal: Digest,
        revision: u64,
        store: &dyn HistoryStore,
    ) -> Result<Self, CheckpointError> {
        let Start {
            format,
            run,
            opening,
            image,
            parameters,
        } = start;
        let mut history = Self::from_opening(format, body, opening, run, image, parameters);
        let mut walk = Walk {
            store,
            format,
            shape: (body.conserved_basis().combinations().len(), body.columns().len()),
            origin: history.origin.seal,
            at: seal,
            revision,
            done: false,
        };
        let newest = walk.next().transpose()?;
        let mut opened = newest.as_ref().map(|walked| walked.chunk.interval.start());
        for walked in walk {
            opened = Some(walked?.chunk.interval.start());
        }
        if let (Some(revision), Some(opened), Some(newest)) = (NonZeroU64::new(revision), opened, newest) {
            let Walked {
                chunk,
                image,
                seal,
                previous,
            } = newest;
            let head = Head {
                chunk,
                image,
                seal,
                previous,
            };
            history.chain = Chain::Grown {
                revision,
                opened,
                head,
            };
        }
        Ok(history)
    }
    pub(crate) fn digest(&self) -> Digest {
        let Self {
            format: _,
            opening,
            incidence,
            origin: _,
            chain,
            unkept: _,
        } = self;
        let mut bytes = Vec::new();
        for value in opening {
            push_bits(&mut bytes, *value);
        }
        for coefficient in incidence.iter().flatten() {
            bytes.extend_from_slice(&coefficient.to_be_bytes());
        }
        if let Some(head) = chain.head() {
            head.chunk.write(&mut bytes);
        }
        Digest::of_canonical(&bytes)
    }
}

pub(crate) struct Walk<'s> {
    store: &'s dyn HistoryStore,
    format: RecordFormat,
    shape: (usize, usize),
    origin: Digest,
    at: Digest,
    revision: u64,
    done: bool,
}
pub(crate) struct Walked {
    chunk: LedgerChunk,
    image: Digest,
    seal: Digest,
    previous: Digest,
}
impl Walk<'_> {
    fn read(&self) -> Result<Walked, CheckpointError> {
        let at = self.at;
        let altered = CheckpointError::HistoryAltered { seal: at };
        let link = (self.store.resolve(&at)).ok_or(CheckpointError::HistoryMissing { seal: at })?;
        let (previous, rest) = link.split_at_checked(32).ok_or(altered)?;
        let (image, bytes) = rest.split_at_checked(32).ok_or(altered)?;
        let previous = Digest::from_bytes(*previous.first_chunk().ok_or(altered)?);
        let image = Digest::from_bytes(*image.first_chunk().ok_or(altered)?);
        if linked(self.format.hash(), previous, self.revision, image, bytes) != at {
            return Err(altered);
        }
        let mut read = ByteReader::new(bytes);
        let (quantities, columns) = self.shape;
        let chunk = LedgerChunk::read(&mut read, quantities, columns).map_err(|_| altered)?;
        read.finish().map_err(|_| altered)?;
        Ok(Walked {
            chunk,
            image,
            seal: at,
            previous,
        })
    }
}
impl Iterator for Walk<'_> {
    type Item = Result<Walked, CheckpointError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if self.revision == 0 {
            self.done = true;
            let altered = CheckpointError::HistoryAltered { seal: self.at };
            return (self.at != self.origin).then_some(Err(altered));
        }
        let walked = self.read();
        match &walked {
            Ok(link) => {
                self.at = link.previous;
                self.revision -= 1;
            }
            Err(_) => self.done = true,
        }
        Some(walked)
    }
}

pub(crate) struct Start {
    pub(crate) format: RecordFormat,
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

/// One immutable view of one record's history over one conserved basis: the head from memory and every older
/// segment through the session's store.
pub struct ConservationView<'r> {
    pub(crate) history: &'r History,
    pub(crate) store: &'r dyn HistoryStore,
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
        let Chain::Grown { opened, head, .. } = &self.history.chain else {
            return None;
        };
        Interval::new(*opened, head.chunk.interval.end())
    }
    /// The last accepted segment; `None` before the first accepted step.
    pub fn head(&self) -> Option<HistorySegment> {
        let chunk = self.history.chain.head()?.chunk.clone();
        Some(HistorySegment { chunk })
    }
    /// Every accepted segment, newest first, each starting where the one before it in time ends.
    pub fn newest_first(&self) -> NewestFirst<'r> {
        NewestFirst {
            head: self.head(),
            walk: self.history.before_head(self.store),
        }
    }
}
pub struct NewestFirst<'r> {
    head: Option<HistorySegment>,
    walk: Option<Walk<'r>>,
}
impl Iterator for NewestFirst<'_> {
    type Item = Result<HistorySegment, CheckpointError>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(head) = self.head.take() {
            return Some(Ok(head));
        }
        let walked = self.walk.as_mut()?.next()?;
        Some(walked.map(|walked| HistorySegment { chunk: walked.chunk }))
    }
}
#[derive(Clone)]
pub struct HistorySegment {
    chunk: LedgerChunk,
}
impl HistorySegment {
    pub fn interval(&self) -> Interval {
        self.chunk.interval
    }
    pub fn basis(&self) -> DenseBasis {
        DenseBasis::CHORD
    }
    pub fn inventory(&self, quantity: QuantityId) -> Curve<'_> {
        self.curve(&self.chunk.quantities[quantity.index()].inventory)
    }
    /// Cumulative gross inflow since admission.
    pub fn inflow(&self, quantity: QuantityId) -> Curve<'_> {
        self.curve(&self.chunk.quantities[quantity.index()].inflow)
    }
    /// Cumulative gross outflow since admission.
    pub fn outflow(&self, quantity: QuantityId) -> Curve<'_> {
        self.curve(&self.chunk.quantities[quantity.index()].outflow)
    }
    /// One column's cumulative gross forward and reverse extents since admission.
    pub fn exchange(&self, column: ColumnId) -> (Curve<'_>, Curve<'_>) {
        let step = &self.chunk.columns[column.index()];
        (self.curve(&step.forward), self.curve(&step.reverse))
    }
    fn curve<'s>(&self, coefficients: &'s [Compensated; 2]) -> Curve<'s> {
        Curve {
            interval: self.chunk.interval,
            coefficients,
        }
    }
}
/// One quantity's or one column's curve over one segment, compensated, in Bernstein form.
pub struct Curve<'s> {
    interval: Interval,
    coefficients: &'s [Compensated; 2],
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
