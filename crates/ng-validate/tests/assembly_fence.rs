use core::num::{NonZeroU32, NonZeroU64};
use ng_contract::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;

struct TwoSecondCycle;

impl CaseAssembly for TwoSecondCycle {
    fn compiled() -> Result<CompiledBody, AdmissionError> {
        let mut map = AnatomyBuilder::new();
        let site = map.site::<VesselSegment>(
            "synthetic vessel",
            Position::new(Measure::new(0.0)?, Measure::new(0.0)?, Measure::new(0.0)?),
        );
        let mut schema = Schema::new(map.finish()?)?;
        schema.boundary::<Water>("gut water", BoundaryDirection::IntoBody, site.id(), Space::Lumen)?;
        assemble(schema, |mut body| {
            let (mut laws, ports) = body.boundary()?.parts();
            let BoundaryRoute { route, drive } = ports.route::<Water>("gut water")?;
            let class = RateClass::new(RateBand::Wave, SECOND)?;
            let flow = laws.kernel(|mut k| {
                let value = k.read(&drive);
                k.finish(value, laws.every_mode(), class)
            });
            laws.transfer(Transfer::new(&route, flow));
            body.add(laws.finish());
            body.compile()
        })
    }

    fn packet(body: &CompiledBody) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
        let cycle = Cycle::new(
            NonZeroU64::new(2 * SECOND).expect("a period"),
            NonZeroU32::new(2).expect("two intervals"),
        )?;
        let mut packet = ParameterPacket::periodic(cycle);
        packet.within(admission_tolerance());
        packet.start(body.rows()[0].kind(), Measure::<Water>::new(10.0)?, "synthetic")?;
        Ok(packet)
    }

    fn lens(body: &CompiledBody) -> Result<ObservationLens, AdmissionError> {
        body.lens(
            vec![ChannelSpec::new("synthetic gauge", body.rows()[0].kind())],
            NonZeroU64::new(SECOND).expect("a grid"),
        )
    }
}

#[test]
fn an_assembly_case_places_its_inputs_at_the_fence_its_packets_admission_cycle_sets() {
    let case = AssemblyCase::<TwoSecondCycle>::new().expect("solved");
    let fence = SimTime::from_ns(anchor_moment().nanoseconds() + 2 * SECOND);
    let events = case.events().expect("the inputs");
    assert_eq!(events.len(), 2);
    let stamps: Vec<SimTime> = events.iter().map(|event| event.at()).collect();
    assert_eq!(stamps, [fence, fence]);

    let core = ReferenceIntegrator::new(SECOND / 10);
    let shared_links = MemoryStore::default();
    let shared = case.admit(&core, &shared_links).expect("admitted from the solve");
    let body = case.body();
    let (packet, lens) = (
        TwoSecondCycle::packet(body).expect("a packet"),
        TwoSecondCycle::lens(body).expect("a lens"),
    );
    let port = BoundaryPort::new(events).expect("a port");
    let own_links = MemoryStore::default();
    let own = admit_at(body, packet, port, &core, &lens, anchor_moment(), &own_links).expect("admitted");
    let (shared, own) = (shared.record(), own.session.record());
    assert_eq!(own.earliest_stamp(), fence);
    assert_eq!(shared.identity().digest(), own.identity().digest());
    assert_eq!(shared.digests(), own.digests());
    assert_eq!(shared.seal().digest(), own.seal().digest());
}
