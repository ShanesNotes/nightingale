use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn slow() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Hours, 3_600_000_000_000)
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = at().expect("finite");
    AnatomyBuilder::new().site::<C>(key, origin).id()
}

fn law(owner: &'static str, local: &'static str) -> LawName {
    LawName { owner, local }
}

fn defect(found: CompileDefect) -> AdmissionError {
    AdmissionError::Compile(found)
}

fn lone() -> Result<Schema, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    Schema::new(map.finish()?)
}

fn enclosed(card: &'static str, fluid: bool) -> Result<Schema, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let bed = map.site::<CapillaryBed>("bed", at()?);
    let space = map.site::<EnclosedSpace>("space", at()?);
    for child in [vessel.id(), bed.id(), space.id()] {
        map.contain(root.id(), child);
    }
    for (from, to) in [(vessel.id(), bed.id()), (bed.id(), vessel.id())] {
        map.connect(Network::Vascular, from, to);
    }
    map.enclose(&space, &[bed.id()], Compliance::Held { card });
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Cells, "a bench with no cell water");
    if !fluid {
        schema.leave_out(Space::Fluid, "a bench whose enclosure holds no fluid");
    }
    Ok(schema)
}

fn organs(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let class = slow()?;
    let (mut cv, ports) = body.cardiovascular()?.parts();
    let route = ports.circulation(&mut cv);
    let still = cv.edge_kernel(|mut k| {
        let zero = k.zero::<Rate<Water>>();
        k.finish(zero, cv.every_mode(), class)
    });
    cv.transfer_along(&route, still);
    body.add(cv.finish());
    let (mut fl, ports) = body.fluids()?.parts();
    let route = ports.starling(&mut fl);
    let kf = fl.site_parameter::<CapillaryBed, Conductance<Water>>(
        ParameterKey::new("fl.kf"),
        Admissible::nonnegative(),
    );
    let flow = fl.site_kernel(|mut k| {
        let zero = k.zero::<Pressure>();
        let outside = k.at_site(&ports.extravascular_pressure);
        let head = k.subtract(zero, outside);
        let conductance = k.site_parameter(&kf);
        let flow = k.conductance_flow(conductance, head);
        k.finish(flow, fl.every_mode(), class)
    });
    fl.transfer_at(&route, flow);
    body.add(fl.finish());
    Ok(())
}

fn pressure<'b, O: Owner>(
    laws: &mut FragmentBuilder<'b, O>,
    output: &Output<'b, O, PerSite<EnclosedSpace, Pressure>>,
) -> Result<(), AdmissionError> {
    let held = laws.parameter::<Pressure>(ParameterKey::new("space.pressure"), Admissible::finite());
    let class = slow()?;
    let value = laws.site_kernel(|mut k| {
        let value = k.parameter(&held);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(output, value);
    Ok(())
}

#[test]
fn a_card_outside_the_slice_is_issued_once_held_or_built() -> Result<(), AdmissionError> {
    let declaration = || HeldDeclaration::<Respiratory>::new(vec![], vec![], "a bench");
    let twice = assemble(lone()?, |mut body| {
        body.respiratory(None)?;
        body.respiratory(None)?;
        body.compile()
    });
    let held_then_built = assemble(lone()?, |mut body| {
        body.hold(declaration())?;
        body.respiratory(None)?;
        body.compile()
    });
    let built_then_held = assemble(lone()?, |mut body| {
        body.respiratory(None)?;
        body.hold(declaration())?;
        body.compile()
    });
    for found in [twice, held_then_built, built_then_held] {
        assert_eq!(
            found.map(|body| body.model_hash()),
            Err(AdmissionError::ScopeIssuedTwice { owner: "RS" })
        );
    }
    Ok(())
}

#[test]
fn every_card_outside_the_slice_has_a_scope_of_its_own() -> Result<(), AdmissionError> {
    let body = assemble(lone()?, |mut body| {
        let fragment = body.respiratory(None)?.parts().0.finish();
        body.add(fragment);
        let fragment = body.hepatic(None)?.parts().0.finish();
        body.add(fragment);
        let fragment = body.gastrointestinal(None)?.parts().0.finish();
        body.add(fragment);
        let fragment = body.endocrine(None)?.parts().0.finish();
        body.add(fragment);
        let fragment = body.blood_chemistry(None)?.parts().0.finish();
        body.add(fragment);
        let fragment = body.hematology(None)?.parts().0.finish();
        body.add(fragment);
        let fragment = body.immune(None)?.parts().0.finish();
        body.add(fragment);
        let fragment = body.metabolism(None)?.parts().0.finish();
        body.add(fragment);
        let fragment = body.musculoskeletal(None)?.parts().0.finish();
        body.add(fragment);
        body.compile()
    })?;
    assert_eq!(body.held().count(), 0);
    Ok(())
}

fn built(fluid: bool) -> Result<CompiledBody, AdmissionError> {
    assemble(enclosed(Respiratory::CARD, fluid)?, |mut body| {
        organs(&mut body)?;
        let (mut rs, ports) = body.respiratory(None)?.parts();
        pressure(&mut rs, &ports.enclosure_pressure)?;
        body.add(rs.finish());
        body.compile()
    })
}

#[test]
fn a_built_respiratory_card_defines_its_enclosures_pressure_and_holds_no_fluid() -> Result<(), AdmissionError>
{
    assert_eq!(built(false)?.held().count(), 0);
    Ok(())
}

#[test]
fn a_built_cards_enclosure_fluid_that_nothing_moves_is_an_orphan() {
    assert_eq!(
        built(true).map(|body| body.model_hash()),
        Err(defect(CompileDefect::OrphanPool {
            site: site::<EnclosedSpace>("space"),
            space: Space::Fluid,
        }))
    );
}

#[test]
fn a_card_whose_ports_carry_no_pressure_cannot_own_an_enclosure_unheld() -> Result<(), AdmissionError> {
    let found = assemble(enclosed(Gastrointestinal::CARD, false)?, |mut body| {
        organs(&mut body)?;
        let fragment = body.gastrointestinal(None)?.parts().0.finish();
        body.add(fragment);
        body.compile()
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::OmittedLaw {
            law: law("GI", "enclosure_pressure")
        }))
    );
    Ok(())
}

#[test]
fn a_built_card_the_map_does_not_name_cannot_define_an_enclosures_pressure() -> Result<(), AdmissionError> {
    let found = assemble(enclosed(Gastrointestinal::CARD, true)?, |mut body| {
        organs(&mut body)?;
        let declaration = HeldDeclaration::<Gastrointestinal>::new(vec![], vec![], "a bench");
        let (mut gi, held) = body.hold(declaration)?.parts();
        pressure(&mut gi, &held.enclosure_pressure)?;
        body.add(gi.finish());
        let (mut rs, ports) = body.respiratory(None)?.parts();
        pressure(&mut rs, &ports.enclosure_pressure)?;
        body.add(rs.finish());
        body.compile()
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::UnclaimedLaw {
            law: law("RS", "enclosure_pressure")
        }))
    );
    Ok(())
}

#[test]
fn a_card_that_keeps_a_declaration_holds_its_enclosures_fluid_while_its_crate_defines_the_pressure()
-> Result<(), AdmissionError> {
    let body = assemble(enclosed(Respiratory::CARD, true)?, |mut body| {
        organs(&mut body)?;
        let validity = "the pleural fluid: a healthy day moves none";
        let rs = body.hold(HeldDeclaration::<Respiratory>::new(vec![], vec![], validity))?;
        let (mut laws, ports) = body.respiratory(Some(rs))?.parts();
        pressure(&mut laws, &ports.enclosure_pressure)?;
        body.add(laws.finish());
        body.compile()
    })?;
    assert_eq!(body.held().collect::<Vec<_>>(), ["RS"]);
    let pools: Vec<(&str, RowKind)> = (body.conserved_basis().coverage().pools().iter())
        .map(|pool| (pool.card(), body.rows()[pool.row().index()].kind()))
        .collect();
    let fluid = RowKind::Amount {
        site: site::<EnclosedSpace>("space"),
        space: Space::Fluid,
        species: Species::Water,
    };
    assert_eq!(pools, [("RS", fluid)]);
    Ok(())
}

fn gut(handed: bool) -> Result<CompiledBody, AdmissionError> {
    let mut schema = enclosed(Gastrointestinal::CARD, true)?;
    let vessel = site::<VesselSegment>("vessel");
    schema.boundary::<Water>("gut water", BoundaryDirection::IntoBody, vessel, Space::Lumen)?;
    assemble(schema, |mut body| {
        organs(&mut body)?;
        let declaration = HeldDeclaration::<Gastrointestinal>::new(vec!["gut water"], vec![], "a bench");
        let mut gi = body.hold(declaration)?;
        let (laws, held) = gi.stand_ins();
        pressure(laws, &held.enclosure_pressure)?;
        let route = held.route::<Water>("gut water")?.route;
        let class = slow()?;
        let rate = laws.kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
        let laws = if handed {
            body.gastrointestinal(Some(gi))?.parts().0
        } else {
            gi.parts().0
        };
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn handing_a_held_scope_to_its_cards_scope_moves_no_byte() -> Result<(), AdmissionError> {
    let (kept, handed) = (gut(false)?, gut(true)?);
    assert_eq!(kept.model_hash(), handed.model_hash());
    assert_eq!(kept.conserved_basis().digest(), handed.conserved_basis().digest());
    Ok(())
}
