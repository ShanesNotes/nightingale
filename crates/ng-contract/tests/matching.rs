use ng_contract::*;

/// The product over this many two-valued variables would never finish, so each test below passes only if
/// compilation does not enumerate it.
const VARIABLES: usize = 30;

fn schema() -> Result<Schema, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let at = Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?);
    map.site::<TissueSite>("body", at);
    Schema::new(map.finish()?)
}

fn name(prefix: &str, at: usize) -> &'static str {
    Box::leak(format!("{prefix}{at}").into_boxed_str())
}

type Slot<'b> = AlgebraicSlot<'b, Nervous, Pressure>;

fn gap<'b>(
    laws: &FragmentBuilder<'b, Nervous>,
    reads: &[&Slot<'b>],
    level: &Parameter<'b, Nervous, Pressure>,
    modes: ModeSet<'b, Nervous>,
) -> Result<PureKernel<'b, Nervous, Pressure>, AdmissionError> {
    let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
    Ok(laws.kernel(|mut k| {
        let mut left = k.parameter(level);
        for slot in reads {
            let value = k.algebraic(slot);
            left = k.subtract(left, value);
        }
        k.finish(left, modes, wave)
    }))
}

fn compiled(body: Result<CompiledBody, AdmissionError>) -> Result<ModelHash, AdmissionError> {
    body.map(|compiled| compiled.model_hash())
}

#[test]
fn modes_on_rows_that_share_no_unknown_are_matched_apart() -> Result<(), AdmissionError> {
    let body = assemble(schema()?, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let level = laws.parameter::<Pressure>(ParameterKey::new("ns.level"), Admissible::finite());
        for at in 0..VARIABLES {
            let p = laws.algebraic::<Pressure>(name("p", at), Admissible::finite());
            let q = laws.algebraic::<Pressure>(name("q", at), Admissible::finite());
            let [off, on] = laws.modes(name("switch", at), ["off", "on"]);
            let both = gap(&laws, &[&p, &q], &level, laws.every_mode())?;
            let reads_p = gap(&laws, &[&p], &level, off.only())?;
            let reads_q = gap(&laws, &[&q], &level, on.only())?;
            laws.residual(&p, both);
            laws.residual(&q, reads_p);
            laws.residual(&q, reads_q);
        }
        body.add(laws.finish());
        body.compile()
    });
    compiled(body).map(drop)
}

#[test]
fn mode_values_with_the_same_sparsity_are_one_case() -> Result<(), AdmissionError> {
    let body = assemble(schema()?, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let low = laws.parameter::<Pressure>(ParameterKey::new("ns.low"), Admissible::finite());
        let high = laws.parameter::<Pressure>(ParameterKey::new("ns.high"), Admissible::finite());
        let ring: Vec<Slot<'_>> = (0..VARIABLES)
            .map(|at| laws.algebraic::<Pressure>(name("ring", at), Admissible::finite()))
            .collect();
        for at in 0..VARIABLES {
            let [off, on] = laws.modes(name("switch", at), ["off", "on"]);
            let next = &ring[(at + 1) % VARIABLES];
            let by_low = gap(&laws, &[&ring[at], next], &low, off.only())?;
            let by_high = gap(&laws, &[&ring[at], next], &high, on.only())?;
            laws.residual(&ring[at], by_low);
            laws.residual(&ring[at], by_high);
        }
        body.add(laws.finish());
        body.compile()
    });
    compiled(body).map(drop)
}

#[test]
fn a_combination_singular_only_together_is_refused_though_each_value_is_fine_alone()
-> Result<(), AdmissionError> {
    let body = assemble(schema()?, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let level = laws.parameter::<Pressure>(ParameterKey::new("ns.level"), Admissible::finite());
        let p = laws.algebraic::<Pressure>("p", Admissible::finite());
        let q = laws.algebraic::<Pressure>("q", Admissible::finite());
        let [left_on_p, left_on_q] = laws.modes("left", ["on p", "on q"]);
        let [right_on_p, right_on_q] = laws.modes("right", ["on p", "on q"]);
        let left_p = gap(&laws, &[&p], &level, left_on_p.only())?;
        let left_q = gap(&laws, &[&q], &level, left_on_q.only())?;
        let right_p = gap(&laws, &[&p], &level, right_on_p.only())?;
        let right_q = gap(&laws, &[&q], &level, right_on_q.only())?;
        laws.residual(&p, left_p);
        laws.residual(&p, left_q);
        laws.residual(&q, right_p);
        laws.residual(&q, right_q);
        body.add(laws.finish());
        body.compile()
    });
    assert_eq!(
        compiled(body),
        Err(AdmissionError::Compile(CompileDefect::StructuralSingularity {
            law: LawName {
                owner: "NS",
                local: "q"
            }
        }))
    );
    Ok(())
}
