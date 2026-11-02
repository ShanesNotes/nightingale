use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

#[derive(Clone, Copy, PartialEq)]
enum Defect {
    None,
    NoBoundaryScope,
    InflowIntoCells,
    InflowAtUndeclaredSite,
    InflowNamedTwice,
    UnknownBoundaryName,
    BoundarySlotNamedLikeInflow,
    LumenLeftOut,
}

struct Reservoirs {
    a: SiteId,
    b: SiteId,
    compiled: Result<CompiledBody, AdmissionError>,
}

fn reservoirs(defect: Defect) -> Result<Reservoirs, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let a = map.site::<VesselSegment>("a", at()?);
    let b = map.site::<VesselSegment>("b", at()?);
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish()?)?;
    let (site, space) = match defect {
        Defect::InflowIntoCells => (a.id(), Space::Cells),
        Defect::InflowAtUndeclaredSite => (
            AnatomyBuilder::new().site::<VesselSegment>("c", at()?).id(),
            Space::Lumen,
        ),
        _ => (a.id(), Space::Lumen),
    };
    schema.boundary::<Water>("inflow", BoundaryDirection::IntoBody, site, space)?;
    if defect == Defect::InflowNamedTwice {
        schema.boundary::<Water>("inflow", BoundaryDirection::OutOfBody, b.id(), Space::Lumen)?;
    }
    if defect == Defect::LumenLeftOut {
        schema.leave_out(Space::Lumen, "a defect: the inflow and the circulation move it");
    }
    schema.partial_volume(Measure::new(1.0)?)?;
    let compiled = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let e = laws.end_parameter::<Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
        let g = laws
            .edge_parameter::<_, Conductance<Water>>(ParameterKey::new("cv.g"), Admissible::nonnegative());
        let wave = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let upstream = k.at_edge(&route.from_amount());
            let downstream = k.at_edge(&route.to_amount());
            let upstream = k.volume(upstream);
            let downstream = k.volume(downstream);
            let upstream_elastance = k.upstream_parameter(&e);
            let downstream_elastance = k.downstream_parameter(&e);
            let upstream = k.multiply(upstream_elastance, upstream);
            let downstream = k.multiply(downstream_elastance, downstream);
            let upstream = k.as_dimension::<_, Pressure>(upstream);
            let downstream = k.as_dimension::<_, Pressure>(downstream);
            let head = k.subtract(upstream, downstream);
            let conductance = k.edge_parameter(&g);
            let flow = k.conductance_flow(conductance, head);
            k.finish(flow, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        if defect != Defect::NoBoundaryScope {
            let (mut laws, ports) = body.boundary()?.parts();
            let name = if defect == Defect::UnknownBoundaryName {
                "outflow"
            } else {
                "inflow"
            };
            let BoundaryRoute { route: pair, drive } = ports.route::<Water>(name)?;
            if defect == Defect::BoundarySlotNamedLikeInflow {
                laws.state::<Dimensionless>("inflow", Admissible::finite());
            }
            let rate = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, laws.every_mode(), wave)
            });
            laws.transfer(Transfer::new(&pair, rate));
            body.add(laws.finish());
        }
        body.compile()
    });
    Ok(Reservoirs {
        a: a.id(),
        b: b.id(),
        compiled,
    })
}

fn refusal(defect: Defect) -> AdmissionError {
    match reservoirs(defect) {
        Ok(Reservoirs {
            compiled: Ok(compiled),
            ..
        }) => panic!("compiled to {:?}", compiled.model_hash()),
        Ok(Reservoirs {
            compiled: Err(refused),
            ..
        })
        | Err(refused) => refused,
    }
}

fn law(owner: &'static str, local: &'static str) -> LawName {
    LawName { owner, local }
}

fn column(compiled: &CompiledBody, origin: LawName) -> &Column {
    let found = compiled.columns().iter().find(|column| column.origin() == origin);
    found.unwrap_or_else(|| panic!("no column of {origin:?}"))
}

fn coefficients(column: &Column, rows: usize) -> Vec<i64> {
    (0..rows)
        .map(|row| {
            let term = column.terms().iter().find(|term| term.0.index() == row);
            term.map_or(0, |term| term.1)
        })
        .collect()
}

#[test]
fn the_two_reservoir_body_has_the_hand_derived_incidence_and_basis() -> Result<(), AdmissionError> {
    let Reservoirs { a, b, compiled } = reservoirs(Defect::None)?;
    let compiled = compiled?;
    let lumen = |site| RowKind::Amount {
        site,
        space: Space::Lumen,
        species: Species::Water,
    };
    let kinds: Vec<RowKind> = compiled.rows().iter().map(Row::kind).collect();
    assert_eq!(kinds, [lumen(a), lumen(b)]);
    assert_eq!(compiled.columns().len(), 2);
    let circulation = column(&compiled, law("CV", "circulation"));
    assert_eq!(circulation.context(), Context::Edge { from: a, to: b });
    assert_eq!(coefficients(circulation, 2), [-1, 1]);
    assert_eq!(circulation.receipt(), None);
    let inflow = column(&compiled, law("Boundary", "inflow"));
    assert_eq!(inflow.context(), Context::Body);
    assert_eq!(coefficients(inflow, 2), [1, 0]);
    let receipt = inflow
        .receipt()
        .map(|receipt| (receipt.boundary(), receipt.coefficient()));
    assert_eq!(receipt, Some(("inflow", -1)));
    let basis: Vec<(Book, Vec<(usize, i64)>)> = (compiled.conserved_basis().combinations().iter())
        .map(|combination| {
            let terms = combination
                .terms()
                .iter()
                .map(|term| (term.0.index(), term.1))
                .collect();
            (combination.book(), terms)
        })
        .collect();
    assert_eq!(basis, [(Book::Of(Species::Water), vec![(0, 1), (1, 1)])]);
    Ok(())
}

#[test]
fn one_edge_law_reads_each_ends_own_elastance() -> Result<(), AdmissionError> {
    let Reservoirs { a, b, compiled } = reservoirs(Defect::None)?;
    let compiled = compiled?;
    let slots: Vec<(ParameterKey, Binding, Option<Binding>)> = (compiled.parameters().iter())
        .map(|slot| (slot.key(), slot.binding(), slot.default_binding()))
        .collect();
    let network = Some(Binding::Network(Network::Vascular));
    let (e, g) = (ParameterKey::new("cv.e"), ParameterKey::new("cv.g"));
    assert_eq!(
        slots,
        [
            (e, Binding::Site(a), network),
            (e, Binding::Site(b), network),
            (g, Binding::Edge { from: a, to: b }, network),
        ]
    );
    let flow = (compiled.instances().iter())
        .find(|instance| instance.context() == Context::Edge { from: a, to: b })
        .unwrap_or_else(|| panic!("no instance along the edge"));
    let mut read: Vec<usize> = flow.parameters().iter().map(|id| id.index()).collect();
    read.sort_unstable();
    assert_eq!(read, [0, 1, 2]);
    Ok(())
}

#[test]
fn both_reservoirs_form_one_block_through_the_flow_between_them() -> Result<(), AdmissionError> {
    let compiled = reservoirs(Defect::None)?.compiled?;
    let sparsity = compiled.sparsity();
    assert_eq!(sparsity.row_offsets(), [0, 2, 4]);
    let unknowns: Vec<usize> = sparsity
        .unknowns()
        .iter()
        .map(|unknown| unknown.index())
        .collect();
    assert_eq!(unknowns, [0, 1, 0, 1]);
    let blocks: Vec<(Vec<usize>, RateBand, u64)> = (compiled.blocks().iter())
        .map(|block| {
            let rows = block.rows().iter().map(|row| row.index()).collect();
            (rows, block.rate_class().band(), block.rate_class().maximum_ns())
        })
        .collect();
    assert_eq!(blocks, [(vec![0, 1], RateBand::Wave, 3_600_000_000_000)]);
    Ok(())
}

#[test]
fn a_declared_boundary_with_no_law_is_an_omitted_law() {
    assert_eq!(
        refusal(Defect::NoBoundaryScope),
        AdmissionError::Compile(CompileDefect::OmittedLaw {
            law: law("Boundary", "inflow")
        })
    );
}

#[test]
fn a_boundary_at_a_space_its_site_does_not_have_is_refused() {
    let refused = AdmissionError::Compile(CompileDefect::BoundaryEndpoint {
        law: law("Boundary", "inflow"),
    });
    assert_eq!(refusal(Defect::InflowIntoCells), refused);
    assert_eq!(refusal(Defect::InflowAtUndeclaredSite), refused);
}

#[test]
fn a_boundary_name_is_declared_once() {
    assert_eq!(
        refusal(Defect::InflowNamedTwice),
        AdmissionError::BoundaryNamedTwice { name: "inflow" }
    );
}

#[test]
fn a_boundary_port_names_only_a_declared_boundary() {
    assert_eq!(
        refusal(Defect::UnknownBoundaryName),
        AdmissionError::UnknownBoundary { name: "outflow" }
    );
}

#[test]
fn a_boundary_slot_under_a_boundarys_name_is_named_twice() {
    assert_eq!(
        refusal(Defect::BoundarySlotNamedLikeInflow),
        AdmissionError::Compile(CompileDefect::NamedTwice {
            law: law("Boundary", "inflow")
        })
    );
}

#[test]
fn a_boundary_transfer_into_a_left_out_pool_is_refused() -> Result<(), AdmissionError> {
    let Reservoirs { a, .. } = reservoirs(Defect::None)?;
    assert_eq!(
        refusal(Defect::LumenLeftOut),
        AdmissionError::Compile(CompileDefect::LeftOutTouched {
            law: law("Boundary", "inflow"),
            site: a,
            space: Space::Lumen,
        })
    );
    Ok(())
}

#[derive(Clone, Copy)]
enum Reads {
    Upstream,
    Downstream,
}

fn one_end(reads: Reads) -> Result<(SiteId, SiteId, CompiledBody), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let a = map.site::<VesselSegment>("a", at()?);
    let b = map.site::<VesselSegment>("b", at()?);
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    let compiled = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let e = laws.end_parameter::<Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
        let g = laws
            .edge_parameter::<_, Conductance<Water>>(ParameterKey::new("cv.g"), Admissible::nonnegative());
        let wave = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let elastance = match reads {
                Reads::Upstream => k.upstream_parameter(&e),
                Reads::Downstream => k.downstream_parameter(&e),
            };
            let amount = k.at_edge(&route.from_amount());
            let volume = k.volume(amount);
            let pressure = k.multiply(elastance, volume);
            let head = k.as_dimension::<_, Pressure>(pressure);
            let conductance = k.edge_parameter(&g);
            let flow = k.conductance_flow(conductance, head);
            k.finish(flow, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        body.compile()
    })?;
    Ok((a.id(), b.id(), compiled))
}

#[test]
fn the_upstream_parameter_binds_at_the_edges_upstream_site_and_the_downstream_at_its_downstream()
-> Result<(), AdmissionError> {
    let network = Some(Binding::Network(Network::Vascular));
    let (e, g) = (ParameterKey::new("cv.e"), ParameterKey::new("cv.g"));
    for reads in [Reads::Upstream, Reads::Downstream] {
        let (a, b, compiled) = one_end(reads)?;
        let site = match reads {
            Reads::Upstream => a,
            Reads::Downstream => b,
        };
        let slots: Vec<(ParameterKey, Binding, Option<Binding>)> = (compiled.parameters().iter())
            .map(|slot| (slot.key(), slot.binding(), slot.default_binding()))
            .collect();
        assert_eq!(
            slots,
            [
                (e, Binding::Site(site), network),
                (g, Binding::Edge { from: a, to: b }, network),
            ]
        );
    }
    Ok(())
}
