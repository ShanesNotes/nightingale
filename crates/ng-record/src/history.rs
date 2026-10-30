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
        let previous = history.links.last().map(|link| &link.chunk);
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
    fn bytes(&self) -> Vec<u8> {
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
        bytes
    }
    fn digest(&self) -> Digest {
        Digest::of_canonical(&self.bytes())
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

fn linked(previous: Digest, revision: u64, image: Digest, ledger: Digest) -> Digest {
    let mut bytes = Vec::with_capacity(104);
    bytes.extend_from_slice(previous.as_bytes());
    bytes.extend_from_slice(&revision.to_be_bytes());
    bytes.extend_from_slice(image.as_bytes());
    bytes.extend_from_slice(ledger.as_bytes());
    Digest::of_canonical(&bytes)
}

pub(crate) struct Link {
    chunk: LedgerChunk,
    image: Digest,
    seal: Digest,
}
impl Link {
    pub(crate) fn seal(&self) -> Digest {
        self.seal
    }
}

/// The seal of revision 0 also covers the values bound for the run, because no later revision changes them.
pub(crate) struct Origin {
    pub(crate) image: Digest,
    seal: Digest,
}

pub(crate) struct History {
    opening: Vec<Compensated>,
    incidence: Vec<Vec<i64>>,
    origin: Origin,
    links: Vec<Link>,
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
            links: Vec::new(),
        }
    }
    pub(crate) fn opening_values(&self) -> &[Compensated] {
        &self.opening
    }
    pub(crate) fn origin(&self) -> &Origin {
        &self.origin
    }
    pub(crate) fn seal(&self) -> Digest {
        self.links.last().map_or(self.origin.seal, |link| link.seal)
    }
    pub(crate) fn head_image(&self) -> Digest {
        self.links.last().map_or(self.origin.image, |link| link.image)
    }
    pub(crate) fn linked(&self, chunk: LedgerChunk, revision: u64, image: Digest) -> Link {
        let seal = linked(self.seal(), revision, image, chunk.digest());
        Link { chunk, image, seal }
    }
    pub(crate) fn push(&mut self, link: Link) {
        self.links.push(link);
    }
    fn stored(&self, at: usize) -> Vec<u8> {
        let previous = (at.checked_sub(1)).map_or(self.origin.seal, |before| self.links[before].seal);
        let link = &self.links[at];
        let mut bytes = previous.as_bytes().to_vec();
        bytes.extend_from_slice(link.image.as_bytes());
        bytes.extend_from_slice(&link.chunk.bytes());
        bytes
    }
    /// The links after the newest one `held`, oldest first, so a store that fails part way still holds a
    /// chain with no gap. A store is asked to keep what it took, so nothing older is offered again.
    pub(crate) fn unkept(&self, held: &dyn Fn(&Digest) -> bool) -> Vec<(Digest, Vec<u8>)> {
        let kept = self.links.iter().rposition(|link| held(&link.seal));
        let from = kept.map_or(0, |at| at + 1);
        (from..self.links.len())
            .map(|at| (self.links[at].seal, self.stored(at)))
            .collect()
    }
    /// Answers a walk from this record's own chain, newest first, as a store would.
    pub(crate) fn resolver(&self) -> impl Fn(&Digest) -> Option<Vec<u8>> + '_ {
        let end = Cell::new(self.links.len());
        move |seal| {
            let links = &self.links[..end.get()];
            let at = links.iter().rposition(|link| link.seal == *seal)?;
            end.set(at);
            Some(self.stored(at))
        }
    }
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
        let mut links = Vec::new();
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
            let chunk = LedgerChunk::read(&mut read, quantities, columns).map_err(|_| altered)?;
            read.finish().map_err(|_| altered)?;
            links.push(Link {
                chunk,
                image,
                seal: at,
            });
            at = previous;
        }
        if at != history.origin.seal {
            return Err(CheckpointError::HistoryAltered { seal: at });
        }
        links.reverse();
        history.links = links;
        Ok(history)
    }
    pub(crate) fn digest(&self) -> Digest {
        let Self {
            opening,
            incidence,
            origin: _,
            links,
        } = self;
        let mut bytes = Vec::new();
        for value in opening {
            push_bits(&mut bytes, *value);
        }
        for coefficient in incidence.iter().flatten() {
            bytes.extend_from_slice(&coefficient.to_be_bytes());
        }
        for link in links {
            bytes.extend_from_slice(link.chunk.digest().as_bytes());
        }
        Digest::of_canonical(&bytes)
    }
}

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
        let links = &self.history.links;
        Interval::new(
            links.first()?.chunk.interval.start(),
            links.last()?.chunk.interval.end(),
        )
    }
    /// Every accepted segment in time order, each ending where the next starts.
    pub fn segments(&self) -> Segments<'r> {
        Segments {
            links: self.history.links.iter(),
        }
    }
}
pub struct Segments<'r> {
    links: core::slice::Iter<'r, Link>,
}
impl<'r> Iterator for Segments<'r> {
    type Item = HistorySegment<'r>;
    fn next(&mut self) -> Option<HistorySegment<'r>> {
        let chunk = &self.links.next()?.chunk;
        Some(HistorySegment { chunk })
    }
}
impl<'r> DoubleEndedIterator for Segments<'r> {
    fn next_back(&mut self) -> Option<HistorySegment<'r>> {
        let chunk = &self.links.next_back()?.chunk;
        Some(HistorySegment { chunk })
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
