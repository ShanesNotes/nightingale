use crate::graph::Digraph;
use crate::identity::{Canonical, Seal};
use crate::quantity::dimension;
use crate::scope::HeldCard;
use crate::{BoundaryDirection, Column, ColumnId, Digest, Dimension, RowId, Species};

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

/// The conserved combinations of the body's amounts: one per set of amounts internal transfers join, a
/// basis of the left null space of the internal incidence in exact integers, then charge, which that
/// basis spans.
#[derive(Debug, PartialEq)]
pub struct ConservedBasis {
    combinations: Vec<Combination>,
    coverage: CoverageManifest,
}
impl ConservedBasis {
    /// Ordered by each combination's least row, with charge last.
    pub fn combinations(&self) -> &[Combination] {
        &self.combinations
    }
    pub fn coverage(&self) -> &CoverageManifest {
        &self.coverage
    }
    pub fn ids(&self) -> impl Iterator<Item = QuantityId> + '_ {
        self.combinations.iter().map(Combination::id)
    }
    /// Also seals the held routes and pools, which the model hash covers through the layout they come from.
    pub fn digest(&self) -> Digest {
        let mut out = Canonical::new("ng-contract conserved basis 1");
        self.seal(&mut out);
        let CoverageManifest {
            declarations: _,
            routes,
            pools,
        } = &self.coverage;
        routes.seal(&mut out);
        pools.seal(&mut out);
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
        let Self {
            combinations,
            coverage,
        } = self;
        combinations.seal(out);
        coverage.seal(out);
    }
}

/// What the basis rests on outside the slice: each held declaration, each boundary column its routes move, and each
/// pool it holds. Declared coverage, not proof of the outside system's physiology.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverageManifest {
    declarations: Vec<HeldCard>,
    routes: Vec<HeldRoute>,
    pools: Vec<HeldPool>,
}
impl CoverageManifest {
    /// In card order.
    pub fn declarations(&self) -> &[HeldCard] {
        &self.declarations
    }
    /// One per column of a held route, in column order.
    pub fn held_routes(&self) -> &[HeldRoute] {
        &self.routes
    }
    /// In row order.
    pub fn pools(&self) -> &[HeldPool] {
        &self.pools
    }
}
impl Seal for CoverageManifest {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            declarations,
            routes: _,
            pools: _,
        } = self;
        declarations.seal(out);
    }
}
/// One column of a route a held stand-in supplies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldRoute {
    card: &'static str,
    name: &'static str,
    column: ColumnId,
    quantity: QuantityId,
    forward: BoundaryDirection,
}
impl HeldRoute {
    pub fn card(&self) -> &'static str {
        self.card
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn column(&self) -> ColumnId {
        self.column
    }
    pub fn quantity(&self) -> QuantityId {
        self.quantity
    }
    /// The way the column's forward extent crosses the boundary.
    pub fn forward(&self) -> BoundaryDirection {
        self.forward
    }
}
impl Seal for HeldRoute {
    fn seal(&self, out: &mut Canonical) {
        let Self {
            card,
            name,
            column,
            quantity,
            forward,
        } = self;
        out.text(card);
        out.text(name);
        column.seal(out);
        out.length(quantity.index);
        out.number(*forward as u64);
    }
}
/// A pool a held stand-in answers for. No law moves it, so it is a combination of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeldPool {
    card: &'static str,
    row: RowId,
    quantity: QuantityId,
}
impl HeldPool {
    pub fn card(&self) -> &'static str {
        self.card
    }
    pub fn row(&self) -> RowId {
        self.row
    }
    pub fn quantity(&self) -> QuantityId {
        self.quantity
    }
}
impl Seal for HeldPool {
    fn seal(&self, out: &mut Canonical) {
        let Self { card, row, quantity } = self;
        out.text(card);
        row.seal(out);
        out.length(quantity.index);
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

pub(crate) fn conserved_basis(
    amounts: &[Species],
    columns: &[Column],
    reads: &[bool],
    held: Vec<HeldCard>,
    pools: Vec<(RowId, &'static str)>,
) -> ConservedBasis {
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
    let quantity = |row: RowId| {
        let holds = |combination: &&Combination| combination.terms.iter().any(|term| term.0 == row);
        (combinations.iter().find(holds))
            .expect("every amount row is in one combination")
            .id
    };
    let mut routes = Vec::new();
    for (at, column) in columns.iter().enumerate() {
        let (Some(receipt), &[(row, coefficient)]) = (column.receipt, &column.terms[..]) else {
            continue;
        };
        let supplied = |card: &&HeldCard| card.supplies.contains(&receipt.boundary);
        if let Some(card) = held.iter().find(supplied) {
            routes.push(HeldRoute {
                card: card.card,
                name: receipt.boundary,
                column: ColumnId::at(at),
                quantity: quantity(row),
                forward: if coefficient > 0 {
                    BoundaryDirection::IntoBody
                } else {
                    BoundaryDirection::OutOfBody
                },
            });
        }
    }
    let pools = (pools.into_iter())
        .map(|(row, card)| HeldPool {
            card,
            row,
            quantity: quantity(row),
        })
        .collect();
    ConservedBasis {
        combinations,
        coverage: CoverageManifest {
            declarations: held,
            routes,
            pools,
        },
    }
}
