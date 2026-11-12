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

const THREE_BEDS: [(&str, Tissue); 3] = [
    ("bone bed", Tissue::Bone),
    ("liver bed", Tissue::Liver),
    ("muscle bed", Tissue::Muscle),
];

fn tissues(beds: &[(&'static str, Tissue)], left_out: &[Space]) -> Result<Schema, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    map.contain(root.id(), vessel.id());
    for &(key, tissue) in beds {
        let bed = map.site::<CapillaryBed>(key, at()?);
        map.contain(root.id(), bed.id());
        map.connect(Network::Vascular, vessel.id(), bed.id());
        map.connect(Network::Vascular, bed.id(), vessel.id());
        map.serves(&bed, tissue);
    }
    let mut schema = Schema::new(map.finish()?)?;
    for &space in left_out {
        schema.leave_out(space, "a bench of one wall");
    }
    Ok(schema)
}

fn still<'b, O: Owner, C: SiteSet, Q: Substance>(
    laws: &mut FragmentBuilder<'b, O>,
    route: &SiteRoute<'b, O, C, Q>,
) -> Result<(), AdmissionError> {
    let class = slow()?;
    let rate = laws.site_kernel(|mut k| {
        let zero = k.zero::<Rate<Q>>();
        k.finish(zero, laws.every_mode(), class)
    });
    laws.transfer_at(route, rate);
    Ok(())
}

fn liver_glucose(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let (mut hp, ports) = body.hepatic(None)?.parts();
    let route = ports.fuel_into_cells::<Glucose>(&mut hp);
    still(&mut hp, &route)?;
    body.add(hp.finish());
    Ok(())
}

fn fluids_and_liver(liver_first: bool) -> Result<CompiledBody, AdmissionError> {
    let mut schema = tissues(&THREE_BEDS, &[Space::Plasma, Space::Lumen])?;
    schema.carry::<Glucose, CapillaryBed>(Space::Interstitium);
    schema.carry::<Glucose, CapillaryBed>(Space::Cells);
    assemble(schema, |mut body| {
        if liver_first {
            liver_glucose(&mut body)?;
        }
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.cell_membrane::<CapillaryBed, Glucose>(&mut fl);
        still(&mut fl, &route)?;
        body.add(fl.finish());
        if !liver_first {
            liver_glucose(&mut body)?;
        }
        body.compile()
    })
}

fn glucose_crossed_twice() -> AdmissionError {
    defect(CompileDefect::CrossedTwice {
        laws: [
            law("FL", "cell membrane glucose"),
            law("HP", "cell membrane glucose"),
        ],
        site: site::<CapillaryBed>("liver bed"),
    })
}

#[test]
fn two_cards_crossing_one_wall_with_one_species_at_one_bed_is_refused() {
    assert_eq!(
        fluids_and_liver(false).map(|body| body.model_hash()),
        Err(glucose_crossed_twice())
    );
}

#[test]
fn the_metabolism_card_and_fluids_cannot_both_move_oxygen_across_the_capillary_wall()
-> Result<(), AdmissionError> {
    let mut schema = tissues(&THREE_BEDS, &[Space::Lumen, Space::Cells])?;
    schema.carry::<Oxygen, CapillaryBed>(Space::Plasma);
    schema.carry::<Oxygen, CapillaryBed>(Space::Interstitium);
    let found = assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.exchange::<Oxygen>(&mut fl);
        still(&mut fl, &route)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let route = ports.gas_exchange::<Oxygen>(&mut mt);
        still(&mut mt, &route)?;
        body.add(mt.finish());
        body.compile()
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::CrossedTwice {
            laws: [law("FL", "exchange oxygen"), law("MT", "exchange oxygen")],
            site: site::<CapillaryBed>("bone bed"),
        }))
    );
    Ok(())
}

#[test]
fn disjoint_sets_of_one_wall_and_species_take_two_cards() -> Result<(), AdmissionError> {
    let mut schema = tissues(&THREE_BEDS, &[Space::Plasma, Space::Lumen])?;
    for space in [Space::Interstitium, Space::Cells] {
        schema.carry::<Glucose, CapillaryBed>(space);
        schema.carry::<Phosphate, CapillaryBed>(space);
    }
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        let phosphate = ports.cell_membrane::<Besides<Bone>, Phosphate>(&mut fl);
        still(&mut fl, &phosphate)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let route = ports.fuel_into_cells::<Glucose>(&mut mt);
        still(&mut mt, &route)?;
        body.add(mt.finish());
        let (mut hp, ports) = body.hepatic(None)?.parts();
        let route = ports.fuel_into_cells::<Glucose>(&mut hp);
        still(&mut hp, &route)?;
        body.add(hp.finish());
        let (mut ms, ports) = body.musculoskeletal(None)?.parts();
        let route = ports.bone::<Phosphate>(&mut ms);
        still(&mut ms, &route)?;
        body.add(ms.finish());
        body.compile()
    })?;
    Ok(())
}

#[test]
fn the_crossing_refusal_names_the_same_pair_whatever_the_order() {
    assert_eq!(
        fluids_and_liver(true).map(|body| body.model_hash()),
        Err(glucose_crossed_twice())
    );
}

const MUSCLE_BED: [(&str, Tissue); 1] = [("muscle bed", Tissue::Muscle)];

fn fat_stored<Q: Substance>(
    crossing: impl for<'b> FnOnce(
        &mut FragmentBuilder<'b, Fluids>,
        &FluidsPorts<'b>,
    ) -> Result<(), AdmissionError>,
) -> Result<CompiledBody, AdmissionError> {
    let mut schema = tissues(&MUSCLE_BED, &[Space::Lumen])?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells] {
        schema.carry::<Triglyceride, CapillaryBed>(space);
        schema.carry::<Q, CapillaryBed>(space);
    }
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut fl);
        still(&mut fl, &route)?;
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        crossing(&mut fl, &ports)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let route = ports.fat_into_cells::<Beds<Muscle>>(&mut mt);
        still(&mut mt, &route)?;
        body.add(mt.finish());
        body.compile()
    })
}

fn fat_crossed_twice(fluids: &'static str) -> AdmissionError {
    defect(CompileDefect::CrossedTwice {
        laws: [law("FL", fluids), law("MT", "endothelium triglyceride")],
        site: site::<CapillaryBed>("muscle bed"),
    })
}

#[test]
fn the_endothelial_route_crosses_the_capillary_wall_another_card_crosses() {
    let found = fat_stored::<Triglyceride>(|fl, ports| {
        let route = ports.exchange::<Triglyceride>(fl);
        still(fl, &route)
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(fat_crossed_twice("exchange triglyceride"))
    );
}

#[test]
fn the_endothelial_route_crosses_the_cell_membrane_another_card_crosses() {
    let found = fat_stored::<Triglyceride>(|fl, ports| {
        let route = ports.cell_membrane::<Beds<Muscle>, Triglyceride>(fl);
        still(fl, &route)
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(fat_crossed_twice("cell membrane triglyceride"))
    );
}

#[test]
fn another_species_crosses_both_walls_the_endothelial_route_spans() -> Result<(), AdmissionError> {
    fat_stored::<Glucose>(|fl, ports| {
        let wall = ports.exchange::<Glucose>(fl);
        still(fl, &wall)?;
        let membrane = ports.cell_membrane::<CapillaryBed, Glucose>(fl);
        still(fl, &membrane)
    })?;
    Ok(())
}

#[test]
fn oxygen_crosses_the_gas_exchange_surface_and_the_capillary_wall_of_one_lung_bed_by_two_cards()
-> Result<(), AdmissionError> {
    let mut schema = tissues(&[("lung bed", Tissue::Lung)], &[Space::Lumen, Space::Cells])?;
    for space in [Space::Gas, Space::Plasma, Space::Interstitium] {
        schema.carry::<Oxygen, CapillaryBed>(space);
    }
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut fl);
        still(&mut fl, &route)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let route = ports.gas_exchange::<Oxygen>(&mut mt);
        still(&mut mt, &route)?;
        body.add(mt.finish());
        let (mut rs, ports) = body.respiratory(None)?.parts();
        let route = ports.alveolar::<Oxygen>(&mut rs);
        still(&mut rs, &route)?;
        body.add(rs.finish());
        body.compile()
    })?;
    Ok(())
}

#[test]
fn water_and_oxygen_cross_one_cell_membrane_by_two_cards() -> Result<(), AdmissionError> {
    let mut schema = tissues(&MUSCLE_BED, &[Space::Lumen, Space::Plasma])?;
    for space in [Space::Interstitium, Space::Cells] {
        schema.carry::<Oxygen, CapillaryBed>(space);
    }
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &route)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let route = ports.gas_into_cells::<Oxygen>(&mut mt);
        still(&mut mt, &route)?;
        body.add(mt.finish());
        body.compile()
    })?;
    Ok(())
}

#[test]
fn glucose_crosses_the_capillary_wall_by_fluids_and_each_beds_cell_membrane_by_its_own_card()
-> Result<(), AdmissionError> {
    let mut schema = tissues(&THREE_BEDS, &[Space::Lumen])?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells] {
        schema.carry::<Glucose, CapillaryBed>(space);
    }
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut fl);
        still(&mut fl, &route)?;
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        let glucose = ports.exchange::<Glucose>(&mut fl);
        still(&mut fl, &glucose)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let route = ports.fuel_into_cells::<Glucose>(&mut mt);
        still(&mut mt, &route)?;
        body.add(mt.finish());
        liver_glucose(&mut body)?;
        body.compile()
    })?;
    Ok(())
}

#[test]
fn one_card_crosses_both_walls_of_one_bed_with_oxygen() -> Result<(), AdmissionError> {
    let mut schema = tissues(&MUSCLE_BED, &[Space::Lumen])?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells] {
        schema.carry::<Oxygen, CapillaryBed>(space);
    }
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut fl);
        still(&mut fl, &route)?;
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let wall = ports.gas_exchange::<Oxygen>(&mut mt);
        still(&mut mt, &wall)?;
        let membrane = ports.gas_into_cells::<Oxygen>(&mut mt);
        still(&mut mt, &membrane)?;
        body.add(mt.finish());
        body.compile()
    })?;
    Ok(())
}

fn glycogen_made(glucose: Space) -> Result<CompiledBody, AdmissionError> {
    let mut schema = tissues(&MUSCLE_BED, &[Space::Plasma, Space::Lumen])?;
    for space in [Space::Interstitium, Space::Cells] {
        schema.carry::<Glucose, CapillaryBed>(space);
    }
    schema.carry::<Glycogen, CapillaryBed>(Space::Cells);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    schema.energy_content::<Glucose>(Measure::new(1_000.0)?)?;
    schema.energy_content::<Glycogen>(Measure::new(900.0)?)?;
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let route = ports.fuel_into_cells::<Glucose>(&mut mt);
        still(&mut mt, &route)?;
        body.add(mt.finish());
        let (mut ms, _) = body.musculoskeletal(None)?.parts();
        let stored = Proportions::<Glucose>::using(glucose)
            .makes::<Glycogen, 1>(Space::Cells)
            .makes::<Water, 1>(Space::Cells)
            .energy_into(Space::Thermal);
        let made = ms.reaction::<Beds<Muscle>, Glucose, _>("glycogen made", stored);
        let class = slow()?;
        let rate = ms.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Glucose>>();
            k.finish(zero, ms.every_mode(), class)
        });
        ms.react_at(&made, rate);
        body.add(ms.finish());
        body.compile()
    })
}

#[test]
fn a_reaction_that_takes_its_glucose_across_the_cell_membrane_is_refused_naming_both_spaces()
-> Result<(), AdmissionError> {
    glycogen_made(Space::Cells)?;
    assert_eq!(
        glycogen_made(Space::Interstitium).map(|body| body.model_hash()),
        Err(defect(CompileDefect::ReactionInTwoSpaces {
            law: law("MS", "glycogen made"),
            spaces: [Space::Interstitium, Space::Cells],
        }))
    );
    Ok(())
}

fn react<'b, O: ReactsAt<C>, C: SiteSet, Q: Reactant, const N: u32>(
    laws: &mut FragmentBuilder<'b, O>,
    name: &'static str,
    proportions: Proportions<Q, N>,
) -> Result<(), AdmissionError> {
    let made = laws.reaction::<C, Q, N>(name, proportions);
    let class = slow()?;
    let rate = laws.site_kernel(|mut k| {
        let zero = k.zero::<Rate<Q>>();
        k.finish(zero, laws.every_mode(), class)
    });
    laws.react_at(&made, rate);
    Ok(())
}

fn stored<const N: u32>() -> Proportions<Glucose, N> {
    Proportions::<Glucose, N>::using(Space::Cells)
        .makes::<Glycogen, N>(Space::Cells)
        .makes::<Water, N>(Space::Cells)
        .energy_into(Space::Thermal)
}

fn burned() -> Proportions<Glucose> {
    Proportions::<Glucose>::using(Space::Cells)
        .uses::<Oxygen, 6>(Space::Cells)
        .makes::<CarbonDioxide, 6>(Space::Cells)
        .makes::<Water, 6>(Space::Cells)
        .energy_into(Space::Thermal)
}

const BOTH_BEDS: [(&str, Tissue); 2] = [("liver bed", Tissue::Liver), ("muscle bed", Tissue::Muscle)];

fn chemistry(
    mut schema: Schema,
    glycogen_at: &[&'static str],
    cards: impl for<'b> FnOnce(&mut Assembly<'b>) -> Result<(), AdmissionError>,
) -> Result<CompiledBody, AdmissionError> {
    for space in [Space::Interstitium, Space::Cells] {
        schema.carry::<Glucose, CapillaryBed>(space);
    }
    schema.carry::<Oxygen, CapillaryBed>(Space::Cells);
    schema.carry::<CarbonDioxide, CapillaryBed>(Space::Cells);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    for &key in glycogen_at {
        schema.pool::<Glycogen>(site::<CapillaryBed>(key), Space::Cells)?;
    }
    schema.energy_content::<Glucose>(Measure::new(1_000.0)?)?;
    schema.energy_content::<Glycogen>(Measure::new(900.0)?)?;
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        body.add(fl.finish());
        cards(&mut body)?;
        body.compile()
    })
}

fn cell_schema(beds: &[(&'static str, Tissue)]) -> Result<Schema, AdmissionError> {
    tissues(beds, &[Space::Plasma, Space::Lumen])
}

fn metabolism<'b>(
    body: &mut Assembly<'b>,
    reactions: impl FnOnce(&mut FragmentBuilder<'b, Metabolism>) -> Result<(), AdmissionError>,
) -> Result<(), AdmissionError> {
    let (mut mt, ports) = body.metabolism(None)?.parts();
    let route = ports.fuel_into_cells::<Glucose>(&mut mt);
    still(&mut mt, &route)?;
    reactions(&mut mt)?;
    body.add(mt.finish());
    Ok(())
}

fn hepatic<'b>(
    body: &mut Assembly<'b>,
    reactions: impl FnOnce(&mut FragmentBuilder<'b, Hepatic>) -> Result<(), AdmissionError>,
) -> Result<(), AdmissionError> {
    let (mut hp, ports) = body.hepatic(None)?.parts();
    let route = ports.fuel_into_cells::<Glucose>(&mut hp);
    still(&mut hp, &route)?;
    reactions(&mut hp)?;
    body.add(hp.finish());
    Ok(())
}

fn musculoskeletal<'b, const N: u32>(
    body: &mut Assembly<'b>,
    made: Proportions<Glucose, N>,
) -> Result<(), AdmissionError> {
    let (mut ms, _) = body.musculoskeletal(None)?.parts();
    react::<_, Beds<Muscle>, _, N>(&mut ms, "glycogen made", made)?;
    body.add(ms.finish());
    Ok(())
}

fn stores_and_burns(mt: &mut FragmentBuilder<'_, Metabolism>) -> Result<(), AdmissionError> {
    react::<_, CapillaryBed, _, 1>(mt, "glycogen made", stored())?;
    react::<_, CapillaryBed, _, 1>(mt, "burn", burned())
}

fn burns(mt: &mut FragmentBuilder<'_, Metabolism>) -> Result<(), AdmissionError> {
    react::<_, CapillaryBed, _, 1>(mt, "burn", burned())
}

#[test]
fn the_musculoskeletal_card_making_metabolisms_glycogen_at_the_muscle_bed_is_refused_naming_both_laws_and_the_bed()
-> Result<(), AdmissionError> {
    let found = chemistry(cell_schema(&MUSCLE_BED)?, &["muscle bed"], |body| {
        metabolism(body, stores_and_burns)?;
        musculoskeletal(body, stored::<1>())
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::ReactionOwnedTwice {
            laws: [law("MS", "glycogen made"), law("MT", "glycogen made")],
            site: site::<CapillaryBed>("muscle bed"),
        }))
    );
    Ok(())
}

#[test]
fn the_hepatic_card_burning_glucose_as_metabolism_does_at_the_liver_bed_is_refused()
-> Result<(), AdmissionError> {
    let found = chemistry(cell_schema(&BOTH_BEDS)?, &["muscle bed"], |body| {
        metabolism(body, burns)?;
        hepatic(body, |hp| {
            react::<_, Beds<Liver>, _, 1>(hp, "storage burn", burned())
        })?;
        musculoskeletal(body, stored::<1>())
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::ReactionOwnedTwice {
            laws: [law("HP", "storage burn"), law("MT", "burn")],
            site: site::<CapillaryBed>("liver bed"),
        }))
    );
    Ok(())
}

#[test]
fn a_copy_with_every_count_doubled_is_the_same_chemistry_and_is_refused() -> Result<(), AdmissionError> {
    let found = chemistry(cell_schema(&MUSCLE_BED)?, &["muscle bed"], |body| {
        metabolism(body, stores_and_burns)?;
        musculoskeletal(body, stored::<2>())
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::ReactionOwnedTwice {
            laws: [law("MS", "glycogen made"), law("MT", "glycogen made")],
            site: site::<CapillaryBed>("muscle bed"),
        }))
    );
    Ok(())
}

#[test]
fn the_hepatic_cards_storage_beside_metabolisms_burn_at_the_liver_bed_compiles() -> Result<(), AdmissionError>
{
    chemistry(cell_schema(&BOTH_BEDS)?, &["liver bed"], |body| {
        metabolism(body, burns)?;
        hepatic(body, |hp| {
            react::<_, Beds<Liver>, _, 1>(hp, "glycogen made", stored())
        })
    })?;
    Ok(())
}

#[test]
fn one_card_declaring_one_chemistry_under_two_names_compiles() -> Result<(), AdmissionError> {
    chemistry(cell_schema(&MUSCLE_BED)?, &["muscle bed"], |body| {
        metabolism(body, |mt| {
            stores_and_burns(mt)?;
            react::<_, CapillaryBed, _, 2>(mt, "glycogen kept", stored())
        })
    })?;
    Ok(())
}

#[test]
fn two_cards_with_one_chemistry_at_two_beds_compile() -> Result<(), AdmissionError> {
    chemistry(cell_schema(&BOTH_BEDS)?, &BOTH_BEDS.map(|bed| bed.0), |body| {
        metabolism(body, burns)?;
        hepatic(body, |hp| {
            react::<_, Beds<Liver>, _, 1>(hp, "glycogen made", stored())
        })?;
        musculoskeletal(body, stored::<1>())
    })?;
    Ok(())
}

#[test]
fn a_grant_shared_where_one_chemistry_has_no_pools_is_no_second_owner() -> Result<(), AdmissionError> {
    let found = chemistry(cell_schema(&BOTH_BEDS)?, &["liver bed"], |body| {
        metabolism(body, stores_and_burns)?;
        hepatic(body, |_| Ok(()))?;
        musculoskeletal(body, stored::<1>())
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::ReactionNowhere {
            law: law("MS", "glycogen made"),
        }))
    );
    Ok(())
}

fn lactate_in_cells(beds: &[(&'static str, Tissue)], content: f64) -> Result<Schema, AdmissionError> {
    let mut schema = cell_schema(beds)?;
    schema.carry::<Lactate, CapillaryBed>(Space::Cells);
    schema.energy_content::<Lactate>(Measure::new(content)?)?;
    Ok(schema)
}

fn glycolysis() -> Proportions<Glucose> {
    Proportions::<Glucose>::using(Space::Cells).makes::<Lactate, 2>(Space::Cells)
}

fn beside_glycolysis(made: Proportions<Glucose>) -> Result<CompiledBody, AdmissionError> {
    chemistry(lactate_in_cells(&BOTH_BEDS, 400.0)?, &[], |body| {
        metabolism(body, |mt| {
            burns(mt)?;
            react::<_, CapillaryBed, _, 1>(mt, "glycolysis", glycolysis().energy_into(Space::Thermal))
        })?;
        hepatic(body, |hp| {
            react::<_, Beds<Liver>, _, 1>(hp, "glucose from lactate", made)
        })
    })
}

#[test]
fn the_hepatic_card_writing_metabolisms_glycolysis_in_reverse_at_the_liver_bed_is_refused_naming_both_laws_and_the_bed()
-> Result<(), AdmissionError> {
    let reverse = Proportions::<Glucose>::making(Space::Cells)
        .uses::<Lactate, 2>(Space::Cells)
        .energy_into(Space::Thermal);
    assert_eq!(
        beside_glycolysis(reverse).map(|body| body.model_hash()),
        Err(defect(CompileDefect::ReactionOwnedTwice {
            laws: [law("HP", "glucose from lactate"), law("MT", "glycolysis")],
            site: site::<CapillaryBed>("liver bed"),
        }))
    );
    Ok(())
}

#[test]
fn the_hepatic_cards_glucose_from_lactate_with_its_own_fuel_terms_beside_metabolisms_glycolysis_compiles()
-> Result<(), AdmissionError> {
    let forward = Proportions::<Glucose>::making(Space::Cells)
        .uses::<Lactate, 3>(Space::Cells)
        .uses::<Oxygen, 3>(Space::Cells)
        .makes::<CarbonDioxide, 3>(Space::Cells)
        .makes::<Water, 3>(Space::Cells)
        .energy_into(Space::Thermal);
    beside_glycolysis(forward)?;
    Ok(())
}

#[test]
fn two_cards_with_one_chemistry_that_leaves_no_energy_are_refused_though_only_one_names_an_energy_space()
-> Result<(), AdmissionError> {
    let found = chemistry(lactate_in_cells(&MUSCLE_BED, 500.0)?, &[], |body| {
        metabolism(body, |mt| {
            burns(mt)?;
            react::<_, CapillaryBed, _, 1>(mt, "glycolysis", glycolysis().energy_into(Space::Thermal))
        })?;
        let (mut ms, _) = body.musculoskeletal(None)?.parts();
        react::<_, Beds<Muscle>, _, 1>(&mut ms, "glycolysis", glycolysis())?;
        body.add(ms.finish());
        Ok(())
    });
    assert_eq!(
        found.map(|body| body.model_hash()),
        Err(defect(CompileDefect::ReactionOwnedTwice {
            laws: [law("MS", "glycolysis"), law("MT", "glycolysis")],
            site: site::<CapillaryBed>("muscle bed"),
        }))
    );
    Ok(())
}

fn still_along<'b, O: Owner, Q: Substance, E: EdgeKind>(
    laws: &mut FragmentBuilder<'b, O>,
    route: &EdgeRoute<'b, O, Q, E>,
) -> Result<(), AdmissionError> {
    let class = slow()?;
    let rate = laws.edge_kernel::<E, _, _>(|mut k| {
        let zero = k.zero::<Rate<Q>>();
        k.finish(zero, laws.every_mode(), class)
    });
    laws.transfer_along(route, rate);
    Ok(())
}

fn moves(body: &CompiledBody, owner: &str) -> Vec<String> {
    let pool = |row: RowId| match body.rows()[row.index()].kind() {
        RowKind::Amount { site, space, species } => (species, format!("{} {space:?}", site.key())),
        other => panic!("a transfer moves amounts, not {other:?}"),
    };
    let mut found: Vec<String> = (body.columns().iter())
        .filter(|column| column.origin().owner == owner)
        .map(|column| {
            let ((cargo, from), (moved, to)) = match column.terms() {
                [(from, -1), (to, 1)] | [(to, 1), (from, -1)] => (pool(*from), pool(*to)),
                terms => panic!("{:?} has the terms {terms:?}", column.origin()),
            };
            assert_eq!(cargo, moved, "{:?} moves one species", column.origin());
            format!("{}: {cargo:?} {from} -> {to}", column.origin().local)
        })
        .collect();
    found.sort_unstable();
    found
}

fn perfuse(map: &mut AnatomyBuilder, root: SiteId, bed: SiteId) -> Result<(), AdmissionError> {
    let vessel = map.site::<VesselSegment>("vessel", at()?).id();
    map.contain(root, vessel);
    map.connect(Network::Vascular, vessel, bed);
    map.connect(Network::Vascular, bed, vessel);
    Ok(())
}

fn breathing() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let airway = map.site::<TissueSite>("airway", at()?);
    let lung = map.site::<CapillaryBed>("lung bed", at()?);
    for child in [airway.id(), lung.id()] {
        map.contain(root.id(), child);
    }
    perfuse(&mut map, root.id(), lung.id())?;
    map.connect(Network::Airway, airway.id(), lung.id());
    map.serves(&lung, Tissue::Lung);
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Lumen, "a bench whose blood stands still");
    schema.carry::<Oxygen, Airway>(Space::Gas);
    for space in [Space::Plasma, Space::Interstitium] {
        schema.carry::<Oxygen, CapillaryBed>(space);
    }
    schema.carry::<Water, CapillaryBed>(Space::Gas);
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let starling = ports.starling(&mut fl);
        still(&mut fl, &starling)?;
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        body.add(fl.finish());
        let (mut rs, ports) = body.respiratory(None)?.parts();
        let alveolar = ports.alveolar::<Oxygen>(&mut rs);
        still(&mut rs, &alveolar)?;
        let airway = ports.airway::<Oxygen>(&mut rs);
        still_along(&mut rs, &airway)?;
        let lining = ports.lining(&mut rs);
        still(&mut rs, &lining)?;
        body.add(rs.finish());
        body.compile()
    })
}

fn digesting() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let stomach = map.site::<VesselSegment>("stomach", at()?);
    let intestine = map.site::<VesselSegment>("intestine", at()?);
    let gut = map.site::<CapillaryBed>("gut bed", at()?);
    for child in [stomach.id(), intestine.id(), gut.id()] {
        map.contain(root.id(), child);
    }
    perfuse(&mut map, root.id(), gut.id())?;
    map.connect(Network::Gut, stomach.id(), intestine.id());
    map.connect(Network::Gut, intestine.id(), gut.id());
    map.serves(&gut, Tissue::Gut);
    let mut schema = Schema::new(map.finish()?)?;
    for (site, space) in [(intestine.id(), Space::Lumen), (gut.id(), Space::Interstitium)] {
        schema.pool::<Triglyceride>(site, space)?;
    }
    for (site, space) in [(gut.id(), Space::Cells), (intestine.id(), Space::Lumen)] {
        schema.pool::<Iron>(site, space)?;
    }
    assemble(schema, |mut body| {
        circulation(&mut body)?;
        let (mut fl, ports) = body.fluids()?.parts();
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        body.add(fl.finish());
        let (mut gi, ports) = body.gastrointestinal(None)?.parts();
        let transit = ports.transit::<Water>(&mut gi);
        still_along(&mut gi, &transit)?;
        let absorption = ports.absorption::<Water>(&mut gi);
        still_along(&mut gi, &absorption)?;
        let chyle = ports.chyle(&mut gi);
        still_along(&mut gi, &chyle)?;
        let shedding = ports.shedding(&mut gi);
        still_along(&mut gi, &shedding)?;
        body.add(gi.finish());
        body.compile()
    })
}

fn cells() -> Result<CompiledBody, AdmissionError> {
    let beds = [
        ("bone bed", Tissue::Bone),
        ("gut bed", Tissue::Gut),
        ("liver bed", Tissue::Liver),
    ];
    let mut schema = tissues(&beds, &[Space::Plasma, Space::Lumen])?;
    for space in [Space::Interstitium, Space::Cells] {
        schema.carry::<Oxygen, CapillaryBed>(space);
        schema.carry::<Glucose, CapillaryBed>(space);
        schema.carry::<Calcium, CapillaryBed>(space);
        for bed in ["gut bed", "liver bed"] {
            schema.pool::<Iron>(site::<CapillaryBed>(bed), space)?;
        }
    }
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        let calcium = ports.cell_membrane::<Besides<Bone>, Calcium>(&mut fl);
        still(&mut fl, &calcium)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let gas = ports.gas_into_cells::<Oxygen>(&mut mt);
        still(&mut mt, &gas)?;
        let fuel = ports.fuel_into_cells::<Glucose>(&mut mt);
        still(&mut mt, &fuel)?;
        body.add(mt.finish());
        liver_glucose(&mut body)?;
        let (mut ms, ports) = body.musculoskeletal(None)?.parts();
        let mineral = ports.bone::<Calcium>(&mut ms);
        still(&mut ms, &mineral)?;
        body.add(ms.finish());
        let (mut hc, ports) = body.hematology(None)?.parts();
        let iron = ports.iron_into_cells(&mut hc);
        still(&mut hc, &iron)?;
        body.add(hc.finish());
        body.compile()
    })
}

fn walls() -> Result<CompiledBody, AdmissionError> {
    let beds = [("muscle bed", Tissue::Muscle), ("skin bed", Tissue::Skin)];
    let mut schema = tissues(&beds, &[Space::Lumen])?;
    for space in [Space::Plasma, Space::Interstitium] {
        schema.carry::<Oxygen, CapillaryBed>(space);
    }
    for space in [Space::Plasma, Space::Interstitium, Space::Cells] {
        schema.pool::<Triglyceride>(site::<CapillaryBed>("muscle bed"), space)?;
    }
    assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let starling = ports.starling(&mut fl);
        still(&mut fl, &starling)?;
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        body.add(fl.finish());
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let gas = ports.gas_exchange::<Oxygen>(&mut mt);
        still(&mut mt, &gas)?;
        let fat = ports.fat_into_cells::<Beds<Muscle>>(&mut mt);
        still(&mut mt, &fat)?;
        body.add(mt.finish());
        body.compile()
    })
}

fn heat() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let skin = map.site::<TissueSite>("skin", at()?);
    let bed = map.site::<CapillaryBed>("muscle bed", at()?);
    for child in [skin.id(), bed.id()] {
        map.contain(root.id(), child);
    }
    perfuse(&mut map, root.id(), bed.id())?;
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of heat alone");
    }
    for site in [root.id(), skin.id(), bed.id()] {
        schema.pool::<Energy>(site, Space::Thermal)?;
    }
    assemble(schema, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let from_beds = ports.bed_heat(&mut mt);
        still(&mut mt, &from_beds)?;
        let conducted = ports.conduction(&mut mt);
        still(&mut mt, &conducted)?;
        body.add(mt.finish());
        body.compile()
    })
}

fn liver_flow() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let artery = map.site::<VesselSegment>("artery", at()?);
    let vein = map.site::<VesselSegment>("vein", at()?);
    let liver = map.site::<CapillaryBed>("liver bed", at()?);
    for child in [artery.id(), vein.id(), liver.id()] {
        map.contain(root.id(), child);
    }
    for (from, to) in [
        (artery.id(), liver.id()),
        (liver.id(), vein.id()),
        (vein.id(), artery.id()),
    ] {
        map.connect(Network::Vascular, from, to);
    }
    map.serves(&liver, Tissue::Liver);
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Interstitium, Space::Cells] {
        schema.leave_out(space, "a bench of blood alone");
    }
    assemble(schema, |mut body| {
        circulation(&mut body)?;
        let (mut hp, ports) = body.hepatic(None)?.parts();
        let sinusoids = ports.sinusoids(&mut hp);
        still_along(&mut hp, &sinusoids)?;
        body.add(hp.finish());
        body.compile()
    })
}

#[test]
fn each_port_moves_its_cargo_between_the_two_pools_its_route_names() -> Result<(), AdmissionError> {
    assert_eq!(
        moves(&breathing()?, "RS"),
        [
            "airway oxygen: Oxygen airway Gas -> lung bed Gas",
            "alveolar oxygen: Oxygen lung bed Gas -> lung bed Plasma",
            "lining water: Water lung bed Interstitium -> lung bed Gas",
        ]
    );
    assert_eq!(
        moves(&digesting()?, "GI"),
        [
            "absorb water: Water intestine Lumen -> gut bed Plasma",
            "chyle triglyceride: Triglyceride intestine Lumen -> gut bed Interstitium",
            "shed iron: Iron gut bed Cells -> intestine Lumen",
            "transit water: Water stomach Lumen -> intestine Lumen",
        ]
    );
    let body = cells()?;
    assert_eq!(
        moves(&body, "MT"),
        [
            "cell membrane glucose: Glucose bone bed Interstitium -> bone bed Cells",
            "cell membrane glucose: Glucose gut bed Interstitium -> gut bed Cells",
            "cell membrane oxygen: Oxygen bone bed Interstitium -> bone bed Cells",
            "cell membrane oxygen: Oxygen gut bed Interstitium -> gut bed Cells",
            "cell membrane oxygen: Oxygen liver bed Interstitium -> liver bed Cells",
        ]
    );
    assert_eq!(
        moves(&body, "HP"),
        ["cell membrane glucose: Glucose liver bed Interstitium -> liver bed Cells"]
    );
    assert_eq!(
        moves(&body, "MS"),
        ["cell membrane calcium: Calcium bone bed Interstitium -> bone bed Cells"]
    );
    assert_eq!(
        moves(&body, "HC"),
        [
            "cell membrane iron: Iron gut bed Interstitium -> gut bed Cells",
            "cell membrane iron: Iron liver bed Interstitium -> liver bed Cells",
        ]
    );
    assert_eq!(
        moves(&walls()?, "MT"),
        [
            "endothelium triglyceride: Triglyceride muscle bed Plasma -> muscle bed Cells",
            "exchange oxygen: Oxygen muscle bed Plasma -> muscle bed Interstitium",
            "exchange oxygen: Oxygen skin bed Plasma -> skin bed Interstitium",
        ]
    );
    assert_eq!(
        moves(&heat()?, "MT"),
        [
            "bed heat: Energy muscle bed Thermal -> body Thermal",
            "conduction: Energy body Thermal -> skin Thermal",
        ]
    );
    assert_eq!(
        moves(&liver_flow()?, "HP"),
        [
            "sinusoids: Water artery Lumen -> liver bed Plasma",
            "sinusoids: Water liver bed Plasma -> vein Lumen",
        ]
    );
    Ok(())
}

#[test]
fn hematology_crosses_iron_into_cells_only_at_the_beds_whose_cells_hold_iron() -> Result<(), AdmissionError> {
    let beds = [
        ("gut bed", Tissue::Gut),
        ("liver bed", Tissue::Liver),
        ("muscle bed", Tissue::Muscle),
    ];
    let mut schema = tissues(&beds, &[Space::Lumen])?;
    for space in [Space::Plasma, Space::Interstitium] {
        schema.carry::<Iron, CapillaryBed>(space);
    }
    for bed in ["gut bed", "liver bed"] {
        schema.pool::<Iron>(site::<CapillaryBed>(bed), Space::Cells)?;
    }
    let body = assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let starling = ports.starling(&mut fl);
        still(&mut fl, &starling)?;
        let water = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        still(&mut fl, &water)?;
        body.add(fl.finish());
        let (mut hc, ports) = body.hematology(None)?.parts();
        let iron = ports.iron_into_cells(&mut hc);
        still(&mut hc, &iron)?;
        body.add(hc.finish());
        body.compile()
    })?;
    assert_eq!(
        moves(&body, "HC"),
        [
            "cell membrane iron: Iron gut bed Interstitium -> gut bed Cells",
            "cell membrane iron: Iron liver bed Interstitium -> liver bed Cells",
        ]
    );
    Ok(())
}
