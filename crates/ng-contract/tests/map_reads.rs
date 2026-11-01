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

fn with_a_valve<F>(extra: F) -> Result<CompiledBody, AdmissionError>
where
    F: for<'b> FnOnce(
        &mut FragmentBuilder<'b, Cardiovascular>,
        &StateSlot<'b, Cardiovascular, Rate<Water>>,
    ) -> Result<(), AdmissionError>,
{
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
    assemble(schema, |mut body| {
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
        let rate = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
            let zero = k.zero::<Rate<Rate<Water>>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.memory_rate(&flow, rate);
        extra(&mut cv, &flow)?;
        body.add(cv.finish());
        body.compile()
    })
}

/// A valve's flow stands on its edge, so a law of the whole body has no such row to read.
#[test]
fn a_body_law_that_reads_an_edges_memory_is_refused_by_name() -> Result<(), AdmissionError> {
    let compiled = with_a_valve(|cv, flow| {
        let class = fast()?;
        let mark = cv.algebraic::<Rate<Water>>("marked", Admissible::finite());
        let held = cv.kernel(|mut k| {
            let (value, read) = (k.algebraic(&mark), k.state(flow));
            let gap = k.subtract(value, read);
            k.finish(gap, cv.every_mode(), class)
        });
        cv.residual(&mark, held);
        Ok(())
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

/// A mode of the whole body has no edge, so a firing of it cannot reset what stands on one.
#[test]
fn a_body_guard_that_resets_an_edges_memory_is_refused_by_name() -> Result<(), AdmissionError> {
    let compiled = with_a_valve(|cv, flow| {
        let class = fast()?;
        let [on, off] = cv.modes("switch", ["on", "off"]);
        let zero = cv.parameter::<Dimensionless>(ParameterKey::new("zero"), Admissible::finite());
        let gap = cv.kernel(|mut k| {
            let value = k.parameter(&zero);
            k.finish(value, on.only(), class)
        });
        cv.event(Guard::new(gap, Crossing::Rising, on.to(&off)?).resetting(flow));
        Ok(())
    });
    let law = LawName {
        owner: "CV",
        local: "switch",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::NoSuchRead { law }))
    );
    Ok(())
}

/// A route stands only where its pools do, so a law over every junction has nothing to read at one that holds none.
#[test]
fn a_read_of_a_route_that_does_not_reach_a_site_is_refused_by_name() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let holding = map.site::<Junction>("holding", at()?);
    let bare = map.site::<Junction>("bare", at()?);
    map.contain(root.id(), holding.id());
    map.contain(root.id(), bare.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.pool::<Norepinephrine>(holding.id(), Space::Vesicle)?;
    schema.pool::<Norepinephrine>(holding.id(), Space::Cleft)?;
    let compiled = assemble(schema, |mut body| {
        let class = fast()?;
        let (mut ns, ports) = body.nervous()?.parts();
        let release = ports.release::<Norepinephrine>(&mut ns);
        let still = ns.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Norepinephrine>>();
            k.finish(zero, ns.every_mode(), class)
        });
        ns.transfer_at(&release, still);
        let (row, _) = ns.site_field::<Junction, Norepinephrine>("stored", Admissible::finite());
        let value = ns.site_kernel(|mut k| {
            let held = k.at_site(&release.from_amount::<Norepinephrine>());
            k.finish(held, ns.every_mode(), class)
        });
        ns.define_at(&row, value);
        body.add(ns.finish());
        body.compile()
    });
    let law = LawName {
        owner: "NS",
        local: "stored",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::NoSuchRead { law }))
    );
    Ok(())
}

/// A binding declared for junctions must not run at a bed that happens to hold the same pools: a cleft's law on plasma
/// would pass every orphan check.
#[test]
fn a_binding_stands_only_on_its_own_set_of_sites() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let bed = map.site::<CapillaryBed>("bed", at()?);
    let junction = map.site::<Junction>("junction", at()?);
    for site in [vessel.id(), bed.id(), junction.id()] {
        map.contain(root.id(), site);
    }
    map.connect(Network::Vascular, vessel.id(), bed.id());
    map.connect(Network::Vascular, bed.id(), vessel.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Cells, "a bench with no tissue");
    schema.leave_out(Space::Interstitium, "a bench with no tissue");
    for (site, space) in [(bed.id(), Space::Plasma), (junction.id(), Space::Cleft)] {
        schema.pool::<Norepinephrine>(site, space)?;
        schema.pool::<Norepinephrine>(site, Space::Bound)?;
    }
    let compiled = assemble(schema, |mut body| {
        let class = fast()?;
        let (mut laws, ports) = body.receptor()?.parts();
        let route = ports.binding::<Junction, Norepinephrine>(&mut laws);
        let rate = laws.site_kernel(|mut k| {
            let zero = k.zero();
            k.finish(zero, laws.every_mode(), class)
        });
        laws.transfer_at(&route, rate);
        body.add(laws.finish());
        body.compile()
    });
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::OrphanPool {
            site: bed.id(),
            space: Space::Plasma,
        }))
    );
    Ok(())
}

fn a_lumen_read_at_a_mark(on_tract: bool) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    if on_tract {
        map.connect(Network::Urinary, vessel.id(), vessel.id());
    }
    map.landmark(Landmark::MaculaDensa, vessel.id(), vessel.id());
    let mut schema = Schema::new(map.finish()?)?;
    let into = BoundaryDirection::IntoBody;
    schema.boundary::<Water>("water", into, vessel.id(), Space::Lumen)?;
    assemble(schema, |mut body| {
        let class = fast()?;
        let (mut laws, ports) = body.boundary()?.parts();
        let route = ports.route::<Water>("water")?.route;
        let rate = laws.kernel(|mut k| {
            let zero = k.zero();
            k.finish(zero, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
        body.add(laws.finish());
        let (mut rn, ports) = body.renal()?.parts();
        let lumen = ports.lumen::<Water>();
        let row = rn.algebraic::<Water>("reading", Admissible::finite());
        let residual = rn.kernel(|mut k| {
            let (read, held) = (k.at_landmark(Landmark::MaculaDensa, &lumen), k.algebraic(&row));
            let gap = k.subtract(held, read);
            k.finish(gap, rn.every_mode(), class)
        });
        rn.residual(&row, residual);
        body.add(rn.finish());
        body.compile()
    })
}

/// A mark off the urinary network would feed the kidney's feedback another space's sodium, so the port's own set refuses it.
#[test]
fn a_landmark_off_a_ports_set_is_refused_by_name() -> Result<(), AdmissionError> {
    assert!(a_lumen_read_at_a_mark(true).is_ok());
    let law = LawName {
        owner: "RN",
        local: "reading",
    };
    assert_eq!(
        a_lumen_read_at_a_mark(false).err(),
        Some(AdmissionError::Compile(CompileDefect::NoSuchRead { law }))
    );
    Ok(())
}

/// A drain's law holds its flow row and its columns read the row, so its own read of a drive is what carries a delivery.
#[test]
fn a_drain_takes_a_delivery_its_own_law_reads() -> Result<(), InitializationFailure> {
    let mut map = AnatomyBuilder::new();
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let mut schema = Schema::new(map.finish()?)?;
    schema.drain("outlet", vessel.id(), Space::Lumen)?;
    let body = assemble(schema, |mut body| {
        let class = fast()?;
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryRoute { route, drive } = ports.route::<Water>("outlet")?;
        let rate = laws.kernel(|mut k| {
            let value = k.read(&drive);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
        body.add(laws.finish());
        body.compile()
    })?;
    let mut packet = ParameterPacket::new();
    packet.start_rest(&body, |_| 1.0, "synthetic")?;
    let parameters = body.bind(packet)?;
    let water = BoundaryInput::Water(Delivered {
        boundary: "outlet",
        amount: Measure::new(1.0)?,
        ends: SimTime::from_ns(1_000_000_000),
    });
    let id = InputId::new(Digest::of_canonical(b"withdrawal"));
    let event = BoundaryEvent::new(id, SimTime::from_ns(0), water);
    let request = InitializationRequest::at(SimTime::from_ns(0));
    body.initialization(parameters, BoundaryPort::new(vec![event])?, request)?;
    Ok(())
}

fn a_small_loop() -> Result<Schema, AdmissionError> {
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
    Schema::new(map.finish()?)
}

/// A kernel keeps no place once it is attached, so a site's rate on a whole body's memory reads a site it does not have.
#[test]
fn a_site_rate_on_a_body_memory_is_refused_by_name() -> Result<(), AdmissionError> {
    let compiled = assemble(a_small_loop()?, |mut body| {
        let class = fast()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        let memory = cv.state::<Dimensionless>("memory", Admissible::finite());
        let pace = cv
            .site_parameter::<Blood, Rate<Dimensionless>>(ParameterKey::new("pace"), Admissible::positive());
        let rate = cv.site_kernel(|mut k| {
            let value = k.site_parameter(&pace);
            k.finish(value, cv.every_mode(), class)
        });
        cv.memory_rate(&memory, rate);
        body.add(cv.finish());
        body.compile()
    });
    let law = LawName {
        owner: "CV",
        local: "memory",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::NoSuchRead { law }))
    );
    Ok(())
}

/// The same for a guard: an edge's zero on a whole body's mode.
#[test]
fn an_edge_guard_on_a_body_mode_is_refused_by_name() -> Result<(), AdmissionError> {
    let compiled = assemble(a_small_loop()?, |mut body| {
        let class = fast()?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        let [first, second] = cv.modes("switch", ["first", "second"]);
        let threshold = cv
            .edge_parameter::<VascularEdge, Pressure>(ParameterKey::new("threshold"), Admissible::positive());
        let zero = cv.edge_kernel(|mut k| {
            let value = k.edge_parameter(&threshold);
            k.finish(value, first.only(), class)
        });
        cv.event(Guard::new(zero, Crossing::Rising, first.to(&second)?));
        body.add(cv.finish());
        body.compile()
    });
    let law = LawName {
        owner: "CV",
        local: "switch",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::NoSuchRead { law }))
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
            let moved = k.volume_flow(flow);
            let product = k.multiply(head, moved);
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
