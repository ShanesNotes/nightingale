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

fn enclosed(cards: &[&'static str], fluid: bool) -> Result<Schema, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    map.contain(root.id(), vessel.id());
    for (&card, (bed, space)) in cards.iter().zip([("bed", "space"), ("bed two", "space two")]) {
        let bed = map.site::<CapillaryBed>(bed, at()?);
        let space = map.site::<EnclosedSpace>(space, at()?);
        for child in [bed.id(), space.id()] {
            map.contain(root.id(), child);
        }
        for (from, to) in [(vessel.id(), bed.id()), (bed.id(), vessel.id())] {
            map.connect(Network::Vascular, from, to);
        }
        map.enclose(&space, &[bed.id()], Compliance::Held { card });
    }
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Cells, "a bench with no cell water");
    if !fluid {
        schema.leave_out(Space::Fluid, "a bench whose enclosure holds no fluid");
    }
    Ok(schema)
}

fn heart<'b>(body: &mut Assembly<'b>) -> Result<FragmentBuilder<'b, Cardiovascular>, AdmissionError> {
    let class = slow()?;
    let (mut cv, ports) = body.cardiovascular()?.parts();
    let route = ports.circulation(&mut cv);
    let still = cv.edge_kernel(|mut k| {
        let zero = k.zero::<Rate<Water>>();
        k.finish(zero, cv.every_mode(), class)
    });
    cv.transfer_along(&route, still);
    Ok(cv)
}

fn circulation(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let cv = heart(body)?;
    body.add(cv.finish());
    Ok(())
}

fn organs(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    circulation(body)?;
    fluids(body)
}

fn fluids(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let class = slow()?;
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

fn serve<'b, O: Owner, Q: Substance>(
    laws: &mut FragmentBuilder<'b, O>,
    route: &Pair<'b, O, Q>,
) -> Result<(), AdmissionError> {
    let class = slow()?;
    let rate = laws.kernel(|mut k| {
        let zero = k.zero::<Rate<Q>>();
        k.finish(zero, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(route, rate));
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

fn built(schema: Schema, beside_held: bool) -> Result<CompiledBody, AdmissionError> {
    assemble(schema, |mut body| {
        organs(&mut body)?;
        if beside_held {
            let bc = body.hold(HeldDeclaration::<BloodChemistry>::new(vec![], vec![], "a bench"))?;
            body.add(bc.parts().0.finish());
        }
        let (mut rs, ports) = body.respiratory(None)?.parts();
        pressure(&mut rs, &ports.enclosure_pressure)?;
        body.add(rs.finish());
        body.compile()
    })
}

#[test]
fn a_built_respiratory_card_defines_its_enclosures_pressure_and_holds_no_fluid() -> Result<(), AdmissionError>
{
    let alone = built(enclosed(&[Respiratory::CARD], false)?, false)?;
    assert_eq!(alone.held().count(), 0);
    let beside = built(enclosed(&[Respiratory::CARD], false)?, true)?;
    assert_eq!(beside.held().collect::<Vec<_>>(), ["BC"]);
    Ok(())
}

#[test]
fn a_built_cards_enclosure_fluid_that_nothing_moves_is_an_orphan() -> Result<(), AdmissionError> {
    for beside_held in [false, true] {
        assert_eq!(
            built(enclosed(&[Respiratory::CARD], true)?, beside_held).map(|body| body.model_hash()),
            Err(defect(CompileDefect::OrphanPool {
                site: site::<EnclosedSpace>("space"),
                space: Space::Fluid,
            }))
        );
    }
    Ok(())
}

#[test]
fn another_cards_owed_pressure_does_not_cover_an_enclosure_whose_card_owes_none() -> Result<(), AdmissionError>
{
    let found = built(
        enclosed(&[Respiratory::CARD, Cardiovascular::CARD], false)?,
        false,
    );
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::OmittedLaw {
            law: law("CV", "enclosure_pressure")
        }))
    );
    Ok(())
}

#[test]
fn a_field_named_like_the_enclosure_pressure_is_not_the_pressure_an_enclosure_is_owed()
-> Result<(), AdmissionError> {
    let found = assemble(enclosed(&[Cardiovascular::CARD], false)?, |mut body| {
        let mut cv = heart(&mut body)?;
        let (field, _) = cv.site_field::<EnclosedSpace, Pressure>("enclosure_pressure", Admissible::finite());
        pressure(&mut cv, &field)?;
        body.add(cv.finish());
        fluids(&mut body)?;
        body.compile()
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::OmittedLaw {
            law: law("CV", "enclosure_pressure")
        }))
    );
    Ok(())
}

#[test]
fn a_card_whose_ports_carry_no_pressure_cannot_own_an_enclosure_unheld() -> Result<(), AdmissionError> {
    let found = assemble(enclosed(&[Gastrointestinal::CARD], false)?, |mut body| {
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
    let found = assemble(enclosed(&[Gastrointestinal::CARD], true)?, |mut body| {
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
    let body = assemble(enclosed(&[Respiratory::CARD], true)?, |mut body| {
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

fn gut(handed: bool, feces: bool) -> Result<CompiledBody, AdmissionError> {
    let mut schema = enclosed(&[Gastrointestinal::CARD], true)?;
    let vessel = site::<VesselSegment>("vessel");
    schema.boundary::<Water>("gut water", BoundaryDirection::IntoBody, vessel, Space::Lumen)?;
    if feces {
        schema.process::<Feces<Water>>(vessel, Space::Lumen)?;
    }
    assemble(schema, |mut body| {
        organs(&mut body)?;
        let declaration = HeldDeclaration::<Gastrointestinal>::new(vec!["gut water"], vec![], "a bench");
        let mut gi = body.hold(declaration)?;
        let (laws, held) = gi.stand_ins();
        pressure(laws, &held.enclosure_pressure)?;
        serve(laws, &held.route::<Water>("gut water")?.route)?;
        let laws = if handed {
            let (mut laws, ports) = body.gastrointestinal(Some(gi))?.parts();
            if feces {
                serve(&mut laws, &ports.boundary::<Feces<Water>>()?.route)?;
            }
            laws
        } else {
            gi.parts().0
        };
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn handing_a_held_scope_to_its_cards_scope_moves_no_byte() -> Result<(), AdmissionError> {
    let (kept, handed) = (gut(false, false)?, gut(true, false)?);
    assert_eq!(kept.model_hash(), handed.model_hash());
    assert_eq!(kept.conserved_basis().digest(), handed.conserved_basis().digest());
    Ok(())
}

fn hormones(
    place: impl FnOnce(&mut Schema, SiteId) -> Result<(), AdmissionError>,
) -> Result<Schema, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let veins = map.site::<VesselSegment>("veins", at()?);
    let bed = map.site::<CapillaryBed>("bed", at()?);
    for child in [veins.id(), bed.id()] {
        map.contain(root.id(), child);
    }
    for (from, to) in [(veins.id(), bed.id()), (bed.id(), veins.id())] {
        map.connect(Network::Vascular, from, to);
    }
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Interstitium, Space::Cells] {
        schema.leave_out(space, "a bench of blood alone");
    }
    schema.carry::<Insulin, Blood>(Space::Plasma);
    schema.carry::<Insulin, Blood>(Space::Lumen);
    place(&mut schema, veins.id())?;
    Ok(schema)
}

fn insulin_both(schema: &mut Schema, veins: SiteId) -> Result<(), AdmissionError> {
    schema.process::<Secretion<Insulin>>(veins, Space::Lumen)?;
    schema.process::<Clearance<Insulin>>(veins, Space::Lumen)
}

fn held_routes(body: &CompiledBody) -> Vec<(&'static str, &'static str)> {
    (body.conserved_basis().coverage().held_routes().iter())
        .map(|route| (route.card(), route.name()))
        .collect()
}

#[test]
fn a_card_serves_its_own_boundary_processes_through_its_ports() -> Result<(), AdmissionError> {
    let body = assemble(hormones(insulin_both)?, |mut body| {
        circulation(&mut body)?;
        let (mut en, ports) = body.endocrine(None)?.parts();
        serve(&mut en, &ports.boundary::<Secretion<Insulin>>()?.route)?;
        serve(&mut en, &ports.boundary::<Clearance<Insulin>>()?.route)?;
        body.add(en.finish());
        body.compile()
    })?;
    let served: Vec<(LawName, Option<&str>, Vec<i64>)> = (body.columns().iter())
        .filter(|column| column.origin().owner == "EN")
        .map(|column| {
            (
                column.origin(),
                column.receipt().map(|receipt| receipt.boundary()),
                column.terms().iter().map(|term| term.1).collect(),
            )
        })
        .collect();
    assert_eq!(
        served,
        [
            (
                law("EN", "insulin clearance"),
                Some("insulin clearance"),
                vec![-1]
            ),
            (law("EN", "insulin secretion"), Some("insulin secretion"), vec![1]),
        ]
    );
    assert!(held_routes(&body).is_empty());
    Ok(())
}

#[test]
fn a_port_refuses_a_process_the_schema_does_not_place() -> Result<(), AdmissionError> {
    let found = assemble(hormones(insulin_both)?, |mut body| {
        circulation(&mut body)?;
        let (en, ports) = body.endocrine(None)?.parts();
        ports.boundary::<Secretion<Glucagon>>()?;
        body.add(en.finish());
        body.compile()
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(AdmissionError::UnknownBoundary {
            name: "glucagon secretion"
        })
    );
    Ok(())
}

#[test]
fn a_held_card_cannot_supply_another_cards_process() -> Result<(), AdmissionError> {
    let found = assemble(hormones(insulin_both)?, |mut body| {
        body.hold(HeldDeclaration::<Hepatic>::new(
            vec!["insulin secretion"],
            vec![],
            "a bench",
        ))?;
        body.compile()
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(AdmissionError::BoundaryNamedTwice {
            name: "insulin secretion"
        })
    );
    Ok(())
}

#[test]
fn the_boundary_scope_cannot_serve_a_cards_process() -> Result<(), AdmissionError> {
    let found = assemble(hormones(insulin_both)?, |mut body| {
        body.boundary()?.parts().1.route::<Insulin>("insulin secretion")?;
        body.compile()
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(AdmissionError::UnknownBoundary {
            name: "insulin secretion"
        })
    );
    Ok(())
}

fn insulin_held(supplies: Vec<&'static str>) -> Result<CompiledBody, AdmissionError> {
    assemble(hormones(insulin_both)?, |mut body| {
        circulation(&mut body)?;
        let declaration = HeldDeclaration::<Endocrine>::new(supplies, vec![], "a bench");
        let (mut en, ports) = body.hold(declaration)?.parts();
        for name in ["insulin secretion", "insulin clearance"] {
            serve(&mut en, &ports.route::<Insulin>(name)?.route)?;
        }
        body.add(en.finish());
        body.compile()
    })
}

#[test]
fn a_held_stand_in_serves_its_own_process_only_when_its_declaration_supplies_it() -> Result<(), AdmissionError>
{
    assert_eq!(
        insulin_held(vec![]).map(|body| body.model_hash()),
        Err(AdmissionError::UnknownBoundary {
            name: "insulin secretion"
        })
    );
    let body = insulin_held(vec!["insulin secretion", "insulin clearance"])?;
    assert_eq!(
        held_routes(&body),
        [("EN", "insulin clearance"), ("EN", "insulin secretion")]
    );
    Ok(())
}

#[test]
fn a_process_whose_card_has_no_scope_is_omitted_under_that_card() -> Result<(), AdmissionError> {
    let found = assemble(hormones(insulin_both)?, |mut body| {
        circulation(&mut body)?;
        body.compile()
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::OmittedLaw {
            law: law("EN", "insulin clearance")
        }))
    );
    Ok(())
}

fn kept_endocrine(another_held: bool) -> Result<CompiledBody, AdmissionError> {
    assemble(hormones(insulin_both)?, |mut body| {
        circulation(&mut body)?;
        if another_held {
            let gi = body.hold(HeldDeclaration::<Gastrointestinal>::new(
                vec![],
                vec![],
                "a bench",
            ))?;
            body.add(gi.parts().0.finish());
        }
        let declaration = HeldDeclaration::<Endocrine>::new(vec!["insulin secretion"], vec![], "a bench");
        let mut en = body.hold(declaration)?;
        let (laws, held) = en.stand_ins();
        serve(laws, &held.route::<Insulin>("insulin secretion")?.route)?;
        let (mut laws, ports) = body.endocrine(Some(en))?.parts();
        assert_eq!(
            ports.boundary::<Secretion<Insulin>>().err(),
            Some(AdmissionError::UnknownBoundary {
                name: "insulin secretion"
            })
        );
        serve(&mut laws, &ports.boundary::<Clearance<Insulin>>()?.route)?;
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn a_kept_cards_ports_leave_the_routes_its_stand_ins_supply() -> Result<(), AdmissionError> {
    for another_held in [false, true] {
        assert_eq!(
            held_routes(&kept_endocrine(another_held)?),
            [("EN", "insulin secretion")]
        );
    }
    Ok(())
}

fn secreted(typed: bool) -> Result<CompiledBody, AdmissionError> {
    let place = |schema: &mut Schema, veins: SiteId| {
        if typed {
            schema.process::<Secretion<Insulin>>(veins, Space::Lumen)
        } else {
            schema.boundary::<Insulin>(
                "insulin secretion",
                BoundaryDirection::IntoBody,
                veins,
                Space::Lumen,
            )
        }
    };
    assemble(hormones(place)?, |mut body| {
        circulation(&mut body)?;
        let declaration = HeldDeclaration::<Endocrine>::new(vec!["insulin secretion"], vec![], "a bench");
        let (mut en, held) = body.hold(declaration)?.parts();
        serve(&mut en, &held.route::<Insulin>("insulin secretion")?.route)?;
        body.add(en.finish());
        body.compile()
    })
}

#[test]
fn the_card_a_process_names_enters_the_model_hash() -> Result<(), AdmissionError> {
    assert_ne!(secreted(true)?.model_hash(), secreted(false)?.model_hash());
    Ok(())
}

#[test]
fn a_process_at_a_pool_the_body_lacks_is_named_under_its_card() -> Result<(), AdmissionError> {
    let lacking =
        |schema: &mut Schema, veins: SiteId| schema.process::<Secretion<Insulin>>(veins, Space::Gas);
    let found = assemble(hormones(lacking)?, |mut body| {
        circulation(&mut body)?;
        body.compile()
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::BoundaryEndpoint {
            law: law("EN", "insulin secretion")
        }))
    );
    Ok(())
}

#[test]
fn the_renal_card_serves_the_urine_drain_placed_as_its_process() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let bladder = map.site::<VesselSegment>("bladder", at()?);
    map.contain(root.id(), bladder.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.process::<Urine>(bladder.id(), Space::Lumen)?;
    let body = assemble(schema, |mut body| {
        let (mut rn, ports) = body.renal()?.parts();
        serve(&mut rn, &ports.boundary::<Urine>()?.route)?;
        body.add(rn.finish());
        body.compile()
    })?;
    let urine = RowKind::Algebraic(Slot {
        law: law("RN", "urine"),
        at: Context::Body,
    });
    assert!(body.rows().iter().any(|row| row.kind() == urine));
    Ok(())
}

#[test]
fn a_kept_cards_built_process_is_no_held_support() -> Result<(), AdmissionError> {
    let body = gut(true, true)?;
    assert_eq!(held_routes(&body), [("GI", "gut water")]);
    let feces = (body.columns().iter()).find(|column| column.origin() == law("GI", "feces water"));
    assert_eq!(
        feces
            .and_then(|column| column.receipt())
            .map(|receipt| receipt.boundary()),
        Some("feces water")
    );
    Ok(())
}
