use crate::identity::{Canonical, Seal};
use crate::quantity::dimension;
use crate::scope::{BINARY64_EXACT, HeldCard};
use crate::{
    AdmissionRow, Atoms, BoundaryDirection, Column, ColumnId, CompileDefect, Digest, Dimension, Element,
    Energy, LawName, Makeup, RowId, Species,
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

/// A conserved quantity a reaction's counts balance: an element's atoms, or energy. No reactant holds a fixed charge,
/// so no reaction can leave charge open.
#[derive(Clone, Copy)]
pub(crate) enum Balanced {
    Element(Element),
    Energy,
}
impl Balanced {
    /// Each element, then energy: the order in which a reaction's first open quantity is named.
    pub(crate) fn all() -> impl Iterator<Item = Balanced> {
        Element::ALL
            .into_iter()
            .map(Balanced::Element)
            .chain([Balanced::Energy])
    }
    pub(crate) fn book(self) -> Book {
        match self {
            Balanced::Element(element) => Book::Element(element),
            Balanced::Energy => Book::Energy,
        }
    }
}

pub(crate) fn weight(book: Balanced, species: Species, contents: &[(Species, i64)]) -> i64 {
    match (book, species.makeup()) {
        (Balanced::Element(element), Makeup::Atoms { elements, .. }) => (elements.iter())
            .find(|held| held.0 == element)
            .map_or(0, |held| held.1),
        (Balanced::Energy, Makeup::Energy) => 1,
        (Balanced::Energy, Makeup::Atoms { fuel: true, .. }) => (contents.iter())
            .find(|content| content.0 == species)
            .map_or(0, |content| content.1),
        (Balanced::Element(_) | Balanced::Energy, _) => 0,
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
    contents: &[(Species, i64)],
    held: Vec<HeldCard>,
    pools: Vec<(RowId, &'static str)>,
) -> Result<ConservedBasis, CompileDefect> {
    let internal: Vec<&Column> = columns.iter().filter(|column| column.receipt.is_none()).collect();
    let ends = |column: &&Column| -> Vec<usize> { column.terms.iter().map(|term| term.0.index()).collect() };
    let parts = connected_sets(amounts.len(), internal.iter().map(ends));
    let mut part_of = vec![0; amounts.len()];
    for (at, part) in parts.iter().enumerate() {
        for &row in part {
            part_of[row] = at;
        }
    }
    let mut inside: Vec<Vec<&Column>> = vec![Vec::new(); parts.len()];
    for &column in &internal {
        if let Some(&(row, _)) = column.terms.first() {
            inside[part_of[row.index()]].push(column);
        }
    }
    let mut found: Vec<(Book, Terms)> = Vec::new();
    for (part, inside) in parts.iter().zip(&inside) {
        let part: Vec<RowId> = part.iter().map(|&row| RowId::at(row)).collect();
        match inside.iter().find(|column| joins_species(column, amounts)) {
            None => found.push((
                Book::Of(amounts[part[0].index()]),
                part.iter().map(|&row| (row, 1)).collect(),
            )),
            Some(reaction) => found.extend(quantities(&part, inside, amounts, contents, reaction.origin)?),
        }
    }
    found.sort_by_key(|(book, terms)| (terms[0].0, *book));
    let mut combinations: Vec<Combination> = (found.into_iter().enumerate())
        .map(|(index, (book, terms))| Combination {
            id: QuantityId { index, book },
            terms,
            regulated: false,
        })
        .collect();
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
    Ok(ConservedBasis {
        combinations,
        coverage: CoverageManifest {
            declarations: held,
            routes,
            pools,
        },
    })
}

type Terms = Vec<(RowId, i64)>;

fn joins_species(column: &Column, amounts: &[Species]) -> bool {
    let mut species = column.terms.iter().map(|term| amounts[term.0.index()]);
    let first = species.next();
    species.any(|other| Some(other) != first)
}

fn connected_sets(count: usize, groups: impl Iterator<Item = Vec<usize>>) -> Vec<Vec<usize>> {
    fn root(parent: &mut [usize], mut at: usize) -> usize {
        while parent[at] != at {
            parent[at] = parent[parent[at]];
            at = parent[at];
        }
        at
    }
    let mut parent: Vec<usize> = (0..count).collect();
    for group in groups {
        if let Some((&first, rest)) = group.split_first() {
            for &other in rest {
                let (a, b) = (root(&mut parent, first), root(&mut parent, other));
                parent[a.max(b)] = a.min(b);
            }
        }
    }
    let (mut sets, mut set_of): (Vec<Vec<usize>>, Vec<Option<usize>>) = (Vec::new(), vec![None; count]);
    for at in 0..count {
        let top = root(&mut parent, at);
        match set_of[top] {
            Some(set) => sets[set].push(at),
            None => {
                set_of[top] = Some(sets.len());
                sets.push(vec![at]);
            }
        }
    }
    sets
}

fn quantities(
    part: &[RowId],
    inside: &[&Column],
    amounts: &[Species],
    contents: &[(Species, i64)],
    law: LawName,
) -> Result<Vec<(Book, Terms)>, CompileDefect> {
    let mut found = Vec::new();
    for book in Balanced::all() {
        let weighed: Vec<(RowId, i64)> = (part.iter())
            .map(|&row| (row, weight(book, amounts[row.index()], contents)))
            .filter(|&(_, weight)| weight != 0)
            .collect();
        let at = |row: &RowId| weighed.binary_search_by_key(row, |held| held.0).ok();
        let groups = inside
            .iter()
            .map(|column| column.terms.iter().filter_map(|term| at(&term.0)).collect());
        for piece in connected_sets(weighed.len(), groups) {
            found.push((book.book(), piece.into_iter().map(|at| weighed[at]).collect()));
        }
    }
    let overflow = CompileDefect::ConservationOverflow { law: Some(law) };
    if !spans_conserved(part, inside, amounts, &found).ok_or(overflow)? {
        return Err(CompileDefect::UnnamedConservedQuantity { law });
    }
    Ok(found)
}

/// A transfer moves one species, so a combination the part's columns leave unchanged is constant on the rows transfers
/// join, and on those sets it is any the reactions leave unchanged.
fn spans_conserved(
    part: &[RowId],
    inside: &[&Column],
    amounts: &[Species],
    found: &[(Book, Terms)],
) -> Option<bool> {
    let index = |row: RowId| {
        part.binary_search(&row)
            .expect("a column inside a part has its rows there")
    };
    let (reactions, transfers): (Vec<&Column>, Vec<&Column>) = inside
        .iter()
        .copied()
        .partition(|column| joins_species(column, amounts));
    let ends = |column: &&Column| -> Vec<usize> { column.terms.iter().map(|term| index(term.0)).collect() };
    let sets = connected_sets(part.len(), transfers.iter().map(ends));
    let mut set_of = vec![0; part.len()];
    for (at, set) in sets.iter().enumerate() {
        for &row in set {
            set_of[row] = at;
        }
    }
    let mut columns = Echelon::default();
    for column in reactions {
        let mut vector = vec![0; sets.len()];
        for &(row, coefficient) in &column.terms {
            vector[set_of[index(row)]] += i128::from(coefficient);
        }
        columns.reduce(vector, sets.len())?;
    }
    let mut spanned = Echelon::default();
    for (_, terms) in found {
        let mut vector = vec![0; sets.len()];
        for &(row, weight) in terms {
            vector[set_of[index(row)]] = i128::from(weight);
        }
        spanned.reduce(vector, sets.len())?;
    }
    Some(spanned.kept.len() + columns.kept.len() == sets.len())
}

#[derive(Default)]
struct Echelon {
    kept: Vec<(usize, Vec<i128>)>,
}
enum Reduced {
    Leads(usize),
    Held(Vec<i128>),
}
impl Echelon {
    fn reduce(&mut self, mut vector: Vec<i128>, leads_within: usize) -> Option<Reduced> {
        for (lead, kept) in &self.kept {
            let (by, scale) = (vector[*lead], kept[*lead]);
            if by == 0 {
                continue;
            }
            for (entry, &other) in vector.iter_mut().zip(kept) {
                *entry = entry.checked_mul(scale)?.checked_sub(other.checked_mul(by)?)?;
            }
            lowest(&mut vector)?;
        }
        match vector[..leads_within].iter().position(|&entry| entry != 0) {
            Some(lead) => {
                let at = self.kept.partition_point(|kept| kept.0 < lead);
                self.kept.insert(at, (lead, vector));
                Some(Reduced::Leads(lead))
            }
            None => Some(Reduced::Held(vector)),
        }
    }
}

fn lowest(vector: &mut [i128]) -> Option<()> {
    let mut divisor: u128 = 0;
    for entry in vector.iter() {
        let (mut a, mut b) = (divisor, entry.unsigned_abs());
        while b != 0 {
            (a, b) = (b, a % b);
        }
        divisor = a;
    }
    let first = vector.iter().find(|&&entry| entry != 0).copied().unwrap_or(0);
    let divisor = i128::try_from(divisor).ok()?.checked_mul(first.signum())?;
    if divisor != 0 {
        for entry in vector.iter_mut() {
            *entry /= divisor;
        }
    }
    Some(())
}

#[derive(Debug, PartialEq)]
pub(crate) struct AdmissionTotal {
    pub(crate) replaces: RowId,
    pub(crate) terms: Vec<(RowId, i64)>,
    pub(crate) label: AdmissionRow,
}

/// A total admission states that is no one combination's: an integer combination of several that the boundary columns
/// reading the body move only together. Its index is its place among admission's totals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConservedRowId {
    index: usize,
}
impl ConservedRowId {
    pub fn index(self) -> usize {
        self.index
    }
}

/// Each total is placed on the row its reduction against those before it leads with, so no two share a row and the
/// replaced rows keep their rank. Charge follows from the rest.
pub(crate) fn admission_totals(
    basis: &ConservedBasis,
    columns: &[Column],
    reads: &[bool],
    amounts: usize,
) -> Option<Vec<AdmissionTotal>> {
    let reading: Vec<&Column> = (columns.iter().zip(reads))
        .filter(|&(column, reads)| *reads && column.receipt.is_some())
        .map(|(column, _)| column)
        .collect();
    let incidence = |combination: &Combination, column: &Column| -> i128 {
        let weight = |row| {
            (combination.terms.iter())
                .find(|term| term.0 == row)
                .map_or(0, |term| term.1)
        };
        (column.terms.iter())
            .map(|&(row, coefficient)| i128::from(coefficient) * i128::from(weight(row)))
            .sum()
    };
    let books = (basis.combinations.iter()).filter(|combination| combination.book() != Book::Charge);
    let (untouched, touched): (Vec<&Combination>, Vec<&Combination>) =
        books.partition(|&combination| reading.iter().all(|&column| incidence(combination, column) == 0));
    let mut totals: Vec<(Terms, Option<QuantityId>)> = (untouched.iter())
        .map(|combination| (combination.terms.clone(), Some(combination.id)))
        .collect();
    let mut moved = Echelon::default();
    for (at, &combination) in touched.iter().enumerate() {
        let mut vector: Vec<i128> = reading
            .iter()
            .map(|&column| incidence(combination, column))
            .collect();
        vector.extend((0..touched.len()).map(|other| i128::from(other == at)));
        let Reduced::Held(held) = moved.reduce(vector, reading.len())? else {
            continue;
        };
        let mut sum = vec![0_i128; amounts];
        for (combination, &by) in touched.iter().zip(&held[reading.len()..]) {
            for &(row, weight) in &combination.terms {
                let entry = &mut sum[row.index()];
                *entry = entry.checked_add(by.checked_mul(i128::from(weight))?)?;
            }
        }
        lowest(&mut sum)?;
        let exact = |entry: &i128| entry.unsigned_abs() <= BINARY64_EXACT.unsigned_abs().into();
        let terms = (sum.iter().enumerate())
            .filter(|&(_, &entry)| entry != 0)
            .map(|(row, &entry)| exact(&entry).then_some((RowId::at(row), entry as i64)))
            .collect::<Option<Vec<_>>>()?;
        totals.push((terms, None));
    }
    let mut rows = Echelon::default();
    let mut plan = Vec::new();
    for (terms, quantity) in totals {
        let mut vector = vec![0; amounts];
        for &(row, weight) in &terms {
            vector[row.index()] = i128::from(weight);
        }
        if let Reduced::Leads(lead) = rows.reduce(vector, amounts)? {
            let index = plan.len();
            let label = quantity.map_or(
                AdmissionRow::Conserved(ConservedRowId { index }),
                AdmissionRow::Total,
            );
            plan.push(AdmissionTotal {
                replaces: RowId::at(lead),
                terms,
                label,
            });
        }
    }
    Some(plan)
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
            let burn = Proportions::<Glucose>::using(Space::Interstitium)
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
        type Placed = (usize, Vec<(usize, i64)>, AdmissionRow);
        let plan: Vec<Placed> = (body.admission_totals().iter())
            .map(|total| {
                let terms = total.terms.iter().map(|&(row, weight)| (row.index(), weight));
                (total.replaces.index(), terms.collect(), total.label)
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
