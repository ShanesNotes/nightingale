use crate::graph::Digraph;
use crate::identity::{Canonical, Seal};
use crate::{Column, RowId};

/// The conserved quantity a combination balances.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Book {
    Water,
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

/// An internal transfer takes from one amount and gives the same to another, so a vector `l` with
/// `l N = 0` is constant across each transfer, and the indicator vectors of the connected parts of the
/// transfer graph are a complete, exact basis. An exterior transfer has one term in the body, so it
/// joins nothing, and an amount only exterior transfers move is its own part.
pub(crate) fn conserved_basis(amounts: usize, columns: &[Column]) -> ConservedBasis {
    let mut edges = Vec::new();
    for column in columns {
        if let [(from, _), (to, _)] = column.terms[..] {
            edges.push((from.index(), to.index()));
            edges.push((to.index(), from.index()));
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
