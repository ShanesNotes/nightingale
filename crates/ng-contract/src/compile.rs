use crate::body::{
    Binding, Block, Bound, Code, Column, ColumnId, CompiledBody, Context, GuardLayout, Instance, Layout,
    ModeLayout, ParameterId, ParameterSlot, Program, ProgramId, Receipt, Row, RowId, RowKind, Sparsity,
    Target, UnknownId,
};
use crate::conservation::conserved_basis;
use crate::graph::Digraph;
use crate::identity::{Canonical, Seal};
use crate::law::{
    End, Family, FragmentData, GuardDeclaration, Input, KernelData, ModeDeclaration, Obligation, Op,
    ParameterContext, ParameterDeclaration, Selector, SlotDeclaration, SlotKind,
};
use crate::quantity::dimension;
use crate::scope::{PoolRef, WaterBoundary};
use crate::{
    AdmissibleSet, AdmissionError, Anatomy, Boundary, BoundaryDirection, Class, CompileDefect, LawName,
    ModeValue, Network, Owner, ParameterKey, Predicate, RateClass, Schema, Space, Water,
};

pub(crate) fn compile(
    schema: Schema,
    issued: Vec<&'static str>,
    mut fragments: Vec<FragmentData>,
) -> Result<CompiledBody, AdmissionError> {
    fragments.sort_by_key(|fragment| fragment.owner);
    check_fragments(&issued, &fragments, &schema)?;
    check_boundaries(&schema)?;
    let declarations = declarations(&fragments)?;
    let unissued = FragmentData::empty(Boundary::CARD);
    let owed = obligations(&fragments, &schema, &unissued)?;
    check_guards(&fragments)?;
    check_volume(&schema, &owed, &fragments)?;

    let rows = Rows::lay_out(&schema, &fragments);
    let reaches = Reaches::of(&schema.anatomy);
    let places = Places {
        rows: &rows,
        reaches: &reaches,
    };
    let placement = place(&owed, &places)?;
    check_orphans(&rows, &placement.columns)?;
    let guards = guards(&fragments, &places)?;
    let (parameters, instances, guards) = bind(placement.instances, guards, &declarations);
    let (reads, rates) = reads(&rows.rows, &placement.columns, &placement.programs, &instances);
    let modes = modes(&fragments);
    match_each_mode(&rows.rows, &placement.programs, &instances, &modes)?;
    let solved_by = solved_by(&rows.rows, &reads)?;
    let blocks = blocks(&solved_by, &reads, &rates);
    let basis = conserved_basis(rows.amounts.len(), &placement.columns);
    Ok(CompiledBody::seal(Layout {
        schema,
        rows: rows.rows,
        columns: placement.columns,
        programs: placement.programs,
        instances,
        parameters,
        sparsity: sparsity(reads),
        blocks,
        modes,
        guards,
        basis,
    }))
}

fn check_fragments(
    issued: &[&'static str],
    fragments: &[FragmentData],
    schema: &Schema,
) -> Result<(), CompileDefect> {
    let added = |owner: &&'static str| fragments.iter().any(|fragment| fragment.owner == *owner);
    if let Some(&owner) = issued.iter().filter(|owner| !added(owner)).min() {
        return Err(CompileDefect::MissingFragment { owner });
    }
    fragments
        .iter()
        .try_for_each(|fragment| check_names(fragment, schema))
}

fn check_names(fragment: &FragmentData, schema: &Schema) -> Result<(), CompileDefect> {
    let named = |local| LawName {
        owner: fragment.owner,
        local,
    };
    let mut names: Vec<&'static str> = (fragment.slots.iter().map(|slot| slot.name))
        .chain(fragment.modes.iter().map(|mode| mode.name))
        .chain(fragment.families.iter().map(|family| family.name()))
        .chain(
            (schema.boundaries.iter())
                .filter(|_| fragment.owner == Boundary::CARD)
                .map(|boundary| boundary.name),
        )
        .collect();
    names.sort_unstable();
    if let Some(pair) = names.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(CompileDefect::NamedTwice { law: named(pair[0]) });
    }
    let mut bad: Vec<&'static str> = (fragment.modes.iter())
        .filter(|mode| {
            let mut values = mode.values.clone();
            values.sort_unstable();
            values.is_empty() || values.windows(2).any(|pair| pair[0] == pair[1])
        })
        .map(|mode| mode.name)
        .collect();
    bad.sort_unstable();
    match bad.first() {
        Some(&name) => Err(CompileDefect::BadModeVariable {
            variable: named(name),
        }),
        None => Ok(()),
    }
}

fn check_boundaries(schema: &Schema) -> Result<(), CompileDefect> {
    let pools: Vec<PoolRef> = schema
        .pools()
        .map(|(site, space)| PoolRef { site, space })
        .collect();
    match schema
        .boundaries
        .iter()
        .find(|boundary| !pools.contains(&boundary.pool))
    {
        Some(boundary) => Err(CompileDefect::BoundaryEndpoint {
            law: LawName {
                owner: Boundary::CARD,
                local: boundary.name,
            },
        }),
        None => Ok(()),
    }
}

fn declarations(fragments: &[FragmentData]) -> Result<Vec<&ParameterDeclaration>, CompileDefect> {
    let mut all: Vec<&ParameterDeclaration> = fragments
        .iter()
        .flat_map(|fragment| &fragment.parameters)
        .collect();
    all.sort_by_key(|declared| declared.key);
    let differ = |a: &ParameterDeclaration, b: &ParameterDeclaration| {
        a.context != b.context || a.dimension != b.dimension || a.set != b.set
    };
    if let Some(pair) = all
        .windows(2)
        .find(|pair| pair[0].key == pair[1].key && differ(pair[0], pair[1]))
    {
        return Err(CompileDefect::ParameterConflict { key: pair[0].key });
    }
    all.dedup_by_key(|declared| declared.key);
    Ok(all)
}

struct Owed<'f> {
    name: LawName,
    fragment: &'f FragmentData,
    serves: Serves<'f>,
    laws: Vec<(Predicate, &'f KernelData)>,
}
#[derive(Clone, Copy)]
enum Serves<'f> {
    Slot,
    Family(Family),
    Boundary(&'f WaterBoundary),
}

fn obligations<'f>(
    fragments: &'f [FragmentData],
    schema: &'f Schema,
    unissued: &'f FragmentData,
) -> Result<Vec<Owed<'f>>, CompileDefect> {
    let mut owed = Vec::new();
    let boundary = (fragments.iter())
        .find(|fragment| fragment.owner == Boundary::CARD)
        .unwrap_or(unissued);
    for water in &schema.boundaries {
        owed.push(Owed::gather(
            boundary,
            water.name,
            Serves::Boundary(water),
            Obligation::Boundary(water.name),
        ));
    }
    for fragment in fragments {
        for slot in &fragment.slots {
            owed.push(Owed::gather(
                fragment,
                slot.name,
                Serves::Slot,
                Obligation::Slot(slot.name),
            ));
        }
        for &family in &fragment.families {
            owed.push(Owed::gather(
                fragment,
                family.name(),
                Serves::Family(family),
                Obligation::Family(family),
            ));
        }
    }
    owed.sort_by_key(|owed| owed.name);
    for owed in &owed {
        owed.check_coverage()?;
    }
    Ok(owed)
}

impl<'f> Owed<'f> {
    fn gather(
        fragment: &'f FragmentData,
        local: &'static str,
        serves: Serves<'f>,
        obligation: Obligation,
    ) -> Self {
        let mut laws: Vec<(Predicate, &KernelData)> = (fragment.laws.iter())
            .filter(|law| law.serves == obligation)
            .map(|law| (predicate(fragment, law.kernel.modes), &law.kernel))
            .collect();
        laws.sort_by_key(|law| law.0);
        Self {
            name: LawName {
                owner: fragment.owner,
                local,
            },
            fragment,
            serves,
            laws,
        }
    }

    fn check_coverage(&self) -> Result<(), CompileDefect> {
        let law = self.name;
        let Some(&(_, first)) = self.laws.first() else {
            return Err(CompileDefect::OmittedLaw { law });
        };
        for (at, &(one, a)) in self.laws.iter().enumerate() {
            if let Some(&(other, _)) = self.laws[at + 1..]
                .iter()
                .find(|(_, b)| overlap(a.modes, b.modes))
            {
                return Err(CompileDefect::SecondOwner {
                    law,
                    first: one,
                    second: other,
                });
            }
        }
        if let Selector::Only { variable, .. } = first.modes {
            let mode = &self.fragment.modes[variable];
            let covered = |value| {
                self.laws
                    .iter()
                    .any(|law| law.1.modes == Selector::Only { variable, value })
            };
            let uncovered = (0..mode.values.len()).filter(|&value| !covered(value));
            if let Some(value) = uncovered.min_by_key(|&value| mode.values[value]) {
                return Err(CompileDefect::UncoveredMode {
                    law,
                    mode: ModeValue {
                        variable: mode.name,
                        value: mode.values[value],
                    },
                });
            }
        }
        Ok(())
    }
}

fn overlap(a: Selector, b: Selector) -> bool {
    match (a, b) {
        (
            Selector::Only { variable, value },
            Selector::Only {
                variable: other,
                value: then,
            },
        ) => variable != other || value == then,
        _ => true,
    }
}

fn predicate(fragment: &FragmentData, selector: Selector) -> Predicate {
    match selector {
        Selector::Every => Predicate::EveryMode,
        Selector::Only { variable, value } => {
            let mode = &fragment.modes[variable];
            Predicate::Only(ModeValue {
                variable: mode.name,
                value: mode.values[value],
            })
        }
    }
}

fn check_guards(fragments: &[FragmentData]) -> Result<(), CompileDefect> {
    let holds = |guard: &GuardDeclaration| match guard.zero.modes {
        Selector::Every => true,
        Selector::Only { variable, value } => variable == guard.variable && value == guard.leaves,
    };
    for fragment in fragments {
        let mut outside: Vec<ModeValue> = (fragment.guards.iter())
            .filter(|guard| !holds(guard))
            .map(|guard| {
                let mode = &fragment.modes[guard.variable];
                ModeValue {
                    variable: mode.name,
                    value: mode.values[guard.leaves],
                }
            })
            .collect();
        outside.sort_unstable();
        if let Some(first) = outside.first() {
            return Err(CompileDefect::GuardOutsideMode {
                variable: LawName {
                    owner: fragment.owner,
                    local: first.variable,
                },
                leaves: first.value,
            });
        }
    }
    Ok(())
}

fn check_volume(schema: &Schema, owed: &[Owed], fragments: &[FragmentData]) -> Result<(), CompileDefect> {
    if schema.partial_volume.is_some() {
        return Ok(());
    }
    let reads_volume = |kernel: &KernelData| kernel.code.iter().any(|op| matches!(op, Op::Volume(_)));
    let laws = (owed.iter())
        .filter(|law| law.laws.iter().any(|(_, kernel)| reads_volume(kernel)))
        .map(|law| law.name);
    let guards = fragments.iter().flat_map(|fragment| {
        (fragment.guards.iter())
            .filter(|guard| reads_volume(&guard.zero))
            .map(|guard| LawName {
                owner: fragment.owner,
                local: fragment.modes[guard.variable].name,
            })
    });
    match laws.chain(guards).min() {
        Some(law) => Err(CompileDefect::MissingPartialVolume { law }),
        None => Ok(()),
    }
}

struct Rows {
    rows: Vec<Row>,
    amounts: Vec<PoolRef>,
    slots: Vec<(LawName, usize)>,
}
impl Rows {
    fn lay_out(schema: &Schema, fragments: &[FragmentData]) -> Self {
        let left_out = |space| schema.left_out.iter().any(|&(out, _)| out == space);
        let amounts: Vec<PoolRef> = (schema.pools())
            .filter(|&(_, space)| !left_out(space))
            .map(|(site, space)| PoolRef { site, space })
            .collect();
        let mut slots: Vec<(SlotKind, LawName, AdmissibleSet, &String)> = (fragments.iter())
            .flat_map(|fragment| {
                fragment.slots.iter().map(|slot| {
                    let SlotDeclaration {
                        name,
                        kind,
                        dimension,
                        set,
                    } = slot;
                    let name = LawName {
                        owner: fragment.owner,
                        local: name,
                    };
                    (*kind, name, *set, dimension)
                })
            })
            .collect();
        slots.sort_unstable_by_key(|slot| (slot.0, slot.1));
        let amount_rows = amounts.iter().map(|pool| {
            let kind = RowKind::Amount {
                site: pool.site,
                space: pool.space,
            };
            (kind, AdmissibleSet::Nonnegative, dimension::<Water>())
        });
        let slot_rows = slots.iter().map(|&(kind, name, set, dimension)| {
            let kind = match kind {
                SlotKind::Memory => RowKind::Memory(name),
                SlotKind::Algebraic => RowKind::Algebraic(name),
            };
            (kind, set, dimension.clone())
        });
        let rows = (amount_rows.chain(slot_rows).enumerate())
            .map(|(at, (kind, admissible, dimension))| Row {
                unknown: UnknownId::at(at),
                kind,
                admissible,
                dimension,
            })
            .collect();
        let mut index: Vec<(LawName, usize)> = (slots.iter().enumerate())
            .map(|(at, slot)| (slot.1, amounts.len() + at))
            .collect();
        index.sort_unstable();
        Self {
            rows,
            amounts,
            slots: index,
        }
    }
    fn amount(&self, pool: PoolRef) -> Option<usize> {
        self.amounts.binary_search(&pool).ok()
    }
    fn slot(&self, name: LawName) -> usize {
        self.slots[self.slots.partition_point(|entry| entry.0 < name)].1
    }
}

struct Reach {
    context: Context,
    ends: [PoolRef; 2],
}
struct Reaches {
    circulation: Vec<Reach>,
    starling: Vec<Reach>,
}
impl Reaches {
    fn of(anatomy: &Anatomy) -> Self {
        let vascular = |site: crate::SiteId| PoolRef {
            site,
            space: if site.class() == Class::CapillaryBed {
                Space::Plasma
            } else {
                Space::Lumen
            },
        };
        let circulation = (anatomy.vascular())
            .map(|(from, to)| Reach {
                context: Context::Edge { from, to },
                ends: [vascular(from), vascular(to)],
            })
            .collect();
        let starling = (anatomy.sites())
            .filter(|site| site.class() == Class::CapillaryBed)
            .map(|site| Reach {
                context: Context::Site(site),
                ends: [Space::Plasma, Space::Interstitium].map(|space| PoolRef { site, space }),
            })
            .collect();
        Self {
            circulation,
            starling,
        }
    }
    fn of_family(&self, family: Family) -> &[Reach] {
        match family {
            Family::Circulation => &self.circulation,
            Family::Starling => &self.starling,
        }
    }
    fn pool(&self, family: Family, end: End, context: Context) -> PoolRef {
        let reach = (self.of_family(family).iter())
            .find(|reach| reach.context == context)
            .expect("a kernel reads a route's amounts only where a route of its own context applies");
        reach.ends[end as usize]
    }
}

struct Placement {
    columns: Vec<Column>,
    programs: Vec<Program>,
    instances: Vec<Placed>,
}
struct Placed {
    program: ProgramId,
    context: Context,
    target: Target,
    addresses: Vec<Address>,
}
#[derive(Clone, Copy)]
enum Address {
    Parameter(ParameterKey, Binding),
    Unknown(UnknownId),
    Boundary(&'static str),
}

struct Places<'a> {
    rows: &'a Rows,
    reaches: &'a Reaches,
}

fn place(owed: &[Owed], places: &Places) -> Result<Placement, CompileDefect> {
    let Places { rows, reaches } = *places;
    let mut placement = Placement {
        columns: Vec::new(),
        programs: Vec::new(),
        instances: Vec::new(),
    };
    for law in owed {
        let targets = match law.serves {
            Serves::Slot => vec![(Context::Body, Target::Row(RowId::at(rows.slot(law.name))))],
            Serves::Boundary(water) => {
                let target = Target::Column(ColumnId::at(placement.columns.len()));
                placement.columns.push(exterior(law.name, water, rows)?);
                vec![(Context::Body, target)]
            }
            Serves::Family(family) => {
                let mut targets = Vec::new();
                for reach in reaches.of_family(family) {
                    targets.push((
                        reach.context,
                        Target::Column(ColumnId::at(placement.columns.len())),
                    ));
                    placement.columns.push(column(law.name, reach, rows)?);
                }
                targets
            }
        };
        for &(_, kernel) in &law.laws {
            let program = ProgramId::at(placement.programs.len());
            placement.programs.push(Program {
                id: program,
                law: law.name,
                code: code(law.fragment, kernel),
            });
            for &(context, target) in &targets {
                placement.instances.push(Placed {
                    program,
                    context,
                    target,
                    addresses: addresses(law.name, kernel, context, places)?,
                });
            }
        }
    }
    Ok(placement)
}

fn column(law: LawName, reach: &Reach, rows: &Rows) -> Result<Column, CompileDefect> {
    let [from, to] = reach.ends;
    let mut terms = Vec::new();
    if from != to {
        for (pool, coefficient) in [(from, -1), (to, 1)] {
            let row = rows.amount(pool).ok_or(left_out(law, pool))?;
            terms.push((RowId::at(row), coefficient));
        }
        terms.sort_unstable();
    }
    Ok(Column {
        terms,
        origin: law,
        context: reach.context,
        receipt: None,
    })
}

fn exterior(law: LawName, water: &WaterBoundary, rows: &Rows) -> Result<Column, CompileDefect> {
    let row = rows.amount(water.pool).ok_or(left_out(law, water.pool))?;
    let coefficient = match water.direction {
        BoundaryDirection::IntoBody => 1,
        BoundaryDirection::OutOfBody => -1,
    };
    Ok(Column {
        terms: vec![(RowId::at(row), coefficient)],
        origin: law,
        context: Context::Body,
        receipt: Some(Receipt {
            boundary: water.name,
            coefficient: -coefficient,
        }),
    })
}

fn left_out(law: LawName, pool: PoolRef) -> CompileDefect {
    CompileDefect::LeftOutTouched {
        law,
        site: pool.site,
        space: pool.space,
    }
}

fn code(fragment: &FragmentData, kernel: &KernelData) -> Code {
    let KernelData {
        id,
        code,
        dimension,
        modes,
        rate,
    } = kernel;
    let mut validity: Vec<_> = (fragment.validity.iter())
        .filter(|(kernel, _)| kernel == id)
        .map(|&(_, range)| range)
        .collect();
    validity.sort_unstable();
    Code {
        ops: code.clone(),
        dimension: dimension.clone(),
        predicate: predicate(fragment, *modes),
        rate: *rate,
        validity,
    }
}

fn addresses(
    law: LawName,
    kernel: &KernelData,
    context: Context,
    places: &Places,
) -> Result<Vec<Address>, CompileDefect> {
    let Places { rows, reaches } = *places;
    let inputs = kernel.code.iter().filter_map(|op| match op {
        Op::Input(input) => Some(*input),
        _ => None,
    });
    let site = |end, from, to| if end == End::From { from } else { to };
    inputs
        .map(|input| {
            Ok(match (input, context) {
                (Input::Memory(local) | Input::Algebraic(local), _) => {
                    let slot = LawName {
                        owner: law.owner,
                        local,
                    };
                    Address::Unknown(UnknownId::at(rows.slot(slot)))
                }
                (Input::Amount(family, end), _) => {
                    let pool = reaches.pool(family, end, context);
                    Address::Unknown(UnknownId::at(rows.amount(pool).ok_or(left_out(law, pool))?))
                }
                (Input::Drive(name), _) => Address::Boundary(name),
                (Input::Parameter(key), _) => Address::Parameter(key, Binding::Body),
                (Input::SiteParameter(key), Context::Site(at)) => Address::Parameter(key, Binding::Site(at)),
                (Input::EdgeParameter(key), Context::Edge { from, to }) => {
                    Address::Parameter(key, Binding::Edge { from, to })
                }
                (Input::EndParameter(key, end), Context::Edge { from, to }) => {
                    Address::Parameter(key, Binding::Site(site(end, from, to)))
                }
                (Input::SiteParameter(_) | Input::EdgeParameter(_) | Input::EndParameter(..), _) => {
                    unreachable!("a kernel reads a site or edge parameter only in a context of its kind")
                }
            })
        })
        .collect()
}

fn check_orphans(rows: &Rows, columns: &[Column]) -> Result<(), CompileDefect> {
    let mut moved = vec![false; rows.amounts.len()];
    for &(row, _) in columns.iter().flat_map(|column| &column.terms) {
        moved[row.index()] = true;
    }
    match rows.amounts.iter().zip(&moved).find(|(_, moved)| !**moved) {
        Some((pool, _)) => Err(CompileDefect::OrphanPool {
            site: pool.site,
            space: pool.space,
        }),
        None => Ok(()),
    }
}

fn guards(
    fragments: &[FragmentData],
    places: &Places,
) -> Result<Vec<(GuardLayout, Vec<Address>)>, CompileDefect> {
    let mut guards = Vec::new();
    for fragment in fragments {
        for guard in &fragment.guards {
            let GuardDeclaration {
                zero,
                crossing,
                variable,
                leaves,
                enters,
            } = guard;
            let mode = &fragment.modes[*variable];
            let variable = LawName {
                owner: fragment.owner,
                local: mode.name,
            };
            let layout = GuardLayout {
                variable,
                leaves: mode.values[*leaves],
                enters: mode.values[*enters],
                crossing: *crossing,
                code: code(fragment, zero),
                inputs: Vec::new(),
            };
            let addresses = addresses(variable, zero, Context::Body, places)?;
            guards.push((layout, addresses));
        }
    }
    Ok(guards)
}

fn bind(
    placed: Vec<Placed>,
    guards: Vec<(GuardLayout, Vec<Address>)>,
    declarations: &[&ParameterDeclaration],
) -> (Vec<ParameterSlot>, Vec<Instance>, Vec<GuardLayout>) {
    let mut wanted: Vec<(ParameterKey, Binding)> = (placed.iter().map(|placed| &placed.addresses))
        .chain(guards.iter().map(|guard| &guard.1))
        .flatten()
        .filter_map(|address| match *address {
            Address::Parameter(key, binding) => Some((key, binding)),
            Address::Unknown(_) | Address::Boundary(_) => None,
        })
        .collect();
    wanted.sort_unstable();
    wanted.dedup();
    let parameters = (wanted.iter())
        .map(|&(key, binding)| {
            let ParameterDeclaration {
                key: _,
                context,
                dimension,
                set,
            } = declarations[declarations.partition_point(|declared| declared.key < key)];
            ParameterSlot {
                key,
                binding,
                default: match context {
                    ParameterContext::Body => None,
                    ParameterContext::Site(class) => Some(Binding::Class(*class)),
                    ParameterContext::Edge | ParameterContext::End => {
                        Some(Binding::Network(Network::Vascular))
                    }
                },
                admissible: *set,
                dimension: dimension.clone(),
            }
        })
        .collect();
    let resolve = |address: &Address| match *address {
        Address::Parameter(key, binding) => Bound::Parameter(ParameterId::at(
            wanted.partition_point(|entry| *entry < (key, binding)),
        )),
        Address::Unknown(unknown) => Bound::Unknown(unknown),
        Address::Boundary(boundary) => Bound::Boundary(boundary),
    };
    let instances = (placed.into_iter())
        .map(|placed| {
            let inputs: Vec<Bound> = placed.addresses.iter().map(resolve).collect();
            let mut parameters = Vec::new();
            for input in &inputs {
                if let Bound::Parameter(parameter) = *input
                    && !parameters.contains(&parameter)
                {
                    parameters.push(parameter);
                }
            }
            Instance {
                program: placed.program,
                context: placed.context,
                target: placed.target,
                parameters,
                inputs,
            }
        })
        .collect();
    let mut guards: Vec<GuardLayout> = (guards.into_iter())
        .map(|(mut layout, addresses)| {
            layout.inputs = addresses.iter().map(resolve).collect();
            layout
        })
        .collect();
    guards.sort_by_cached_key(|guard| {
        let mut out = Canonical::new("guard");
        guard.seal(&mut out);
        out.digest()
    });
    (parameters, instances, guards)
}

fn reads(
    rows: &[Row],
    columns: &[Column],
    programs: &[Program],
    instances: &[Instance],
) -> (Vec<Vec<UnknownId>>, Vec<Vec<RateClass>>) {
    let mut reads: Vec<Vec<UnknownId>> = vec![Vec::new(); rows.len()];
    let mut rates: Vec<Vec<RateClass>> = vec![Vec::new(); rows.len()];
    for instance in instances {
        let unknowns: Vec<UnknownId> = (instance.inputs.iter())
            .filter_map(|input| match *input {
                Bound::Unknown(unknown) => Some(unknown),
                Bound::Parameter(_) | Bound::Boundary(_) => None,
            })
            .collect();
        let defined: Vec<RowId> = match instance.target {
            Target::Row(row) => vec![row],
            Target::Column(column) => columns[column.index()].terms.iter().map(|term| term.0).collect(),
        };
        for row in defined {
            reads[row.index()].extend(&unknowns);
            rates[row.index()].push(programs[instance.program.index()].code.rate);
        }
    }
    for (at, row) in rows.iter().enumerate() {
        if !matches!(row.kind, RowKind::Algebraic(_)) {
            reads[at].push(UnknownId::at(at));
        }
        reads[at].sort_unstable();
        reads[at].dedup();
    }
    (reads, rates)
}

fn sparsity(reads: Vec<Vec<UnknownId>>) -> Sparsity {
    let mut offsets = vec![0];
    offsets.extend(reads.iter().scan(0, |total, row| {
        *total += row.len();
        Some(*total)
    }));
    Sparsity {
        offsets,
        unknowns: reads.concat(),
    }
}

fn algebraic_laws(rows: &[Row]) -> Vec<LawName> {
    (rows.iter())
        .filter_map(|row| match row.kind {
            RowKind::Algebraic(law) => Some(law),
            _ => None,
        })
        .collect()
}

fn matching(adjacency: &[Vec<usize>], rows: &[usize]) -> Result<Vec<Option<usize>>, usize> {
    let mut matched = vec![None; adjacency.len()];
    for &row in rows {
        if !augment(adjacency, row, &mut vec![false; adjacency.len()], &mut matched) {
            return Err(row);
        }
    }
    Ok(matched)
}

fn solved_by(rows: &[Row], reads: &[Vec<UnknownId>]) -> Result<Vec<RowId>, CompileDefect> {
    let algebraic = algebraic_laws(rows);
    let first = rows.len() - algebraic.len();
    let adjacency: Vec<Vec<usize>> = (first..rows.len())
        .map(|row| {
            (reads[row].iter())
                .filter_map(|unknown| unknown.index().checked_sub(first))
                .collect()
        })
        .collect();
    let every: Vec<usize> = (0..algebraic.len()).collect();
    let matched = matching(&adjacency, &every)
        .map_err(|row| CompileDefect::StructuralSingularity { law: algebraic[row] })?;
    let mut solved_by: Vec<RowId> = (0..rows.len()).map(RowId::at).collect();
    for (unknown, row) in matched.iter().enumerate() {
        if let Some(row) = row {
            solved_by[first + unknown] = RowId::at(first + row);
        }
    }
    Ok(solved_by)
}

struct AlgebraicLaw {
    row: usize,
    reads: Vec<usize>,
    holds: Option<(LawName, &'static str)>,
}

fn laws_on_algebraic_rows(first: usize, programs: &[Program], instances: &[Instance]) -> Vec<AlgebraicLaw> {
    (instances.iter())
        .filter_map(|instance| match instance.target {
            Target::Row(row) if row.index() >= first => {
                let program = &programs[instance.program.index()];
                let mut reads: Vec<usize> = (instance.inputs.iter())
                    .filter_map(|input| match *input {
                        Bound::Unknown(unknown) => unknown.index().checked_sub(first),
                        Bound::Parameter(_) | Bound::Boundary(_) => None,
                    })
                    .collect();
                reads.sort_unstable();
                reads.dedup();
                let holds = match program.code.predicate {
                    Predicate::EveryMode => None,
                    Predicate::Only(mode) => {
                        let variable = LawName {
                            owner: program.law.owner,
                            local: mode.variable,
                        };
                        Some((variable, mode.value))
                    }
                };
                Some(AlgebraicLaw {
                    row: row.index() - first,
                    reads,
                    holds,
                })
            }
            Target::Row(_) | Target::Column(_) => None,
        })
        .collect()
}

/// A variable's values that change what its rows read: one value stands for each distinct set of reads.
fn values_with_distinct_reads(
    laws: &[&AlgebraicLaw],
    variable: LawName,
    values: &[&'static str],
) -> Vec<&'static str> {
    let reads_under = |value: &'static str| -> Vec<(usize, &[usize])> {
        (laws.iter())
            .filter(|law| law.holds == Some((variable, value)))
            .map(|law| (law.row, law.reads.as_slice()))
            .collect()
    };
    let mut distinct: Vec<&'static str> = Vec::new();
    for &value in values {
        if !distinct
            .iter()
            .any(|&kept| reads_under(kept) == reads_under(value))
        {
            distinct.push(value);
        }
    }
    distinct
}

/// A law that does not hold cannot stand in for one that does, so the algebraic rows are matched in each
/// combination of the mode values their laws hold in. Rows that share no unknown and no mode variable have
/// independent matchings, and only values that change a row's reads change a matching, so the work
/// follows the body's coupling and not the product of its mode variables.
fn match_each_mode(
    rows: &[Row],
    programs: &[Program],
    instances: &[Instance],
    modes: &[ModeLayout],
) -> Result<(), CompileDefect> {
    let algebraic = algebraic_laws(rows);
    let count = algebraic.len();
    let laws = laws_on_algebraic_rows(rows.len() - count, programs, instances);
    let mut variables: Vec<LawName> = (laws.iter())
        .filter_map(|law| law.holds.map(|holds| holds.0))
        .collect();
    variables.sort_unstable();
    variables.dedup();
    let mut edges = Vec::new();
    for law in &laws {
        let unknowns = law.reads.iter().map(|&unknown| count + unknown);
        let variable = law
            .holds
            .map(|holds| 2 * count + variables.partition_point(|name| *name < holds.0));
        for node in unknowns.chain(variable) {
            edges.push((law.row, node));
            edges.push((node, law.row));
        }
    }
    let part = Digraph::new(2 * count + variables.len(), &edges).components();
    let mut parts: Vec<Vec<usize>> = vec![Vec::new(); part.len()];
    for row in 0..count {
        parts[part[row]].push(row);
    }
    parts.retain(|members| !members.is_empty());
    parts.sort_unstable();
    for members in &parts {
        let laws: Vec<&AlgebraicLaw> = (laws.iter())
            .filter(|law| members.binary_search(&law.row).is_ok())
            .collect();
        let cases: Vec<(LawName, Vec<&'static str>)> = (variables.iter())
            .filter(|&&name| {
                laws.iter()
                    .any(|law| law.holds.is_some_and(|holds| holds.0 == name))
            })
            .map(|&name| {
                let values = &modes[modes.partition_point(|mode| mode.variable < name)].values;
                (name, values_with_distinct_reads(&laws, name, values))
            })
            .collect();
        let mut chosen = vec![0; cases.len()];
        loop {
            let holds = |law: &&&AlgebraicLaw| match law.holds {
                None => true,
                Some((variable, value)) => {
                    let at = cases.partition_point(|case| case.0 < variable);
                    cases[at].1[chosen[at]] == value
                }
            };
            let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); count];
            for law in laws.iter().filter(holds) {
                adjacency[law.row].extend(&law.reads);
            }
            matching(&adjacency, members)
                .map_err(|row| CompileDefect::StructuralSingularity { law: algebraic[row] })?;
            let Some(at) = (0..chosen.len())
                .rev()
                .find(|&at| chosen[at] + 1 < cases[at].1.len())
            else {
                break;
            };
            chosen[at] += 1;
            chosen[at + 1..].fill(0);
        }
    }
    Ok(())
}

fn augment(adjacency: &[Vec<usize>], row: usize, seen: &mut [bool], matched: &mut [Option<usize>]) -> bool {
    for &unknown in &adjacency[row] {
        if seen[unknown] {
            continue;
        }
        seen[unknown] = true;
        let free = match matched[unknown] {
            None => true,
            Some(other) => augment(adjacency, other, seen, matched),
        };
        if free {
            matched[unknown] = Some(row);
            return true;
        }
    }
    false
}

fn blocks(solved_by: &[RowId], reads: &[Vec<UnknownId>], rates: &[Vec<RateClass>]) -> Vec<Block> {
    let edges: Vec<(usize, usize)> = (reads.iter().enumerate())
        .flat_map(|(row, read)| {
            read.iter()
                .map(move |unknown| (row, solved_by[unknown.index()].index()))
        })
        .collect();
    let part = Digraph::new(reads.len(), &edges).components();
    let mut solves = vec![0; reads.len()];
    for (unknown, row) in solved_by.iter().enumerate() {
        solves[row.index()] = unknown;
    }
    let count = part.iter().max().map_or(0, |last| last + 1);
    (0..count)
        .map(|at| {
            let rows: Vec<usize> = (0..reads.len()).filter(|&row| part[row] == at).collect();
            let mut unknowns: Vec<UnknownId> = rows.iter().map(|&row| UnknownId::at(solves[row])).collect();
            unknowns.sort_unstable();
            let rate = (rows.iter().flat_map(|&row| &rates[row]))
                .copied()
                .min_by_key(|rate| (rate.maximum_ns(), rate.band()))
                .expect("coverage and the orphan rule give every row at least one law");
            Block {
                rows: rows.into_iter().map(RowId::at).collect(),
                unknowns,
                rate,
            }
        })
        .collect()
}

fn modes(fragments: &[FragmentData]) -> Vec<ModeLayout> {
    let mut modes: Vec<ModeLayout> = (fragments.iter())
        .flat_map(|fragment| {
            fragment.modes.iter().map(|ModeDeclaration { name, values }| {
                let mut values = values.clone();
                values.sort_unstable();
                ModeLayout {
                    variable: LawName {
                        owner: fragment.owner,
                        local: name,
                    },
                    values,
                }
            })
        })
        .collect();
    modes.sort_by_key(|mode| mode.variable);
    modes
}
