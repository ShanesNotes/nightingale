use ng_contract::*;

fn schema() -> Result<Schema, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let at = Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?);
    map.site::<TissueSite>("body", at);
    Schema::new(map.finish()?)
}

/// The one defect a test adds to the valid body.
#[derive(Clone, Copy, PartialEq)]
enum Defect {
    None,
    UnattachedSlot,
    UnclosedAlgebraicSlot,
    SecondEveryModeRate,
    LawOnSecondVariable,
    UncoveredValue,
    GuardInOtherValue,
    UnaddedFragment,
    SlotNamedTwice,
    ModeVariableNamedLikeSlot,
    ParameterRedimensioned,
    ParameterResetToOtherSet,
    ParameterRepeated,
    NoModeValues,
    RepeatedModeValue,
    ResidualWithoutUnknown,
    ScopeIssuedTwice,
    TransitionToSameValue,
    TransitionToOtherVariable,
    ZeroRateInterval,
    ReversedValidityRange,
}

/// A filter relaxing toward a parameter, a gate with one law per value of a switch, a guard leaving
/// `on`, and a pressure closed by its residual. Every value is synthetic.
fn body(defect: Defect) -> Result<CompiledBody, AdmissionError> {
    assemble(schema()?, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        if defect == Defect::ScopeIssuedTwice {
            body.nervous()?;
        }
        let interval = if defect == Defect::ZeroRateInterval {
            0
        } else {
            1_000_000_000
        };
        let reflex = RateClass::new(RateBand::Reflex, interval)?;
        let target =
            laws.parameter::<Dimensionless>(ParameterKey::new("ns.target"), Admissible::unit_interval());
        let speed =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.speed"), Admissible::nonnegative());
        let reference = laws.parameter::<Pressure>(ParameterKey::new("ns.pressure"), Admissible::finite());
        let filter = laws.state::<Dimensionless>("filter", Admissible::unit_interval());
        let gate = laws.state::<Dimensionless>("gate", Admissible::unit_interval());
        let pressure = laws.algebraic::<Pressure>("pressure", Admissible::finite());
        let [on, off] = laws.modes("switch", ["on", "off"]);
        let valve = matches!(
            defect,
            Defect::LawOnSecondVariable | Defect::TransitionToOtherVariable
        )
        .then(|| laws.modes("valve", ["shut", "open"]));
        match defect {
            Defect::UnattachedSlot => {
                laws.state::<Dimensionless>("unattached", Admissible::finite());
            }
            Defect::UnclosedAlgebraicSlot => {
                laws.algebraic::<Pressure>("unclosed", Admissible::finite());
            }
            Defect::SlotNamedTwice => {
                laws.state::<Dimensionless>("filter", Admissible::finite());
            }
            Defect::ModeVariableNamedLikeSlot => {
                laws.modes("filter", ["low", "high"]);
            }
            Defect::ParameterRedimensioned => {
                laws.parameter::<Pressure>(ParameterKey::new("ns.target"), Admissible::unit_interval());
            }
            Defect::ParameterResetToOtherSet => {
                laws.parameter::<Dimensionless>(ParameterKey::new("ns.target"), Admissible::finite());
            }
            Defect::ParameterRepeated => {
                laws.parameter::<Dimensionless>(ParameterKey::new("ns.target"), Admissible::unit_interval());
            }
            Defect::NoModeValues => {
                laws.modes("empty", []);
            }
            Defect::RepeatedModeValue => {
                laws.modes("twice", ["level", "level"]);
            }
            _ => {}
        }
        let transition = match (defect, &valve) {
            (Defect::TransitionToSameValue, _) => on.to(&on)?,
            (Defect::TransitionToOtherVariable, Some([_, open])) => on.to(open)?,
            _ => on.to(&off)?,
        };

        let relax = |modes| {
            laws.kernel(|mut k| {
                let toward = k.parameter(&target);
                let now = k.state(&filter);
                let pace = k.parameter(&speed);
                let rate = k.relax(toward, now, pace);
                k.finish(rate, modes, reflex)
            })
        };
        let still = |modes| {
            laws.kernel(|mut k| {
                let zero = k.zero::<Rate<Dimensionless>>();
                k.finish(zero, modes, reflex)
            })
        };
        let gap = |modes| {
            laws.kernel(|mut k| {
                let actual = k.algebraic(&pressure);
                let wanted = k.parameter(&reference);
                let gap = k.subtract(actual, wanted);
                k.finish(gap, modes, reflex)
            })
        };
        let rate = relax(laws.every_mode());
        let again = (defect == Defect::SecondEveryModeRate).then(|| relax(laws.every_mode()));
        let when_on = still(on.only());
        let when_off = (defect != Defect::UncoveredValue).then(|| still(off.only()));
        let when_shut = (valve.as_ref())
            .filter(|_| defect == Defect::LawOnSecondVariable)
            .map(|[shut, _]| still(shut.only()));
        let residual = if defect == Defect::ResidualWithoutUnknown {
            laws.kernel(|mut k| {
                let zero = k.zero::<Pressure>();
                k.finish(zero, laws.every_mode(), reflex)
            })
        } else {
            gap(laws.every_mode())
        };
        let head = gap(if defect == Defect::GuardInOtherValue {
            off.only()
        } else {
            on.only()
        });

        let (lower, upper) = if defect == Defect::ReversedValidityRange {
            (1.0, -1.0)
        } else {
            (-1.0, 1.0)
        };
        let trusted = ValidityRange::new(
            Measure::new(lower)?,
            Measure::new(upper)?,
            Basis::Fixture("synthetic"),
        )?;
        laws.validity(&rate, trusted);
        laws.memory_rate(&filter, rate);
        for rate in [Some(when_on), when_off, when_shut].into_iter().flatten() {
            laws.memory_rate(&gate, rate);
        }
        if let Some(rate) = again {
            laws.memory_rate(&filter, rate);
        }
        laws.residual(&pressure, residual);
        laws.event(Guard::new(head, Crossing::Rising, transition));
        if defect != Defect::UnaddedFragment {
            body.add(laws.finish());
        }
        body.compile()
    })
}

fn named(local: &'static str) -> LawName {
    LawName { owner: "NS", local }
}

fn refusal(defect: Defect) -> AdmissionError {
    match body(defect) {
        Ok(compiled) => panic!("compiled to {:?}", compiled.model_hash()),
        Err(refused) => refused,
    }
}

fn compile_refusal(defect: Defect) -> CompileDefect {
    match refusal(defect) {
        AdmissionError::Compile(found) => found,
        other => panic!("refused at construction: {other:?}"),
    }
}

#[test]
fn the_valid_body_lays_out_memory_then_algebraic_rows() -> Result<(), AdmissionError> {
    let compiled = body(Defect::None)?;
    let kinds: Vec<RowKind> = compiled.rows().iter().map(Row::kind).collect();
    assert_eq!(
        kinds,
        [
            RowKind::Memory(named("filter")),
            RowKind::Memory(named("gate")),
            RowKind::Algebraic(named("pressure")),
        ]
    );
    let unknowns: Vec<usize> = compiled.rows().iter().map(|row| row.unknown().index()).collect();
    assert_eq!(unknowns, [0, 1, 2]);
    let sets: Vec<AdmissibleSet> = compiled.rows().iter().map(Row::admissible).collect();
    assert_eq!(
        sets,
        [
            AdmissibleSet::UnitInterval,
            AdmissibleSet::UnitInterval,
            AdmissibleSet::Finite
        ]
    );
    Ok(())
}

#[test]
fn each_law_is_one_program_with_one_whole_body_instance() -> Result<(), AdmissionError> {
    let compiled = body(Defect::None)?;
    assert_eq!(compiled.programs().len(), 4);
    let row = |target: Target| match target {
        Target::Row(row) => Some(row.index()),
        Target::Column(_) => None,
    };
    let placed: Vec<(usize, Context, Option<usize>)> = (compiled.instances().iter())
        .map(|instance| {
            (
                instance.program().index(),
                instance.context(),
                row(instance.target()),
            )
        })
        .collect();
    assert_eq!(
        placed,
        [
            (0, Context::Body, Some(0)),
            (1, Context::Body, Some(1)),
            (2, Context::Body, Some(1)),
            (3, Context::Body, Some(2))
        ]
    );
    Ok(())
}

#[test]
fn whole_body_parameters_have_one_slot_each_and_no_default() -> Result<(), AdmissionError> {
    let compiled = body(Defect::None)?;
    let slots: Vec<(ParameterKey, Binding, Option<Binding>, AdmissibleSet)> = (compiled.parameters().iter())
        .map(|slot| {
            (
                slot.key(),
                slot.binding(),
                slot.default_binding(),
                slot.admissible(),
            )
        })
        .collect();
    assert_eq!(
        slots,
        [
            (
                ParameterKey::new("ns.pressure"),
                Binding::Body,
                None,
                AdmissibleSet::Finite
            ),
            (
                ParameterKey::new("ns.speed"),
                Binding::Body,
                None,
                AdmissibleSet::Nonnegative
            ),
            (
                ParameterKey::new("ns.target"),
                Binding::Body,
                None,
                AdmissibleSet::UnitInterval
            ),
        ]
    );
    let filter = &compiled.instances()[0];
    let keys: Vec<ParameterKey> = (filter.parameters().iter())
        .map(|id| compiled.parameters()[id.index()].key())
        .collect();
    assert_eq!(
        keys,
        [ParameterKey::new("ns.target"), ParameterKey::new("ns.speed")]
    );
    Ok(())
}

#[test]
fn sparsity_reads_each_rows_unknowns_and_a_differential_row_its_own() -> Result<(), AdmissionError> {
    let compiled = body(Defect::None)?;
    let sparsity = compiled.sparsity();
    assert_eq!(sparsity.row_offsets(), [0, 1, 2, 3]);
    let unknowns: Vec<usize> = sparsity
        .unknowns()
        .iter()
        .map(|unknown| unknown.index())
        .collect();
    assert_eq!(unknowns, [0, 1, 2]);
    Ok(())
}

#[test]
fn an_identical_repeat_of_a_parameter_is_the_same_parameter() -> Result<(), AdmissionError> {
    let compiled = body(Defect::ParameterRepeated)?;
    assert_eq!(compiled.parameters().len(), 3);
    assert_eq!(compiled.model_hash(), body(Defect::None)?.model_hash());
    Ok(())
}

#[test]
fn a_slot_no_law_defines_is_an_omitted_law() {
    assert_eq!(
        compile_refusal(Defect::UnattachedSlot),
        CompileDefect::OmittedLaw {
            law: named("unattached")
        }
    );
}

#[test]
fn an_algebraic_slot_no_residual_closes_is_an_omitted_law() {
    assert_eq!(
        compile_refusal(Defect::UnclosedAlgebraicSlot),
        CompileDefect::OmittedLaw {
            law: named("unclosed")
        }
    );
}

#[test]
fn two_every_mode_laws_on_one_row_are_a_second_owner() {
    assert_eq!(
        compile_refusal(Defect::SecondEveryModeRate),
        CompileDefect::SecondOwner {
            law: named("filter"),
            first: Predicate::EveryMode,
            second: Predicate::EveryMode,
        }
    );
}

#[test]
fn laws_on_two_mode_variables_hold_together_as_a_second_owner() {
    let mode = |variable, value| Predicate::Only(ModeValue { variable, value });
    assert_eq!(
        compile_refusal(Defect::LawOnSecondVariable),
        CompileDefect::SecondOwner {
            law: named("gate"),
            first: mode("switch", "off"),
            second: mode("valve", "shut"),
        }
    );
}

#[test]
fn a_value_no_law_holds_in_is_an_uncovered_mode() {
    assert_eq!(
        compile_refusal(Defect::UncoveredValue),
        CompileDefect::UncoveredMode {
            law: named("gate"),
            mode: ModeValue {
                variable: "switch",
                value: "off"
            },
        }
    );
}

#[test]
fn a_guard_holds_in_the_value_it_leaves() {
    assert_eq!(
        compile_refusal(Defect::GuardInOtherValue),
        CompileDefect::GuardOutsideMode {
            variable: named("switch"),
            leaves: "on"
        }
    );
}

#[test]
fn a_scope_whose_fragment_never_arrives_is_a_missing_fragment() {
    assert_eq!(
        compile_refusal(Defect::UnaddedFragment),
        CompileDefect::MissingFragment { owner: "NS" }
    );
}

#[test]
fn two_slots_under_one_name_are_named_twice() {
    assert_eq!(
        compile_refusal(Defect::SlotNamedTwice),
        CompileDefect::NamedTwice { law: named("filter") }
    );
}

#[test]
fn a_mode_variable_under_a_slots_name_is_named_twice() {
    assert_eq!(
        compile_refusal(Defect::ModeVariableNamedLikeSlot),
        CompileDefect::NamedTwice { law: named("filter") }
    );
}

#[test]
fn one_parameter_key_with_two_dimensions_is_a_conflict() {
    assert_eq!(
        compile_refusal(Defect::ParameterRedimensioned),
        CompileDefect::ParameterConflict {
            key: ParameterKey::new("ns.target")
        }
    );
}

#[test]
fn one_parameter_key_with_two_admissible_sets_is_a_conflict() {
    assert_eq!(
        compile_refusal(Defect::ParameterResetToOtherSet),
        CompileDefect::ParameterConflict {
            key: ParameterKey::new("ns.target")
        }
    );
}

#[test]
fn a_mode_variable_with_no_values_is_refused() {
    assert_eq!(
        compile_refusal(Defect::NoModeValues),
        CompileDefect::BadModeVariable {
            variable: named("empty")
        }
    );
}

#[test]
fn a_mode_variable_with_a_repeated_value_is_refused() {
    assert_eq!(
        compile_refusal(Defect::RepeatedModeValue),
        CompileDefect::BadModeVariable {
            variable: named("twice")
        }
    );
}

#[test]
fn a_residual_that_reads_no_unknown_is_structurally_singular() {
    assert_eq!(
        compile_refusal(Defect::ResidualWithoutUnknown),
        CompileDefect::StructuralSingularity {
            law: named("pressure")
        }
    );
}

#[test]
fn a_scope_is_issued_once() {
    assert_eq!(
        refusal(Defect::ScopeIssuedTwice),
        AdmissionError::ScopeIssuedTwice { owner: "NS" }
    );
}

#[test]
fn a_transition_to_its_own_value_is_refused() {
    let on = ModeValue {
        variable: "switch",
        value: "on",
    };
    assert_eq!(
        refusal(Defect::TransitionToSameValue),
        AdmissionError::InvalidTransition { from: on, to: on }
    );
}

#[test]
fn a_transition_to_another_variables_value_is_refused() {
    assert_eq!(
        refusal(Defect::TransitionToOtherVariable),
        AdmissionError::InvalidTransition {
            from: ModeValue {
                variable: "switch",
                value: "on"
            },
            to: ModeValue {
                variable: "valve",
                value: "open"
            },
        }
    );
}

#[test]
fn a_rate_class_has_a_nonzero_interval() {
    assert_eq!(
        refusal(Defect::ZeroRateInterval),
        AdmissionError::ZeroRateInterval
    );
}

#[test]
fn a_validity_range_runs_from_lower_to_upper() {
    assert_eq!(
        refusal(Defect::ReversedValidityRange),
        AdmissionError::ReversedValidityRange
    );
}

fn blocks(compiled: &CompiledBody) -> Vec<(Vec<usize>, Vec<usize>, RateBand)> {
    (compiled.blocks().iter())
        .map(|block| {
            let rows = block.rows().iter().map(|row| row.index()).collect();
            let unknowns = block.unknowns().iter().map(|unknown| unknown.index()).collect();
            (rows, unknowns, block.rate_class().band())
        })
        .collect()
}

#[test]
fn a_block_comes_after_the_blocks_it_depends_on() -> Result<(), AdmissionError> {
    let compiled = assemble(schema()?, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let reflex = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let hours = RateClass::new(RateBand::Hours, 3_600_000_000_000)?;
        let speed =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("ns.speed"), Admissible::nonnegative());
        let follower = laws.state::<Dimensionless>("follower", Admissible::finite());
        let leader = laws.state::<Dimensionless>("leader", Admissible::finite());
        let follow = laws.kernel(|mut k| {
            let toward = k.state(&leader);
            let now = k.state(&follower);
            let pace = k.parameter(&speed);
            let rate = k.relax(toward, now, pace);
            k.finish(rate, laws.every_mode(), reflex)
        });
        let rest = laws.kernel(|mut k| {
            let zero = k.zero::<Rate<Dimensionless>>();
            k.finish(zero, laws.every_mode(), hours)
        });
        laws.memory_rate(&follower, follow);
        laws.memory_rate(&leader, rest);
        body.add(laws.finish());
        body.compile()
    })?;
    assert_eq!(
        blocks(&compiled),
        [
            (vec![1], vec![1], RateBand::Hours),
            (vec![0], vec![0], RateBand::Reflex)
        ]
    );
    Ok(())
}

#[test]
fn the_matching_pairs_each_residual_with_an_unknown_it_reads() -> Result<(), AdmissionError> {
    let compiled = assemble(schema()?, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let level = laws.parameter::<Pressure>(ParameterKey::new("ns.level"), Admissible::finite());
        let p = laws.algebraic::<Pressure>("p", Admissible::finite());
        let q = laws.algebraic::<Pressure>("q", Admissible::finite());
        let closes_q = laws.kernel(|mut k| {
            let q = k.algebraic(&q);
            let level = k.parameter(&level);
            let gap = k.subtract(q, level);
            k.finish(gap, laws.every_mode(), wave)
        });
        let ties_p = laws.kernel(|mut k| {
            let p = k.algebraic(&p);
            let q = k.algebraic(&q);
            let gap = k.subtract(p, q);
            k.finish(gap, laws.every_mode(), wave)
        });
        laws.residual(&p, closes_q);
        laws.residual(&q, ties_p);
        body.add(laws.finish());
        body.compile()
    })?;
    let unknowns: Vec<usize> = compiled
        .sparsity()
        .unknowns()
        .iter()
        .map(|unknown| unknown.index())
        .collect();
    assert_eq!(
        (compiled.sparsity().row_offsets(), &unknowns[..]),
        (&[0, 1, 3][..], &[1, 0, 1][..])
    );
    assert_eq!(
        blocks(&compiled),
        [
            (vec![0], vec![1], RateBand::Wave),
            (vec![1], vec![0], RateBand::Wave)
        ]
    );
    Ok(())
}
