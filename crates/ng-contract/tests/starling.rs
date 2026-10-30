use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

struct Beds {
    schema: Schema,
    a: SiteId,
    b: SiteId,
    vessel: SiteId,
    space: SiteId,
}

fn beds() -> Result<Beds, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let a = map.site::<CapillaryBed>("a", at()?);
    let b = map.site::<CapillaryBed>("b", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let space = map.site::<EnclosedSpace>("space", at()?);
    let junction = map.site::<Junction>("junction", at()?);
    let nerve = map.site::<NerveSegment>("nerve", at()?);
    for site in [a.id(), b.id(), vessel.id(), space.id(), junction.id(), nerve.id()] {
        map.contain(root.id(), site);
    }
    for (from, to) in [(vessel.id(), a.id()), (a.id(), b.id()), (b.id(), vessel.id())] {
        map.connect(Network::Vascular, from, to);
    }
    map.enclose(&space, &[b.id()], Compliance::Rigid);
    map.served_by(a.id(), &nerve, Afferent::Stretch);
    Ok(Beds {
        schema: Schema::new(map.finish()?)?,
        a: a.id(),
        b: b.id(),
        vessel: vessel.id(),
        space: space.id(),
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Defect {
    None,
    CellsModeled,
    LumenModeled,
    FluidModeled,
    NothingLeftOut,
    InterstitiumLeftOut,
    NoPartialVolume,
    NoStarlingLaw,
    SlotNamedLikeRoute,
    ParameterInTwoContexts,
}

fn starling(defect: Defect) -> Result<CompiledBody, AdmissionError> {
    let mut schema = beds()?.schema;
    if defect != Defect::NoPartialVolume {
        schema.partial_volume(Measure::new(1.0)?)?;
    }
    let left_out = [
        (
            Space::Cells,
            "no cell-water law in this bench",
            Defect::CellsModeled,
        ),
        (
            Space::Lumen,
            "no circulation law in this bench",
            Defect::LumenModeled,
        ),
        (
            Space::Fluid,
            "no enclosure law in this bench",
            Defect::FluidModeled,
        ),
    ];
    for (space, reason, modeled) in left_out {
        if defect != modeled && defect != Defect::NothingLeftOut {
            schema.leave_out(space, reason);
        }
    }
    if defect == Defect::InterstitiumLeftOut {
        schema.leave_out(Space::Interstitium, "a defect: the Starling law moves it");
    }
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut laws);
        let e =
            laws.site_parameter::<CapillaryBed, Elastance>(ParameterKey::new("fl.e"), Admissible::positive());
        let kf = laws.site_parameter::<CapillaryBed, Conductance<Water>>(
            ParameterKey::new("fl.kf"),
            Admissible::nonnegative(),
        );
        match defect {
            Defect::SlotNamedLikeRoute => {
                laws.state::<Dimensionless>("starling", Admissible::finite());
            }
            Defect::ParameterInTwoContexts => {
                laws.parameter::<Conductance<Water>>(ParameterKey::new("fl.kf"), Admissible::nonnegative());
            }
            _ => {}
        }
        let minutes = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
        let filtration = laws.site_kernel(|mut k| {
            let plasma = k.at_site(&route.from_amount());
            let interstitial = k.at_site(&route.to_amount());
            let pv = k.volume(plasma);
            let iv = k.volume(interstitial);
            let pe = k.site_parameter(&e);
            let ie = k.site_parameter(&e);
            let inside = k.multiply(pe, pv);
            let outside = k.multiply(ie, iv);
            let inside = k.as_dimension::<_, Pressure>(inside);
            let outside = k.as_dimension::<_, Pressure>(outside);
            let head = k.subtract(inside, outside);
            let conductance = k.site_parameter(&kf);
            let flow = k.conductance_flow(conductance, head);
            k.finish(flow, laws.every_mode(), minutes)
        });
        if defect != Defect::NoStarlingLaw {
            laws.transfer_at(&route, filtration);
        }
        body.add(laws.finish());
        body.compile()
    })
}

fn compile_refusal(defect: Defect) -> CompileDefect {
    match starling(defect) {
        Ok(compiled) => panic!("compiled to {:?}", compiled.model_hash()),
        Err(AdmissionError::Compile(found)) => found,
        Err(other) => panic!("refused at construction: {other:?}"),
    }
}

fn incidence(compiled: &CompiledBody) -> Vec<Vec<i64>> {
    let amounts = (compiled.rows().iter())
        .take_while(|row| matches!(row.kind(), RowKind::Amount { .. }))
        .count();
    (0..amounts)
        .map(|row| {
            (compiled.columns().iter())
                .map(|column| {
                    let term = column.terms().iter().find(|term| term.0.index() == row);
                    term.map_or(0, |term| term.1)
                })
                .collect()
        })
        .collect()
}

fn basis(compiled: &CompiledBody) -> Vec<(Book, Vec<(usize, i64)>)> {
    (compiled.conserved_basis().combinations().iter())
        .map(|combination| {
            let terms = combination
                .terms()
                .iter()
                .map(|term| (term.0.index(), term.1))
                .collect();
            (combination.book(), terms)
        })
        .collect()
}

#[test]
fn the_starling_law_has_the_hand_derived_incidence_and_basis() -> Result<(), AdmissionError> {
    let Beds { a, b, .. } = beds()?;
    let compiled = starling(Defect::None)?;
    let amount = |site, space| RowKind::Amount { site, space };
    let kinds: Vec<RowKind> = compiled.rows().iter().map(Row::kind).collect();
    assert_eq!(
        kinds,
        [
            amount(a, Space::Plasma),
            amount(a, Space::Interstitium),
            amount(b, Space::Plasma),
            amount(b, Space::Interstitium),
        ]
    );
    let starling = LawName {
        owner: "FL",
        local: "starling",
    };
    let columns: Vec<(LawName, Context)> = (compiled.columns().iter())
        .map(|column| (column.origin(), column.context()))
        .collect();
    assert_eq!(
        columns,
        [(starling, Context::Site(a)), (starling, Context::Site(b))]
    );
    assert_eq!(incidence(&compiled), [[-1, 0], [1, 0], [0, -1], [0, 1]]);
    assert_eq!(
        basis(&compiled),
        [
            (Book::Water, vec![(0, 1), (1, 1)]),
            (Book::Water, vec![(2, 1), (3, 1)]),
        ]
    );
    Ok(())
}

#[test]
fn one_starling_program_applies_at_both_beds_with_their_own_parameters() -> Result<(), AdmissionError> {
    let Beds { a, b, .. } = beds()?;
    let compiled = starling(Defect::None)?;
    assert_eq!(compiled.programs().len(), 1);
    let instances = compiled.instances();
    assert_eq!(instances.len(), 2);
    assert_eq!(instances[0].program(), instances[1].program());
    let contexts: Vec<Context> = instances.iter().map(Instance::context).collect();
    assert_eq!(contexts, [Context::Site(a), Context::Site(b)]);
    let column = |target: Target| match target {
        Target::Column(column) => Some(column.index()),
        Target::Row(_) => None,
    };
    let targets: Vec<Option<usize>> = instances
        .iter()
        .map(|instance| column(instance.target()))
        .collect();
    assert_eq!(targets, [Some(0), Some(1)]);
    for (instance, site) in instances.iter().zip([a, b]) {
        let mut read: Vec<(ParameterKey, Binding)> = (instance.parameters().iter())
            .map(|id| {
                let slot = &compiled.parameters()[id.index()];
                (slot.key(), slot.binding())
            })
            .collect();
        read.sort_unstable();
        let site = Binding::Site(site);
        assert_eq!(
            read,
            [
                (ParameterKey::new("fl.e"), site),
                (ParameterKey::new("fl.kf"), site)
            ]
        );
    }
    Ok(())
}

#[test]
fn a_site_parameter_binds_per_site_with_its_class_as_default() -> Result<(), AdmissionError> {
    let Beds { a, b, .. } = beds()?;
    let compiled = starling(Defect::None)?;
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
    let bed = Some(Binding::Class(Class::CapillaryBed));
    let (e, kf) = (ParameterKey::new("fl.e"), ParameterKey::new("fl.kf"));
    assert_eq!(
        slots,
        [
            (e, Binding::Site(a), bed, AdmissibleSet::Positive),
            (e, Binding::Site(b), bed, AdmissibleSet::Positive),
            (kf, Binding::Site(a), bed, AdmissibleSet::Nonnegative),
            (kf, Binding::Site(b), bed, AdmissibleSet::Nonnegative),
        ]
    );
    Ok(())
}

#[test]
fn each_bed_is_its_own_block_and_its_amounts_read_each_other() -> Result<(), AdmissionError> {
    let compiled = starling(Defect::None)?;
    let sparsity = compiled.sparsity();
    assert_eq!(sparsity.row_offsets(), [0, 2, 4, 6, 8]);
    let unknowns: Vec<usize> = sparsity
        .unknowns()
        .iter()
        .map(|unknown| unknown.index())
        .collect();
    assert_eq!(unknowns, [0, 1, 0, 1, 2, 3, 2, 3]);
    let blocks: Vec<(Vec<usize>, RateBand)> = (compiled.blocks().iter())
        .map(|block| {
            let rows = block.rows().iter().map(|row| row.index()).collect();
            (rows, block.rate_class().band())
        })
        .collect();
    assert_eq!(
        blocks,
        [(vec![0, 1], RateBand::Minutes), (vec![2, 3], RateBand::Minutes)]
    );
    Ok(())
}

#[test]
fn the_body_compiles_only_with_cells_lumen_and_enclosure_fluid_left_out() -> Result<(), AdmissionError> {
    let Beds { a, vessel, space, .. } = beds()?;
    let orphan = |site, space| CompileDefect::OrphanPool { site, space };
    assert_eq!(compile_refusal(Defect::NothingLeftOut), orphan(a, Space::Cells));
    assert_eq!(compile_refusal(Defect::CellsModeled), orphan(a, Space::Cells));
    assert_eq!(
        compile_refusal(Defect::LumenModeled),
        orphan(vessel, Space::Lumen)
    );
    assert_eq!(compile_refusal(Defect::FluidModeled), orphan(space, Space::Fluid));
    Ok(())
}

#[test]
fn a_law_that_moves_a_left_out_pool_is_refused() -> Result<(), AdmissionError> {
    let Beds { a, .. } = beds()?;
    assert_eq!(
        compile_refusal(Defect::InterstitiumLeftOut),
        CompileDefect::LeftOutTouched {
            law: LawName {
                owner: "FL",
                local: "starling"
            },
            site: a,
            space: Space::Interstitium,
        }
    );
    Ok(())
}

#[test]
fn a_volume_needs_the_schemas_partial_volume() {
    assert_eq!(
        compile_refusal(Defect::NoPartialVolume),
        CompileDefect::MissingPartialVolume {
            law: LawName {
                owner: "FL",
                local: "starling"
            }
        }
    );
}

#[test]
fn a_requested_route_family_with_no_law_is_an_omitted_law() {
    assert_eq!(
        compile_refusal(Defect::NoStarlingLaw),
        CompileDefect::OmittedLaw {
            law: LawName {
                owner: "FL",
                local: "starling"
            }
        }
    );
}

#[test]
fn a_slot_under_a_route_familys_name_is_named_twice() {
    assert_eq!(
        compile_refusal(Defect::SlotNamedLikeRoute),
        CompileDefect::NamedTwice {
            law: LawName {
                owner: "FL",
                local: "starling"
            }
        }
    );
}

#[test]
fn one_parameter_key_in_two_contexts_is_a_conflict() {
    assert_eq!(
        compile_refusal(Defect::ParameterInTwoContexts),
        CompileDefect::ParameterConflict {
            key: ParameterKey::new("fl.kf")
        }
    );
}

#[test]
fn a_partial_volume_is_positive() -> Result<(), AdmissionError> {
    for refused in [0.0, -1.0] {
        let mut schema = beds()?.schema;
        assert_eq!(
            schema.partial_volume(Measure::new(refused)?),
            Err(AdmissionError::NonPositivePartialVolume)
        );
    }
    Ok(())
}

#[test]
fn a_second_partial_volume_must_repeat_the_first() -> Result<(), AdmissionError> {
    let mut schema = beds()?.schema;
    schema.partial_volume(Measure::new(1.0)?)?;
    assert_eq!(schema.partial_volume(Measure::new(1.0)?), Ok(()));
    assert_eq!(
        schema.partial_volume(Measure::new(2.0)?),
        Err(AdmissionError::PartialVolumeTwice)
    );
    Ok(())
}

fn unread(left_out: Space) -> Result<CompiledBody, AdmissionError> {
    let mut schema = beds()?.schema;
    schema.partial_volume(Measure::new(1.0)?)?;
    schema.leave_out(Space::Cells, "no cell-water law in this bench");
    schema.leave_out(Space::Lumen, "no circulation law in this bench");
    schema.leave_out(Space::Fluid, "no enclosure law in this bench");
    schema.leave_out(left_out, "a defect: the Starling law moves it");
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.fluids()?.parts();
        let route = ports.starling(&mut laws);
        let minutes = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
        let still = laws.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), minutes)
        });
        laws.transfer_at(&route, still);
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn a_law_that_moves_a_left_out_pool_without_reading_it_is_refused() -> Result<(), AdmissionError> {
    let Beds { a, .. } = beds()?;
    for space in [Space::Plasma, Space::Interstitium] {
        assert_eq!(
            unread(space).map(|compiled| compiled.model_hash()),
            Err(AdmissionError::Compile(CompileDefect::LeftOutTouched {
                law: LawName {
                    owner: "FL",
                    local: "starling"
                },
                site: a,
                space,
            }))
        );
    }
    Ok(())
}

fn looped(lumen: Option<&'static str>, reads_amount: bool) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    map.contain(root.id(), vessel.id());
    map.connect(Network::Vascular, vessel.id(), vessel.id());
    let mut schema = Schema::new(map.finish()?)?;
    if let Some(reason) = lumen {
        schema.leave_out(Space::Lumen, reason);
    }
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let speed =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("cv.speed"), Admissible::nonnegative());
        let wave = RateClass::new(RateBand::Wave, 1_000_000)?;
        let rate = laws.edge_kernel(|mut k| {
            let rate = if reads_amount {
                let empty = k.zero::<Water>();
                let held = k.at_edge(&route.from_amount());
                let pace = k.parameter(&speed);
                k.relax(empty, held, pace)
            } else {
                k.zero::<Rate<Water>>()
            };
            k.finish(rate, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, rate);
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn a_transfer_whose_ends_are_one_pool_moves_nothing() -> Result<(), AdmissionError> {
    let compiled = looped(Some("the loop moves no water"), false)?;
    assert!(compiled.rows().is_empty());
    let columns: Vec<usize> = compiled
        .columns()
        .iter()
        .map(|column| column.terms().len())
        .collect();
    assert_eq!(columns, [0]);
    Ok(())
}

#[test]
fn a_read_of_a_left_out_pool_is_refused() -> Result<(), AdmissionError> {
    let refused = looped(Some("the loop moves no water"), true).map(|compiled| compiled.model_hash());
    let vessel = AnatomyBuilder::new().site::<VesselSegment>("vessel", at()?).id();
    assert_eq!(
        refused,
        Err(AdmissionError::Compile(CompileDefect::LeftOutTouched {
            law: LawName {
                owner: "CV",
                local: "circulation"
            },
            site: vessel,
            space: Space::Lumen,
        }))
    );
    Ok(())
}

#[test]
fn a_read_never_moves_a_pool() -> Result<(), AdmissionError> {
    let refused = looped(None, true).map(|compiled| compiled.model_hash());
    let vessel = AnatomyBuilder::new().site::<VesselSegment>("vessel", at()?).id();
    assert_eq!(
        refused,
        Err(AdmissionError::Compile(CompileDefect::OrphanPool {
            site: vessel,
            space: Space::Lumen
        }))
    );
    Ok(())
}
