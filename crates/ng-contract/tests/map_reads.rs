//! What a wrong map does to a law that names no site: each read resolves at compile or is refused by name, because a
//! read that came back as nothing would pass every count and lose a feedback loop.

use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn fast() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Wave, 3_600_000_000_000)
}

#[derive(Clone, Copy, PartialEq)]
enum Nerve {
    Supplied,
    Stated,
    Lost,
    NoReceptor,
}

fn wall_law(nerve: Nerve) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let a = map.site::<VesselSegment>("a", at()?);
    let b = map.site::<VesselSegment>("b", at()?);
    let junction = map.site::<Junction>("junction", at()?);
    for site in [a.id(), b.id(), junction.id()] {
        map.contain(root.id(), site);
    }
    map.connect(Network::Vascular, a.id(), b.id());
    map.connect(Network::Vascular, b.id(), a.id());
    match nerve {
        Nerve::Lost => map.supply(&junction, &[a.id()]),
        Nerve::Stated | Nerve::NoReceptor => {
            map.supply(&junction, &[a.id()]);
            map.unsupplied(&[b.id()]);
        }
        Nerve::Supplied => map.supply(&junction, &[a.id(), b.id()]),
    }
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    if nerve != Nerve::NoReceptor {
        schema.carry::<Norepinephrine, Junction>(Space::Cleft);
        schema.carry::<Norepinephrine, Junction>(Space::Bound);
    }
    assemble(schema, |mut body| {
        let class = fast()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        let alpha = ports.effect::<Norepinephrine>();
        let (row, _) = cv.site_field::<Blood, Dimensionless>("felt", Admissible::finite());
        let felt = cv.site_kernel(|mut k| {
            let effect = k.at_junction(&alpha);
            k.finish(effect, cv.every_mode(), class)
        });
        cv.define_at(&row, felt);
        body.add(cv.finish());
        if nerve != Nerve::NoReceptor {
            let (mut laws, ports) = body.receptor()?.parts();
            let binding = ports.binding::<Junction, Norepinephrine>(&mut laws);
            let none = laws.site_kernel(|mut k| {
                let zero = k.zero::<Rate<Norepinephrine>>();
                k.finish(zero, laws.every_mode(), class)
            });
            laws.transfer_at(&binding, none);
            let output = ports.effect::<Junction, Norepinephrine>(&mut laws);
            let share = laws.site_parameter::<Junction, Dimensionless>(
                ParameterKey::new("re.share"),
                Admissible::finite(),
            );
            let effect = laws.site_kernel(|mut k| {
                let value = k.site_parameter(&share);
                k.finish(value, laws.every_mode(), class)
            });
            laws.define_at(&output, effect);
            body.add(laws.finish());
        }
        body.compile()
    })
}

#[test]
fn a_site_a_law_reads_a_junction_at_must_be_supplied_or_stated_bare() -> Result<(), AdmissionError> {
    assert!(wall_law(Nerve::Supplied).is_ok());
    assert!(wall_law(Nerve::Stated).is_ok());
    let site = AnatomyBuilder::new().site::<VesselSegment>("b", at()?).id();
    let law = LawName {
        owner: "CV",
        local: "felt",
    };
    assert_eq!(
        wall_law(Nerve::Lost).err(),
        Some(AdmissionError::Compile(CompileDefect::NoSupply { law, site }))
    );
    Ok(())
}

#[test]
fn a_junction_with_no_such_receptor_is_refused_not_read_as_nothing() -> Result<(), AdmissionError> {
    let junction = AnatomyBuilder::new().site::<Junction>("junction", at()?).id();
    let law = LawName {
        owner: "CV",
        local: "felt",
    };
    assert_eq!(
        wall_law(Nerve::NoReceptor).err(),
        Some(AdmissionError::Compile(CompileDefect::NoReceptor {
            law,
            junction
        }))
    );
    Ok(())
}

#[test]
fn a_site_supplied_and_stated_bare_is_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let a = map.site::<VesselSegment>("a", at()?);
    let junction = map.site::<Junction>("junction", at()?);
    map.contain(root.id(), a.id());
    map.contain(root.id(), junction.id());
    map.connect(Network::Vascular, a.id(), a.id());
    map.supply(&junction, &[a.id()]);
    map.unsupplied(&[a.id()]);
    assert_eq!(
        map.finish().err(),
        Some(AdmissionError::Anatomy(AnatomyDefect::SuppliedTwice {
            site: "a"
        }))
    );
    Ok(())
}

#[test]
fn a_network_edge_of_the_wrong_classes_and_a_landmark_placed_twice_are_refused() -> Result<(), AdmissionError>
{
    let build = |twice: bool| -> Result<Result<Anatomy, AdmissionError>, AdmissionError> {
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", at()?);
        let (a, b) = (
            map.site::<VesselSegment>("a", at()?),
            map.site::<VesselSegment>("b", at()?),
        );
        map.contain(root.id(), a.id());
        map.contain(root.id(), b.id());
        map.connect(Network::Vascular, a.id(), b.id());
        map.connect(Network::Vascular, b.id(), a.id());
        if twice {
            map.landmark(Landmark::CarotidSinus, a.id(), a.id());
            map.landmark(Landmark::CarotidSinus, b.id(), b.id());
        } else {
            map.connect(Network::Lymph, a.id(), b.id());
        }
        Ok(map.finish())
    };
    assert_eq!(
        build(false)?.err(),
        Some(AdmissionError::Anatomy(AnatomyDefect::WrongNetwork { site: "a" }))
    );
    assert_eq!(
        build(true)?.err(),
        Some(AdmissionError::Anatomy(AnatomyDefect::LandmarkTwice {
            site: "a"
        }))
    );
    Ok(())
}

/// A tissue has no lumen, so a route from one would lose its flow with nothing said.
#[test]
fn a_urinary_edge_out_of_a_site_with_no_lumen_is_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let tract = map.site::<VesselSegment>("tract", at()?);
    map.contain(root.id(), tract.id());
    map.connect(Network::Urinary, root.id(), tract.id());
    assert_eq!(
        map.finish().err(),
        Some(AdmissionError::Anatomy(AnatomyDefect::WrongNetwork {
            site: "body"
        }))
    );
    Ok(())
}

/// An energy route that took any pool in its space would turn water into energy, and the conserved total that joined the
/// two would hide it.
#[test]
fn an_energy_route_takes_no_pool_of_another_species() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    map.region("whole", root.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.pool::<Water>(root.id(), Space::Elastic)?;
    schema.pool::<Energy>(root.id(), Space::Thermal)?;
    let compiled = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.dissipation(&mut cv);
        let power = cv.parameter::<Rate<Energy>>(ParameterKey::new("power"), Admissible::positive());
        let class = RateClass::new(RateBand::Wave, 1_000_000_000)?;
        let law = cv.kernel(|mut k| {
            let value = k.parameter(&power);
            k.finish(value, cv.every_mode(), class)
        });
        cv.transfer(Transfer::new(&route, law));
        body.add(cv.finish());
        body.compile()
    });
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::OrphanPool {
            site: root.id(),
            space: Space::Elastic,
        }))
    );
    Ok(())
}

/// A route of water alone is its one column, with no flow row, so a read of that flow has nothing to land on.
#[test]
fn a_read_of_an_inflow_that_is_no_row_is_refused_by_name() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let bed = map.site::<CapillaryBed>("bed", at()?);
    let tubule = map.site::<VesselSegment>("tubule", at()?);
    let duct = map.site::<VesselSegment>("duct", at()?);
    for site in [vessel.id(), bed.id(), tubule.id(), duct.id()] {
        map.contain(root.id(), site);
    }
    map.connect(Network::Vascular, vessel.id(), bed.id());
    map.connect(Network::Vascular, bed.id(), vessel.id());
    map.connect(Network::Urinary, bed.id(), tubule.id());
    map.connect(Network::Urinary, tubule.id(), duct.id());
    map.connect(Network::Reclaim, duct.id(), bed.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Interstitium, "a bench of water alone");
    schema.leave_out(Space::Cells, "a bench of water alone");
    let compiled = assemble(schema, |mut body| {
        let class = fast()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        body.add(cv.finish());
        let (mut rn, ports) = body.renal()?.parts();
        let filter = ports.filtration(&mut rn);
        let still = rn.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, rn.every_mode(), class)
        });
        rn.transfer_along(&filter, still);
        let tubular = ports.tubular(&mut rn);
        let still = rn.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, rn.every_mode(), class)
        });
        rn.transfer_along(&tubular, still);
        let reclaim = ports.reclaim::<Water>(&mut rn);
        let incoming = rn.edge_kernel(|mut k| {
            let value = k.inflow();
            k.finish(value, rn.every_mode(), class)
        });
        rn.transfer_along(&reclaim, incoming);
        body.add(rn.finish());
        body.compile()
    });
    let law = LawName {
        owner: "RN",
        local: "reclaim water",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::NoSuchRead { law }))
    );
    Ok(())
}

/// A map's edges, regions and enclosures all read its sites, so a pool at a site it does not hold would stand outside
/// every check.
#[test]
fn a_pool_at_a_site_the_map_does_not_hold_is_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("root", at()?);
    map.region("whole", root.id());
    let mut schema = Schema::new(map.finish()?)?;
    let elsewhere = AnatomyBuilder::new()
        .site::<VesselSegment>("a misspelled vein", at()?)
        .id();
    assert_eq!(
        schema.pool::<Sodium>(elsewhere, Space::Lumen).err(),
        Some(AdmissionError::SiteOffTheMap { site: elsewhere })
    );
    Ok(())
}

#[test]
fn a_pool_in_a_space_its_site_has_not_is_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let a = map.site::<VesselSegment>("a", at()?);
    map.contain(root.id(), a.id());
    map.connect(Network::Vascular, a.id(), a.id());
    let mut schema = Schema::new(map.finish()?)?;
    assert_eq!(
        schema.pool::<Norepinephrine>(a.id(), Space::Vesicle).err(),
        Some(AdmissionError::SpaceOfAnotherClass {
            site: a.id(),
            space: Space::Vesicle
        })
    );
    Ok(())
}

/// One bed with a vessel around it, so Starling joins a plasma that holds albumin to a tissue space that may not.
fn cargo(tissue_holds: bool, kept_back: bool) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let bed = map.site::<CapillaryBed>("bed", at()?);
    map.contain(root.id(), vessel.id());
    map.contain(root.id(), bed.id());
    map.connect(Network::Vascular, vessel.id(), bed.id());
    map.connect(Network::Vascular, bed.id(), vessel.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    schema.leave_out(Space::Cells, "no cell-water law in this bench");
    schema.carry::<Albumin, Blood>(Space::Plasma);
    schema.carry::<Albumin, Blood>(Space::Lumen);
    if tissue_holds {
        schema.carry::<Albumin, CapillaryBed>(Space::Interstitium);
    }
    assemble(schema, |mut body| {
        let class = fast()?;
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
        if kept_back {
            route.keeps_back::<Albumin>(&mut fl);
        }
        let none = fl.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, fl.every_mode(), class)
        });
        fl.transfer_at(&route, none);
        body.add(fl.finish());
        body.compile()
    })
}

#[test]
fn a_species_one_end_of_a_route_lacks_is_refused_until_the_route_keeps_it_back() {
    let law = LawName {
        owner: "FL",
        local: "starling",
    };
    assert_eq!(
        cargo(false, false).err(),
        Some(AdmissionError::Compile(CompileDefect::CargoStranded {
            law,
            species: Species::Albumin
        }))
    );
    let kept = cargo(false, true).expect("a stated wall");
    let crossing = |body: &CompiledBody| {
        (body.columns().iter())
            .filter(|column| column.origin().local == "starling")
            .count()
    };
    assert_eq!(
        crossing(&kept),
        1,
        "water alone crosses a wall that keeps albumin back"
    );
    assert_eq!(crossing(&cargo(true, false).expect("both ends hold it")), 2);
}

#[test]
fn a_landmark_the_map_does_not_place_is_refused_by_the_law_that_reads_it() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let a = map.site::<VesselSegment>("a", at()?);
    map.contain(root.id(), a.id());
    map.connect(Network::Vascular, a.id(), a.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    let compiled = assemble(schema, |mut body| {
        let class = fast()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        let (row, read) = cv.site_field::<Blood, Pressure>("inside", Admissible::finite());
        let zero = cv.site_kernel(|mut k| {
            let zero = k.zero();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.define_at(&row, zero);
        let mark = cv.algebraic::<Pressure>("marked", Admissible::finite());
        let held = cv.kernel(|mut k| {
            let at_mark = k.at_landmark(Landmark::CarotidSinus, &read);
            let own = k.algebraic(&mark);
            let residual = k.subtract(at_mark, own);
            k.finish(residual, cv.every_mode(), class)
        });
        cv.residual(&mark, held);
        body.add(cv.finish());
        body.compile()
    });
    let law = LawName {
        owner: "CV",
        local: "marked",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::NoSuchRead { law }))
    );
    Ok(())
}

#[test]
fn a_kernel_that_holds_by_a_valves_mode_at_a_body_row_is_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let (a, b) = (
        map.site::<VesselSegment>("a", at()?),
        map.site::<VesselSegment>("b", at()?),
    );
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    map.valve(b.id(), a.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    let compiled = assemble(schema, |mut body| {
        let class = fast()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        let valves = ports.valves(&mut cv);
        let flow = valves.flow_memory();
        let [open, shut] = cv.modes_along(&valves, "valve", ["open", "shut"]);
        let rate = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
            let zero = k.zero::<Rate<Rate<Water>>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.memory_rate(&flow, rate);
        let mark = cv.algebraic::<Dimensionless>("marked", Admissible::finite());
        let held = cv.kernel(|mut k| {
            let value = k.algebraic(&mark);
            k.finish(value, open.only(), class)
        });
        cv.residual(&mark, held);
        let other = cv.kernel(|mut k| {
            let value = k.algebraic(&mark);
            k.finish(value, shut.only(), class)
        });
        cv.residual(&mark, other);
        body.add(cv.finish());
        body.compile()
    });
    let law = LawName {
        owner: "CV",
        local: "valve",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::ModeOfAnotherPlace { law }))
    );
    Ok(())
}

/// In a body of water alone a plain edge's flow is its column and no row, so a read of it is refused by name.
#[test]
fn a_read_of_a_flow_that_is_no_row_is_refused_by_name() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let a = map.site::<VesselSegment>("a", at()?);
    map.contain(root.id(), a.id());
    map.connect(Network::Vascular, a.id(), a.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    schema.pool::<Energy>(root.id(), Space::Elastic)?;
    let compiled = assemble(schema, |mut body| {
        let class = fast()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        let fallen = ports.gravity_work(&mut cv);
        let head = cv.parameter::<Pressure>(ParameterKey::new("cv.head"), Admissible::finite());
        let power = cv.edge_kernel(|mut k| {
            let (head, flow) = (k.parameter(&head), k.edge_flow());
            let product = k.multiply(head, flow);
            let power = k.as_dimension(product);
            k.finish(power, cv.every_mode(), class)
        });
        cv.transfer_along(&fallen, power);
        body.add(cv.finish());
        body.compile()
    });
    let law = LawName {
        owner: "CV",
        local: "gravity work",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::NoSuchRead { law }))
    );
    Ok(())
}
