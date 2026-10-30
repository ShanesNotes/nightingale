use crate::body::{
    Binding, Block, Bound, Code, CompiledBody, Context, GuardLayout, Instance, Layout, ModeLayout,
    ParameterId, ParameterSlot, Program, ProgramId, Row, RowId, RowKind, Sparsity, Target, UnknownId,
};
use crate::graph::Digraph;
use crate::identity::{Canonical, Seal};
use crate::law::{
    FragmentData, GuardDeclaration, Input, KernelData, Obligation, Op, ParameterContext,
    ParameterDeclaration, Selector, SlotDeclaration, SlotKind,
};
use crate::{AdmissionError, CompileDefect, LawName, ModeValue, ParameterKey, Predicate, RateClass, Schema};

/// The passes run in the order of `CompileDefect`'s refusals that need no layout, then lay the body
/// out, so a body with one defect is refused for that defect and no other.
pub(crate) fn compile(
    schema: Schema,
    issued: Vec<&'static str>,
    mut fragments: Vec<FragmentData>,
) -> Result<CompiledBody, AdmissionError> {
    fragments.sort_by_key(|fragment| fragment.owner);
    let added = |owner: &&'static str| fragments.iter().any(|fragment| fragment.owner == *owner);
    if let Some(&owner) = issued.iter().filter(|owner| !added(owner)).min() {
        return Err(CompileDefect::MissingFragment { owner }.into());
    }
    for fragment in &fragments {
        check_names(fragment)?;
    }
    let declarations = declarations(&fragments)?;
    let owed = obligations(&fragments)?;
    check_guards(&fragments)?;

    let slots = slots(&fragments);
    let rows: Vec<Row> = slots
        .iter()
        .enumerate()
        .map(|(at, &(kind, name, slot))| Row {
            unknown: UnknownId::at(at),
            kind: match kind {
                SlotKind::Memory => RowKind::Memory(name),
                SlotKind::Algebraic => RowKind::Algebraic(name),
            },
            admissible: slot.set,
            dimension: slot.dimension.clone(),
        })
        .collect();
    let mut index: Vec<(LawName, usize)> = slots.iter().enumerate().map(|(at, slot)| (slot.1, at)).collect();
    index.sort_unstable();
    let row_of = |name: LawName| index[index.partition_point(|entry| entry.0 < name)].1;
    let unknown = |name: LawName| UnknownId::at(row_of(name));

    let mut programs = Vec::new();
    let mut placed = Vec::new();
    for law in &owed {
        for &(predicate, kernel) in &law.laws {
            let id = ProgramId::at(programs.len());
            programs.push(Program {
                id,
                law: law.name,
                code: code(law.fragment, predicate, kernel),
            });
            let (context, target) = match law.serves {
                Serves::Slot => (Context::Body, Target::Row(RowId::at(row_of(law.name)))),
            };
            placed.push(Placed {
                program: id,
                context,
                target,
                addresses: addresses(law.fragment.owner, kernel, &unknown),
            });
        }
    }
    let mut guards = Vec::new();
    for fragment in &fragments {
        for guard in &fragment.guards {
            let mode = &fragment.modes[guard.variable];
            let layout = GuardLayout {
                variable: LawName {
                    owner: fragment.owner,
                    local: mode.name,
                },
                leaves: mode.values[guard.leaves],
                enters: mode.values[guard.enters],
                crossing: guard.crossing,
                code: code(fragment, predicate(fragment, guard.zero.modes), &guard.zero),
                inputs: Vec::new(),
            };
            guards.push((layout, addresses(fragment.owner, &guard.zero, &unknown)));
        }
    }

    let mut wanted: Vec<(ParameterKey, Binding)> = (placed.iter().map(|placed| &placed.addresses))
        .chain(guards.iter().map(|guard| &guard.1))
        .flatten()
        .filter_map(|address| match *address {
            Address::Parameter(key, binding) => Some((key, binding)),
            Address::Unknown(_) => None,
        })
        .collect();
    wanted.sort_unstable();
    wanted.dedup();
    let parameters = wanted
        .iter()
        .map(|&(key, binding)| {
            let declared = declarations[declarations.partition_point(|declared| declared.key < key)];
            ParameterSlot {
                key,
                binding,
                default: match declared.context {
                    ParameterContext::Body => None,
                },
                admissible: declared.set,
                dimension: declared.dimension.clone(),
            }
        })
        .collect();
    let resolve = |address: &Address| match *address {
        Address::Parameter(key, binding) => Bound::Parameter(ParameterId::at(
            wanted.partition_point(|entry| *entry < (key, binding)),
        )),
        Address::Unknown(unknown) => Bound::Unknown(unknown),
    };
    let instances: Vec<Instance> = placed
        .into_iter()
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
    let mut guards: Vec<GuardLayout> = guards
        .into_iter()
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

    let mut reads: Vec<Vec<UnknownId>> = vec![Vec::new(); rows.len()];
    let mut rates: Vec<Vec<RateClass>> = vec![Vec::new(); rows.len()];
    for instance in &instances {
        let Target::Row(row) = instance.target;
        reads[row.index()].extend(instance.inputs.iter().filter_map(|input| match *input {
            Bound::Unknown(unknown) => Some(unknown),
            Bound::Parameter(_) => None,
        }));
        rates[row.index()].push(programs[instance.program.index()].code.rate);
    }
    for (at, row) in rows.iter().enumerate() {
        if !matches!(row.kind, RowKind::Algebraic(_)) {
            reads[at].push(UnknownId::at(at));
        }
        reads[at].sort_unstable();
        reads[at].dedup();
    }
    let solves = solve_order(&rows, &reads)?;
    let blocks = blocks(&solves, &reads, &rates);
    let mut offsets = vec![0];
    offsets.extend(reads.iter().scan(0, |total, row| {
        *total += row.len();
        Some(*total)
    }));
    let sparsity = Sparsity {
        offsets,
        unknowns: reads.concat(),
    };
    let mut modes: Vec<ModeLayout> = fragments
        .iter()
        .flat_map(|fragment| {
            fragment.modes.iter().map(|mode| ModeLayout {
                variable: LawName {
                    owner: fragment.owner,
                    local: mode.name,
                },
                values: mode.values.clone(),
            })
        })
        .collect();
    modes.sort_by_key(|mode| mode.variable);
    Ok(CompiledBody::seal(Layout {
        schema,
        rows,
        programs,
        instances,
        parameters,
        sparsity,
        blocks,
        modes,
        guards,
    }))
}

/// Slots, mode variables and route families share one namespace per owner, so a law's name names
/// one law.
fn check_names(fragment: &FragmentData) -> Result<(), CompileDefect> {
    let named = |local| LawName {
        owner: fragment.owner,
        local,
    };
    let mut names: Vec<&'static str> = (fragment.slots.iter().map(|slot| slot.name))
        .chain(fragment.modes.iter().map(|mode| mode.name))
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

/// Every parameter declaration, one per key: an identical repeat is the same parameter.
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

/// What a fragment owes and the laws it attached to it, by predicate.
struct Owed<'f> {
    name: LawName,
    fragment: &'f FragmentData,
    serves: Serves,
    laws: Vec<(Predicate, &'f KernelData)>,
}
enum Serves {
    Slot,
}

fn obligations(fragments: &[FragmentData]) -> Result<Vec<Owed<'_>>, CompileDefect> {
    let mut owed = Vec::new();
    for fragment in fragments {
        for slot in &fragment.slots {
            owed.push(Owed::gather(
                fragment,
                slot.name,
                Serves::Slot,
                Obligation::Slot(slot.name),
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
        serves: Serves,
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

    /// Exactly one law holds in each mode: none at all is an omitted law, two holding together is a
    /// second owner, and a value of the row's variable that no law holds in is uncovered.
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
            if let Some(value) = (0..mode.values.len()).find(|&value| !covered(value)) {
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

/// A guard fires on leaving a value, so its zero kernel must hold throughout that value.
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

/// Memory slots, then algebraic slots, each by law name.
fn slots(fragments: &[FragmentData]) -> Vec<(SlotKind, LawName, &SlotDeclaration)> {
    let mut slots: Vec<(SlotKind, LawName, &SlotDeclaration)> = (fragments.iter())
        .flat_map(|fragment| {
            fragment.slots.iter().map(|slot| {
                let name = LawName {
                    owner: fragment.owner,
                    local: slot.name,
                };
                (slot.kind, name, slot)
            })
        })
        .collect();
    slots.sort_unstable_by_key(|slot| (slot.0, slot.1));
    slots
}

fn code(fragment: &FragmentData, predicate: Predicate, kernel: &KernelData) -> Code {
    let mut validity: Vec<_> = (fragment.validity.iter())
        .filter(|(id, _)| *id == kernel.id)
        .map(|&(_, range)| range)
        .collect();
    validity.sort_unstable();
    Code {
        ops: kernel.code.clone(),
        dimension: kernel.dimension.clone(),
        predicate,
        rate: kernel.rate,
        validity,
    }
}

/// A program instance before the parameter layout exists.
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
}

/// What each input instruction of `kernel` reads, in instruction order.
fn addresses(
    owner: &'static str,
    kernel: &KernelData,
    unknown: &impl Fn(LawName) -> UnknownId,
) -> Vec<Address> {
    let inputs = kernel.code.iter().filter_map(|op| match op {
        Op::Input(input) => Some(*input),
        _ => None,
    });
    inputs
        .map(|input| match input {
            Input::Parameter(key) => Address::Parameter(key, Binding::Body),
            Input::Memory(local) | Input::Algebraic(local) => {
                Address::Unknown(unknown(LawName { owner, local }))
            }
        })
        .collect()
}

/// The row that determines each unknown: a differential row its own unknown, an algebraic row the
/// unknown a structural matching pairs it with (Kuhn's augmenting paths, rows in order).
fn solve_order(rows: &[Row], reads: &[Vec<UnknownId>]) -> Result<Vec<usize>, CompileDefect> {
    let algebraic: Vec<LawName> = (rows.iter())
        .filter_map(|row| match row.kind {
            RowKind::Algebraic(law) => Some(law),
            _ => None,
        })
        .collect();
    let first = rows.len() - algebraic.len();
    let adjacency: Vec<Vec<usize>> = (first..rows.len())
        .map(|row| {
            (reads[row].iter())
                .filter_map(|unknown| unknown.index().checked_sub(first))
                .collect()
        })
        .collect();
    let mut matched: Vec<Option<usize>> = vec![None; algebraic.len()];
    for (row, &law) in algebraic.iter().enumerate() {
        if !augment(&adjacency, row, &mut vec![false; algebraic.len()], &mut matched) {
            return Err(CompileDefect::StructuralSingularity { law });
        }
    }
    let mut solved_by: Vec<usize> = (0..rows.len()).collect();
    for (unknown, row) in matched.iter().enumerate() {
        if let Some(row) = row {
            solved_by[first + unknown] = first + row;
        }
    }
    Ok(solved_by)
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

/// Strongly connected parts of the graph from each row to the rows that determine what it reads.
/// Tarjan's algorithm numbers a part only after every part it reaches, so ascending numbers put each
/// block after the blocks it depends on.
fn blocks(solved_by: &[usize], reads: &[Vec<UnknownId>], rates: &[Vec<RateClass>]) -> Vec<Block> {
    let edges: Vec<(usize, usize)> = (reads.iter().enumerate())
        .flat_map(|(row, read)| read.iter().map(move |unknown| (row, solved_by[unknown.index()])))
        .collect();
    let part = Digraph::new(reads.len(), &edges).components();
    let mut solves = vec![0; reads.len()];
    for (unknown, &row) in solved_by.iter().enumerate() {
        solves[row] = unknown;
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
                .expect("coverage gives every row at least one law");
            Block {
                rows: rows.into_iter().map(RowId::at).collect(),
                unknowns,
                rate,
            }
        })
        .collect()
}
