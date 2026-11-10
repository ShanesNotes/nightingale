use crate::graph::Digraph;
use crate::identity::{Canonical, Seal};
use crate::quantity::dimension;
use crate::scope::HeldCard;
use crate::{
    Atoms, BoundaryDirection, Column, ColumnId, Digest, Dimension, Element, Energy, Makeup, RowId, Species,
};

/// The conserved quantity a combination balances. Where no reaction joins species, what its pools hold: a moiety is
/// its species across spaces, because a transfer never changes a species. Where a reaction joins species, an
/// element's atoms, or energy. Charge is the sum of every ion by its sign. The order breaks ties between combinations
/// that start on one row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Book {
    Of(Species),
    Charge,
    Element(Element),
    /// The energy species, and each fuel at the content the schema states.
    Energy,
}
impl Book {
    pub fn measures<U: Dimension>(self) -> bool {
        match self {
            Book::Of(species) => dimension::<U>() == species.dimension(),
            Book::Charge => false,
            Book::Element(_) => dimension::<U>() == dimension::<Atoms>(),
            Book::Energy => dimension::<U>() == dimension::<Energy>(),
        }
    }
}

/// What one unit of `species` counts in `book`: one of itself, its charge, its atoms of an element, or its energy, a
/// fuel's at the content the schema states.
pub(crate) fn weight(book: Book, species: Species, contents: &[(Species, i64)]) -> i64 {
    match (book, species.makeup()) {
        (Book::Of(own), _) => i64::from(own == species),
        (Book::Charge, _) => species.charge(),
        (Book::Element(element), Makeup::Atoms { elements, .. }) => (elements.iter())
            .find(|held| held.0 == element)
            .map_or(0, |held| held.1),
        (Book::Energy, Makeup::Energy) => 1,
        (Book::Energy, Makeup::Atoms { fuel: true, .. }) => (contents.iter())
            .find(|content| content.0 == species)
            .map_or(0, |content| content.1),
        (Book::Element(_) | Book::Energy, _) => 0,
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
            Book::Element(element) => {
                out.number(u64::MAX - 1);
                out.number(element as u64);
            }
            Book::Energy => out.number(u64::MAX - 2),
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

#[cfg(test)]
mod tests {
    use crate::*;

    fn at_origin() -> Result<Position, AdmissionError> {
        Ok(Position::new(
            Measure::new(0.0)?,
            Measure::new(0.0)?,
            Measure::new(0.0)?,
        ))
    }

    /// A bed that burns glucose in its interstitium into its own heat, its glucose topped up toward a level through a
    /// boundary that reads it. Rows: the interstitium's water, oxygen, carbon dioxide and glucose, then the heat.
    fn topped_up_bed() -> Result<CompiledBody, AdmissionError> {
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", at_origin()?);
        let vessel = map.site::<VesselSegment>("vessel", at_origin()?);
        let bed = map.site::<CapillaryBed>("bed", at_origin()?);
        for site in [vessel.id(), bed.id()] {
            map.contain(root.id(), site);
        }
        map.connect(Network::Vascular, vessel.id(), bed.id());
        map.connect(Network::Vascular, bed.id(), vessel.id());
        let mut schema = Schema::new(map.finish()?)?;
        for space in [Space::Plasma, Space::Cells, Space::Lumen] {
            schema.leave_out(space, "the burn draws on the interstitium alone");
        }
        schema.carry::<Oxygen, CapillaryBed>(Space::Interstitium);
        schema.carry::<CarbonDioxide, CapillaryBed>(Space::Interstitium);
        schema.carry::<Glucose, CapillaryBed>(Space::Interstitium);
        schema.carry::<Energy, CapillaryBed>(Space::Thermal);
        schema.energy_content::<Glucose>(Measure::new(1_000.0)?)?;
        let supply = "bed glucose";
        schema.boundary::<Glucose>(supply, BoundaryDirection::IntoBody, bed.id(), Space::Interstitium)?;
        let class = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
        assemble(schema, |mut body| {
            let held = HeldDeclaration::<Metabolism>::new(Vec::new(), Vec::new(), "bench: synthetic");
            let (mut laws, _) = body.hold(held)?.parts();
            let burn = Proportions::using::<Glucose>(Space::Interstitium)
                .uses::<Oxygen, 6>(Space::Interstitium)
                .makes::<CarbonDioxide, 6>(Space::Interstitium)
                .makes::<Water, 6>(Space::Interstitium)
                .energy_into(Space::Thermal);
            let burn = laws.reaction::<CapillaryBed, Glucose>("burn", burn);
            let pace = laws.site_parameter::<CapillaryBed, Rate<Dimensionless>>(
                ParameterKey::new("mt.burn-speed"),
                Admissible::nonnegative(),
            );
            let fuel = burn.amount::<Glucose>();
            let rate = laws.site_kernel(|mut k| {
                let fuel = k.at_site(&fuel);
                let zero = k.zero();
                let pace = k.site_parameter(&pace);
                let rate = k.relax(fuel, zero, pace);
                k.finish(rate, laws.every_mode(), class)
            });
            laws.react_at(&burn, rate);
            body.add(laws.finish());
            let (mut laws, ports) = body.boundary()?.parts();
            let BoundaryRoute { route, .. } = ports.route::<Glucose>(supply)?;
            let level =
                laws.parameter::<Glucose>(ParameterKey::new("bench.glucose-level"), Admissible::positive());
            let speed = laws.parameter::<Rate<Dimensionless>>(
                ParameterKey::new("bench.glucose-speed"),
                Admissible::nonnegative(),
            );
            let held = route.amount::<Glucose>();
            let rate = laws.kernel(|mut k| {
                let (level, held, speed) = (k.parameter(&level), k.read(&held), k.parameter(&speed));
                let rate = k.relax(level, held, speed);
                k.finish(rate, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&route, rate));
            body.add(laws.finish());
            body.compile()
        })
    }

    /// The supply reads glucose, so it answers its own flow and each of the burn's four quantities, which all hold
    /// glucose, may move: what stays put is each combination of them the supply leaves unchanged. Oxygen and carbon
    /// dioxide together, water less carbon dioxide, and 500 water less 3 heat each lead with a row of their own.
    #[test]
    fn a_fuel_supply_that_reads_the_body_leaves_three_totals_each_on_its_own_row() {
        let body = topped_up_bed().expect("compiled");
        let plan: Vec<(usize, Vec<(usize, i64)>, AdmissionRow)> = (body.admission_totals().iter())
            .map(|total| {
                let terms = total.terms.iter().map(|&(row, weight)| (row.index(), weight));
                (total.row.index(), terms.collect(), total.label)
            })
            .collect();
        let conserved = |index| AdmissionRow::Conserved(ConservedRowId { index });
        assert_eq!(
            plan,
            [
                (1, vec![(1, 1), (2, 1)], conserved(0)),
                (0, vec![(0, 1), (2, -1)], conserved(1)),
                (2, vec![(0, 500), (4, -3)], conserved(2)),
            ]
        );
    }
}
