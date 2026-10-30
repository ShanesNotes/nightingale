use crate::graph::Digraph;
use crate::identity::{Canonical, Seal};
use crate::quantity::dimension;
use crate::{Column, Digest, Dimension, RowId, Water};

/// The conserved quantity a combination balances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Book {
    Water,
}
impl Book {
    pub fn measures<U: Dimension>(self) -> bool {
        match self {
            Book::Water => dimension::<U>() == dimension::<Water>(),
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
        (self.combinations.iter().enumerate()).map(|(index, combination)| QuantityId {
            index,
            book: combination.book,
        })
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
    book: Book,
    terms: Vec<(RowId, i64)>,
}
impl Combination {
    pub fn book(&self) -> Book {
        self.book
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
        let Self { book, terms } = self;
        out.number(*book as u64);
        terms.seal(out);
    }
}

pub(crate) fn conserved_basis(amounts: usize, columns: &[Column]) -> ConservedBasis {
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
    let part = Digraph::new(amounts, &edges).components();
    let mut seen: Vec<usize> = Vec::new();
    let mut combinations: Vec<Combination> = Vec::new();
    for (row, &part) in part.iter().enumerate() {
        let at = match seen.iter().position(|&known| known == part) {
            Some(at) => at,
            None => {
                seen.push(part);
                combinations.push(Combination {
                    book: Book::Water,
                    terms: Vec::new(),
                });
                seen.len() - 1
            }
        };
        combinations[at].terms.push((RowId::at(row), 1));
    }
    ConservedBasis { combinations }
}
