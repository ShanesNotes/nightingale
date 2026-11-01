use crate::body::{
    Binding, Block, Bound, Code, Column, ColumnId, CompiledBody, Context, ForcingLayout, GuardLayout,
    HazardLayout, Instance, Layout, ModeLayout, ParameterId, ParameterSlot, Program, ProgramId, Receipt, Row,
    RowId, RowKind, Slot, Sparsity, StreamLayout, Target, UnknownId,
};
use crate::chance::key_word;
use crate::conservation::conserved_basis;
use crate::elimination::Elimination;
use crate::graph::Digraph;
use crate::identity::{Canonical, Seal};
use crate::law::{
    End, Family, FragmentData, GuardDeclaration, Input, KernelData, ModeDeclaration, Obligation, Op, Over,
    ParameterContext, ParameterDeclaration, Selector, SlotDeclaration, SlotKind, Source, Where,
};
use crate::scope::{ENCLOSURE_PRESSURE, HeldCard, PoolRef, RouteBoundary};
use crate::{
    AdmissibleSet, AdmissionError, Anatomy, Boundary, BoundaryDirection, Class, CompileDefect, Compliance,
    LawName, ModeValue, Network, Owner, ParameterKey, Predicate, RateClass, Schema, SiteId, Sites, Space,
    Species,
};

pub(crate) fn compile(
    schema: Schema,
    issued: Vec<&'static str>,
    mut held: Vec<HeldCard>,
    mut fragments: Vec<FragmentData>,
) -> Result<CompiledBody, AdmissionError> {
    fragments.sort_by_key(|fragment| fragment.owner);
    lift_flows(&mut fragments, &schema);
    held.sort_by_key(|card| card.card);
    check_fragments(&issued, &fragments, &schema, &held)?;
    check_boundaries(&schema)?;
    let declarations = declarations(&fragments)?;
    let unissued = FragmentData::empty(Boundary::CARD);
    let owed = obligations(&fragments, &schema, &held, &unissued)?;
    check_guards(&fragments)?;
    check_chance(&fragments)?;
    check_volume(&schema, &owed, &fragments)?;

    let reaches = Reaches::of(&schema, &fragments);
    let rows = Rows::lay_out(&schema, &fragments, &reaches, &held);
    let chance = ChanceNames::of(&fragments);
    let places = Places {
        rows: &rows,
        reaches: &reaches,
        chance: &chance,
        anatomy: &schema.anatomy,
        schema: &schema,
        fragments: &fragments,
        held: &held,
    };
    let placement = place(&owed, &places)?;
    check_holds(&rows, &schema, &held)?;
    check_orphans(&rows, &placement.columns, &schema, &held)?;
    let modes = modes(&fragments, &places);
    let guards = guards(&fragments, &places, &modes)?;
    let forced = forcing_parameters(&fragments);
    let (parameters, instances, guards) = bind(placement.instances, guards, &forced, &declarations);
    let closed = closed_rows(&fragments, &rows);
    let (reads, rates) = reads(
        &rows.rows,
        &placement.columns,
        &placement.programs,
        &instances,
        &closed,
    );
    let hazards = hazards(&fragments, &rows, &chance, &modes);
    let instances = marked(instances, &placement.programs, &modes, &hazards)?;
    let (streams, forcings) = chance.laid_out(&fragments, &parameters);
    match_each_mode(&rows.rows, &placement.programs, &instances, &modes, &closed)?;
    let solved_by = solved_by(&rows.rows, &reads)?;
    let blocks = blocks(&solved_by, &reads, &rates);
    let species: Vec<Species> = rows.amounts.iter().map(|pool| pool.species).collect();
    let read_by = |column: usize| {
        let target = Target::Column(ColumnId::at(column));
        (instances.iter())
            .filter(|instance| instance.target == target)
            .any(|instance| instance.inputs.iter().any(|input| input.unknown().is_some()))
    };
    let column_reads: Vec<bool> = (0..placement.columns.len()).map(read_by).collect();
    let basis = conserved_basis(&species, &placement.columns, &column_reads);
    let pivots = pivots(solved_by.clone(), &reads, &closed);
    let sparsity = sparsity(reads);
    let elimination = Elimination::plan(&sparsity.offsets, &sparsity.unknowns, &pivots);
    Ok(CompiledBody::seal(Layout {
        schema,
        rows: rows.rows,
        columns: placement.columns,
        programs: placement.programs,
        instances,
        parameters,
        sparsity,
        elimination,
        blocks,
        modes,
        guards,
        streams,
        forcings,
        hazards,
        basis,
        held,
    }))
}

fn check_fragments(
    issued: &[&'static str],
    fragments: &[FragmentData],
    schema: &Schema,
    held: &[HeldCard],
) -> Result<(), CompileDefect> {
    let added = |owner: &&'static str| fragments.iter().any(|fragment| fragment.owner == *owner);
    if let Some(&owner) = issued.iter().filter(|owner| !added(owner)).min() {
        return Err(CompileDefect::MissingFragment { owner });
    }
    fragments
        .iter()
        .try_for_each(|fragment| check_names(fragment, schema, held))
}

fn route_owner(held: &[HeldCard], route: &'static str) -> &'static str {
    (held.iter())
        .find(|card| card.supplies.contains(&route))
        .map_or(Boundary::CARD, |card| card.card)
}

/// Whether a route's columns come from one flow row: a valve's always, and any fluid route's in a body that carries
/// more than water, where several columns share the rate. A route of water alone is its one column.
fn flows(family: Family, schema: &Schema) -> bool {
    family == Family::Valves || (family.fluid() && !schema.carried.is_empty())
}

/// Turns each flowing route's law into the definition of its flow row, once the schema says which routes flow.
fn lift_flows(fragments: &mut [FragmentData], schema: &Schema) {
    for fragment in fragments {
        for law in &mut fragment.laws {
            if let Obligation::Family(family) = law.serves
                && flows(family, schema)
            {
                law.serves = Obligation::Defined(family.name());
                law.kernel = law.kernel.clone().defining(family.name());
            }
        }
        for family in fragment.families.clone() {
            if flows(family, schema) && family != Family::Valves {
                fragment.owe_flow(family);
            }
        }
    }
}

fn check_names(fragment: &FragmentData, schema: &Schema, held: &[HeldCard]) -> Result<(), CompileDefect> {
    let named = |local| LawName {
        owner: fragment.owner,
        local,
    };
    let mut names: Vec<&'static str> = (fragment.slots.iter().map(|slot| slot.name))
        .chain(fragment.modes.iter().map(|mode| mode.name))
        .chain(fragment.streams.iter().copied())
        .chain(fragment.forcings.iter().map(|forcing| forcing.name))
        .chain(
            (fragment.families.iter())
                .filter(|family| !flows(**family, schema))
                .map(|family| family.name()),
        )
        .chain(
            (schema.boundaries.iter())
                .filter(|boundary| route_owner(held, boundary.name) == fragment.owner)
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
        .map(|(site, space, species)| PoolRef { site, space, species })
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
    obligation: Obligation,
    laws: Vec<(Predicate, &'f KernelData)>,
}
#[derive(Clone, Copy)]
enum Serves<'f> {
    Slot(Over),
    Family(Family),
    Boundary(&'f RouteBoundary),
    /// One body row's other laws, one instance at every site of a set.
    Sum(Sites),
}

fn obligations<'f>(
    fragments: &'f [FragmentData],
    schema: &'f Schema,
    held: &[HeldCard],
    unissued: &'f FragmentData,
) -> Result<Vec<Owed<'f>>, CompileDefect> {
    let unheld = (schema.anatomy.held_enclosures())
        .map(|(_, card)| card)
        .filter(|card| !held.iter().any(|held| held.card == *card))
        .min();
    if let Some(owner) = unheld {
        let local = ENCLOSURE_PRESSURE;
        return Err(CompileDefect::OmittedLaw {
            law: LawName { owner, local },
        });
    }
    let mut owed = Vec::new();
    for water in &schema.boundaries {
        let owner = route_owner(held, water.name);
        let fragment = (fragments.iter())
            .find(|fragment| fragment.owner == owner)
            .unwrap_or(unissued);
        owed.push(Owed::gather(
            fragment,
            water.name,
            Serves::Boundary(water),
            Obligation::Boundary(water.name),
        ));
    }
    for fragment in fragments {
        for slot in &fragment.slots {
            let obligation = match slot.kind {
                SlotKind::Defined => Obligation::Defined(slot.name),
                SlotKind::Memory | SlotKind::Algebraic => Obligation::Slot(slot.name),
            };
            owed.push(Owed::gather(
                fragment,
                slot.name,
                Serves::Slot(slot.over),
                obligation,
            ));
        }
        for &family in fragment.families.iter().filter(|family| !flows(**family, schema)) {
            owed.push(Owed::gather(
                fragment,
                family.name(),
                Serves::Family(family),
                Obligation::Family(family),
            ));
        }
        for law in &fragment.laws {
            if let Obligation::Sum(name, set) = law.serves
                && !owed.iter().any(|known| known.obligation == law.serves)
            {
                owed.push(Owed::gather(fragment, name, Serves::Sum(set), law.serves));
            }
        }
    }
    let unclaimed = (fragments.iter())
        .flat_map(|fragment| fragment.laws.iter().map(|law| (fragment.owner, law.serves)))
        .filter(|&(owner, serves)| {
            !(owed.iter()).any(|owed| owed.name.owner == owner && owed.obligation == serves)
        })
        .map(|(owner, serves)| LawName {
            owner,
            local: serves.local(),
        })
        .min();
    if let Some(law) = unclaimed {
        return Err(CompileDefect::UnclaimedLaw { law });
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
            obligation,
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

fn check_chance(fragments: &[FragmentData]) -> Result<(), CompileDefect> {
    for fragment in fragments {
        let named = |local| LawName {
            owner: fragment.owner,
            local,
        };
        let mut streams = fragment.streams.clone();
        streams.sort_unstable();
        for &stream in &streams {
            let guards = (fragment.hazards.iter()).filter(|hazard| hazard.stream == stream);
            match guards.count() {
                0 => {
                    return Err(CompileDefect::UndrawnStream {
                        stream: named(stream),
                    });
                }
                1 => {}
                _ => {
                    return Err(CompileDefect::SecondGuardOnStream {
                        stream: named(stream),
                    });
                }
            }
        }
        let mut integrals: Vec<&'static str> =
            (fragment.hazards.iter()).map(|hazard| hazard.integral).collect();
        integrals.sort_unstable();
        if let Some(pair) = integrals.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(CompileDefect::SecondGuardOnIntegral {
                integral: named(pair[0]),
            });
        }
        for &integral in &integrals {
            let slot = fragment.slots.iter().find(|slot| slot.name == integral);
            if slot.map(|slot| slot.set) != Some(AdmissibleSet::Nonnegative) {
                return Err(CompileDefect::HazardSet {
                    integral: named(integral),
                });
            }
        }
        let mut forcings: Vec<&'static str> =
            (fragment.forcings.iter()).map(|forcing| forcing.name).collect();
        forcings.sort_unstable();
        for &forcing in &forcings {
            let read = Op::Input(Input::Forcing(forcing));
            if !(fragment.laws.iter()).any(|law| law.kernel.code.contains(&read)) {
                return Err(CompileDefect::UnreadForcing {
                    forcing: named(forcing),
                });
            }
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

/// Every place a row, a mode variable or a law stands.
fn places_of(
    anatomy: &Anatomy,
    reaches: &Reaches,
    schema: &Schema,
    owner: &'static str,
    over: Over,
) -> Vec<Context> {
    match over {
        Over::Body => vec![Context::Body],
        Over::Every(set) => anatomy.in_set(set).into_iter().map(Context::Site).collect(),
        Over::HeldEnclosures => (anatomy.held_enclosures())
            .filter(|&(_, card)| card == owner)
            .map(|(site, _)| Context::Site(site))
            .collect(),
        Over::Route(family) => reaches
            .of_family(family)
            .iter()
            .map(|reach| reach.context)
            .collect(),
        Over::WherePool(set, space, species) => (anatomy.in_set(set).into_iter())
            .filter(|&site| schema.pools().any(|pool| pool == (site, space, species)))
            .map(Context::Site)
            .collect(),
    }
}

struct Rows {
    rows: Vec<Row>,
    amounts: Vec<PoolRef>,
    slots: Vec<(Slot, usize)>,
    defined: Vec<LawName>,
}
impl Rows {
    fn lay_out(schema: &Schema, fragments: &[FragmentData], reaches: &Reaches, held: &[HeldCard]) -> Self {
        let left_out = |space| schema.left_out.iter().any(|&(out, _)| out == space);
        let amounts: Vec<PoolRef> = (schema.pools())
            .filter(|&(_, space, _)| !left_out(space))
            .map(|(site, space, species)| PoolRef { site, space, species })
            .collect();
        let mut slots: Vec<(SlotKind, Slot, AdmissibleSet, &String)> = (fragments.iter())
            .flat_map(|fragment| {
                fragment.slots.iter().flat_map(|slot| {
                    let SlotDeclaration {
                        name,
                        kind,
                        dimension,
                        set,
                        over,
                    } = slot;
                    let law = LawName {
                        owner: fragment.owner,
                        local: name,
                    };
                    let places = places_of(&schema.anatomy, reaches, schema, fragment.owner, *over);
                    places
                        .into_iter()
                        .map(move |at| (*kind, Slot { law, at }, *set, dimension))
                })
            })
            .collect();
        let rate = crate::quantity::dimension::<crate::Rate<crate::Water>>();
        for boundary in schema.boundaries.iter().filter(|boundary| boundary.drains) {
            let law = LawName {
                owner: route_owner(held, boundary.name),
                local: boundary.name,
            };
            slots.push((SlotKind::Defined, Slot::from(law), AdmissibleSet::Finite, &rate));
        }
        slots.sort_unstable_by_key(|slot| (slot.0, slot.1));
        let amount_rows = amounts.iter().map(|pool| {
            let kind = RowKind::Amount {
                site: pool.site,
                space: pool.space,
                species: pool.species,
            };
            (kind, AdmissibleSet::Nonnegative, pool.species.dimension())
        });
        let slot_rows = slots.iter().map(|&(kind, slot, set, dimension)| {
            let kind = match kind {
                SlotKind::Memory => RowKind::Memory(slot),
                SlotKind::Algebraic | SlotKind::Defined => RowKind::Algebraic(slot),
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
        let mut index: Vec<(Slot, usize)> = (slots.iter().enumerate())
            .map(|(at, slot)| (slot.1, amounts.len() + at))
            .collect();
        index.sort_unstable();
        let defined = (slots.iter())
            .filter(|slot| slot.0 == SlotKind::Defined)
            .map(|slot| slot.1.law)
            .collect();
        Self {
            rows,
            amounts,
            slots: index,
            defined,
        }
    }
    fn amount(&self, pool: PoolRef) -> Option<usize> {
        self.amounts.binary_search(&pool).ok()
    }
    /// A body row answers at every place; a row of a place answers at that place.
    fn slot(&self, law: LawName, at: Context) -> Option<usize> {
        let find = |at| {
            let found = self
                .slots
                .binary_search_by_key(&Slot { law, at }, |entry| entry.0);
            found.ok().map(|at| self.slots[at].1)
        };
        find(Context::Body).or_else(|| find(at))
    }
    fn defined(&self, law: LawName, at: Context) -> Option<usize> {
        self.defined.binary_search(&law).ok()?;
        self.slot(law, at)
    }
    fn declared(&self, law: LawName, at: Context) -> usize {
        self.slot(law, at)
            .expect("a slot a law serves or reads is laid out at every place it stands over")
    }
}

#[derive(Clone, Copy)]
enum Terminal {
    Pool(PoolRef),
    /// The far end of a route out of the body, named for its receipt.
    Exterior(&'static str),
}
struct Reach {
    context: Context,
    ends: [Terminal; 2],
}
impl Reach {
    fn pool(&self, end: End, species: Species) -> Option<PoolRef> {
        match self.ends[end as usize] {
            Terminal::Pool(pool) => Some(PoolRef { species, ..pool }),
            Terminal::Exterior(_) => None,
        }
    }
}
struct Reaches {
    routes: Vec<(Family, Vec<Reach>)>,
    pools: Vec<PoolRef>,
}
fn water(site: SiteId, space: Space) -> PoolRef {
    PoolRef {
        site,
        space,
        species: Species::Water,
    }
}
/// A vessel's blood or a bed's plasma, the space an edge of a flow network meets.
fn flowing(site: SiteId) -> PoolRef {
    let space = if site.class() == Class::CapillaryBed {
        Space::Plasma
    } else {
        Space::Lumen
    };
    water(site, space)
}
impl Reaches {
    /// A route stands wherever both its pools exist, so the schema's pools say where every law reaches.
    fn of(schema: &Schema, fragments: &[FragmentData]) -> Self {
        let mut pools: Vec<PoolRef> = (schema.pools())
            .map(|(site, space, species)| PoolRef { site, space, species })
            .collect();
        pools.sort_unstable();
        let families = fragments
            .iter()
            .flat_map(|fragment| fragment.families.iter().copied());
        let routes = families
            .map(|family| {
                let held = |reach: &Reach| {
                    reach.ends.iter().all(|end| match end {
                        Terminal::Pool(pool) => pools.binary_search(pool).is_ok(),
                        Terminal::Exterior(_) => true,
                    })
                };
                let mut reaches = reaches_of(family, schema);
                reaches.retain(held);
                (family, reaches)
            })
            .collect();
        Self { routes, pools }
    }
    fn of_family(&self, family: Family) -> &[Reach] {
        (self.routes.iter())
            .find(|route| route.0 == family)
            .map_or(&[], |route| route.1.as_slice())
    }
    fn reach(&self, family: Family, context: Context) -> &Reach {
        (self.of_family(family).iter())
            .find(|reach| reach.context == context)
            .expect("a kernel reads a route's amounts only where a route of its own context applies")
    }
    /// Every species both of a fluid reach's ends hold, water first, so no list of them exists to keep in step.
    fn carried(&self, reach: &Reach) -> Vec<Species> {
        let held = |pool: Option<PoolRef>| pool.is_some_and(|pool| self.pools.binary_search(&pool).is_ok());
        let every = [
            Species::Water,
            Species::Sodium,
            Species::Potassium,
            Species::Chloride,
            Species::Albumin,
            Species::Vasopressin,
            Species::Norepinephrine,
            Species::Acetylcholine,
        ];
        (every.into_iter())
            .filter(|&species| held(reach.pool(End::From, species)) && held(reach.pool(End::To, species)))
            .collect()
    }
}
/// The one table of what each family joins. A new family is one arm here and one port.
fn reaches_of(family: Family, schema: &Schema) -> Vec<Reach> {
    let anatomy = &schema.anatomy;
    let edge = |(from, to): (SiteId, SiteId), ends: [PoolRef; 2]| Reach {
        context: Context::Edge { from, to },
        ends: ends.map(Terminal::Pool),
    };
    let at = |site: SiteId, ends: [PoolRef; 2]| Reach {
        context: Context::Site(site),
        ends: ends.map(Terminal::Pool),
    };
    let blood = |pair: (SiteId, SiteId)| edge(pair, [flowing(pair.0), flowing(pair.1)]);
    let species_at = |site, space, species| PoolRef { site, space, species };
    let junctions = || anatomy.in_set(Sites::Class(Class::Junction)).into_iter();
    let store = |space: Space| {
        let found = schema.pools().find(|pool| pool.1 == space);
        found.map(|(site, space, species)| PoolRef { site, space, species })
    };
    match family {
        Family::Circulation => (anatomy.vascular())
            .filter(|&pair| !anatomy.is_valve(pair))
            .map(blood)
            .collect(),
        Family::Valves => (anatomy.vascular())
            .filter(|&pair| anatomy.is_valve(pair))
            .map(blood)
            .collect(),
        Family::Starling => (anatomy.in_set(Sites::Class(Class::CapillaryBed)).into_iter())
            .map(|bed| at(bed, [water(bed, Space::Plasma), water(bed, Space::Interstitium)]))
            .collect(),
        Family::Lymph => (anatomy.edges(Network::Lymph))
            .map(|pair| edge(pair, [water(pair.0, Space::Interstitium), flowing(pair.1)]))
            .collect(),
        Family::Filtration => (anatomy.edges(Network::Urinary))
            .filter(|pair| pair.0.class() == Class::CapillaryBed)
            .map(blood)
            .collect(),
        Family::Tubular => (anatomy.edges(Network::Urinary))
            .filter(|pair| pair.0.class() != Class::CapillaryBed)
            .map(blood)
            .collect(),
        Family::Reclaim(species) => (anatomy.edges(Network::Reclaim))
            .map(|pair| {
                let (from, to) = (flowing(pair.0), flowing(pair.1));
                edge(pair, [PoolRef { species, ..from }, PoolRef { species, ..to }])
            })
            .collect(),
        Family::Release(species) => junctions()
            .map(|site| {
                at(
                    site,
                    [
                        species_at(site, Space::Vesicle, species),
                        species_at(site, Space::Cleft, species),
                    ],
                )
            })
            .collect(),
        Family::Reuptake(species) => junctions()
            .map(|site| {
                at(
                    site,
                    [
                        species_at(site, Space::Cleft, species),
                        species_at(site, Space::Vesicle, species),
                    ],
                )
            })
            .collect(),
        Family::Binding(species) => (anatomy.sites())
            .filter(|site| matches!(site.class(), Class::Junction | Class::CapillaryBed))
            .map(|site| {
                let free = if site.class() == Class::Junction {
                    Space::Cleft
                } else {
                    Space::Plasma
                };
                at(
                    site,
                    [
                        species_at(site, free, species),
                        species_at(site, Space::Bound, species),
                    ],
                )
            })
            .collect(),
        Family::StrokeWork => (anatomy.in_set(Sites::Blood).into_iter())
            .filter_map(|site| {
                Some(Reach {
                    context: Context::Site(site),
                    ends: [
                        Terminal::Exterior("chemical energy"),
                        Terminal::Pool(store(Space::Elastic)?),
                    ],
                })
            })
            .collect(),
        Family::Dissipation => (store(Space::Elastic).zip(store(Space::Thermal)).into_iter())
            .map(|(elastic, thermal)| Reach {
                context: Context::Body,
                ends: [Terminal::Pool(elastic), Terminal::Pool(thermal)],
            })
            .collect(),
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
    Draw(usize),
    Forcing(usize),
    Gauge,
    Height(u64),
}

struct Places<'a> {
    rows: &'a Rows,
    reaches: &'a Reaches,
    chance: &'a ChanceNames,
    anatomy: &'a Anatomy,
    schema: &'a Schema,
    fragments: &'a [FragmentData],
    held: &'a [HeldCard],
}

struct ChanceNames {
    streams: Vec<LawName>,
    forcings: Vec<LawName>,
}
impl ChanceNames {
    fn of(fragments: &[FragmentData]) -> Self {
        let named = |owner, local| LawName { owner, local };
        let mut streams: Vec<LawName> = (fragments.iter())
            .flat_map(|fragment| fragment.streams.iter().map(|name| named(fragment.owner, *name)))
            .collect();
        let mut forcings: Vec<LawName> = (fragments.iter())
            .flat_map(|fragment| {
                fragment
                    .forcings
                    .iter()
                    .map(|forcing| named(fragment.owner, forcing.name))
            })
            .collect();
        streams.sort_unstable();
        forcings.sort_unstable();
        Self { streams, forcings }
    }
    fn stream(&self, name: LawName) -> usize {
        self.streams.partition_point(|known| *known < name)
    }
    fn forcing(&self, name: LawName) -> usize {
        self.forcings.partition_point(|known| *known < name)
    }
    fn laid_out(
        &self,
        fragments: &[FragmentData],
        parameters: &[ParameterSlot],
    ) -> (Vec<StreamLayout>, Vec<ForcingLayout>) {
        let streams = (self.streams.iter())
            .map(|&name| StreamLayout {
                name,
                key: key_word(name),
            })
            .collect();
        let forcings = (fragments.iter())
            .flat_map(|fragment| {
                fragment
                    .forcings
                    .iter()
                    .map(move |forcing| (fragment.owner, forcing))
            })
            .map(|(owner, forcing)| {
                let name = LawName {
                    owner,
                    local: forcing.name,
                };
                let slot = |key| {
                    let at = parameters.iter().position(|slot| slot.key == key);
                    ParameterId::at(at.expect("bind lays out both of a forcing's parameters"))
                };
                ForcingLayout {
                    name,
                    key: key_word(name),
                    period: slot(forcing.period),
                    amplitude: slot(forcing.amplitude),
                }
            });
        let mut forcings: Vec<ForcingLayout> = forcings.collect();
        forcings.sort_unstable_by_key(|forcing| forcing.name);
        (streams, forcings)
    }
}

/// No kernel reads these, so no address lays them out.
fn forcing_parameters(fragments: &[FragmentData]) -> Vec<(ParameterKey, Binding)> {
    (fragments.iter())
        .flat_map(|fragment| &fragment.forcings)
        .flat_map(|forcing| [forcing.period, forcing.amplitude])
        .map(|key| (key, Binding::Body))
        .collect()
}

fn hazards(
    fragments: &[FragmentData],
    rows: &Rows,
    chance: &ChanceNames,
    modes: &[ModeLayout],
) -> Vec<HazardLayout> {
    let mut hazards: Vec<HazardLayout> = Vec::new();
    for fragment in fragments {
        for hazard in &fragment.hazards {
            let named = |local| LawName {
                owner: fragment.owner,
                local,
            };
            let declared = &fragment.modes[hazard.variable];
            let variable = Slot::from(named(declared.name));
            let at = modes.partition_point(|mode| mode.variable < variable);
            let value = |index: usize| {
                let wanted = declared.values[index];
                let sorted = &modes[at].values;
                sorted.partition_point(|known| *known < wanted)
            };
            hazards.push(HazardLayout {
                integral: RowId::at(rows.declared(named(hazard.integral), Context::Body)),
                stream: chance.stream(named(hazard.stream)),
                variable: at,
                leaves: value(hazard.leaves),
                enters: value(hazard.enters),
            });
        }
    }
    hazards.sort_unstable_by_key(|hazard| hazard.stream);
    hazards
}

fn marked(
    mut instances: Vec<Instance>,
    programs: &[Program],
    modes: &[ModeLayout],
    hazards: &[HazardLayout],
) -> Result<Vec<Instance>, CompileDefect> {
    for instance in &mut instances {
        let program = &programs[instance.program.index()];
        if let Predicate::Only(mode) = program.code.predicate {
            let law = LawName {
                owner: program.law.owner,
                local: mode.variable,
            };
            // A route's mode answers at its own edge, a whole-body mode at every place.
            let found = |at| {
                modes
                    .binary_search_by_key(&Slot { law, at }, |known| known.variable)
                    .ok()
            };
            let at = found(Context::Body).or_else(|| found(instance.context));
            let at = at.ok_or(CompileDefect::ModeOfAnotherPlace { law })?;
            let value = modes[at].values.partition_point(|known| *known < mode.value);
            instance.mode = Some((at, value));
        }
        instance.rectified = matches!(instance.target, Target::Row(row)
            if hazards.iter().any(|hazard| hazard.integral == row));
    }
    Ok(instances)
}

fn place(owed: &[Owed], places: &Places) -> Result<Placement, CompileDefect> {
    let Places {
        rows,
        reaches,
        anatomy,
        schema,
        ..
    } = *places;
    let mut placement = Placement {
        columns: Vec::new(),
        programs: Vec::new(),
        instances: Vec::new(),
    };
    for law in owed {
        let targets = match law.serves {
            Serves::Slot(over) => places_of(anatomy, reaches, schema, law.name.owner, over)
                .into_iter()
                .map(|at| {
                    let row = RowId::at(rows.declared(law.name, at));
                    (at, Target::Row(row))
                })
                .collect(),
            Serves::Sum(set) => {
                let row = RowId::at(rows.declared(law.name, Context::Body));
                let sites = anatomy.in_set(set).into_iter();
                sites
                    .map(|site| (Context::Site(site), Target::Row(row)))
                    .collect()
            }
            Serves::Boundary(water) if water.drains => {
                let row = RowId::at(rows.declared(law.name, Context::Body));
                vec![(Context::Body, Target::Row(row))]
            }
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
            if let Serves::Boundary(water) = law.serves {
                let carried = kernel.code.iter().find_map(|op| match op {
                    Op::Input(Input::Read(Source::Drive(route), _)) if *route != water.name => Some(*route),
                    _ => None,
                });
                if let Some(route) = carried {
                    let law = law.name;
                    return Err(CompileDefect::DriveOfAnotherRoute { law, route });
                }
            }
            let defined;
            let mut kernel = kernel;
            if let Serves::Boundary(water) = law.serves
                && water.drains
            {
                defined = kernel.clone().defining(water.name);
                kernel = &defined;
            }
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
    fluid_columns(&mut placement, places)?;
    Ok(placement)
}

/// Every column of every fluid route, written here and not by a law: water reads the flow row, and each species a
/// reach carries reads the flow and both ends' amounts through one `Op::Upwind` program, less its reflection. A drain
/// does the same at its one pool. So a law states one flow, a species added to the schema needs no line in any
/// system crate, and each column is an ordinary instance the Jacobian and the witness already read.
fn fluid_columns(placement: &mut Placement, places: &Places) -> Result<(), CompileDefect> {
    let Places {
        rows,
        reaches,
        schema,
        fragments,
        held,
        ..
    } = *places;
    for fragment in fragments {
        for &family in fragment.families.iter().filter(|family| flows(**family, schema)) {
            let law = LawName {
                owner: fragment.owner,
                local: family.name(),
            };
            let class = (fragment.laws.iter())
                .find(|known| known.serves.local() == family.name())
                .map_or(RateClass::UNMOVED, |known| known.kernel.rate);
            for reach in reaches.of_family(family) {
                let flow = Address::Unknown(UnknownId::at(rows.declared(law, reach.context)));
                for species in reaches.carried(reach) {
                    let ends = reach.ends.map(|end| match end {
                        Terminal::Pool(pool) => Terminal::Pool(PoolRef { species, ..pool }),
                        other => other,
                    });
                    let one = Reach {
                        context: reach.context,
                        ends,
                    };
                    let target = Target::Column(ColumnId::at(placement.columns.len()));
                    placement.columns.push(column(law, &one, rows)?);
                    let amount = |end, species| -> Result<Address, CompileDefect> {
                        let pool = reach
                            .pool(end, species)
                            .ok_or(CompileDefect::NoPoolRead { law })?;
                        let row = rows.amount(pool).ok_or(left_out(law, pool))?;
                        Ok(Address::Unknown(UnknownId::at(row)))
                    };
                    let reflection = (fragment.reflections.iter())
                        .find(|known| known.0 == family && known.1 == species)
                        .map(|known| known.2);
                    let mut addresses = vec![flow];
                    if species != Species::Water {
                        addresses.extend([
                            amount(End::From, species)?,
                            amount(End::From, Species::Water)?,
                            amount(End::To, species)?,
                            amount(End::To, Species::Water)?,
                        ]);
                    }
                    if let (Some(key), Context::Site(site)) = (reflection, reach.context) {
                        addresses.push(Address::Parameter(key, Binding::Site(site)));
                    }
                    let end = |end, held| Source::Amount(family, end, held);
                    let ends = [
                        end(End::From, species),
                        end(End::From, Species::Water),
                        end(End::To, species),
                        end(End::To, Species::Water),
                    ];
                    let ops = carried_ops(Source::Flow(family), ends, species, reflection);
                    let program = fluid_program(placement, law, ops, class, species.dimension());
                    placement.instances.push(Placed {
                        program,
                        context: reach.context,
                        target,
                        addresses,
                    });
                }
            }
        }
    }
    for boundary in schema.boundaries.iter().filter(|boundary| boundary.drains) {
        let law = LawName {
            owner: route_owner(held, boundary.name),
            local: boundary.name,
        };
        let flow = Address::Unknown(UnknownId::at(rows.declared(law, Context::Body)));
        let spaces = (reaches.pools.iter())
            .filter(|pool| pool.site == boundary.pool.site && pool.space == boundary.pool.space);
        for pool in spaces {
            let target = Target::Column(ColumnId::at(placement.columns.len()));
            let row = rows.amount(*pool).ok_or(left_out(law, *pool))?;
            let water = rows.amount(boundary.pool).ok_or(left_out(law, boundary.pool))?;
            placement.columns.push(Column {
                terms: vec![(RowId::at(row), -1)],
                origin: law,
                context: Context::Body,
                receipt: Some(Receipt {
                    boundary: boundary.name,
                    coefficient: 1,
                }),
            });
            let (amount, solvent) = (
                Address::Unknown(UnknownId::at(row)),
                Address::Unknown(UnknownId::at(water)),
            );
            let addresses = if pool.species == Species::Water {
                vec![flow]
            } else {
                vec![flow, amount, solvent, amount, solvent]
            };
            let held = Source::RoutePool(boundary.name, pool.species);
            let solvent_read = Source::RoutePool(boundary.name, Species::Water);
            let ops = carried_ops(
                Source::Field(law),
                [held, solvent_read, held, solvent_read],
                pool.species,
                None,
            );
            let program = fluid_program(placement, law, ops, RateClass::UNMOVED, pool.species.dimension());
            placement.instances.push(Placed {
                program,
                context: Context::Body,
                target,
                addresses,
            });
        }
    }
    Ok(())
}

/// Water is the flow itself. A species takes its upstream share, and `sigma` of that stays behind.
fn carried_ops(
    flow: Source,
    ends: [Source; 4],
    species: Species,
    reflection: Option<ParameterKey>,
) -> Vec<Op> {
    let read = |source| Op::Input(Input::Read(source, Where::Here));
    if species == Species::Water {
        return vec![read(flow)];
    }
    let mut ops = vec![read(flow)];
    ops.extend(ends.map(read));
    ops.push(Op::Upwind([0, 1, 2, 3, 4]));
    if let Some(key) = reflection {
        ops.push(Op::Input(Input::SiteParameter(key)));
        ops.push(Op::Scale([5, 6]));
        ops.push(Op::Subtract([5, 7]));
    }
    ops
}

/// One program for each shape of carried column, so a route's columns share it as a law's instances do.
fn fluid_program(
    placement: &mut Placement,
    law: LawName,
    ops: Vec<Op>,
    rate: RateClass,
    dimension: String,
) -> ProgramId {
    let known = (placement.programs.iter()).find(|program| program.law == law && program.code.ops == ops);
    if let Some(program) = known {
        return program.id;
    }
    let id = ProgramId::at(placement.programs.len());
    placement.programs.push(Program {
        id,
        law,
        code: Code {
            ops,
            dimension,
            predicate: Predicate::EveryMode,
            rate,
            validity: Vec::new(),
            defines: false,
        },
    });
    id
}

fn column(law: LawName, reach: &Reach, rows: &Rows) -> Result<Column, CompileDefect> {
    let mut terms = Vec::new();
    let mut receipt = None;
    match reach.ends {
        [Terminal::Pool(from), Terminal::Pool(to)] if from != to => {
            for (pool, coefficient) in [(from, -1), (to, 1)] {
                let row = rows.amount(pool).ok_or(left_out(law, pool))?;
                terms.push((RowId::at(row), coefficient));
            }
            terms.sort_unstable();
        }
        [Terminal::Exterior(boundary), Terminal::Pool(to)] => {
            let row = rows.amount(to).ok_or(left_out(law, to))?;
            terms.push((RowId::at(row), 1));
            receipt = Some(Receipt {
                boundary,
                coefficient: -1,
            });
        }
        _ => {}
    }
    Ok(Column {
        terms,
        origin: law,
        context: reach.context,
        receipt,
    })
}

fn exterior(law: LawName, water: &RouteBoundary, rows: &Rows) -> Result<Column, CompileDefect> {
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
        defines,
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
        defines: *defines,
    }
}

/// The site a read lands on, from the kernel's place. `None` only for a read of the place itself.
fn landing(place: Where, context: Context, anatomy: &Anatomy) -> Option<SiteId> {
    let ends = match context {
        Context::Edge { from, to } => Some((from, to)),
        _ => None,
    };
    let here = match context {
        Context::Site(site) => Some(site),
        _ => None,
    };
    match place {
        Where::Here => here,
        Where::Up => ends.map(|end| end.0),
        Where::Down => ends.map(|end| end.1),
        Where::Junction => anatomy.junction(here.or(ends.map(|end| end.1))?),
        Where::Drain => anatomy.drain(here?),
        Where::Mark(landmark) => anatomy.landmark(landmark).map(|mark| mark.0),
        Where::Sensor(landmark) => anatomy.landmark(landmark).map(|mark| mark.1),
    }
}

fn addresses(
    law: LawName,
    kernel: &KernelData,
    context: Context,
    places: &Places,
) -> Result<Vec<Address>, CompileDefect> {
    let Places {
        rows,
        reaches,
        chance,
        anatomy,
        schema,
        held,
        ..
    } = *places;
    let inputs = kernel.code.iter().filter_map(|op| match op {
        Op::Input(input) => Some(*input),
        _ => None,
    });
    let site = |end, from, to| if end == End::From { from } else { to };
    let unknown = |row: usize| Address::Unknown(UnknownId::at(row));
    let read = |source: Source, place: Where| -> Result<Address, CompileDefect> {
        let landed = landing(place, context, anatomy);
        let at = landed.map_or(context, Context::Site);
        Ok(match source {
            Source::Field(_) if place == Where::Junction && context == Context::Body => {
                return Err(CompileDefect::NoPoolRead { law });
            }
            Source::Field(field) => match (rows.defined(field, at), place) {
                (Some(row), _) => unknown(row),
                // No nerve, or no receptor at the nerve's end, is no effect.
                (None, Where::Junction) => Address::Gauge,
                (None, _) => return Err(CompileDefect::UnemittedField { law, field }),
            },
            Source::Flow(family) => {
                let flow = LawName {
                    owner: law.owner,
                    local: family.name(),
                };
                unknown(rows.declared(flow, context))
            }
            Source::Amount(family, end, species) => {
                let reach = reaches.reach(family, context);
                let pool = reach
                    .pool(end, species)
                    .ok_or(CompileDefect::NoPoolRead { law })?;
                unknown(rows.amount(pool).ok_or(left_out(law, pool))?)
            }
            Source::Pool(species) => {
                let site = landed.ok_or(CompileDefect::NoPoolRead { law })?;
                let pool = PoolRef {
                    species,
                    ..flowing(site)
                };
                unknown(rows.amount(pool).ok_or(left_out(law, pool))?)
            }
            Source::RoutePool(route, species) => {
                let boundary = (schema.boundaries.iter()).find(|known| known.name == route);
                let base = match boundary {
                    Some(boundary) => Some(boundary.pool),
                    None => (reaches.of_family(Family::Dissipation).first())
                        .and_then(|reach| reach.pool(End::From, species)),
                };
                let pool = base.ok_or(CompileDefect::NoPoolRead { law })?;
                let pool = PoolRef { species, ..pool };
                unknown(rows.amount(pool).ok_or(left_out(law, pool))?)
            }
            Source::Drive(route) => {
                if route_owner(held, route) != law.owner {
                    return Err(CompileDefect::DriveOfAnotherRoute { law, route });
                }
                Address::Boundary(route)
            }
            Source::Extravascular => {
                let site = landed.ok_or(CompileDefect::NoPoolRead { law })?;
                match anatomy.encloser(site) {
                    None => Address::Gauge,
                    Some((space, Compliance::Held { card })) => {
                        let pressure = LawName {
                            owner: card,
                            local: ENCLOSURE_PRESSURE,
                        };
                        unknown(rows.declared(pressure, Context::Site(space)))
                    }
                    Some((space, _)) => return Err(CompileDefect::NoEnclosurePressure { law, space }),
                }
            }
        })
    };
    inputs
        .map(|input| {
            Ok(match (input, context) {
                (Input::Memory(local) | Input::Algebraic(local), _) => {
                    let slot = LawName {
                        owner: law.owner,
                        local,
                    };
                    unknown(rows.declared(slot, context))
                }
                (Input::Read(source, place), _) => read(source, place)?,
                (Input::Height(place), _) => {
                    let site = landing(place, context, anatomy).ok_or(CompileDefect::NoPoolRead { law })?;
                    Address::Height(anatomy.axial(site).to_bits())
                }
                (Input::Inflow, Context::Edge { from, .. }) => {
                    let into: Vec<(SiteId, SiteId)> = (anatomy.edges(Network::Urinary))
                        .filter(|edge| edge.1 == from)
                        .collect();
                    let [edge] = into[..] else {
                        return Err(CompileDefect::NoPoolRead { law });
                    };
                    let family = if edge.0.class() == Class::CapillaryBed {
                        Family::Filtration
                    } else {
                        Family::Tubular
                    };
                    let flow = LawName {
                        owner: law.owner,
                        local: family.name(),
                    };
                    let at = Context::Edge {
                        from: edge.0,
                        to: edge.1,
                    };
                    unknown(rows.declared(flow, at))
                }
                (Input::Draw(local), _) => Address::Draw(chance.stream(LawName {
                    owner: law.owner,
                    local,
                })),
                (Input::Forcing(local), _) => Address::Forcing(chance.forcing(LawName {
                    owner: law.owner,
                    local,
                })),
                (Input::Parameter(key), _) => Address::Parameter(key, Binding::Body),
                (Input::SiteParameter(key), Context::Site(at)) => Address::Parameter(key, Binding::Site(at)),
                (Input::EdgeParameter(key), Context::Edge { from, to }) => {
                    Address::Parameter(key, Binding::Edge { from, to })
                }
                (Input::EndParameter(key, end), Context::Edge { from, to }) => {
                    Address::Parameter(key, Binding::Site(site(end, from, to)))
                }
                (
                    Input::SiteParameter(_)
                    | Input::EdgeParameter(_)
                    | Input::EndParameter(..)
                    | Input::Inflow,
                    _,
                ) => {
                    unreachable!("a kernel reads a site or edge entry only in a context of its kind")
                }
            })
        })
        .collect()
}

fn holder(schema: &Schema, held: &[HeldCard], pool: PoolRef) -> Result<Option<&'static str>, CompileDefect> {
    let enclosure = match schema.anatomy.compliance(pool.site) {
        Some(Compliance::Held { card }) => Some(card),
        _ => None,
    };
    let declared = (held.iter())
        .filter(|card| card.holds.contains(&pool.space))
        .map(|card| card.card);
    let mut cards = enclosure.into_iter().chain(declared);
    let first = cards.next();
    match (first, cards.find(|&card| Some(card) != first)) {
        (Some(first), Some(second)) => Err(CompileDefect::PoolHeldTwice {
            cards: [first, second],
            site: pool.site,
            space: pool.space,
        }),
        _ => Ok(first),
    }
}

fn check_holds(rows: &Rows, schema: &Schema, held: &[HeldCard]) -> Result<(), CompileDefect> {
    let claims = (held.iter()).flat_map(|card| card.holds.iter().map(|&space| (card.card, space)));
    for (card, space) in claims {
        if !rows.amounts.iter().any(|pool| pool.space == space) {
            return Err(CompileDefect::HeldNothing { card, space });
        }
    }
    for (site, card) in schema.anatomy.held_enclosures() {
        if !rows.amounts.iter().any(|pool| pool.site == site) {
            let space = Space::Fluid;
            return Err(CompileDefect::HeldNothing { card, space });
        }
    }
    Ok(())
}

fn check_orphans(
    rows: &Rows,
    columns: &[Column],
    schema: &Schema,
    held: &[HeldCard],
) -> Result<(), CompileDefect> {
    let mut moved = vec![false; rows.amounts.len()];
    for &(row, _) in columns.iter().flat_map(|column| &column.terms) {
        moved[row.index()] = true;
    }
    for (pool, moved) in rows.amounts.iter().zip(moved) {
        let (site, space) = (pool.site, pool.space);
        match (holder(schema, held, *pool)?, moved) {
            (Some(card), true) => return Err(CompileDefect::HeldPoolMoved { card, site, space }),
            (None, false) => return Err(CompileDefect::OrphanPool { site, space }),
            _ => {}
        }
    }
    Ok(())
}

/// One instance per place of the guard's mode variable, in stable order by the hash of what it is.
fn guards(
    fragments: &[FragmentData],
    places: &Places,
    modes: &[ModeLayout],
) -> Result<Vec<(GuardLayout, Vec<Address>)>, CompileDefect> {
    let Places {
        rows,
        reaches,
        anatomy,
        schema,
        ..
    } = *places;
    let mut guards = Vec::new();
    for fragment in fragments {
        for guard in &fragment.guards {
            let GuardDeclaration {
                zero,
                crossing,
                variable,
                leaves,
                enters,
                resets,
            } = guard;
            let mode = &fragment.modes[*variable];
            let law = LawName {
                owner: fragment.owner,
                local: mode.name,
            };
            let sorted = |index: usize| {
                let mut values = mode.values.clone();
                values.sort_unstable();
                values.partition_point(|known| *known < mode.values[index])
            };
            for at in places_of(anatomy, reaches, schema, fragment.owner, mode.over) {
                let reset = resets.map(|local| {
                    let owner = fragment.owner;
                    RowId::at(rows.declared(LawName { owner, local }, at))
                });
                let variable = Slot { law, at };
                let layout = GuardLayout {
                    variable: modes.partition_point(|mode| mode.variable < variable),
                    leaves: sorted(*leaves),
                    enters: sorted(*enters),
                    crossing: *crossing,
                    code: code(fragment, zero),
                    inputs: Vec::new(),
                    resets: reset,
                };
                guards.push((layout, addresses(law, zero, at, places)?));
            }
        }
    }
    Ok(guards)
}

fn bind(
    placed: Vec<Placed>,
    guards: Vec<(GuardLayout, Vec<Address>)>,
    forced: &[(ParameterKey, Binding)],
    declarations: &[&ParameterDeclaration],
) -> (Vec<ParameterSlot>, Vec<Instance>, Vec<GuardLayout>) {
    let mut wanted: Vec<(ParameterKey, Binding)> = (placed.iter().map(|placed| &placed.addresses))
        .chain(guards.iter().map(|guard| &guard.1))
        .flatten()
        .filter_map(|address| match *address {
            Address::Parameter(key, binding) => Some((key, binding)),
            Address::Unknown(_)
            | Address::Boundary(_)
            | Address::Draw(_)
            | Address::Forcing(_)
            | Address::Gauge
            | Address::Height(_) => None,
        })
        .chain(forced.iter().copied())
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
                    ParameterContext::Site(Sites::Class(class)) => Some(Binding::Class(*class)),
                    ParameterContext::Site(Sites::Blood) => Some(Binding::Network(Network::Vascular)),
                    ParameterContext::Site(Sites::Urinary) => Some(Binding::Network(Network::Urinary)),
                    ParameterContext::Edge(network) => Some(Binding::Network(*network)),
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
        Address::Draw(stream) => Bound::Draw(stream),
        Address::Forcing(forcing) => Bound::Forcing(forcing),
        Address::Gauge => Bound::Gauge,
        Address::Height(axial) => Bound::Height(axial),
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
                mode: None,
                rectified: false,
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

/// Each row `close` declares and the unknown it closes, its own.
fn closed_rows(fragments: &[FragmentData], rows: &Rows) -> Vec<usize> {
    let names = fragments.iter().flat_map(|fragment| {
        let owner = fragment.owner;
        fragment.closed.iter().map(move |&local| LawName { owner, local })
    });
    names.map(|law| rows.declared(law, Context::Body)).collect()
}

fn reads(
    rows: &[Row],
    columns: &[Column],
    programs: &[Program],
    instances: &[Instance],
    closed: &[usize],
) -> (Vec<Vec<UnknownId>>, Vec<Vec<RateClass>>) {
    let mut reads: Vec<Vec<UnknownId>> = vec![Vec::new(); rows.len()];
    let mut rates: Vec<Vec<RateClass>> = vec![Vec::new(); rows.len()];
    for instance in instances {
        let unknowns: Vec<UnknownId> = (instance.inputs.iter()).filter_map(Bound::unknown).collect();
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
        // A closed row's unknown reaches it through a tangent. Its slot stays zero, and it is what the matching reads.
        if !matches!(row.kind, RowKind::Algebraic(_)) || closed.contains(&at) {
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
            RowKind::Algebraic(slot) => Some(slot.law),
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

/// Each unknown's pivot row. A closed row reads no unknown of its own, so it pivots on the amount it reads whose rate
/// reads that unknown, and that amount's row takes the unknown.
fn pivots(mut pivots: Vec<RowId>, reads: &[Vec<UnknownId>], closed: &[usize]) -> Vec<RowId> {
    for &row in closed {
        let unknown = (pivots.iter()).position(|pivot| pivot.index() == row);
        let unknown = UnknownId::at(unknown.expect("a closed row is matched to its unknown"));
        let carries = |amount: &&UnknownId| reads[amount.index()].contains(&unknown);
        if let Some(amount) = reads[row].iter().find(carries) {
            pivots[unknown.index()] = RowId::at(amount.index());
            pivots[amount.index()] = RowId::at(row);
        }
    }
    pivots
}

struct AlgebraicLaw {
    row: usize,
    reads: Vec<usize>,
    holds: Option<(LawName, &'static str)>,
}

fn laws_on_algebraic_rows(
    first: usize,
    programs: &[Program],
    instances: &[Instance],
    closed: &[usize],
) -> Vec<AlgebraicLaw> {
    (instances.iter())
        .filter_map(|instance| match instance.target {
            Target::Row(row) if row.index() >= first => {
                let program = &programs[instance.program.index()];
                let mut reads: Vec<usize> = (instance.inputs.iter())
                    .filter_map(Bound::unknown)
                    .filter_map(|unknown| unknown.index().checked_sub(first))
                    .collect();
                if closed.contains(&row.index()) {
                    reads.push(row.index() - first);
                }
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
    closed: &[usize],
) -> Result<(), CompileDefect> {
    let algebraic = algebraic_laws(rows);
    let count = algebraic.len();
    let laws = laws_on_algebraic_rows(rows.len() - count, programs, instances, closed);
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
    let part = Digraph::new(2 * count + variables.len(), &edges).strongly_connected_components();
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
                let values = &modes[modes.partition_point(|mode| mode.variable.law < name)].values;
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
    let part = Digraph::new(reads.len(), &edges).strongly_connected_components();
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
                .unwrap_or(RateClass::UNMOVED);
            Block {
                rows: rows.into_iter().map(RowId::at).collect(),
                unknowns,
                rate,
            }
        })
        .collect()
}

fn modes(fragments: &[FragmentData], places: &Places) -> Vec<ModeLayout> {
    let Places {
        reaches,
        anatomy,
        schema,
        ..
    } = *places;
    let mut modes: Vec<ModeLayout> = (fragments.iter())
        .flat_map(|fragment| {
            fragment
                .modes
                .iter()
                .flat_map(move |ModeDeclaration { name, values, over }| {
                    let mut values = values.clone();
                    values.sort_unstable();
                    let law = LawName {
                        owner: fragment.owner,
                        local: name,
                    };
                    let at = places_of(anatomy, reaches, schema, fragment.owner, *over);
                    at.into_iter().map(move |at| ModeLayout {
                        variable: Slot { law, at },
                        values: values.clone(),
                    })
                })
        })
        .collect();
    modes.sort_by_key(|mode| mode.variable);
    modes
}
