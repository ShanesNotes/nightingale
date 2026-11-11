use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;

fn admits_inconsistent_store(power_size: f64) -> Result<bool, InitializationFailure> {
    let mut map = AnatomyBuilder::new();
    let zero = || Measure::new(0.0).unwrap();
    let root = map.site::<TissueSite>("body", Position::new(zero(), zero(), zero()));
    let mut schema = Schema::new(map.finish()?)?;
    schema.pool::<Energy>(root.id(), Space::Elastic)?;
    schema.pool::<Energy>(root.id(), Space::Thermal)?;
    let body = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let class = RateClass::new(RateBand::Wave, 1_000_000_000)?;
        let store = ports.dissipation(&mut cv);
        let wall = cv.parameter::<Energy>(ParameterKey::new("synthetic.wall"), Admissible::positive());
        let power = cv.algebraic::<Rate<Energy>>("dissipated power", Admissible::nonnegative());
        let close = cv.kernel(|mut k| {
            let held = k.read(&store.amount::<Energy>());
            let wall = k.parameter(&wall);
            let gap = k.subtract(held, wall);
            k.finish(gap, cv.every_mode(), class)
        });
        cv.closes(&power, close);
        let leave = cv.kernel(|mut k| {
            let value = k.algebraic(&power);
            k.finish(value, cv.every_mode(), class)
        });
        cv.transfer(Transfer::new(&store, leave));
        body.add(cv.finish());
        body.compile()
    })?;
    let elastic = RowKind::Amount {
        site: root.id(),
        space: Space::Elastic,
        species: Species::Energy,
    };
    let thermal = RowKind::Amount {
        site: root.id(),
        space: Space::Thermal,
        species: Species::Energy,
    };
    let power = RowKind::Algebraic(
        LawName {
            owner: "CV",
            local: "dissipated power",
        }
        .into(),
    );
    let mut packet = ParameterPacket::periodic(Cycle::new(
        NonZeroU64::new(2_000_000_000).unwrap(),
        NonZeroU32::new(2).unwrap(),
    )?);
    packet.within(ErrorScales::uniform(Measure::new(1e-9)?, Measure::new(1e-10)?)?);
    packet.synthetic(
        ParameterKey::new("synthetic.wall"),
        Binding::Body,
        Measure::<Energy>::new(1e-9)?,
        "synthetic",
    )?;
    packet.start(elastic, Measure::<Energy>::new(1e-9)?, "synthetic")?;
    packet.start(thermal, Measure::<Energy>::new(1.0)?, "synthetic")?;
    packet.start(power, Measure::<Rate<Energy>>::new(0.0)?, "synthetic")?;
    packet.row_scale(elastic, Measure::<Energy>::new(1e-9)?)?;
    packet.row_scale(power, Measure::<Rate<Energy>>::new(power_size)?)?;
    let index = |kind| {
        body.rows()
            .iter()
            .find(|row| row.kind() == kind)
            .unwrap()
            .id()
            .index()
    };
    let elastic_index = index(elastic);
    let thermal_index = index(thermal);
    let system = body.initialization(
        body.bind(packet)?,
        BoundaryPort::new(Vec::new())?,
        InitializationRequest::at(SimTime::from_ns(0)),
    )?;
    let mut bad = system.workspace();
    for point in 0..system.points() {
        let offset = point * system.rows();
        bad.unknowns_mut()[offset + elastic_index] += 1e-4;
        bad.unknowns_mut()[offset + thermal_index] -= 1e-4;
    }
    let history = MethodHistory::new(
        Digest::of_canonical(b"synthetic"),
        SimTime::from_ns(0),
        Vec::new(),
    );
    let accepted = system.admit(bad, history);
    Ok(accepted.is_ok())
}

#[test]
fn an_energy_closure_is_judged_in_energy_units_not_its_power_unknowns_scale()
-> Result<(), InitializationFailure> {
    assert!(
        !admits_inconsistent_store(1.0)?,
        "the unscaled control must reject the wrong store"
    );
    assert!(
        !admits_inconsistent_store(1e6)?,
        "the store is 100001 times its wall energy, despite an energy row scale of 1e-9"
    );
    Ok(())
}
