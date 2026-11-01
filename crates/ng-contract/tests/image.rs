use ng_contract::*;

const SECOND: u64 = 1_000_000_000;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn origin() -> Result<Position, AdmissionError> {
    Ok(Position::new(measure(0.0), measure(0.0), measure(0.0)))
}

fn body() -> Result<(CompiledBody, [SiteId; 2]), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin()?);
    let a = map.site::<VesselSegment>("a", origin()?);
    let b = map.site::<VesselSegment>("b", origin()?);
    map.contain(root.id(), a.id());
    map.contain(root.id(), b.id());
    map.connect(Network::Vascular, a.id(), b.id());
    let mut schema = Schema::new(map.finish()?)?;
    schema.boundary::<Water>("inflow", BoundaryDirection::IntoBody, a.id(), Space::Lumen)?;
    schema.boundary::<Water>("unread", BoundaryDirection::IntoBody, b.id(), Space::Lumen)?;
    schema.partial_volume(measure(1.0))?;
    let body = assemble(schema, |mut body| {
        let (mut laws, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut laws);
        let e = laws.edge_parameter::<_, Elastance>(ParameterKey::new("cv.e"), Admissible::positive());
        let g = laws
            .edge_parameter::<_, Conductance<Water>>(ParameterKey::new("cv.g"), Admissible::nonnegative());
        let wave = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let flow = laws.edge_kernel(|mut k| {
            let upstream = k.at_edge(&route.from_amount());
            let downstream = k.at_edge(&route.to_amount());
            let upstream = k.volume(upstream);
            let downstream = k.volume(downstream);
            let difference = k.subtract(upstream, downstream);
            let elastance = k.edge_parameter(&e);
            let pressure = k.multiply(elastance, difference);
            let head = k.as_dimension::<_, Pressure>(pressure);
            let conductance = k.edge_parameter(&g);
            let flow = k.conductance_flow(conductance, head);
            k.finish(flow, laws.every_mode(), wave)
        });
        laws.transfer_along(&route, flow);
        body.add(laws.finish());
        let (mut laws, ports) = body.boundary()?.parts();
        let BoundaryRoute { route: pair, drive } = ports.route::<Water>("inflow")?;
        let rate = laws.kernel(|mut k| {
            let value = k.read(&drive);
            k.finish(value, laws.every_mode(), wave)
        });
        laws.transfer(Transfer::new(&pair, rate));
        let BoundaryRoute { route: unread, .. } = ports.route::<Water>("unread")?;
        let nothing = laws.kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, laws.every_mode(), wave)
        });
        laws.transfer(Transfer::new(&unread, nothing));
        body.add(laws.finish());
        body.compile()
    })?;
    Ok((body, [a.id(), b.id()]))
}

fn water(id: &[u8], from: SimTime, to: SimTime) -> BoundaryEvent {
    let delivery = WaterDelivery {
        boundary: "inflow",
        amount: measure(4.0),
        ends: to,
    };
    BoundaryEvent::new(
        InputId::new(Digest::of_canonical(id)),
        from,
        BoundaryInput::Water(delivery),
    )
}

fn image(body: &CompiledBody, sites: [SiteId; 2]) -> ImageParts {
    let mut packet = ParameterPacket::new();
    let edges = Binding::Network(Network::Vascular);
    let elastance = measure::<Elastance>(1.0);
    let conductance = measure::<Conductance<Water>>(0.5);
    (packet.synthetic(ParameterKey::new("cv.e"), edges, elastance, "synthetic")).expect("a value");
    (packet.synthetic(ParameterKey::new("cv.g"), edges, conductance, "synthetic")).expect("a value");
    for site in sites {
        let row = RowKind::Amount {
            site,
            space: Space::Lumen,
            species: Species::Water,
        };
        packet
            .start(row, measure::<Water>(3.0), "synthetic")
            .expect("a start");
    }
    let parameters = body.bind(packet).expect("bound");
    let port = BoundaryPort::new(vec![
        water(b"running", at(0), at(2)),
        water(b"waiting", at(5), at(9)),
    ]);
    let request = InitializationRequest::at(at(0));
    let system = body
        .initialization(parameters, port.expect("a port"), request)
        .expect("a system");
    let solved = system.workspace();
    let schema = Digest::of_canonical(b"an image test");
    let history = MethodHistory::new(schema, at(0), Vec::new());
    let revision = system.admit(solved, history).expect("admitted");
    let mut parts = revision.into_parts();
    let view = AcceptedStateView::new(parts.time, &parts.state, &parts.continuation, &parts.dense);
    let digest = Digest::of_canonical(b"an image test");
    let measure_of = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let scales = ErrorScales::uniform(measure_of(1.0), measure_of(0.0)).expect("scales");
    let resolution = DeclaredResolution::new(
        digest,
        1,
        4,
        ArithmeticProfile::contract_v1(),
        StageScheme::forward_euler(),
        scales,
    );
    let identity = RunIdentity::new(
        body.model_hash(),
        parts.parameters().packet_hash(),
        Seed::new([1, 2, 3, 4]),
        resolution,
    );
    let problem = CoupledProblem::new(body, parts.parameters(), &identity, view, digest);
    let attempt = Attempt::First { span_ns: SECOND };
    let mut issued = None;
    let _ = parts.pending.attempt(&problem, attempt, |ticket| {
        issued = Some(ticket.clone());
        Err::<(VerifiedStep, ()), _>(ticket.mismatch().into())
    });
    let applied = parts.pending.after(&issued.expect("a ticket"));
    parts.pending.advance(applied);
    parts.time = at(1);
    parts
}

const FORMAT: usize = 27;
const TIME: usize = FORMAT;
const AMOUNTS: usize = TIME + 8;
const FIRST_AMOUNT: usize = AMOUNTS + 8;
const MEMORY: usize = FIRST_AMOUNT + 32;
const ALGEBRAIC: usize = MEMORY + 8;
const MODES: usize = ALGEBRAIC + 8;
const ORDINALS: usize = MODES + 8;
const FIRED: usize = ORDINALS + 8;
const SCHEMA: usize = FIRED + 8;
const COUNTERS: usize = SCHEMA + 32 + 8;
const DENSE: usize = COUNTERS + 8;
const TIMELINE: usize = DENSE + 8 + 8 + 8;
const FIRST_DELIVERY: usize = TIMELINE + 8;
const DELIVERY: usize = 32 + 8 + 8 + 8 + 6 + 8 + 8;
const CURSOR: usize = FIRST_DELIVERY + 2 * DELIVERY;
const ACTIVE: usize = CURSOR + 8;
const FENCE: usize = ACTIVE + 8 + 8;
const MODEL: usize = FENCE + 8;
const VALUES: usize = MODEL + 32 + 32;

fn put(bytes: &mut [u8], at: usize, value: u64) {
    bytes[at..at + 8].copy_from_slice(&value.to_be_bytes());
}

fn with_segment(bytes: &mut Vec<u8>, from: u64, to: u64) {
    put(bytes, DENSE, 1);
    let mut segment = from.to_be_bytes().to_vec();
    segment.extend_from_slice(&to.to_be_bytes());
    for _ in 0..2 {
        segment.extend_from_slice(&2_u64.to_be_bytes());
        segment.extend_from_slice(&[0; 32]);
    }
    bytes.splice(DENSE + 8..DENSE + 24, segment);
}

#[test]
fn an_image_reads_back_to_the_same_bytes() {
    let (body, sites) = body().expect("a body");
    let parts = image(&body, sites);
    let bytes = parts.canonical();
    assert_eq!(
        u64::from_be_bytes(*bytes[TIME..].first_chunk().expect("a time")),
        SECOND
    );
    assert_eq!(
        bytes[VALUES..VALUES + 8],
        2_u64.to_be_bytes(),
        "the layout this file assumes"
    );
    let read = ImageParts::parse(&body, &bytes).expect("parsed");
    assert_eq!(read.canonical(), bytes);
    assert_eq!(read.pending.applied().digest(), parts.pending.applied().digest());
    assert_eq!(read.pending.waiting_digest(), parts.pending.waiting_digest());
    assert_eq!(read.state.digest(), parts.state.digest());
    assert_eq!(read.parameters().packet_hash(), parts.parameters().packet_hash());
}

#[test]
fn every_shorter_prefix_and_a_byte_left_over_are_malformed() {
    let (body, sites) = body().expect("a body");
    let bytes = image(&body, sites).canonical();
    for length in 0..bytes.len() {
        assert!(
            ImageParts::parse(&body, &bytes[..length]).is_err(),
            "{length} bytes"
        );
    }
    let mut long = bytes;
    long.push(0);
    assert!(ImageParts::parse(&body, &long).is_err());
}

type Break = fn(&mut Vec<u8>);

#[test]
fn each_field_a_run_never_writes_is_malformed() {
    let (body, sites) = body().expect("a body");
    let bytes = image(&body, sites).canonical();
    let breaks: [(&str, Break); 29] = [
        ("another format", |bytes| bytes[8] ^= 1),
        ("a list that cannot fit", |bytes| put(bytes, TIMELINE, u64::MAX)),
        ("a list past the bytes", |bytes| put(bytes, COUNTERS, 1 << 40)),
        ("one amount short", |bytes| put(bytes, AMOUNTS, 1)),
        ("a memory value no row owns", |bytes| put(bytes, MEMORY, 1)),
        ("a negative amount hidden in its limbs", |bytes| {
            put(bytes, FIRST_AMOUNT, 1.0_f64.to_bits());
            put(bytes, FIRST_AMOUNT + 8, (-2.0_f64).to_bits());
        }),
        ("a negative amount", |bytes| {
            put(bytes, FIRST_AMOUNT, (-1.0_f64).to_bits());
        }),
        ("a dense tag no writer uses", |bytes| {
            put(bytes, DENSE, 2);
            bytes.drain(DENSE + 8..DENSE + 24);
        }),
        ("a segment on an empty interval", |bytes| {
            put(bytes, DENSE, 1);
            bytes.splice(DENSE + 8..DENSE + 8, [0; 16]);
        }),
        ("a segment that ends elsewhere", |bytes| {
            with_segment(bytes, 0, 3 * SECOND)
        }),
        ("a segment short of rows", |bytes| {
            put(bytes, DENSE, 1);
            let mut interval = 0_u64.to_be_bytes().to_vec();
            interval.extend_from_slice(&SECOND.to_be_bytes());
            bytes.splice(DENSE + 8..DENSE + 8, interval);
        }),
        ("a payload no reader knows", |bytes| {
            put(bytes, FIRST_DELIVERY + 40, 1)
        }),
        ("a boundary the body does not declare", |bytes| {
            bytes[FIRST_DELIVERY + 56] = b'x'
        }),
        ("a boundary no law reads", |bytes| {
            bytes[FIRST_DELIVERY + 56..FIRST_DELIVERY + 62].copy_from_slice(b"unread");
        }),
        ("a boundary name that is no text", |bytes| {
            bytes[FIRST_DELIVERY + 56] = 0xFF
        }),
        ("a delivery that ends before it starts", |bytes| {
            put(bytes, FIRST_DELIVERY + 70, 0);
            put(bytes, FIRST_DELIVERY + 32, SECOND);
        }),
        ("an applied input at the head", |bytes| put(bytes, TIME, 0)),
        ("a cursor past the timeline", |bytes| put(bytes, CURSOR, 3)),
        ("a waiting input before the head", |bytes| {
            put(bytes, CURSOR, 0);
            put(bytes, ACTIVE, 0);
            bytes.drain(ACTIVE + 8..ACTIVE + 16);
        }),
        ("a running delivery that never applied", |bytes| {
            put(bytes, ACTIVE + 8, 1)
        }),
        ("a running delivery that has ended", |bytes| {
            put(bytes, TIME, 3 * SECOND);
        }),
        ("a running delivery twice", |bytes| {
            put(bytes, ACTIVE, 2);
            bytes.splice(ACTIVE + 8..ACTIVE + 8, 0_u64.to_be_bytes());
        }),
        ("a mode list the body does not declare", |bytes| {
            put(bytes, MODES, 1)
        }),
        ("an ordinal list the body does not declare", |bytes| {
            put(bytes, ORDINALS, 1)
        }),
        ("a flag that is neither no nor yes", |bytes| put(bytes, FIRED, 2)),
        ("another body's model", |bytes| bytes[MODEL] ^= 1),
        ("a parameter no slot owns", |bytes| put(bytes, VALUES, 3)),
        ("a parameter outside its set", |bytes| {
            put(bytes, VALUES + 16, (-1.0_f64).to_bits())
        }),
        ("a start outside its set", |bytes| {
            put(bytes, VALUES + 32, (-1.0_f64).to_bits())
        }),
    ];
    for (name, damage) in breaks {
        let mut broken = bytes.clone();
        damage(&mut broken);
        assert_eq!(ImageParts::parse(&body, &broken).err(), Some(Malformed), "{name}");
    }
}

#[test]
fn a_timeline_out_of_order_or_with_an_id_twice_is_malformed() {
    let (body, sites) = body().expect("a body");
    let bytes = image(&body, sites).canonical();
    let (first, second) = (FIRST_DELIVERY, FIRST_DELIVERY + DELIVERY);
    let mut swapped = bytes.clone();
    let (head, tail) = swapped.split_at_mut(second);
    head[first..second].swap_with_slice(&mut tail[..DELIVERY]);
    put(&mut swapped, TIME, 6 * SECOND);
    put(&mut swapped, CURSOR, 2);
    assert_eq!(
        ImageParts::parse(&body, &swapped).err(),
        Some(Malformed),
        "out of order, with each read as applied so no other check can see it"
    );
    let mut twice = bytes.clone();
    twice.copy_within(first..first + 32, second);
    assert_eq!(
        ImageParts::parse(&body, &twice).err(),
        Some(Malformed),
        "one id twice"
    );
}

#[test]
fn a_segment_that_ends_at_the_head_reads_back() {
    let (body, sites) = body().expect("a body");
    let mut bytes = image(&body, sites).canonical();
    with_segment(&mut bytes, 0, SECOND);
    let read = ImageParts::parse(&body, &bytes).expect("parsed");
    assert_eq!(read.canonical(), bytes);
}

#[test]
fn the_applied_digest_covers_the_fence_and_an_input_no_list_still_holds() {
    let (body, sites) = body().expect("a body");
    let bytes = image(&body, sites).canonical();
    let applied = |bytes: &[u8]| {
        let parts = ImageParts::parse(&body, bytes).expect("parsed");
        parts.pending.applied().digest()
    };
    let mut fenced = bytes.clone();
    put(&mut fenced, FENCE, 2 * SECOND);
    assert_ne!(applied(&fenced), applied(&bytes));
    let ended = |amount: f64| {
        let mut ended = bytes.clone();
        put(&mut ended, FIRST_DELIVERY + 62, amount.to_bits());
        put(&mut ended, ACTIVE, 0);
        ended.drain(ACTIVE + 8..ACTIVE + 16);
        ended
    };
    assert_ne!(applied(&ended(4.0)), applied(&ended(5.0)));
}

#[test]
fn a_reader_stops_at_its_bytes() {
    let mut read = ByteReader::new(&[0, 0, 0, 0, 0, 0, 0, 1, 9]);
    assert_eq!(read.number(), Ok(1));
    assert_eq!(read.number(), Err(Malformed));
    assert_eq!(read.take(2), Err(Malformed));
    assert_eq!(read.take(1), Ok(&[9][..]));
    assert_eq!(read.finish(), Ok(()));
    let short = ByteReader::new(&[1]);
    assert_eq!(short.finish(), Err(Malformed));
    let mut huge = ByteReader::new(&[0xFF; 8]);
    assert_eq!(huge.count(16), Err(Malformed), "a length whose bytes overflow");
    let mut one = ByteReader::new(&[0, 0, 0, 0, 0, 0, 0, 2, 0]);
    assert_eq!(one.count(1), Err(Malformed), "a length past the bytes left");
    let mut exact = ByteReader::new(&[0, 0, 0, 0, 0, 0, 0, 1, 0]);
    assert_eq!(exact.count(1), Ok(1));
    assert_eq!(ByteReader::new(&[0; 31]).digest(), Err(Malformed));
    assert_eq!(
        ByteReader::new(&[7; 32])
            .digest()
            .map(|digest| *digest.as_bytes()),
        Ok([7; 32])
    );
}
