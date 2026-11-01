use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

const ROUTE: &str = "gut water";
const SECOND_ROUTE: &str = "second route";
const PRESSURE: &str = "enclosure_pressure";

#[derive(Clone, Copy, PartialEq)]
enum Defect {
    None,
    RouteOmitted,
    RouteUnserved,
    BoundaryLawOnHeldRoute,
    RouteUnknown,
    RouteOfTwoCards,
    SlotNamedLikeRoute,
    NoPressureLaw,
    TwoPressureLaws,
    CardNotHeld,
    SliceCardNamed,
    ThePressureOfAnotherCard,
    CellsUnheld,
    MovedSpaceHeld,
    ElasticEnclosure,
    RigidEnclosure,
    OtherValidity,
    OtherRoutes,
    OtherHolds,
    Reversed,
    LiverHoldsCells,
    LiverHoldsNothing,
    LiverAlsoHoldsCells,
    LiverHoldsEnclosureFluid,
    NamedCardListsItsFluid,
    PortNamesNoRoute,
    CellsLeftOut,
    FluidLeftOut,
    LiverReadsTheGutsDrive,
    OneRouteReadsAnother,
}

struct Sites {
    vessel: SiteId,
    bed: SiteId,
    belly: SiteId,
}

/// `vessel -> bed -> vessel`, the bed inside a space `card` holds, and one schema route into the vessel.
fn schema(defect: Defect) -> Result<(Schema, Sites), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let bed = map.site::<CapillaryBed>("bed", at()?);
    let belly = map.site::<EnclosedSpace>("belly", at()?);
    for site in [vessel.id(), bed.id(), belly.id()] {
        map.contain(root.id(), site);
    }
    for (from, to) in [(vessel.id(), bed.id()), (bed.id(), vessel.id())] {
        map.connect(Network::Vascular, from, to);
    }
    let compliance = match defect {
        Defect::SliceCardNamed => Compliance::Held {
            card: Cardiovascular::CARD,
        },
        Defect::ThePressureOfAnotherCard => Compliance::Held {
            card: Respiratory::CARD,
        },
        Defect::ElasticEnclosure => Compliance::Elastic {
            elastance: ParameterKey::new("belly.elastance"),
            unstressed: ParameterKey::new("belly.unstressed"),
        },
        Defect::RigidEnclosure => Compliance::Rigid,
        _ => Compliance::Held {
            card: Gastrointestinal::CARD,
        },
    };
    map.enclose(&belly, &[bed.id()], compliance);
    let mut schema = Schema::new(map.finish()?)?;
    schema.water_boundary(ROUTE, BoundaryDirection::IntoBody, vessel.id(), Space::Lumen)?;
    if matches!(
        defect,
        Defect::ElasticEnclosure | Defect::RigidEnclosure | Defect::FluidLeftOut
    ) {
        schema.leave_out(Space::Fluid, "a bench whose enclosure no card holds");
    }
    if defect == Defect::CellsLeftOut {
        schema.leave_out(Space::Cells, "a bench with no cell water");
    }
    if defect == Defect::OneRouteReadsAnother {
        schema.water_boundary(
            SECOND_ROUTE,
            BoundaryDirection::IntoBody,
            vessel.id(),
            Space::Lumen,
        )?;
    }
    let sites = Sites {
        vessel: vessel.id(),
        bed: bed.id(),
        belly: belly.id(),
    };
    Ok((schema, sites))
}

fn slow() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Hours, 3_600_000_000_000)
}

fn pressure_law<'b, S: OutsideSlice>(
    laws: &FragmentBuilder<'b, S>,
    held: &Parameter<'b, S, Pressure>,
) -> Result<PureKernel<'b, S, Pressure, EnclosedSpace>, AdmissionError> {
    let class = slow()?;
    Ok(laws.site_kernel(|mut k| {
        let value = k.parameter(held);
        k.finish(value, laws.every_mode(), class)
    }))
}

fn gut<'b>(body: &mut Assembly<'b>, defect: Defect) -> Result<(), AdmissionError> {
    let supplies = match defect {
        Defect::RouteOmitted => vec![],
        Defect::RouteUnknown => vec!["no such route"],
        Defect::OtherRoutes => vec![ROUTE, ROUTE],
        _ => vec![ROUTE],
    };
    let validity = if defect == Defect::OtherValidity {
        "another stand-in"
    } else {
        "a synthetic bench"
    };
    let holds = if defect == Defect::NamedCardListsItsFluid {
        vec![Space::Fluid]
    } else {
        vec![]
    };
    let (mut laws, ports) = body
        .hold(HeldDeclaration::<Gastrointestinal>::new(
            supplies, holds, validity,
        ))?
        .parts();
    let held = laws.parameter::<Pressure>(ParameterKey::new("gi.pressure"), Admissible::finite());
    let owes_pressure = !matches!(
        defect,
        Defect::NoPressureLaw | Defect::ElasticEnclosure | Defect::RigidEnclosure
    );
    if owes_pressure {
        laws.define_at(&ports.enclosure_pressure, pressure_law(&laws, &held)?);
    }
    if defect == Defect::TwoPressureLaws {
        laws.define_at(&ports.enclosure_pressure, pressure_law(&laws, &held)?);
    }
    if defect == Defect::SlotNamedLikeRoute {
        laws.state::<Dimensionless>(ROUTE, Admissible::finite());
    }
    if defect != Defect::RouteUnserved {
        let name = if defect == Defect::PortNamesNoRoute {
            "no such route"
        } else {
            ROUTE
        };
        let BoundaryWater { route, drive } = ports.water(name)?;
        let class = slow()?;
        let rate = laws.kernel(|mut k| {
            let value = k.read(&drive);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
    }
    body.add(laws.finish());
    Ok(())
}

fn cell_water<'b>(body: &mut Assembly<'b>, defect: Defect) -> Result<(), AdmissionError> {
    let holds = match defect {
        Defect::CellsUnheld | Defect::LiverHoldsCells => vec![],
        Defect::MovedSpaceHeld => vec![Space::Cells, Space::Plasma],
        Defect::OtherHolds => vec![Space::Cells, Space::Cells],
        _ => vec![Space::Cells],
    };
    let declaration = HeldDeclaration::<BloodChemistry>::new(vec![], holds, "a synthetic bench");
    let (laws, _ports) = body.hold(declaration)?.parts();
    body.add(laws.finish());
    Ok(())
}

fn compiled(defect: Defect) -> Result<CompiledBody, AdmissionError> {
    assemble(schema(defect)?.0, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let class = slow()?;
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
        let class = slow()?;
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
        if defect == Defect::Reversed {
            cell_water(&mut body, defect)?;
        }
        if defect != Defect::CardNotHeld {
            gut(&mut body, defect)?;
        }
        if defect != Defect::Reversed {
            cell_water(&mut body, defect)?;
        }
        match defect {
            Defect::BoundaryLawOnHeldRoute => {
                let (mut laws, ports) = body.boundary()?.parts();
                let BoundaryWater { route, drive } = ports.water(ROUTE)?;
                let class = slow()?;
                let rate = laws.kernel(|mut k| {
                    let value = k.read(&drive);
                    k.finish(value, laws.every_mode(), class)
                });
                laws.transfer(Transfer::new(&route, rate));
                body.add(laws.finish());
            }
            Defect::OneRouteReadsAnother => {
                let (mut laws, ports) = body.boundary()?.parts();
                let BoundaryWater { route, .. } = ports.water(SECOND_ROUTE)?;
                let BoundaryWater { drive, .. } = ports.water(ROUTE)?;
                let class = slow()?;
                let rate = laws.kernel(|mut k| {
                    let value = k.read(&drive);
                    k.finish(value, laws.every_mode(), class)
                });
                laws.transfer(Transfer::new(&route, rate));
                body.add(laws.finish());
            }
            Defect::LiverReadsTheGutsDrive => {
                let declaration = HeldDeclaration::<Hepatic>::new(vec![], vec![], "a synthetic bench");
                let (mut laws, ports) = body.hold(declaration)?.parts();
                let BoundaryWater { drive, .. } = ports.water(ROUTE)?;
                let tap = laws.algebraic::<Rate<Water>>("tap", Admissible::finite());
                let class = slow()?;
                let residual = laws.kernel(|mut k| {
                    let seen = k.algebraic(&tap);
                    let value = k.read(&drive);
                    let gap = k.subtract(seen, value);
                    k.finish(gap, laws.every_mode(), class)
                });
                laws.residual(&tap, residual);
                body.add(laws.finish());
            }
            Defect::RouteOfTwoCards => {
                let declaration = HeldDeclaration::<Hepatic>::new(vec![ROUTE], vec![], "a synthetic bench");
                let (laws, _ports) = body.hold(declaration)?.parts();
                body.add(laws.finish());
            }
            Defect::LiverHoldsCells
            | Defect::LiverHoldsNothing
            | Defect::LiverAlsoHoldsCells
            | Defect::LiverHoldsEnclosureFluid => {
                let holds = match defect {
                    Defect::LiverHoldsNothing => vec![],
                    Defect::LiverHoldsEnclosureFluid => vec![Space::Fluid],
                    _ => vec![Space::Cells],
                };
                let declaration = HeldDeclaration::<Hepatic>::new(vec![], holds, "a synthetic bench");
                let (laws, _ports) = body.hold(declaration)?.parts();
                body.add(laws.finish());
            }
            Defect::ThePressureOfAnotherCard => {
                let declaration = HeldDeclaration::<Respiratory>::new(vec![], vec![], "a synthetic bench");
                let (mut laws, ports) = body.hold(declaration)?.parts();
                let held = laws.parameter::<Pressure>(ParameterKey::new("rs.pressure"), Admissible::finite());
                laws.define_at(&ports.enclosure_pressure, pressure_law(&laws, &held)?);
                body.add(laws.finish());
            }
            _ => {}
        }
        body.compile()
    })
}

fn law(owner: &'static str, local: &'static str) -> LawName {
    LawName { owner, local }
}

fn refused(defect: Defect) -> AdmissionError {
    match compiled(defect) {
        Ok(compiled) => panic!("compiled to {:?}", compiled.model_hash()),
        Err(refusal) => refusal,
    }
}

fn defect(found: CompileDefect) -> AdmissionError {
    AdmissionError::Compile(found)
}

#[test]
fn a_held_route_its_declaration_omits_is_refused_at_compile() {
    assert_eq!(
        refused(Defect::RouteOmitted),
        defect(CompileDefect::UnclaimedLaw {
            law: law("GI", ROUTE)
        })
    );
}

#[test]
fn a_declared_supply_no_law_serves_is_an_omitted_law() {
    assert_eq!(
        refused(Defect::RouteUnserved),
        defect(CompileDefect::OmittedLaw {
            law: law("GI", ROUTE)
        })
    );
}

#[test]
fn a_boundary_scope_cannot_serve_a_route_a_held_card_declared() {
    assert_eq!(
        refused(Defect::BoundaryLawOnHeldRoute),
        defect(CompileDefect::UnclaimedLaw {
            law: law("Boundary", ROUTE)
        })
    );
}

#[test]
fn a_card_cannot_read_the_drive_of_a_route_it_did_not_declare() {
    assert_eq!(
        refused(Defect::LiverReadsTheGutsDrive),
        defect(CompileDefect::DriveOfAnotherRoute {
            law: law("HP", "tap"),
            route: ROUTE
        })
    );
}

#[test]
fn one_routes_law_cannot_read_anothers_drive() {
    assert_eq!(
        refused(Defect::OneRouteReadsAnother),
        defect(CompileDefect::DriveOfAnotherRoute {
            law: law("Boundary", SECOND_ROUTE),
            route: ROUTE
        })
    );
}

#[test]
fn a_declared_route_the_schema_does_not_name_is_refused() {
    assert_eq!(
        refused(Defect::RouteUnknown),
        AdmissionError::UnknownBoundary {
            name: "no such route"
        }
    );
}

#[test]
fn a_held_card_cannot_ask_for_a_route_the_schema_does_not_name() {
    assert_eq!(
        refused(Defect::PortNamesNoRoute),
        AdmissionError::UnknownBoundary {
            name: "no such route"
        }
    );
}

#[test]
fn two_cards_cannot_declare_one_route() {
    assert_eq!(
        refused(Defect::RouteOfTwoCards),
        AdmissionError::BoundaryNamedTwice { name: ROUTE }
    );
}

#[test]
fn a_held_cards_slot_under_its_routes_name_is_named_twice() {
    assert_eq!(
        refused(Defect::SlotNamedLikeRoute),
        defect(CompileDefect::NamedTwice {
            law: law("GI", ROUTE)
        })
    );
}

#[test]
fn a_card_is_held_once() {
    let hold = |body: &mut Assembly<'_>| {
        let declaration = HeldDeclaration::<Hepatic>::new(vec![], vec![], "a synthetic bench");
        body.hold(declaration).map(|_| ())
    };
    let refused = schema(Defect::None).and_then(|(schema, _)| {
        assemble(schema, |mut body| {
            hold(&mut body)?;
            hold(&mut body)?;
            body.compile()
        })
    });
    assert_eq!(
        refused.map(|body| body.model_hash()),
        Err(AdmissionError::ScopeIssuedTwice { owner: "HP" })
    );
}

#[test]
fn a_delivery_on_a_held_route_is_admitted_like_any_boundary_delivery() -> Result<(), InitializationFailure> {
    let body = compiled(Defect::None)?;
    let mut packet = ParameterPacket::<SyntheticFixture>::new();
    let pressure = || Measure::<Pressure>::new(3.0);
    packet.synthetic(
        ParameterKey::new("gi.pressure"),
        Binding::Body,
        pressure()?,
        "synthetic",
    )?;
    let kf = Measure::<Conductance<Water>>::new(1.0)?;
    let class = Binding::Class(Class::CapillaryBed);
    packet.synthetic(ParameterKey::new("fl.kf"), class, kf, "synthetic")?;
    for row in body.rows() {
        match row.kind() {
            RowKind::Algebraic(_) => packet.start(row.kind(), pressure()?, "synthetic")?,
            kind => packet.start(kind, Measure::<Water>::new(1.0)?, "synthetic")?,
        }
    }
    let delivery = BoundaryEvent::new(
        InputId::new(Digest::of_canonical(b"held delivery")),
        SimTime::from_ns(1_000_000_000),
        BoundaryInput::Water(WaterDelivery {
            boundary: ROUTE,
            amount: Measure::new(1.0)?,
            ends: SimTime::from_ns(2_000_000_000),
        }),
    );
    let parameters = body.bind(packet)?;
    let request = InitializationRequest::consistent_instant(SimTime::from_ns(0));
    assert!(
        body.initialization(parameters, BoundaryPort::new(vec![delivery])?, request)
            .is_ok()
    );
    Ok(())
}

#[test]
fn a_held_route_is_a_column_of_its_card_with_a_receipt() -> Result<(), AdmissionError> {
    let body = compiled(Defect::None)?;
    let (_, sites) = schema(Defect::None)?;
    let row = |kind| (body.rows().iter()).find(|row| row.kind() == kind).map(Row::id);
    let lumen = row(RowKind::Amount {
        site: sites.vessel,
        space: Space::Lumen,
    });
    let column = body.columns().iter().find(|column| column.receipt().is_some());
    let column = column.expect("the route's column");
    assert_eq!(column.origin(), law("GI", ROUTE));
    assert_eq!(
        column.terms().iter().map(|term| (term.0, term.1)).next(),
        lumen.map(|row| (row, 1))
    );
    assert_eq!(column.receipt().map(|receipt| receipt.boundary()), Some(ROUTE));
    Ok(())
}

#[test]
fn an_enclosure_with_no_pressure_law_is_an_omitted_law() {
    assert_eq!(
        refused(Defect::NoPressureLaw),
        defect(CompileDefect::OmittedLaw {
            law: law("GI", PRESSURE)
        })
    );
}

#[test]
fn an_enclosure_has_one_pressure_law() {
    assert!(matches!(
        refused(Defect::TwoPressureLaws),
        AdmissionError::Compile(CompileDefect::SecondOwner { law: named, .. }) if named == law("GI", PRESSURE)
    ));
}

#[test]
fn the_card_an_enclosure_names_must_be_held() {
    assert_eq!(
        refused(Defect::CardNotHeld),
        defect(CompileDefect::OmittedLaw {
            law: law("GI", PRESSURE)
        })
    );
    assert_eq!(
        refused(Defect::SliceCardNamed),
        defect(CompileDefect::OmittedLaw {
            law: law("CV", PRESSURE)
        })
    );
}

/// On a map with no enclosure the output owes nothing, so a plain slot can take its name.
#[test]
fn a_definition_cannot_land_on_a_plain_slot_that_shares_its_name() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    let found = assemble(Schema::new(map.finish()?)?, |mut body| {
        let declaration = HeldDeclaration::<Hepatic>::new(vec![], vec![], "a synthetic bench");
        let (mut laws, ports) = body.hold(declaration)?.parts();
        laws.algebraic::<Pressure>(PRESSURE, Admissible::finite());
        let held = laws.site_parameter::<EnclosedSpace, Pressure>(
            ParameterKey::new("hp.pressure"),
            Admissible::finite(),
        );
        let class = slow()?;
        let value = laws.site_kernel(|mut k| {
            let value = k.site_parameter(&held);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define_at(&ports.enclosure_pressure, value);
        body.add(laws.finish());
        body.compile()
    });
    assert_eq!(
        found.err(),
        Some(defect(CompileDefect::UnclaimedLaw {
            law: law("HP", PRESSURE)
        }))
    );
    Ok(())
}

#[test]
fn a_card_the_map_does_not_name_cannot_define_an_enclosures_pressure() {
    assert_eq!(
        refused(Defect::ThePressureOfAnotherCard),
        defect(CompileDefect::UnclaimedLaw {
            law: law("GI", PRESSURE)
        })
    );
}

#[test]
fn the_pressure_of_an_elastic_or_rigid_enclosure_is_refused_until_it_has_a_law() -> Result<(), AdmissionError>
{
    for enclosure in [Defect::ElasticEnclosure, Defect::RigidEnclosure] {
        let (_, sites) = schema(enclosure)?;
        assert_eq!(
            refused(enclosure),
            defect(CompileDefect::NoEnclosurePressure {
                law: law("FL", "starling"),
                space: sites.belly,
            })
        );
    }
    Ok(())
}

#[test]
fn a_pool_no_law_moves_and_no_declaration_holds_is_an_orphan() -> Result<(), AdmissionError> {
    let (_, sites) = schema(Defect::None)?;
    assert_eq!(
        refused(Defect::CellsUnheld),
        defect(CompileDefect::OrphanPool {
            site: sites.bed,
            space: Space::Cells,
        })
    );
    Ok(())
}

#[test]
fn a_space_a_law_moves_cannot_be_held() -> Result<(), AdmissionError> {
    let (_, sites) = schema(Defect::None)?;
    assert_eq!(
        refused(Defect::MovedSpaceHeld),
        defect(CompileDefect::HeldPoolMoved {
            card: "BC",
            site: sites.bed,
            space: Space::Plasma,
        })
    );
    Ok(())
}

#[test]
fn two_cards_cannot_hold_one_pool() -> Result<(), AdmissionError> {
    let (_, Sites { bed, .. }) = schema(Defect::None)?;
    assert_eq!(
        refused(Defect::LiverAlsoHoldsCells),
        defect(CompileDefect::PoolHeldTwice {
            cards: ["BC", "HP"],
            site: bed,
            space: Space::Cells,
        })
    );
    compiled(Defect::LiverHoldsCells)?;
    Ok(())
}

#[test]
fn a_card_cannot_hold_the_fluid_of_an_enclosure_another_card_is_named_for() -> Result<(), AdmissionError> {
    let (_, Sites { belly, .. }) = schema(Defect::None)?;
    assert_eq!(
        refused(Defect::LiverHoldsEnclosureFluid),
        defect(CompileDefect::PoolHeldTwice {
            cards: ["GI", "HP"],
            site: belly,
            space: Space::Fluid,
        })
    );
    compiled(Defect::NamedCardListsItsFluid)?;
    Ok(())
}

#[test]
fn a_space_cannot_be_both_held_and_left_out() {
    assert_eq!(
        refused(Defect::CellsLeftOut),
        defect(CompileDefect::HeldNothing {
            card: "BC",
            space: Space::Cells
        })
    );
}

#[test]
fn a_held_enclosures_own_fluid_cannot_be_left_out() {
    assert_eq!(
        refused(Defect::FluidLeftOut),
        defect(CompileDefect::HeldNothing {
            card: "GI",
            space: Space::Fluid
        })
    );
}

#[test]
fn a_held_space_the_map_does_not_have_is_refused() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    let found = assemble(Schema::new(map.finish()?)?, |mut body| {
        let declaration = HeldDeclaration::<Hepatic>::new(vec![], vec![Space::Lumen], "a synthetic bench");
        let (laws, _ports) = body.hold(declaration)?.parts();
        body.add(laws.finish());
        body.compile()
    });
    assert_eq!(
        found.err(),
        Some(defect(CompileDefect::HeldNothing {
            card: "HP",
            space: Space::Lumen
        }))
    );
    Ok(())
}

#[test]
fn a_held_pool_is_an_amount_in_the_ledger_with_a_combination_of_its_own() -> Result<(), AdmissionError> {
    let body = compiled(Defect::None)?;
    let (_, sites) = schema(Defect::None)?;
    let amount = |site, space| RowKind::Amount { site, space };
    let held = [amount(sites.bed, Space::Cells), amount(sites.belly, Space::Fluid)];
    for kind in held {
        let row = (body.rows().iter()).find(|row| row.kind() == kind);
        let row = row.expect("a held pool has a row").id();
        let alone: &[(RowId, i64)] = &[(row, 1)];
        assert!((body.conserved_basis().combinations().iter()).any(|sum| sum.terms() == alone));
        assert!(!(body.columns().iter()).any(|column| column.terms().iter().any(|term| term.0 == row)));
    }
    assert_eq!(body.held().collect::<Vec<_>>(), ["BC", "GI"]);
    Ok(())
}

#[test]
fn a_changed_declaration_changes_the_hash_and_its_order_does_not() -> Result<(), AdmissionError> {
    let base = compiled(Defect::None)?.model_hash();
    assert_ne!(compiled(Defect::OtherValidity)?.model_hash(), base);
    assert_eq!(compiled(Defect::OtherRoutes)?.model_hash(), base);
    assert_eq!(compiled(Defect::OtherHolds)?.model_hash(), base);
    assert_eq!(compiled(Defect::Reversed)?.model_hash(), base);
    assert_ne!(
        compiled(Defect::LiverHoldsCells)?.model_hash(),
        compiled(Defect::LiverHoldsNothing)?.model_hash()
    );
    Ok(())
}

#[test]
fn a_bed_reads_its_nearest_enclosure_and_a_bed_nothing_encloses_reads_no_row() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let inner_bed = map.site::<CapillaryBed>("inner bed", at()?);
    let open_bed = map.site::<CapillaryBed>("open bed", at()?);
    let organ = map.site::<TissueSite>("organ", at()?);
    let organ_bed = map.site::<CapillaryBed>("organ bed", at()?);
    let outer = map.site::<EnclosedSpace>("outer", at()?);
    let inner = map.site::<EnclosedSpace>("inner", at()?);
    for site in [
        vessel.id(),
        inner_bed.id(),
        open_bed.id(),
        organ.id(),
        outer.id(),
        inner.id(),
    ] {
        map.contain(root.id(), site);
    }
    map.contain(organ.id(), organ_bed.id());
    for (from, to) in [
        (vessel.id(), inner_bed.id()),
        (inner_bed.id(), vessel.id()),
        (vessel.id(), open_bed.id()),
        (open_bed.id(), vessel.id()),
        (vessel.id(), organ_bed.id()),
        (organ_bed.id(), vessel.id()),
    ] {
        map.connect(Network::Vascular, from, to);
    }
    map.enclose(&outer, &[inner.id()], Compliance::Rigid);
    let held = Compliance::Held {
        card: Respiratory::CARD,
    };
    map.enclose(&inner, &[inner_bed.id(), organ.id()], held);
    let mut schema = Schema::new(map.finish()?)?;
    for (space, reason) in [
        (Space::Lumen, "no circulation in this bench"),
        (Space::Cells, "no cell water in this bench"),
    ] {
        schema.leave_out(space, reason);
    }
    let body = assemble(schema, |mut body| {
        let declaration =
            HeldDeclaration::<Respiratory>::new(vec![], vec![Space::Fluid], "a synthetic bench");
        let (mut laws, ports) = body.hold(declaration)?.parts();
        let held = laws.parameter::<Pressure>(ParameterKey::new("rs.pressure"), Admissible::finite());
        laws.define_at(&ports.enclosure_pressure, pressure_law(&laws, &held)?);
        body.add(laws.finish());
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut fl);
        let class = slow()?;
        let flow = fl.site_kernel(|mut k| {
            let outside = k.at_site(&ports.extravascular_pressure);
            let g = k.zero::<Conductance<Water>>();
            let flow = k.conductance_flow(g, outside);
            k.finish(flow, fl.every_mode(), class)
        });
        fl.transfer_at(&route, flow);
        body.add(fl.finish());
        body.compile()
    })?;
    let row = |kind: RowKind| {
        let found = body.rows().iter().find(|row| row.kind() == kind);
        found.expect("a row").unknown()
    };
    let pressure = row(RowKind::Algebraic(Slot {
        law: law("RS", PRESSURE),
        site: Some(inner.id()),
    }));
    let reads = |site| {
        let at = row(RowKind::Amount {
            site,
            space: Space::Interstitium,
        })
        .index();
        let sparsity = body.sparsity();
        sparsity.unknowns()[sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1]].contains(&pressure)
    };
    assert!(reads(inner_bed.id()));
    assert!(reads(organ_bed.id()));
    assert!(!reads(open_bed.id()));
    Ok(())
}

#[test]
fn a_held_pressure_is_a_kernel_so_it_may_read_a_forcing() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let space = map.site::<EnclosedSpace>("chest", at()?);
    map.contain(root.id(), space.id());
    let held = Compliance::Held {
        card: Respiratory::CARD,
    };
    map.enclose(&space, &[], held);
    let body = assemble(Schema::new(map.finish()?)?, |mut body| {
        let declaration = HeldDeclaration::<Respiratory>::new(vec![], vec![], "a synthetic bench");
        let (mut laws, ports) = body.hold(declaration)?.parts();
        let mean = laws.parameter::<Pressure>(ParameterKey::new("rs.pressure"), Admissible::finite());
        let breath = laws.forcing(
            "breath",
            ParameterKey::new("rs.period"),
            ParameterKey::new("rs.amplitude"),
        );
        let class = slow()?;
        let pressure = laws.site_kernel(|mut k| {
            let mean = k.parameter(&mean);
            let breath = k.forcing(&breath);
            let value = k.scale(mean, breath);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define_at(&ports.enclosure_pressure, pressure);
        body.add(laws.finish());
        body.compile()
    })?;
    assert_eq!(body.programs().len(), 1);
    Ok(())
}
