use crate::graph::Digraph;
use crate::identity::{Canonical, Seal};
use crate::quantity::dimension;
use crate::{Column, Digest, Dimension, RowId, Species};

/// The conserved quantity a combination balances: what its pools hold, or charge, the sum of every ion by its sign.
/// A moiety is its species across spaces, because a transfer never changes a species.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Book {
    Of(Species),
    Charge,
}
impl Book {
    pub fn measures<U: Dimension>(self) -> bool {
        match self {
            Book::Of(species) => dimension::<U>() == species.dimension(),
            Book::Charge => false,
        }
    }
}

/// A combination's position in the basis, with what it balances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuantityId {
    index: usize,
    book: Book,
}
impl QuantityId {
    pub fn index(self) -> usize {
        self.index
    }
    pub fn book(self) -> Book {
        self.book
    }
}

/// Every conserved combination of the body's amounts: a complete basis of the left null space of the
/// internal incidence, in exact integers.
#[derive(Debug, PartialEq)]
pub struct ConservedBasis {
    combinations: Vec<Combination>,
}
impl ConservedBasis {
    /// Ordered by each combination's least row.
    pub fn combinations(&self) -> &[Combination] {
        &self.combinations
    }
    pub fn ids(&self) -> impl Iterator<Item = QuantityId> + '_ {
        self.combinations.iter().map(Combination::id)
    }
    pub fn digest(&self) -> Digest {
        let mut out = Canonical::new("ng-contract conserved basis 1");
        self.seal(&mut out);
        out.digest()
    }
}
/// A sum of amounts no internal transfer changes. An exterior transfer changes it through its receipt.
#[derive(Debug, PartialEq)]
pub struct Combination {
    id: QuantityId,
    terms: Vec<(RowId, i64)>,
    regulated: bool,
}
impl Combination {
    /// A boundary law reads the body, so the total answers its own flows: admission solves it, and the packet's is only
    /// a guess. A total no such law touches stays the packet's. Charge is neither: it follows from the ions, and
    /// admission holds no row of its own for it.
    pub fn regulated(&self) -> bool {
        self.regulated
    }
    pub fn id(&self) -> QuantityId {
        self.id
    }
    pub fn book(&self) -> Book {
        self.id.book
    }
    /// In row order.
    pub fn terms(&self) -> &[(RowId, i64)] {
        &self.terms
    }
}
impl Seal for ConservedBasis {
    fn seal(&self, out: &mut Canonical) {
        let Self { combinations } = self;
        combinations.seal(out);
    }
}
impl Seal for Combination {
    fn seal(&self, out: &mut Canonical) {
        let Self { id, terms, regulated } = self;
        match id.book {
            Book::Of(species) => out.number(species as u64),
            Book::Charge => out.number(u64::MAX),
        }
        terms.seal(out);
        out.number(u64::from(*regulated));
    }
}

/// `amounts[row]` is the species a row holds and `reads[column]` says the column's law reads a row.
pub(crate) fn conserved_basis(amounts: &[Species], columns: &[Column], reads: &[bool]) -> ConservedBasis {
    let mut edges = Vec::new();
    for column in columns {
        match column.terms[..] {
            [(from, -1), (to, 1)] | [(to, 1), (from, -1)] => {
                edges.push((from.index(), to.index()));
                edges.push((to.index(), from.index()));
            }
            [] | [_] => {}
            _ => unreachable!("a transfer moves one unit between two amounts, or across the boundary"),
        }
    }
    let part = Digraph::new(amounts.len(), &edges).strongly_connected_components();
    let mut seen: Vec<usize> = Vec::new();
    let mut combinations: Vec<Combination> = Vec::new();
    for (row, &part) in part.iter().enumerate() {
        let at = match seen.iter().position(|&known| known == part) {
            Some(at) => at,
            None => {
                seen.push(part);
                combinations.push(Combination {
                    id: QuantityId {
                        index: seen.len() - 1,
                        book: Book::Of(amounts[row]),
                    },
                    terms: Vec::new(),
                    regulated: false,
                });
                seen.len() - 1
            }
        };
        combinations[at].terms.push((RowId::at(row), 1));
    }
    for (column, _) in
        (columns.iter().zip(reads)).filter(|&(column, reads)| *reads && column.receipt.is_some())
    {
        let row = column.terms[0].0;
        for combination in &mut combinations {
            combination.regulated |= combination.terms.iter().any(|term| term.0 == row);
        }
    }
    let ions: Vec<(RowId, i64)> = (amounts.iter().enumerate())
        .filter(|(_, species)| species.charge() != 0)
        .map(|(row, species)| (RowId::at(row), species.charge()))
        .collect();
    if !ions.is_empty() {
        let index = combinations.len();
        let book = Book::Charge;
        combinations.push(Combination {
            id: QuantityId { index, book },
            terms: ions,
            regulated: false,
        });
    }
    ConservedBasis { combinations }
}
