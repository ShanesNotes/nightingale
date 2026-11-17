use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = at().expect("finite");
    AnatomyBuilder::new().site::<C>(key, origin).id()
}

fn wave() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Wave, 1_000_000_000)
}

const BEDS: [&str; 5] = ["bare bed", "gut bed", "liver bed", "lung bed", "skin bed"];

fn whole_body() -> Result<Anatomy, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at()?);
    let airway = map.site::<TissueSite>("conducting airway", at()?);
    let skin = map.site::<TissueSite>("skin heat site", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let stomach = map.site::<VesselSegment>("stomach", at()?);
    let intestine = map.site::<VesselSegment>("small intestine", at()?);
    let lung = map.site::<CapillaryBed>("lung bed", at()?);
    let liver = map.site::<CapillaryBed>("liver bed", at()?);
    let gut = map.site::<CapillaryBed>("gut bed", at()?);
    let skin_bed = map.site::<CapillaryBed>("skin bed", at()?);
    let bare = map.site::<CapillaryBed>("bare bed", at()?);
    let beds = [lung.id(), liver.id(), gut.id(), skin_bed.id(), bare.id()];
    for site in [airway.id(), skin.id(), vessel.id(), stomach.id(), intestine.id()] {
        map.contain(body.id(), site);
    }
    for bed in beds {
        let holder = if bed == skin_bed.id() {
            skin.id()
        } else {
            body.id()
        };
        map.contain(holder, bed);
        map.connect(Network::Vascular, vessel.id(), bed);
        map.connect(Network::Vascular, bed, vessel.id());
    }
    for (bed, tissue) in [
        (&lung, Tissue::Lung),
        (&liver, Tissue::Liver),
        (&gut, Tissue::Gut),
        (&skin_bed, Tissue::Skin),
    ] {
        map.serves(bed, tissue);
    }
    map.connect(Network::Airway, airway.id(), lung.id());
    map.connect(Network::Gut, stomach.id(), intestine.id());
    map.connect(Network::Gut, intestine.id(), gut.id());
    map.finish()
}

fn holders(schema: &Schema, species: Species) -> Vec<(&'static str, Space)> {
    (schema.pools())
        .filter(|pool| pool.2 == species)
        .map(|(site, space, _)| (site.key(), space))
        .collect()
}

#[test]
fn each_site_set_holds_the_sites_the_map_places_on_it() -> Result<(), AdmissionError> {
    let mut schema = Schema::new(whole_body()?)?;
    schema.carry::<Glucose, Alimentary>(Space::Lumen);
    schema.carry::<NitrogenGas, Airway>(Space::Gas);
    schema.carry::<Glycogen, Beds<Liver>>(Space::Cells);
    schema.carry::<BodyProtein, Besides<Liver>>(Space::Cells);
    assert_eq!(
        holders(&schema, Species::Glucose),
        [("small intestine", Space::Lumen), ("stomach", Space::Lumen)]
    );
    assert_eq!(
        holders(&schema, Species::NitrogenGas),
        [("conducting airway", Space::Gas), ("lung bed", Space::Gas)]
    );
    assert_eq!(holders(&schema, Species::Glycogen), [("liver bed", Space::Cells)]);
    assert_eq!(
        holders(&schema, Species::BodyProtein),
        [
            ("bare bed", Space::Cells),
            ("gut bed", Space::Cells),
            ("lung bed", Space::Cells),
            ("skin bed", Space::Cells),
        ]
    );
    Ok(())
}

fn breathed<Q: Substance>() -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at()?);
    let airway = map.site::<TissueSite>("conducting airway", at()?).id();
    map.contain(body.id(), airway);
    let mut schema = Schema::new(map.finish()?)?;
    schema.pool::<Q>(airway, Space::Gas)?;
    schema.boundary::<Q>("inspired", BoundaryDirection::IntoBody, airway, Space::Gas)?;
    let class = wave()?;
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryRoute { route, .. } = ports.route::<Q>("inspired")?;
        let rate = laws.kernel(|mut k| {
            let zero = k.zero::<Rate<Q>>();
            k.finish(zero, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn an_airway_sites_gas_holds_a_gas_and_refuses_what_may_not_sit_in_gas() -> Result<(), AdmissionError> {
    breathed::<Oxygen>()?;
    assert_eq!(
        breathed::<Glucose>().err(),
        Some(AdmissionError::Compile(CompileDefect::OutOfPlace {
            site: site::<TissueSite>("conducting airway"),
            space: Space::Gas,
            species: Species::Glucose,
        }))
    );
    Ok(())
}

fn watery_spaces_left_out(schema: &mut Schema) {
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of energy alone");
    }
}

type Placed = (Context, Option<&'static str>, Vec<(&'static str, Space, i64)>);

fn columns(compiled: &CompiledBody, local: &str) -> Vec<Placed> {
    (compiled.columns().iter())
        .filter(|column| column.origin().local == local)
        .map(|column| {
            let mut ends: Vec<(&'static str, Space, i64)> = (column.terms().iter())
                .map(|&(row, coefficient)| match compiled.rows()[row.index()].kind() {
                    RowKind::Amount { site, space, .. } => (site.key(), space, coefficient),
                    other => panic!("an amount row, not {other:?}"),
                })
                .collect();
            ends.sort_by_key(|end| end.2);
            let receipt = column.receipt().map(|receipt| receipt.boundary());
            (column.context(), receipt, ends)
        })
        .collect()
}

#[test]
fn the_cell_membrane_moves_a_species_from_each_beds_interstitium_to_its_cells() -> Result<(), AdmissionError>
{
    let mut schema = Schema::new(whole_body()?)?;
    for space in [Space::Plasma, Space::Lumen] {
        schema.leave_out(space, "the cell membrane alone");
    }
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        let still = fl.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, fl.every_mode(), class)
        });
        fl.transfer_at(&route, still);
        body.add(fl.finish());
        body.compile()
    })?;
    let crossed = BEDS.map(|bed| {
        let ends = vec![(bed, Space::Interstitium, -1), (bed, Space::Cells, 1)];
        (Context::Site(site::<CapillaryBed>(bed)), None, ends)
    });
    assert_eq!(columns(&compiled, "cell membrane water"), crossed);
    Ok(())
}

#[test]
fn the_pumps_work_turns_chemical_energy_into_each_beds_heat() -> Result<(), AdmissionError> {
    let mut schema = Schema::new(whole_body()?)?;
    watery_spaces_left_out(&mut schema);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.pump_work(&mut fl);
        let still = fl.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, fl.every_mode(), class)
        });
        fl.transfer_at(&route, still);
        body.add(fl.finish());
        body.compile()
    })?;
    let paid = BEDS.map(|bed| {
        let ends = vec![(bed, Space::Thermal, 1)];
        (
            Context::Site(site::<CapillaryBed>(bed)),
            Some("chemical energy"),
            ends,
        )
    });
    assert_eq!(columns(&compiled, "pump work"), paid);
    Ok(())
}

#[test]
fn blood_carries_heat_from_the_core_store_to_the_store_a_bed_sits_in_at_that_bed()
-> Result<(), AdmissionError> {
    let mut schema = Schema::new(whole_body()?)?;
    watery_spaces_left_out(&mut schema);
    for heat_site in ["body", "skin heat site"] {
        schema.pool::<Energy>(site::<TissueSite>(heat_site), Space::Thermal)?;
    }
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.heat_carried(&mut cv);
        let still = cv.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_at(&route, still);
        body.add(cv.finish());
        body.compile()
    })?;
    let carried = (
        Context::Site(site::<CapillaryBed>("skin bed")),
        None,
        vec![
            ("body", Space::Thermal, -1),
            ("skin heat site", Space::Thermal, 1),
        ],
    );
    assert_eq!(columns(&compiled, "heat carried by blood"), [carried]);
    Ok(())
}

#[test]
fn the_tubules_cells_secrete_into_the_lumen_against_each_reclaim_edge() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let tubule = map.site::<VesselSegment>("tubule", at()?).id();
    let bed = map.site::<CapillaryBed>("peritubular bed", at()?).id();
    for (from, to) in [(root.id(), tubule), (root.id(), bed)] {
        map.contain(from, to);
    }
    map.connect(Network::Vascular, tubule, bed);
    map.connect(Network::Vascular, bed, tubule);
    map.connect(Network::Reclaim, tubule, bed);
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Plasma, Space::Interstitium] {
        schema.leave_out(space, "secretion alone");
    }
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut rn, ports) = body.renal()?.parts();
        let route = ports.secretion::<Water>(&mut rn);
        let still = rn.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, rn.every_mode(), class)
        });
        rn.transfer_along(&route, still);
        body.add(rn.finish());
        body.compile()
    })?;
    let secreted = (
        Context::Edge {
            from: tubule,
            to: bed,
        },
        None,
        vec![("peritubular bed", Space::Cells, -1), ("tubule", Space::Lumen, 1)],
    );
    assert_eq!(columns(&compiled, "secrete water"), [secreted]);
    Ok(())
}

#[test]
fn one_owner_crossing_one_wall_for_one_species_at_two_sets_is_named_twice() -> Result<(), AdmissionError> {
    let compiled = assemble(Schema::new(whole_body()?)?, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        ports.cell_membrane::<Beds<Liver>, Water>(&mut fl);
        ports.cell_membrane::<Besides<Liver>, Water>(&mut fl);
        body.add(fl.finish());
        body.compile()
    });
    let law = LawName {
        owner: <Fluids as Owner>::CARD,
        local: "cell membrane water",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::NamedTwice { law }))
    );
    Ok(())
}

fn paid_work(payers: &[(Work, &'static str)], requested: bool) -> Result<CompiledBody, AdmissionError> {
    let mut schema = Schema::new(whole_body()?)?;
    watery_spaces_left_out(&mut schema);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    if requested {
        schema.pool::<Energy>(site::<TissueSite>("body"), Space::Elastic)?;
    }
    for &(work, bed) in payers {
        schema.pay_work(work, site::<CapillaryBed>(bed))?;
    }
    let class = wave()?;
    assemble(schema, |mut body| {
        if requested {
            let (mut cv, ports) = body.cardiovascular()?.parts();
            let stroke = ports.stroke_work(&mut cv);
            let still = cv.site_kernel(|mut k| {
                let zero = k.zero::<Rate<Energy>>();
                k.finish(zero, cv.every_mode(), class)
            });
            cv.transfer_at(&stroke, still);
            let walls = ports.wall_work(&mut cv);
            let none = cv.kernel(|mut k| {
                let zero = k.zero::<Rate<Energy>>();
                k.finish(zero, cv.every_mode(), class)
            });
            cv.transfer(Transfer::new(&walls, none));
            body.add(cv.finish());
        }
        let (mut fl, ports) = body.fluids()?.parts();
        let pumps = ports.pump_work(&mut fl);
        let still = fl.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, fl.every_mode(), class)
        });
        fl.transfer_at(&pumps, still);
        body.add(fl.finish());
        body.compile()
    })
}

#[test]
fn a_named_bed_pays_the_heart_and_the_walls_from_its_heat_and_the_pumps_keep_their_receipt()
-> Result<(), AdmissionError> {
    let unnamed = paid_work(&[], true)?;
    let named = paid_work(&[(Work::Stroke, "liver bed"), (Work::Walls, "gut bed")], true)?;
    let elastic = ("body", Space::Elastic, 1);
    let receipt = Some("chemical energy");
    let stroke = columns(&unnamed, "stroke work");
    assert!(!stroke.is_empty());
    for (context, paid, ends) in &stroke {
        assert!(matches!(context, Context::Site(_)));
        assert_eq!((*paid, ends.as_slice()), (receipt, [elastic].as_slice()));
    }
    let from_liver = (stroke.iter())
        .map(|(context, ..)| (*context, None, vec![("liver bed", Space::Thermal, -1), elastic]))
        .collect::<Vec<_>>();
    assert_eq!(columns(&named, "stroke work"), from_liver);
    assert_eq!(
        columns(&unnamed, "wall work"),
        [(Context::Body, receipt, vec![elastic])]
    );
    let from_gut = (
        Context::Body,
        None,
        vec![("gut bed", Space::Thermal, -1), elastic],
    );
    assert_eq!(columns(&named, "wall work"), [from_gut]);
    assert_eq!(columns(&named, "pump work"), columns(&unnamed, "pump work"));
    assert_eq!(columns(&named, "pump work").len(), BEDS.len());
    Ok(())
}

#[test]
fn a_payer_must_be_a_bed_on_the_map_named_once_for_each_work_and_hold_heat() -> Result<(), AdmissionError> {
    let mut schema = Schema::new(whole_body()?)?;
    let (off, vessel) = (
        site::<CapillaryBed>("no such bed"),
        site::<VesselSegment>("vessel"),
    );
    let (liver, gut) = (site::<CapillaryBed>("liver bed"), site::<CapillaryBed>("gut bed"));
    assert_eq!(
        schema.pay_work(Work::Stroke, off),
        Err(AdmissionError::SiteOffTheMap { site: off })
    );
    assert_eq!(
        schema.pay_work(Work::Stroke, vessel),
        Err(AdmissionError::PayerNotABed { site: vessel })
    );
    schema.pay_work(Work::Stroke, liver)?;
    let twice = Err(AdmissionError::WorkPaidTwice { work: Work::Stroke });
    assert_eq!(schema.pay_work(Work::Stroke, gut), twice);
    schema.pay_work(Work::Walls, liver)?;
    watery_spaces_left_out(&mut schema);
    schema.pool::<Energy>(site::<TissueSite>("body"), Space::Elastic)?;
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let walls = ports.wall_work(&mut cv);
        let none = cv.kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer(Transfer::new(&walls, none));
        body.add(cv.finish());
        body.compile()
    });
    let refused = Err(AdmissionError::Compile(CompileDefect::PayerHoldsNoHeat {
        site: liver,
    }));
    assert_eq!(compiled.map(|_| ()), refused);
    Ok(())
}

#[test]
fn a_named_payer_reaches_the_model_hash_where_it_moves_no_column() -> Result<(), AdmissionError> {
    let unnamed = paid_work(&[], false)?;
    let hashes = [
        unnamed.model_hash(),
        paid_work(&[(Work::Stroke, "liver bed")], false)?.model_hash(),
        paid_work(&[(Work::Stroke, "gut bed")], false)?.model_hash(),
        paid_work(&[(Work::Walls, "liver bed")], false)?.model_hash(),
    ];
    let named = paid_work(&[(Work::Stroke, "liver bed")], false)?;
    assert_eq!(named.rows().len(), unnamed.rows().len());
    assert_eq!(columns(&named, "pump work"), columns(&unnamed, "pump work"));
    assert_eq!(named.columns().len(), unnamed.columns().len());
    for (at, hash) in hashes.iter().enumerate() {
        assert!(
            hashes[at + 1..].iter().all(|other| other != hash),
            "payers {at} share a hash"
        );
    }
    Ok(())
}

#[test]
fn named_work_without_an_elastic_store_is_refused_instead_of_dropped() -> Result<(), AdmissionError> {
    let payers = [(Work::Stroke, "liver bed"), (Work::Walls, "gut bed")];
    let complete = paid_work(&payers, true)?;
    assert!(!columns(&complete, "stroke work").is_empty());
    assert_eq!(columns(&complete, "wall work").len(), 1);

    let mut schema = Schema::new(whole_body()?)?;
    watery_spaces_left_out(&mut schema);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    for &(work, bed) in &payers {
        schema.pay_work(work, site::<CapillaryBed>(bed))?;
    }
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let stroke = ports.stroke_work(&mut cv);
        let still = cv.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_at(&stroke, still);
        let walls = ports.wall_work(&mut cv);
        let none = cv.kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer(Transfer::new(&walls, none));
        body.add(cv.finish());
        let (mut fl, ports) = body.fluids()?.parts();
        let pumps = ports.pump_work(&mut fl);
        let still = fl.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, fl.every_mode(), class)
        });
        fl.transfer_at(&pumps, still);
        body.add(fl.finish());
        body.compile()
    });
    match compiled {
        Err(AdmissionError::Compile(defect)) => {
            assert_eq!(defect, CompileDefect::PaidWorkHasNoStore { work: Work::Stroke });
            Ok(())
        }
        Err(other) => Err(other),
        Ok(body) => panic!(
            "named work without an elastic store compiled: {} stroke work columns and {} wall work columns",
            columns(&body, "stroke work").len(),
            columns(&body, "wall work").len()
        ),
    }
}

#[test]
fn either_order_of_naming_the_payers_compiles_one_body() -> Result<(), AdmissionError> {
    let forward = paid_work(&[(Work::Stroke, "liver bed"), (Work::Walls, "gut bed")], true)?;
    let backward = paid_work(&[(Work::Walls, "gut bed"), (Work::Stroke, "liver bed")], true)?;
    assert_eq!(forward.model_hash(), backward.model_hash());
    Ok(())
}

#[test]
fn every_payer_is_checked_for_heat_whichever_work_it_pays() -> Result<(), AdmissionError> {
    let (liver, gut) = (site::<CapillaryBed>("liver bed"), site::<CapillaryBed>("gut bed"));
    for (stroke, walls, cold) in [(liver, gut, gut), (gut, liver, gut)] {
        let mut schema = Schema::new(whole_body()?)?;
        watery_spaces_left_out(&mut schema);
        schema.pool::<Energy>(liver, Space::Thermal)?;
        schema.pool::<Energy>(site::<TissueSite>("body"), Space::Elastic)?;
        schema.pay_work(Work::Stroke, stroke)?;
        schema.pay_work(Work::Walls, walls)?;
        let class = wave()?;
        let compiled = assemble(schema, |mut body| {
            let (mut cv, ports) = body.cardiovascular()?.parts();
            let walls = ports.wall_work(&mut cv);
            let none = cv.kernel(|mut k| {
                let zero = k.zero::<Rate<Energy>>();
                k.finish(zero, cv.every_mode(), class)
            });
            cv.transfer(Transfer::new(&walls, none));
            body.add(cv.finish());
            body.compile()
        });
        let refused = Err(AdmissionError::Compile(CompileDefect::PayerHoldsNoHeat {
            site: cold,
        }));
        assert_eq!(compiled.map(|_| ()), refused);
    }
    Ok(())
}

#[test]
fn a_named_work_on_a_map_whose_root_is_a_vessel_is_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let aorta = map.site::<VesselSegment>("aorta", at()?);
    let bed = map.site::<CapillaryBed>("bed", at()?).id();
    map.contain(aorta.id(), bed);
    map.connect(Network::Vascular, aorta.id(), bed);
    map.connect(Network::Vascular, bed, aorta.id());
    let mut schema = Schema::new(map.finish()?)?;
    watery_spaces_left_out(&mut schema);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    schema.pay_work(Work::Stroke, bed)?;
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let stroke = ports.stroke_work(&mut cv);
        let still = cv.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_at(&stroke, still);
        body.add(cv.finish());
        body.compile()
    });
    let refused = Err(AdmissionError::Compile(CompileDefect::PaidWorkHasNoStore {
        work: Work::Stroke,
    }));
    assert_eq!(compiled.map(|_| ()), refused);
    Ok(())
}
