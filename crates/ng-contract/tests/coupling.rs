use ng_contract::catalog::{
    BaroreceptorAfferents, CapillaryAndVenousPressure, Defines, LungAfferents, Reads, RespiratoryDrive,
};
use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn law(owner: &'static str, local: &'static str) -> LawName {
    LawName { owner, local }
}

fn wave() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Wave, 1_000_000)
}

struct Beds {
    schema: Schema,
    a: SiteId,
    b: SiteId,
}

fn beds() -> Result<Beds, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let a = map.site::<CapillaryBed>("a", at()?);
    let b = map.site::<CapillaryBed>("b", at()?);
    for site in [vessel.id(), a.id(), b.id()] {
        map.contain(root.id(), site);
    }
    for (from, to) in [(vessel.id(), a.id()), (a.id(), b.id()), (b.id(), vessel.id())] {
        map.connect(Network::Vascular, from, to);
    }
    let mut schema = Schema::new(map.finish()?)?;
    for (space, reason) in [
        (Space::Lumen, "no circulation in this bench"),
        (Space::Cells, "no cell water in this bench"),
    ] {
        schema.leave_out(space, reason);
    }
    Ok(Beds {
        schema,
        a: a.id(),
        b: b.id(),
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Defect {
    None,
    NoSender,
    OneFieldUndefined,
    DefinedTwice,
    EmittedTwice,
    PlainSlotUnderTheName,
}

fn pressure<'b>(
    laws: &FragmentBuilder<'b, Cardiovascular>,
    value: &SiteParameter<'b, Cardiovascular, CapillaryBed, Pressure>,
) -> Result<PureKernel<'b, Cardiovascular, Pressure, CapillaryBed>, AdmissionError> {
    let class = wave()?;
    Ok(laws.site_kernel(|mut k| {
        let value = k.site_parameter(value);
        k.finish(value, laws.every_mode(), class)
    }))
}

fn compiled(defect: Defect) -> Result<CompiledBody, AdmissionError> {
    assemble(beds()?.schema, |mut body| {
        if defect == Defect::PlainSlotUnderTheName {
            let (mut laws, _ports) = body.cardiovascular()?.parts();
            let plain = laws.algebraic::<Pressure>("capillary_pressure", Admissible::finite());
            let class = wave()?;
            let residual = laws.kernel(|mut k| {
                let seen = k.algebraic(&plain);
                k.finish(seen, laws.every_mode(), class)
            });
            laws.residual(&plain, residual);
            body.add(laws.finish());
        } else if defect != Defect::NoSender {
            let (mut laws, _ports) = body.cardiovascular()?.parts();
            let fields: CapillaryAndVenousPressure<Defines<'_, Cardiovascular>> = laws.emit();
            let value = laws.site_parameter::<CapillaryBed, Pressure>(
                ParameterKey::new("cv.bed-pressure"),
                Admissible::finite(),
            );
            laws.define_at(&fields.capillary_pressure, pressure(&laws, &value)?);
            if defect == Defect::DefinedTwice {
                laws.define_at(&fields.capillary_pressure, pressure(&laws, &value)?);
            }
            if defect != Defect::OneFieldUndefined {
                laws.define_at(&fields.venous_pressure, pressure(&laws, &value)?);
            }
            if defect == Defect::EmittedTwice {
                let _again: CapillaryAndVenousPressure<Defines<'_, Cardiovascular>> = laws.emit();
            }
            body.add(laws.finish());
        }
        let (mut laws, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut laws);
        let fields: CapillaryAndVenousPressure<Reads<'_, Fluids>> = laws.receive();
        let kf = laws.parameter::<Conductance<Water>>(ParameterKey::new("fl.kf"), Admissible::nonnegative());
        let class = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
        let flow = laws.site_kernel(|mut k| {
            let inside = k.at_site(&fields.capillary_pressure);
            let conductance = k.parameter(&kf);
            let flow = k.conductance_flow(conductance, inside);
            k.finish(flow, laws.every_mode(), class)
        });
        laws.transfer_at(&route, flow);
        body.add(laws.finish());
        body.compile()
    })
}

fn refused(defect: Defect) -> AdmissionError {
    match compiled(defect) {
        Ok(compiled) => panic!("compiled to {:?}", compiled.model_hash()),
        Err(refusal) => refusal,
    }
}

#[test]
fn a_per_site_field_has_one_row_per_site_named_by_its_site() -> Result<(), AdmissionError> {
    let Beds { a, b, .. } = beds()?;
    let body = compiled(Defect::None)?;
    let slot = |local, site| {
        RowKind::Algebraic(Slot {
            law: law("CV", local),
            site: Some(site),
        })
    };
    let kinds: Vec<RowKind> = (body.rows().iter())
        .map(Row::kind)
        .filter(|kind| matches!(kind, RowKind::Algebraic(_)))
        .collect();
    assert_eq!(
        kinds,
        [
            slot("capillary_pressure", a),
            slot("capillary_pressure", b),
            slot("venous_pressure", a),
            slot("venous_pressure", b),
        ]
    );
    Ok(())
}

#[test]
fn a_coupling_read_is_a_read_of_the_senders_row_at_the_readers_site() -> Result<(), AdmissionError> {
    let Beds { a, b, .. } = beds()?;
    let body = compiled(Defect::None)?;
    let row = |kind: RowKind| {
        let found = body.rows().iter().find(|row| row.kind() == kind);
        found.expect("a row").unknown()
    };
    let reads = |site| {
        let at = row(RowKind::Amount {
            site,
            space: Space::Plasma,
        })
        .index();
        let sparsity = body.sparsity();
        sparsity.unknowns()[sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1]].to_vec()
    };
    let field = |site| {
        row(RowKind::Algebraic(Slot {
            law: law("CV", "capillary_pressure"),
            site: Some(site),
        }))
    };
    assert!(reads(a).contains(&field(a)) && !reads(a).contains(&field(b)));
    assert!(reads(b).contains(&field(b)) && !reads(b).contains(&field(a)));
    Ok(())
}

#[test]
fn a_read_of_a_field_nobody_emits_is_refused() {
    assert_eq!(
        refused(Defect::NoSender),
        AdmissionError::Compile(CompileDefect::UnemittedField {
            law: law("FL", "starling"),
            field: law("CV", "capillary_pressure"),
        })
    );
}

#[test]
fn a_plain_slot_under_a_fields_name_is_no_field() {
    assert_eq!(
        refused(Defect::PlainSlotUnderTheName),
        AdmissionError::Compile(CompileDefect::UnemittedField {
            law: law("FL", "starling"),
            field: law("CV", "capillary_pressure"),
        })
    );
}

#[test]
fn every_field_of_an_emitted_arrow_is_owed() {
    assert_eq!(
        refused(Defect::OneFieldUndefined),
        AdmissionError::Compile(CompileDefect::OmittedLaw {
            law: law("CV", "venous_pressure"),
        })
    );
}

#[test]
fn a_field_is_defined_once() {
    assert!(matches!(
        refused(Defect::DefinedTwice),
        AdmissionError::Compile(CompileDefect::SecondOwner { law: named, .. })
            if named == law("CV", "capillary_pressure")
    ));
}

#[test]
fn an_arrow_is_emitted_once() {
    assert_eq!(
        refused(Defect::EmittedTwice),
        AdmissionError::Compile(CompileDefect::NamedTwice {
            law: law("CV", "capillary_pressure"),
        })
    );
}

#[test]
fn a_loop_of_two_couplings_is_one_block_of_coupled_residuals() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    let body = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let out: RespiratoryDrive<Defines<'_, Nervous>> = laws.emit();
        let back: LungAfferents<Reads<'_, Nervous>> = laws.receive();
        let class = wave()?;
        let drive = laws.kernel(|mut k| {
            let value = k.read(&back.pulmonary_stretch);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define(&out.respiratory_drive, drive);
        body.add(laws.finish());
        let declaration = HeldDeclaration::<Respiratory>::new(vec![], vec![], "a synthetic bench");
        let (mut laws, _ports) = body.hold(declaration)?.parts();
        let out: LungAfferents<Defines<'_, Respiratory>> = laws.emit();
        let back: RespiratoryDrive<Reads<'_, Respiratory>> = laws.receive();
        let class = wave()?;
        let stretch = laws.kernel(|mut k| {
            let value = k.read(&back.respiratory_drive);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define(&out.pulmonary_stretch, stretch);
        body.add(laws.finish());
        body.compile()
    })?;
    assert_eq!(body.rows().len(), 2);
    assert_eq!(body.blocks().len(), 1);
    assert_eq!(body.blocks()[0].rows().len(), 2);
    Ok(())
}

#[test]
fn a_field_nobody_reads_is_a_row_like_any_other() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    let body = assemble(Schema::new(map.finish()?)?, |mut body| {
        let (mut laws, _ports) = body.cardiovascular()?.parts();
        let out: BaroreceptorAfferents<Defines<'_, Cardiovascular>> = laws.emit();
        let stretch = laws.parameter::<Dimensionless>(ParameterKey::new("cv.stretch"), Admissible::finite());
        for output in [&out.carotid_sinus_stretch, &out.aortic_arch_stretch] {
            let class = wave()?;
            let value = laws.kernel(|mut k| {
                let value = k.parameter(&stretch);
                k.finish(value, laws.every_mode(), class)
            });
            laws.define(output, value);
        }
        body.add(laws.finish());
        body.compile()
    })?;
    assert_eq!(body.rows().len(), 2);
    Ok(())
}

fn both_arrows(capillary_first: bool) -> Result<ModelHash, AdmissionError> {
    let compiled = assemble(beds()?.schema, |mut body| {
        let (mut laws, _ports) = body.cardiovascular()?.parts();
        let (afferents, pressures): (
            BaroreceptorAfferents<Defines<'_, Cardiovascular>>,
            CapillaryAndVenousPressure<Defines<'_, Cardiovascular>>,
        ) = if capillary_first {
            let pressures = laws.emit();
            (laws.emit(), pressures)
        } else {
            let afferents = laws.emit();
            (afferents, laws.emit())
        };
        let stretch = laws.parameter::<Dimensionless>(ParameterKey::new("cv.stretch"), Admissible::finite());
        let value = laws.site_parameter::<CapillaryBed, Pressure>(
            ParameterKey::new("cv.bed-pressure"),
            Admissible::finite(),
        );
        for output in [&afferents.carotid_sinus_stretch, &afferents.aortic_arch_stretch] {
            let class = wave()?;
            let law = laws.kernel(|mut k| {
                let value = k.parameter(&stretch);
                k.finish(value, laws.every_mode(), class)
            });
            laws.define(output, law);
        }
        laws.define_at(&pressures.venous_pressure, pressure(&laws, &value)?);
        laws.define_at(&pressures.capillary_pressure, pressure(&laws, &value)?);
        body.add(laws.finish());
        let (mut laws, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut laws);
        let class = wave()?;
        let still = laws.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), class)
        });
        laws.transfer_at(&route, still);
        body.add(laws.finish());
        body.compile()
    })?;
    Ok(compiled.model_hash())
}

#[test]
fn the_order_of_two_arrows_reaches_nothing() -> Result<(), AdmissionError> {
    assert_eq!(both_arrows(true)?, both_arrows(false)?);
    Ok(())
}
