use core::num::NonZeroU64;
use ng_contract::*;

const SECOND: u64 = 1_000_000_000;
const MINUTE: u64 = 60 * SECOND;
const HOUR: u64 = 60 * MINUTE;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn resolution() -> Result<DeclaredResolution, AdmissionError> {
    let scales = ErrorScales::uniform(Measure::new(1.0)?, Measure::new(0.0)?)?;
    Ok(DeclaredResolution::new(
        Digest::of_canonical(b"day inputs test"),
        NonZeroU64::MIN,
        4,
        ArithmeticProfile::contract_v1(),
        StageScheme::backward_euler_doubled(),
        scales,
    ))
}

struct Bench {
    body: CompiledBody,
    parts: ImageParts,
    identity: RunIdentity,
}
impl Bench {
    fn admitted(
        body: CompiledBody,
        packet: ParameterPacket<SyntheticFixture>,
        boundary: BoundaryPort,
    ) -> Result<Self, InitializationFailure> {
        let parameters = body.bind(packet)?;
        let request = InitializationRequest::at(SimTime::from_ns(0));
        let system = body.initialization(parameters, boundary, request)?;
        let solved = system.workspace();
        let schema = Digest::of_canonical(b"day inputs test");
        let history = MethodHistory::new(schema, SimTime::from_ns(0), Vec::new());
        let revision = system.admit(solved, history)?;
        let identity = RunIdentity::new(
            body.model_hash(),
            revision.packet_hash(),
            Seed::new([1, 2, 3, 4]),
            resolution()?,
        );
        let parts = revision.into_parts();
        Ok(Self {
            body,
            parts,
            identity,
        })
    }
    fn problem(&self) -> CoupledProblem<'_> {
        let parts = &self.parts;
        let accepted = AcceptedStateView::new(parts.time, &parts.state, &parts.continuation, &parts.dense);
        let base = Digest::of_canonical(b"base");
        CoupledProblem::new(&self.body, parts.parameters(), &self.identity, accepted, base)
    }
    fn ticket(&self, span_ns: u64) -> Result<StepTicket, TrialFailure> {
        let attempt = Attempt::First { span_ns };
        let mut issued = None;
        let refused = (self.parts.pending).attempt(&self.problem(), attempt, |ticket| {
            issued = Some(ticket.clone());
            Err::<(VerifiedStep, ()), _>(ticket.mismatch().into())
        });
        match (issued, refused) {
            (Some(ticket), _) => Ok(ticket),
            (None, Err(failure)) => Err(failure),
            (None, Ok(_)) => panic!("an attempt that issued no ticket passed"),
        }
    }
    /// Each slot's residual at the ticket's start with every unknown at zero: minus what its law read.
    fn read(&self) -> Vec<(&'static str, f64)> {
        let problem = self.problem();
        let ticket = self.ticket(SECOND).expect("a ticket");
        let mut workspace = problem.workspace();
        workspace.unknowns_mut().fill(0.0);
        workspace.derivative_mut().fill(0.0);
        let stage = problem
            .evaluate(&workspace.point(ticket.at(0).expect("an offset")))
            .expect("evaluated");
        let rows = self.body.rows().iter().map(|row| match row.kind() {
            RowKind::Algebraic(slot) => slot.law.local,
            _ => "",
        });
        rows.zip(stage.residuals().iter().map(|residual| -residual))
            .collect()
    }
}

/// Every held input of the day, each into its own slot through a gain.
const EVERY: [&str; 12] = [
    "clock cosine",
    "clock sine",
    "sleep",
    "void",
    "air",
    "radiant",
    "humidity",
    "air speed",
    "pressure",
    "oxygen",
    "carbon dioxide",
    "clothing",
];

macro_rules! follows {
    ($reads:ident, $laws:ident, $gain:ident, $wave:ident, $name:literal, $unit:ty, |$k:ident| $read:expr) => {
        if $reads.contains(&$name) {
            let slot = $laws.algebraic::<$unit>($name, Admissible::finite());
            let residual = $laws.kernel(|mut $k| {
                let value = $read;
                let gain = $k.parameter(&$gain);
                let target = $k.scale(value, gain);
                let now = $k.algebraic(&slot);
                let residual = $k.subtract(now, target);
                $k.finish(residual, $laws.every_mode(), $wave)
            });
            $laws.residual(&slot, residual);
        }
    };
}

/// A whole-body law for each named read, and one slot that reads only the gain.
fn reading(reads: &[&str]) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    let schema = Schema::new(map.finish()?)?;
    assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let gain = laws.parameter::<Dimensionless>(ParameterKey::new("ns.gain"), Admissible::positive());
        let wave = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let plain = laws.algebraic::<Dimensionless>("plain", Admissible::finite());
        let residual = laws.kernel(|mut k| {
            let now = k.algebraic(&plain);
            let gain = k.parameter(&gain);
            let residual = k.subtract(now, gain);
            k.finish(residual, laws.every_mode(), wave)
        });
        laws.residual(&plain, residual);
        follows!(reads, laws, gain, wave, "clock cosine", Dimensionless, |k| k
            .clock()
            .0);
        follows!(reads, laws, gain, wave, "clock sine", Dimensionless, |k| k
            .clock()
            .1);
        follows!(reads, laws, gain, wave, "sleep", Dimensionless, |k| k.asleep());
        follows!(reads, laws, gain, wave, "void", Dimensionless, |k| k.voiding());
        follows!(reads, laws, gain, wave, "air", Temperature, |k| k.room_air());
        follows!(reads, laws, gain, wave, "radiant", Temperature, |k| k
            .room_radiant());
        follows!(reads, laws, gain, wave, "humidity", Dimensionless, |k| k
            .room_humidity());
        follows!(reads, laws, gain, wave, "air speed", Dimensionless, |k| k
            .room_air_speed());
        follows!(reads, laws, gain, wave, "pressure", Pressure, |k| k
            .room_pressure());
        follows!(reads, laws, gain, wave, "oxygen", Dimensionless, |k| k
            .inspired_oxygen());
        follows!(reads, laws, gain, wave, "carbon dioxide", Dimensionless, |k| k
            .inspired_carbon_dioxide());
        follows!(reads, laws, gain, wave, "clothing", Dimensionless, |k| k
            .clothing());
        body.add(laws.finish());
        body.compile()
    })
}

fn slot(local: &'static str) -> RowKind {
    RowKind::Algebraic(LawName { owner: "NS", local }.into())
}

/// Each slot starts at what it reads: admission takes a start only where every law holds exactly.
fn packet(starts: &[(&str, f64)]) -> Result<ParameterPacket<SyntheticFixture>, AdmissionError> {
    let mut packet = ParameterPacket::<SyntheticFixture>::new();
    let gain = Measure::<Dimensionless>::new(1.0)?;
    packet.synthetic(ParameterKey::new("ns.gain"), Binding::Body, gain, "synthetic")?;
    packet.start(slot("plain"), Measure::<Dimensionless>::new(1.0)?, "synthetic")?;
    for &(read, value) in starts {
        let local = EVERY.into_iter().find(|known| *known == read).expect("a read");
        match local {
            "air" | "radiant" => {
                packet.start(slot(local), Measure::<Temperature>::new(value)?, "synthetic")?
            }
            "pressure" => packet.start(slot(local), Measure::<Pressure>::new(value)?, "synthetic")?,
            _ => packet.start(slot(local), Measure::<Dimensionless>::new(value)?, "synthetic")?,
        }
    }
    Ok(packet)
}

fn event(name: &[u8], at: u64, payload: BoundaryInput) -> BoundaryEvent {
    BoundaryEvent::new(
        InputId::new(Digest::of_canonical(name)),
        SimTime::from_ns(at),
        payload,
    )
}

fn room(values: [f64; 8]) -> Result<BoundaryInput, AdmissionError> {
    let [
        air,
        radiant,
        humidity,
        air_speed,
        pressure,
        oxygen,
        carbon_dioxide,
        clothing,
    ] = values;
    Ok(BoundaryInput::Room(Room {
        air: Measure::new(air)?,
        radiant: Measure::new(radiant)?,
        humidity: Measure::new(humidity)?,
        air_speed: Measure::new(air_speed)?,
        pressure: Measure::new(pressure)?,
        oxygen: Measure::new(oxygen)?,
        carbon_dioxide: Measure::new(carbon_dioxide)?,
        clothing: Measure::new(clothing)?,
    }))
}

/// What `the_day` gives each read at six in the morning: a quarter turn past midnight.
const READS: [(&str, f64); 12] = [
    ("clock cosine", 0.0),
    ("clock sine", 1.0),
    ("sleep", 1.0),
    ("void", 1.0),
    ("air", 301.0),
    ("radiant", 302.0),
    ("humidity", 0.3),
    ("air speed", 4.0),
    ("pressure", 750.0),
    ("oxygen", 0.2),
    ("carbon dioxide", 0.05),
    ("clothing", 0.7),
];

fn reads(names: &[&str]) -> Vec<(&'static str, f64)> {
    READS
        .into_iter()
        .filter(|(name, _)| names.contains(name))
        .collect()
}

/// Synthetic values, each distinct so a crossed wire shows.
fn the_day(clock: u64) -> Result<Vec<BoundaryEvent>, AdmissionError> {
    Ok(vec![
        event(
            b"clock",
            0,
            BoundaryInput::Clock(Clock {
                since_midnight_ns: clock,
            }),
        ),
        event(b"sleep", 0, BoundaryInput::Sleep(Sleep::Nrem)),
        event(b"void", 0, BoundaryInput::Void(Gate::Open)),
        event(b"room", 0, room([301.0, 302.0, 0.3, 4.0, 750.0, 0.2, 0.05, 0.7])?),
    ])
}

#[test]
fn each_held_input_reaches_the_law_that_reads_it() -> Result<(), InitializationFailure> {
    let day = BoundaryPort::new(the_day(6 * HOUR)?)?;
    let bench = Bench::admitted(reading(&EVERY)?, packet(&READS)?, day)?;
    let mut expected = vec![("plain", 1.0)];
    expected.extend(READS);
    let mut read = bench.read();
    read.sort_by_key(|(name, _)| EVERY.iter().position(|known| known == name));
    expected.sort_by_key(|(name, _)| EVERY.iter().position(|known| known == name));
    assert_eq!(read, expected);
    Ok(())
}

#[test]
fn waking_and_a_shut_gate_read_zero() -> Result<(), InitializationFailure> {
    let starts = [("sleep", 0.0), ("void", 0.0)];
    let day = vec![
        event(b"sleep", 0, BoundaryInput::Sleep(Sleep::Awake)),
        event(b"void", 0, BoundaryInput::Void(Gate::Shut)),
    ];
    let bench = Bench::admitted(
        reading(&["sleep", "void"])?,
        packet(&starts)?,
        BoundaryPort::new(day)?,
    )?;
    let read = bench.read();
    assert!(
        read.contains(&("sleep", 0.0)) && read.contains(&("void", 0.0)),
        "{read:?}"
    );
    Ok(())
}

#[test]
fn admission_refuses_a_body_that_reads_a_held_input_the_day_never_states() -> Result<(), AdmissionError> {
    let names = ["sleep", "humidity"];
    let body = reading(&names)?;
    let only_sleep = vec![event(b"sleep", 0, BoundaryInput::Sleep(Sleep::Nrem))];
    let refused = Bench::admitted(body, packet(&reads(&names))?, BoundaryPort::new(only_sleep)?).err();
    let unstated = AdmissionError::InputUnstated {
        input: Held::Humidity,
    };
    assert_eq!(refused, Some(unstated.into()));
    let late = vec![
        event(b"sleep", 0, BoundaryInput::Sleep(Sleep::Nrem)),
        event(
            b"room",
            SECOND,
            room([293.0, 293.0, 0.5, 1.0, 760.0, 0.21, 0.0, 1.0])?,
        ),
    ];
    let refused = Bench::admitted(
        reading(&names)?,
        packet(&reads(&names))?,
        BoundaryPort::new(late)?,
    )
    .err();
    assert_eq!(
        refused,
        Some(unstated.into()),
        "a room stated only after the anchor"
    );
    Ok(())
}

#[test]
fn admission_takes_a_held_input_no_law_reads() -> Result<(), InitializationFailure> {
    let bench = Bench::admitted(reading(&[])?, packet(&[])?, BoundaryPort::new(the_day(0)?)?)?;
    assert_eq!(bench.read(), [("plain", 1.0)]);
    Ok(())
}

#[test]
fn a_step_never_crosses_a_clock_edge_in_a_body_that_reads_the_clock() -> Result<(), InitializationFailure> {
    let twenty_past = 6 * HOUR + 20 * SECOND;
    let names = ["clock cosine"];
    let bench = Bench::admitted(
        reading(&names)?,
        packet(&reads(&names))?,
        BoundaryPort::new(the_day(twenty_past)?)?,
    )?;
    let ticket = bench.ticket(HOUR).expect("a ticket");
    assert_eq!(
        ticket.length_ns(),
        40 * SECOND,
        "the ticket ends at the minute's edge"
    );
    let blind = Bench::admitted(
        reading(&["sleep"])?,
        packet(&reads(&["sleep"]))?,
        BoundaryPort::new(the_day(twenty_past)?)?,
    )?;
    let ticket = blind.ticket(HOUR).expect("a ticket");
    assert!(
        ticket.length_ns() > 40 * SECOND,
        "a body that reads no clock sees no edge"
    );
    Ok(())
}

/// A law that defines a slot as the clock's cosine sets a produced quantity; its legal twin scales a
/// parameter by it.
fn defining(scaled: bool) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    let schema = Schema::new(map.finish()?)?;
    assemble(schema, |mut body| {
        let (mut laws, _ports) = body.nervous()?.parts();
        let gain = laws.parameter::<Dimensionless>(ParameterKey::new("ns.gain"), Admissible::positive());
        let wave = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let plain = laws.algebraic::<Dimensionless>("plain", Admissible::finite());
        let residual = laws.kernel(|mut k| {
            let now = k.algebraic(&plain);
            let gain = k.parameter(&gain);
            let residual = k.subtract(now, gain);
            k.finish(residual, laws.every_mode(), wave)
        });
        laws.residual(&plain, residual);
        let field = laws.algebraic::<Dimensionless>("field", Admissible::finite());
        let residual = laws.kernel(|mut k| {
            let (cosine, _) = k.clock();
            let target = if scaled {
                let gain = k.parameter(&gain);
                k.scale(gain, cosine)
            } else {
                cosine
            };
            let now = k.algebraic(&field);
            let residual = k.subtract(now, target);
            k.finish(residual, laws.every_mode(), wave)
        });
        laws.residual(&field, residual);
        body.add(laws.finish());
        body.compile()
    })
}

#[test]
fn compile_refuses_a_law_that_sets_its_row_to_an_input_and_takes_one_that_scales_a_parameter_by_it() {
    let law = LawName {
        owner: "NS",
        local: "field",
    };
    let refused = defining(false).err();
    assert_eq!(refused, Some(CompileDefect::UnscaledInput { law }.into()));
    assert!(defining(true).is_ok());
}
