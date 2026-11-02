use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;

fn body(kept_back: bool) -> Result<(CompiledBody, SiteId), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let position = || {
        Position::new(
            Measure::new(0.0).unwrap(),
            Measure::new(0.0).unwrap(),
            Measure::new(0.0).unwrap(),
        )
    };
    let root = map.site::<TissueSite>("body", position());
    let vessel = map.site::<VesselSegment>("vessel", position());
    let bed = map.site::<CapillaryBed>("bed", position());
    map.contain(root.id(), vessel.id());
    map.contain(root.id(), bed.id());
    map.connect(Network::Vascular, vessel.id(), bed.id());
    map.connect(Network::Vascular, bed.id(), vessel.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    schema.leave_out(Space::Cells, "synthetic test has no cell-water law");
    schema.carry::<Albumin, Blood>(Space::Plasma);
    schema.carry::<Albumin, Blood>(Space::Lumen);
    schema.carry::<Albumin, CapillaryBed>(Space::Interstitium);
    let compiled = assemble(schema, |mut assembly| {
        let class = RateClass::new(RateBand::Wave, 1_000_000_000)?;
        let (mut cv, ports) = assembly.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let value = k.zero::<Rate<Water>>();
            k.finish(value, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        assembly.add(cv.finish());
        let (mut fl, ports) = assembly.fluids()?.parts();
        let route = ports.starling(&mut fl);
        if kept_back {
            route.keeps_back::<Albumin>(&mut fl);
        }
        let rate = fl.site_parameter::<CapillaryBed, Rate<Water>>(
            ParameterKey::new("synthetic.flow"),
            Admissible::positive(),
        );
        let moving = fl.site_kernel(|mut k| {
            let value = k.site_parameter(&rate);
            k.finish(value, fl.every_mode(), class)
        });
        fl.transfer_at(&route, moving);
        let exchange = ports.exchange::<Albumin>(&mut fl);
        let still = fl.site_kernel(|mut k| {
            let value = k.zero::<Rate<Albumin>>();
            k.finish(value, fl.every_mode(), class)
        });
        fl.transfer_at(&exchange, still);
        assembly.add(fl.finish());
        assembly.compile()
    })?;
    Ok((compiled, bed.id()))
}

#[test]
fn a_species_a_route_keeps_back_stays_back_when_both_ends_hold_it() -> Result<(), InitializationFailure> {
    let (body, bed) = body(true)?;
    let cycle = Cycle::new(
        NonZeroU64::new(2_000_000_000).unwrap(),
        NonZeroU32::new(2).unwrap(),
    )?;
    let mut packet = ParameterPacket::periodic(cycle);
    packet.synthetic(
        ParameterKey::new("synthetic.flow"),
        Binding::Site(bed),
        Measure::<Rate<Water>>::new(2.0)?,
        "synthetic",
    )?;
    packet.start_rest(
        &body,
        |kind| match kind {
            RowKind::Amount {
                species: Species::Albumin,
                ..
            } => 8.0,
            RowKind::Amount { .. } => 4.0,
            RowKind::Algebraic(Slot {
                law:
                    LawName {
                        owner: "FL",
                        local: "starling",
                    },
                ..
            }) => 2.0,
            _ => 0.0,
        },
        "synthetic",
    )?;
    let row = body
        .rows()
        .iter()
        .find(|row| {
            row.kind()
                == RowKind::Amount {
                    site: bed,
                    space: Space::Interstitium,
                    species: Species::Albumin,
                }
        })
        .unwrap()
        .id()
        .index();
    let system = body.initialization(
        body.bind(packet)?,
        BoundaryPort::new(Vec::new())?,
        InitializationRequest::at(SimTime::from_ns(0)),
    )?;
    let (residual, _) = system.stage(&system.workspace(), 0)?;
    assert_eq!(
        residual[row], 0.0,
        "a wall that keeps albumin back must carry no albumin"
    );
    Ok(())
}

#[test]
fn keeping_a_species_back_changes_the_model() -> Result<(), AdmissionError> {
    let (permeable, _) = body(false)?;
    let (impermeable, _) = body(true)?;
    assert_ne!(
        permeable.model_hash(),
        impermeable.model_hash(),
        "the stated wall changes the body"
    );
    Ok(())
}
