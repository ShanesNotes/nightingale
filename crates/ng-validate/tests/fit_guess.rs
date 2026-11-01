use core::num::NonZeroU64;
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const GAIN: ParameterKey = ParameterKey::new("test.gain");
const ANCHOR: AnchorId = AnchorId::new("synthetic reading");

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).unwrap()
}

fn fitted(guess: f64) -> Result<f64, InitializationFailure> {
    let mut map = AnatomyBuilder::new();
    let site = map.site::<VesselSegment>(
        "synthetic vessel",
        Position::new(measure(0.0), measure(0.0), measure(0.0)),
    );
    let mut schema = Schema::new(map.finish()?)?;
    schema.boundary::<Water>("in", BoundaryDirection::IntoBody, site.id(), Space::Lumen)?;
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryRoute { route, drive } = ports.route::<Water>("in")?;
        let class = RateClass::new(RateBand::Minutes, 60 * SECOND)?;
        let flow = laws.kernel(|mut k| {
            let value = k.read(&drive);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, flow));
        let gain = laws.parameter::<Dimensionless>(GAIN, Admissible::finite());
        let reading = laws.algebraic::<Rate<Water>>("reading", Admissible::finite());
        let residual = laws.kernel(|mut k| {
            let x = k.algebraic(&reading);
            let input = k.read(&drive);
            let gain = k.parameter(&gain);
            let target = k.scale(input, gain);
            let residual = k.subtract(x, target);
            k.finish(residual, laws.every_mode(), class)
        });
        laws.residual(&reading, residual);
        body.add(laws.finish());
        body.compile()
    })?;
    let reading = RowKind::Algebraic(
        LawName {
            owner: "Boundary",
            local: "reading",
        }
        .into(),
    );
    let mut packet = ParameterPacket::new();
    packet.within(admission_tolerance());
    packet.start(
        RowKind::Amount {
            site: site.id(),
            space: Space::Lumen,
            species: Species::Water,
        },
        measure::<Water>(10.0),
        "synthetic",
    )?;
    packet.start(reading, measure::<Rate<Water>>(1.0), "synthetic")?;
    packet.anchor(Synthetic::new(
        ANCHOR,
        Snapshot::new(
            reading,
            "synthetic gauge",
            AnchorStatistic::Instant,
            measure::<Rate<Water>>(6.0),
        ),
        "synthetic",
    ))?;
    packet.calibrated(GAIN, Binding::Body, measure::<Dimensionless>(guess), ANCHOR)?;
    let lens = body.lens(
        vec![ChannelSpec::new("synthetic gauge", reading)],
        NonZeroU64::new(SECOND).unwrap(),
    )?;
    let event = BoundaryEvent::new(
        InputId::new(Digest::of_canonical(b"synthetic input")),
        SimTime::from_ns(0),
        BoundaryInput::Water(WaterDelivery {
            boundary: "in",
            amount: measure(3.0),
            ends: SimTime::from_ns(SECOND),
        }),
    );
    let boundary = BoundaryPort::new(vec![event])?;
    let core = ReferenceIntegrator::new(SECOND);
    let admitted = admit_solved(&body, packet, boundary, &core, &lens)?;
    let record = admitted
        .provenance
        .values()
        .iter()
        .find(|record| {
            record.name
                == ValueName::Parameter {
                    key: GAIN,
                    binding: Binding::Body,
                }
        })
        .unwrap();
    Ok(f64::from_bits(record.value.bits()[0]))
}

#[test]
fn a_fit_from_a_guess_of_one_reaches_its_anchor() {
    assert!((fitted(1.0).expect("linear fit") - 2.0).abs() < 1e-12);
}

#[test]
fn a_fit_can_leave_a_guess_of_zero() {
    assert!((fitted(0.0).expect("zero is a legal initial fit") - 2.0).abs() < 1e-12);
}

#[test]
fn a_finite_fit_can_change_sign() {
    assert!((fitted(-1.0).expect("finite fit may change sign") - 2.0).abs() < 1e-12);
}
