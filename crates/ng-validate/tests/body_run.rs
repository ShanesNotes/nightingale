extern crate alloc;

use alloc::collections::BTreeMap;
use core::cell::Cell;
use core::fmt::Write;
use core::num::NonZeroU64;
use expect_test::expect;
use ng_contract::*;
use ng_record::{AuthorizedInput, Checkpoint, Curve, Running, Session, Step};
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;
const SECOND: u64 = 1_000 * MILLISECOND;
const LONGEST_STEP: u64 = 10 * MILLISECOND;
const PACKET: &str = "c7ec85aee0622edf508da2a0ada7da12c2d77fe49c97c9e790583cbcdc006ef4";
const ADMITTED_SEAL: &str = "10243794e349b938264a4b8ad3beb4b8ad135895d222d424356a4bf25b33820e";

struct Counted {
    core: ng_solver::Core,
    solves: Cell<u64>,
}
impl CoupledSolver for Counted {
    fn resolution(&self) -> DeclaredResolution {
        self.core.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.core.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.solves.set(self.solves.get() + 1);
        self.core.solve(problem, ticket)
    }
}

/// The body's first minute against body 26's ceilings, twice today's counts but for steps, which the heart sets.
#[derive(Clone, Copy, Debug)]
struct Counts {
    rows: usize,
    jacobian_nonzeros: usize,
    steps: u64,
    solves: u64,
}
const CEILINGS: Counts = Counts {
    rows: 2_744,
    jacobian_nonzeros: 17_784,
    steps: 14_000,
    solves: 29_000,
};

fn over_ceilings(counts: Counts, ceilings: Counts) -> Vec<&'static str> {
    let over = [
        ("rows", counts.rows > ceilings.rows),
        (
            "jacobian_nonzeros",
            counts.jacobian_nonzeros > ceilings.jacobian_nonzeros,
        ),
        ("steps", counts.steps > ceilings.steps),
        ("solves", counts.solves > ceilings.solves),
    ];
    over.into_iter()
        .filter(|(_, over)| *over)
        .map(|(name, _)| name)
        .collect()
}

#[test]
fn a_count_over_its_ceiling_fails_and_names_that_count() {
    let ceilings = Counts {
        rows: 10,
        jacobian_nonzeros: 20,
        steps: 30,
        solves: 40,
    };
    assert_eq!(over_ceilings(ceilings, ceilings), Vec::<&str>::new());
    let cases = [
        ("rows", Counts { rows: 11, ..ceilings }),
        (
            "jacobian_nonzeros",
            Counts {
                jacobian_nonzeros: 21,
                ..ceilings
            },
        ),
        (
            "steps",
            Counts {
                steps: 31,
                ..ceilings
            },
        ),
        (
            "solves",
            Counts {
                solves: 41,
                ..ceilings
            },
        ),
    ];
    for (name, counts) in cases {
        assert_eq!(over_ceilings(counts, ceilings), [name]);
    }
}

/// The body's admission costs minutes, so one solve serves every check, each from a checkpoint of the admitted start.
struct Start {
    body: CompiledBody,
    core: Counted,
    saved: Checkpoint,
    identity: RunIdentity,
    links: MemoryStore,
    provenance: InitializationProvenance,
}
impl Start {
    fn admitted() -> Self {
        let body = whole_body().expect("compiled");
        Self::admitted_with(body, |body| body_packet(body).expect("a packet"), Vec::new())
    }
    /// The held run's own admission: admission seals the packet it admits and a run resumes only under its own, so
    /// the held run cannot branch from the controlled run's state.
    fn held() -> Self {
        Self::held_at(body_breath().expect("the breath").rate)
    }
    fn held_at(rate: Measure<Rate<Dimensionless>>) -> Self {
        let body = whole_body().expect("compiled");
        let packet =
            |body: &CompiledBody| body_packet_with_the_drive_scale_held(body, rate).expect("a packet");
        Self::admitted_with(body, packet, Vec::new())
    }
    /// The leaking body held at `rate` with its room shut, so one admission carries the run with no leak and the run
    /// that opens the room.
    fn leaking(rate: f64) -> Self {
        let body = whole_body_leaking().expect("compiled");
        let packet = |body: &CompiledBody| leaking_packet(body, rate);
        Self::admitted_with(body, packet, vec![room(0, 0.0)])
    }
    fn admitted_with(
        body: CompiledBody,
        packet: impl FnOnce(&CompiledBody) -> ParameterPacket<SyntheticFixture>,
        anchored: Vec<BoundaryEvent>,
    ) -> Self {
        let core = Counted {
            core: ng_solver::Core::new(LONGEST_STEP),
            solves: Cell::new(0),
        };
        let links = MemoryStore::default();
        let (saved, identity, provenance) = {
            let (packet, lens) = (packet(&body), body_lens(&body).expect("a lens"));
            let events = [body_awake(anchor_moment())]
                .into_iter()
                .chain(anchored)
                .collect();
            let port = BoundaryPort::new(events).expect("awake");
            let admitted = admit_at(&body, packet, port, &core, &lens, anchor_moment(), &links);
            let Admitted { session, provenance } =
                admitted.unwrap_or_else(|refused| panic!("admission refused: {refused:?}"));
            (
                session.checkpoint().expect("a checkpoint"),
                session.record().identity().clone(),
                provenance,
            )
        };
        Self {
            body,
            core,
            saved,
            identity,
            links,
            provenance,
        }
    }
    fn session(&self) -> Session<'_, Running> {
        let Self {
            body,
            core,
            saved,
            identity,
            links,
            ..
        } = self;
        Session::resume(saved, identity, body, core, links).expect("resumed")
    }
    fn held_pools(&self) -> Vec<(RowId, RowKind)> {
        let pools = self.body.conserved_basis().coverage().pools().iter();
        pools
            .map(|pool| (pool.row(), self.body.rows()[pool.row().index()].kind()))
            .collect()
    }
}

fn closings(provenance: &InitializationProvenance) -> String {
    let beat = anchor_moment().nanoseconds() as f64 / SECOND as f64;
    let shut = |from: &'static str, to: &'static str| {
        let crossing = (provenance.crossings().iter())
            .rfind(|crossing| {
                crossing.enters == "shut"
                    && matches!(crossing.variable.at, Context::Edge { from: at, to: into }
                        if at.key() == from && into.key() == to)
            })
            .unwrap_or_else(|| panic!("no closing of {from} to {to}"));
        f64::from_bits(Compensated::of(&crossing.at()).bits()[0]) - beat
    };
    format!(
        "aortic valve shuts {:.9} s into the beat, leg veins {:.9} s\n",
        shut("left ventricle", "systemic arteries"),
        shut("leg veins", "central veins")
    )
}

fn until<'s>(session: Session<'s, Running>, after: u64) -> Session<'s, Running> {
    match run_until(session, SimTime::from_ns(anchor_moment().nanoseconds() + after)) {
        Ok(reached) => reached,
        Err(stopped) => panic!("{:?}", stopped.fault()),
    }
}

fn lead(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

fn hex(digest: Digest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = || Measure::new(0.0).expect("finite");
    AnatomyBuilder::new()
        .site::<C>(key, Position::new(origin(), origin(), origin()))
        .id()
}

fn field(owner: &'static str, local: &'static str, at: Context) -> RowKind {
    RowKind::Algebraic(Slot {
        law: LawName { owner, local },
        at,
    })
}

fn value(start: &Start, session: &Session<'_, Running>, kind: RowKind) -> f64 {
    let row = (start.body.rows().iter())
        .find(|row| row.kind() == kind)
        .unwrap_or_else(|| panic!("no row {kind:?}"));
    lead(session.record().accepted().value(row.id()))
}

fn concentration(start: &Start, session: &Session<'_, Running>, key: &'static str, species: Species) -> f64 {
    let lumen = |species| RowKind::Amount {
        site: site::<VesselSegment>(key),
        species,
        space: Space::Lumen,
    };
    value(start, session, lumen(species)) / value(start, session, lumen(Species::Water))
}

fn body_gas(start: &Start, session: &Session<'_, Running>, gas: Species) -> f64 {
    let view = session.record().accepted();
    (start.body.rows().iter())
        .filter(|row| matches!(row.kind(), RowKind::Amount { species, .. } if species == gas))
        .map(|row| lead(view.value(row.id())))
        .sum()
}

fn aortic_flow(start: &Start, session: &Session<'_, Running>) -> f64 {
    let at = Context::Edge {
        from: site::<VesselSegment>("left ventricle"),
        to: site::<VesselSegment>("systemic arteries"),
    };
    let valves = LawName {
        owner: "CV",
        local: "valves",
    };
    value(start, session, RowKind::Memory(Slot { law: valves, at }))
}

/// One accepted step at a time until `end` after admission. `each` sees every step's session and, at a step where
/// the aortic valve's flow leaves zero, how long it stood at zero before.
fn beats<'s>(
    start: &Start,
    mut session: Session<'s, Running>,
    end: u64,
    mut each: impl FnMut(&Session<'_, Running>, Option<u64>),
) -> Session<'s, Running> {
    let at = |session: &Session<'_, Running>| session.record().accepted().time().nanoseconds();
    let mut closed = (aortic_flow(start, &session) == 0.0).then(|| at(&session));
    while at(&session) < anchor_moment().nanoseconds() + end {
        let before = at(&session);
        session = until(session, before + 1 - anchor_moment().nanoseconds());
        let (flow, now) = (aortic_flow(start, &session), at(&session));
        let opened = closed.filter(|_| flow > 0.0).map(|since| now - since);
        each(&session, opened);
        closed = match (flow > 0.0, closed) {
            (true, _) => None,
            (false, None) => Some(now),
            (false, since) => since,
        };
    }
    session
}

/// The beats' onsets among the valve's openings: those after a closure at least half the longest, because the valve
/// also closes and opens again briefly within an ejection.
fn onsets(openings: &[(u64, u64)]) -> Vec<u64> {
    let longest = openings.iter().map(|(_, closed)| *closed).max().unwrap_or(0);
    (openings.iter())
        .filter(|(_, closed)| 2 * closed >= longest)
        .map(|(at, _)| *at)
        .collect()
}

fn the_body_admits_at_06_55(start: &Start) {
    let at = start.session().record().accepted().time().nanoseconds();
    assert_eq!(at, (6 * 3_600 + 55 * 60) * SECOND);
    assert_eq!(hex(start.identity.packet().digest()), PACKET);
    assert_eq!(hex(start.session().record().seal().digest()), ADMITTED_SEAL);
    expect![[r#"
        aortic valve shuts 0.285464778 s into the beat, leg veins 0.285689472 s
    "#]]
    .assert_eq(&closings(&start.provenance));
    let force = value(start, &start.session(), field("MS", "pump_force", Context::Body));
    let fitted = lead(Compensated::of(&body_breath().expect("the breath").rest_force));
    assert!(
        (force - fitted).abs() <= 1e-9 * fitted,
        "the pump's force at admission is {force}, the fitted rest {fitted}"
    );
}

fn smooth_max(a: f64, b: f64, width: f64) -> f64 {
    0.5 * (a + b + ((a - b) * (a - b) + width * width).sqrt())
}

fn parameter(start: &Start, name: &'static str) -> Compensated {
    (start.provenance.values().iter())
        .find_map(|record| match record.name {
            ValueName::Parameter { key, .. } if key == ParameterKey::new(name) => Some(record.value),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no parameter {name}"))
}

fn named(start: &Start, owner: &str, local: &str) -> RowId {
    (start.body.rows().iter())
        .find(|row| match row.kind() {
            RowKind::Memory(slot) | RowKind::Algebraic(slot) => {
                slot.law.owner == owner && slot.law.local == local
            }
            _ => false,
        })
        .unwrap_or_else(|| panic!("no row {local}"))
        .id()
}

struct Chemoreflex {
    scale: RowId,
    brain: RowId,
    arteries: [RowId; 4],
    set_points: [RowId; 2],
    rest: Compensated,
    gains: [f64; 2],
    widths: [f64; 2],
    offset: f64,
    smoothing: f64,
}

#[derive(Clone, Copy)]
struct Sensed {
    scale: Compensated,
    acidity: f64,
    saturation: f64,
    oxygen: f64,
    pressure: f64,
    measure: f64,
    set_points: [f64; 2],
}

impl Chemoreflex {
    fn of(start: &Start) -> Self {
        let find = |kind: RowKind| {
            (start.body.rows().iter())
                .find(|row| row.kind() == kind)
                .unwrap_or_else(|| panic!("no row {kind:?}"))
                .id()
        };
        let arteries = Context::Site(site::<VesselSegment>("systemic arteries"));
        let value = |name| lead(parameter(start, name));
        Self {
            scale: named(start, "NS", "drive scale"),
            brain: find(field(
                "BC",
                "acidity",
                Context::Site(site::<CapillaryBed>("brain bed")),
            )),
            arteries: [
                "oxygen saturation",
                "bicarbonate",
                "acidity",
                "oxygen partial pressure",
            ]
            .map(|local| find(field("BC", local, arteries))),
            set_points: [
                named(start, "NS", "central set point"),
                named(start, "NS", "peripheral set point"),
            ],
            rest: parameter(start, "ns.drive-scale-rest"),
            gains: ["ns.central-gain", "ns.peripheral-gain"].map(value),
            widths: ["ns.central-width", "ns.peripheral-width"].map(value),
            offset: value("ns.peripheral-offset"),
            smoothing: value("ns.peripheral-smoothing"),
        }
    }
    fn read(&self, view: &AcceptedStateView<'_>) -> Sensed {
        let at = |row: RowId| lead(view.value(row));
        let [saturation, bicarbonate, acidity, oxygen] = self.arteries.map(at);
        let pressure =
            bicarbonate / exp((acidity - PK) * core::f64::consts::LN_10) / CARBON_DIOXIDE_SOLUBILITY;
        let smooth = smooth_max(pressure - self.offset, 0.0, self.smoothing);
        Sensed {
            scale: view.value(self.scale),
            acidity: at(self.brain),
            saturation,
            oxygen,
            pressure,
            measure: (1.0 - saturation) * smooth,
            set_points: self.set_points.map(at),
        }
    }
    fn arms(&self, sensed: &Sensed) -> [f64; 2] {
        let [central, peripheral] = sensed.set_points;
        [
            -self.gains[0] * (sensed.acidity - central) / self.widths[0],
            self.gains[1] * (sensed.measure - peripheral) / self.widths[1],
        ]
    }
}

/// Admission makes the drive scale one in the admitted beat's mean, through each set point's mean over the beat, and
/// not at an instant: the brain bed's acidity swings with the beat, so a fit at the instant would miss one by that
/// ripple, 7e-3. The run's first beat, to the heart's first phase reset, is not the admitted orbit's, since the breath
/// starts inside it, so its mean, by the trapezoid over accepted states, is held to 1e-4 of the rest. The set points'
/// change gives the mean on the solver's own quadrature, since each set point follows its sensor at the stated speed
/// and the scale is linear in each sensor's distance from its set point, and the two methods differ by 1.6e-5.
fn the_drive_scale_at_admission_and_over_the_first_beat(start: &Start) -> String {
    let reflex = Chemoreflex::of(start);
    let beat = named(start, "CV", "phase");
    let held = |session: &Session<'_, Running>, row: RowId| session.record().accepted().value(row);
    let mut session = start.session();
    let sensed = reflex.read(&session.record().accepted());
    let (rest, admitted) = (lead(reflex.rest), lead(sensed.scale));
    let form = rest + reflex.arms(&sensed).iter().sum::<f64>();
    assert!(
        (admitted - form).abs() <= 1e-12,
        "the drive scale admits at {admitted}, its form at the admitted rows {form}"
    );
    let from = reflex.set_points.map(|row| held(&session, row));
    let (mut area, mut last) = (0.0, (0, admitted));
    loop {
        let phase = lead(held(&session, beat));
        session = until(session, last.0 + 1);
        let now = session.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds();
        let value = lead(held(&session, reflex.scale));
        area += 0.5 * (last.1 + value) * (now - last.0) as f64;
        last = (now, value);
        if lead(held(&session, beat)) < phase {
            break;
        }
    }
    let seconds = last.0 as f64 / SECOND as f64;
    let speed = lead(parameter(start, "ns.chemo-set-point-speed"));
    let [central, peripheral] =
        [0, 1].map(|at| lead(held(&session, reflex.set_points[at]).minus(from[at])) / (speed * seconds));
    let followed =
        rest + reflex.gains[1] * peripheral / reflex.widths[1] - reflex.gains[0] * central / reflex.widths[0];
    let mean = area / last.0 as f64;
    assert!(
        (mean - rest).abs() <= 1e-4,
        "the drive scale's mean over the first beat is {mean}, its rest {rest}"
    );
    format!(
        "drive scale at admission {admitted:.12}; over the first beat, to {seconds:.9} s, {mean:.12} by the trapezoid and {followed:.12} from the set points\n"
    )
}

#[derive(Default)]
struct Minute {
    attempts: u64,
    refused: BTreeMap<String, u64>,
}

fn counted_until<'s>(
    start: &Start,
    mut session: Session<'s, Running>,
    after: u64,
    minute: &mut Minute,
    watch: &mut Watch,
) -> Session<'s, Running> {
    let row = |row: RowId| format!("{:?}", start.body.rows()[row.index()].kind());
    while session.record().accepted().time().nanoseconds() < anchor_moment().nanoseconds() + after {
        let mut attempt = session.step();
        minute.attempts += 1;
        session = loop {
            match attempt {
                Step::Accepted(next, _) => break next,
                Step::Refused(retrying) => {
                    let reason = match retrying.refused().reason() {
                        NumericalRefusal::Accuracy { row: at } => format!("Accuracy {}", row(at)),
                        NumericalRefusal::StageResidual { row: at } => format!("StageResidual {}", row(at)),
                        other => format!("{other:?}"),
                    };
                    *minute.refused.entry(reason).or_default() += 1;
                    attempt = retrying.retry();
                    minute.attempts += 1;
                }
                Step::Faulted(faulted) => panic!("{:?}", faulted.fault()),
            }
        };
        watch.see(&session);
    }
    session
}

/// The books whose inventories move with each breath: the gases the lungs hold, and vasopressin and energy with the
/// circulation's breath-scale swing.
const BREATHING_BOOKS: [Book; 7] = [
    Book::Of(Species::Vasopressin),
    Book::Of(Species::Water),
    Book::Element(Element::Oxygen),
    Book::Element(Element::Carbon),
    Book::Element(Element::Hydrogen),
    Book::Of(Species::NitrogenGas),
    Book::Energy,
];
const DRY: f64 = 650.0;

struct Figures {
    from_admission: f64,
    steady: f64,
    inside_a_breath: f64,
}

fn larger(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

fn widen((low, high): (f64, f64), x: f64) -> (f64, f64) {
    (if x < low { x } else { low }, if x > high { x } else { high })
}

fn paid_by_a_works_payer(body: &CompiledBody, terms: &[(RowId, i64)]) -> bool {
    terms.iter().all(|(row, _)| {
        matches!(body.rows()[row.index()].kind(), RowKind::Amount { site, .. } if matches!(site.key(), "coronary bed" | "muscle bed"))
    })
}

#[derive(Clone)]
struct Moment {
    at: u64,
    phase: f64,
    venous: f64,
    aortic: f64,
    alveolar: [f64; 2],
    volume: f64,
    wet: f64,
    plasma: [f64; 3],
    work: f64,
    inventories: Vec<f64>,
    sensed: Sensed,
}

struct Watch {
    phase: RowId,
    venous: RowId,
    aortic: RowId,
    lung: [RowId; 3],
    lung_water: RowId,
    plasma: [RowId; 4],
    work: RowId,
    books: Vec<(Book, Vec<(RowId, i64)>)>,
    reflex: Chemoreflex,
    moments: Vec<Moment>,
}
impl Watch {
    fn new(start: &Start) -> Self {
        let body = &start.body;
        let find = |kind: RowKind| {
            (body.rows().iter())
                .find(|row| row.kind() == kind)
                .unwrap_or_else(|| panic!("no row {kind:?}"))
                .id()
        };
        let phase = (body.rows().iter())
            .find(|row| matches!(row.kind(), RowKind::Memory(slot) if slot.law.owner == "NS" && slot.law.local == "breath phase"))
            .expect("the breath's phase")
            .id();
        let algebraic = |local: &str| {
            (body.rows().iter())
                .find(|row| matches!(row.kind(), RowKind::Algebraic(slot) if slot.law.owner == "RS" && slot.law.local == local))
                .unwrap_or_else(|| panic!("no row {local}"))
                .id()
        };
        let venous = Context::Edge {
            from: site::<VesselSegment>("central veins"),
            to: site::<VesselSegment>("right atrium"),
        };
        let aortic = Context::Edge {
            from: site::<VesselSegment>("left ventricle"),
            to: site::<VesselSegment>("systemic arteries"),
        };
        let valves = LawName {
            owner: "CV",
            local: "valves",
        };
        let lung = site::<CapillaryBed>("pulmonary bed");
        let gas = |species| {
            find(RowKind::Amount {
                site: lung,
                species,
                space: Space::Gas,
            })
        };
        let plasma = |species| {
            find(RowKind::Amount {
                site: lung,
                species,
                space: Space::Plasma,
            })
        };
        let books = (body.conserved_basis().combinations().iter())
            .filter(|combination| BREATHING_BOOKS.contains(&combination.book()))
            .filter(|combination| !paid_by_a_works_payer(body, combination.terms()))
            .map(|combination| (combination.book(), combination.terms().to_vec()))
            .collect();
        Self {
            phase,
            venous: find(field("CV", "circulation", venous)),
            aortic: find(RowKind::Memory(Slot {
                law: valves,
                at: aortic,
            })),
            lung: [Species::Oxygen, Species::CarbonDioxide, Species::NitrogenGas].map(gas),
            lung_water: gas(Species::Water),
            plasma: [
                plasma(Species::Oxygen),
                plasma(Species::Hemoglobin),
                plasma(Species::Water),
                find(field("BC", "oxygen partial pressure", Context::Site(lung))),
            ],
            work: algebraic("work_of_breathing"),
            books,
            reflex: Chemoreflex::of(start),
            moments: Vec::new(),
        }
    }
    fn see(&mut self, session: &Session<'_, Running>) {
        let view = session.record().accepted();
        let at = |row: RowId| lead(view.value(row));
        let [o, c, n] = self.lung.map(at);
        let dry = o + c + n;
        let [oxygen, carrier, water, pressure] = self.plasma.map(at);
        let inventories = (self.books.iter())
            .map(|(_, terms)| terms.iter().map(|(row, count)| *count as f64 * at(*row)).sum())
            .collect();
        self.moments.push(Moment {
            at: view.time().nanoseconds() - anchor_moment().nanoseconds(),
            phase: at(self.phase),
            venous: at(self.venous),
            aortic: at(self.aortic),
            alveolar: [DRY * o / dry, DRY * c / dry],
            volume: dry,
            wet: WATER_VOLUME * at(self.lung_water) / dry,
            plasma: [oxygen / water, carrier / water, pressure],
            work: at(self.work),
            inventories,
            sensed: self.reflex.read(&view),
        });
    }
    fn edges(&self) -> Vec<usize> {
        (1..self.moments.len())
            .filter(|&at| self.moments[at].phase < self.moments[at - 1].phase)
            .collect()
    }
    fn onsets(&self) -> Vec<usize> {
        let mut openings = Vec::new();
        let mut closed: Option<usize> = None;
        for (at, moment) in self.moments.iter().enumerate() {
            match (moment.aortic > 0.0, closed) {
                (true, Some(since)) => {
                    openings.push((at as u64, moment.at - self.moments[since].at));
                    closed = None;
                }
                (false, None) => closed = Some(at),
                _ => {}
            }
        }
        onsets(&openings).into_iter().map(|at| at as usize).collect()
    }
    fn mean(&self, from: usize, to: usize, of: impl Fn(&Moment) -> f64) -> f64 {
        let moments = &self.moments[from..=to];
        let area: f64 = (moments.windows(2))
            .map(|pair| 0.5 * (of(&pair[0]) + of(&pair[1])) * (pair[1].at - pair[0].at) as f64)
            .sum();
        area / (moments[moments.len() - 1].at - moments[0].at) as f64
    }
    fn figures(&self, steady: u64) -> Vec<(Book, Figures)> {
        let steady = (self.moments.iter())
            .position(|moment| moment.at >= steady * SECOND)
            .expect("the steady stretch's start");
        let ten = (self.moments.iter())
            .position(|moment| moment.at >= 10 * SECOND)
            .expect("ten seconds");
        let edges: Vec<usize> = self.edges().into_iter().filter(|&edge| edge >= steady).collect();
        BREATHING_BOOKS
            .map(|book| {
                let mut most = [0.0_f64; 3];
                for (index, _) in self.books.iter().enumerate().filter(|(_, (of, _))| *of == book) {
                    let held = |at: usize| self.moments[at].inventories[index];
                    most[0] = larger(most[0], (held(ten) - held(0)).abs());
                    let mut later = steady;
                    for at in steady..self.moments.len() {
                        while later < self.moments.len()
                            && self.moments[later].at < self.moments[at].at + 10 * SECOND
                        {
                            later += 1;
                        }
                        if later == self.moments.len() {
                            break;
                        }
                        most[1] = larger(most[1], (held(later) - held(at)).abs());
                    }
                    for pair in edges.windows(2) {
                        let (low, high) = (pair[0]..=pair[1])
                            .map(held)
                            .fold((f64::INFINITY, f64::NEG_INFINITY), widen);
                        most[2] = larger(most[2], high - low);
                    }
                }
                let [from_admission, steady, inside_a_breath] = most;
                (
                    book,
                    Figures {
                        from_admission,
                        steady,
                        inside_a_breath,
                    },
                )
            })
            .into_iter()
            .collect()
    }
}

const SATURATED_VAPOUR_SHARE: f64 = (700.0 - DRY) / DRY;

fn the_alveolar_water_share_over_whole_breaths(watch: &Watch) -> String {
    let steady = (watch.moments.iter())
        .position(|moment| moment.at >= 20 * SECOND)
        .expect("twenty seconds");
    let edges: Vec<usize> = watch.edges().into_iter().filter(|&edge| edge >= steady).collect();
    let (from, to) = (edges[0], edges[edges.len() - 1]);
    let share = watch.mean(from, to, |moment| moment.wet) / SATURATED_VAPOUR_SHARE;
    format!(
        "alveolar water over whole breaths from {:.3} s: {share:.5} of the saturated vapour share\n",
        watch.moments[from].at as f64 / SECOND as f64
    )
}

/// The breath has started once the alveolar oxygen pressure swings over the minute; then each whole breath swings
/// both gases, and each moves venous return's beat means, since the central veins and the right atrium sit inside
/// the pleural space and the beds that fill them outside it.
fn each_breath_swings_the_alveolar_gases_and_venous_return(watch: &Watch) -> String {
    let after = (watch.moments.iter())
        .position(|moment| moment.at >= 5 * SECOND)
        .expect("five seconds");
    let (least, most) = (watch.moments[after..].iter())
        .map(|moment| moment.alveolar[0])
        .fold((f64::INFINITY, f64::NEG_INFINITY), widen);
    assert!(
        most - least >= 1.0,
        "the alveolar oxygen pressure stays within {least} to {most}: no breath"
    );
    let edges = watch.edges();
    let onsets = watch.onsets();
    let beats: Vec<(usize, usize, f64)> = (onsets.windows(2))
        .map(|pair| {
            (
                pair[0],
                pair[1],
                watch.mean(pair[0], pair[1], |moment| moment.venous),
            )
        })
        .collect();
    let first = edges.first().copied().expect("a breath edge");
    let mut report = format!(
        "before the first breath edge: alveolar oxygen {:.3}, carbon dioxide {:.3}, venous return {:.2}\n",
        watch.mean(0, first, |moment| moment.alveolar[0]),
        watch.mean(0, first, |moment| moment.alveolar[1]),
        watch.mean(0, first, |moment| moment.venous)
    );
    for pair in edges.windows(2) {
        let (from, to) = (pair[0], pair[1]);
        let swing = |gas: usize| {
            let (low, high) = (watch.moments[from..=to].iter())
                .map(|moment| moment.alveolar[gas])
                .fold((f64::INFINITY, f64::NEG_INFINITY), widen);
            high - low
        };
        let (oxygen, carbon_dioxide) = (swing(0), swing(1));
        let (shallow, deep) = (watch.moments[from..=to].iter())
            .map(|moment| moment.volume)
            .fold((f64::INFINITY, f64::NEG_INFINITY), widen);
        let at = watch.moments[from].at as f64 / SECOND as f64;
        assert!(
            oxygen >= 1.0 && carbon_dioxide >= 0.5,
            "the breath from {at} s swings alveolar oxygen by {oxygen} and carbon dioxide by {carbon_dioxide}"
        );
        let means: Vec<f64> = (beats.iter())
            .filter(|(begins, ends, _)| *begins >= from && *ends <= to)
            .map(|(_, _, mean)| *mean)
            .collect();
        let (low, high) = means
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |range, &x| widen(range, x));
        let mean = watch.mean(from, to, |moment| moment.venous);
        assert!(
            means.len() >= 2 && high - low >= 0.02 * mean,
            "the breath from {at} s holds venous return's beat means {means:?}"
        );
        let means = [0, 1].map(|gas| watch.mean(from, to, |moment| moment.alveolar[gas]));
        writeln!(
            report,
            "breath from {at:.3} s: alveolar oxygen {:.3} swinging {oxygen:.3}, carbon dioxide {:.3} swinging {carbon_dioxide:.3}, venous return's beat means {low:.2} to {high:.2} around {mean:.2}, tidal volume {:.1}",
            means[0], means[1], deep - shallow
        )
        .expect("a string takes it");
    }
    report
}

/// The work of breathing admits at the breath's stated mean and its memory then follows the pump's work, which over
/// the minute's last two whole breaths meets the stated mean within 1%: the packet's mechanics leave out only the
/// heads' smooth width and the gases' net uptake.
fn the_work_of_breathing_follows_the_pump_from_the_stated_mean(watch: &Watch) -> String {
    let edges = watch.edges();
    let (from, to) = (edges[edges.len() - 3], edges[edges.len() - 1]);
    let stated = watch.moments[0].work;
    let held = watch.mean(from, to, |moment| moment.work);
    assert!(
        (held - stated).abs() <= 0.01 * stated,
        "the work of breathing means {held} over the last whole breaths, the stated mean {stated}"
    );
    format!(
        "work of breathing: stated {stated:.1}, over two whole breaths from {:.3} s {held:.1}\n",
        watch.moments[from].at as f64 / SECOND as f64
    )
}

fn the_pulmonary_plasma_stays_under_capacity_at_the_highest_alveolar_oxygen(watch: &Watch) -> String {
    let highest = (watch.moments.iter())
        .max_by(|a, b| a.alveolar[0].total_cmp(&b.alveolar[0]))
        .expect("a moment");
    let [oxygen, carrier, pressure] = highest.plasma;
    let bound = oxygen - SOLUBILITY * pressure;
    assert!(
        bound < HEME * carrier,
        "at alveolar oxygen {} the pulmonary plasma holds {oxygen}, over capacity {} plus dissolved {}",
        highest.alveolar[0],
        HEME * carrier,
        SOLUBILITY * pressure
    );
    format!(
        "highest alveolar oxygen {:.3} ({:.3} times P50) at {:.3} s: plasma oxygen pressure {pressure:.3} ({:.3} times P50), bound {:.6} of capacity\n",
        highest.alveolar[0],
        highest.alveolar[0] / P50,
        highest.at as f64 / SECOND as f64,
        pressure / P50,
        bound / (HEME * carrier)
    )
}

fn the_first_minute_moves_no_held_pool_and_saves_a_branch(
    start: &Start,
) -> (Session<'_, Running>, Minute, Watch, Checkpoint) {
    let pools = start.held_pools();
    assert_eq!(pools.len(), 318);
    let amounts = |session: &Session<'_, Running>| -> Vec<[u64; 2]> {
        let view = session.record().accepted();
        pools.iter().map(|(row, _)| view.value(*row).bits()).collect()
    };
    let mut session = start.session();
    let admitted = amounts(&session);
    for ((_, kind), amount) in pools.iter().zip(&admitted) {
        let guess = body_start_guess(*kind).expect("a finite guess");
        assert_eq!(*amount, guess.bits(), "{kind:?} moved at admission");
    }
    let mut minute = Minute::default();
    let mut watch = Watch::new(start);
    watch.see(&session);
    let mut branch = None;
    for mark in 1..=6 {
        if 10 * mark == ASLEEP {
            let before = ASLEEP * SECOND - LONGEST_STEP;
            session = counted_until(start, session, before, &mut minute, &mut watch);
            branch = Some(session.checkpoint().expect("a checkpoint"));
        }
        session = counted_until(start, session, 10 * mark * SECOND, &mut minute, &mut watch);
        for ((_, kind), (now, then)) in pools.iter().zip(amounts(&session).iter().zip(&admitted)) {
            assert_eq!(now, then, "{kind:?} moved by {} s", 10 * mark);
        }
    }
    (
        session,
        minute,
        watch,
        branch.expect("the minute passes sleep's stamp"),
    )
}

fn the_certificate_passes_on_ten_second_windows_over_the_first_minute_with_its_held_support(
    start: &Start,
    minute: &Session<'_, Running>,
    watch: &Watch,
) -> String {
    let certificate = certified(start, minute);
    let mut report = format!(
        "steps {}, quantities {}, held routes {}\n",
        minute.record().revision(),
        certificate.results().len(),
        certificate.support().len()
    );
    let moved =
        |support: &&HeldSupport| lead(support.most_inflow()) != 0.0 || lead(support.most_outflow()) != 0.0;
    for support in certificate.support().iter().filter(moved) {
        let route = support.route();
        writeln!(report, "{} {} moved", route.card(), route.name()).expect("a string takes it");
    }
    expect![[r#"
        steps 11666, quantities 398, held routes 88
        EN vasopressin clearance moved
        EN vasopressin secretion moved
        GI gut chloride moved
        GI gut potassium moved
        GI gut sodium moved
        GI gut water moved
        MS skin water moved
        MT bone bed cell water out moved
        MT bone bed fatty acid feed moved
        MT bone bed glucose feed moved
        MT brain bed cell water out moved
        MT brain bed fatty acid feed moved
        MT brain bed glucose feed moved
        MT coronary bed cell water out moved
        MT coronary bed fatty acid feed moved
        MT coronary bed glucose feed moved
        MT fat bed cell water out moved
        MT fat bed fatty acid feed moved
        MT fat bed glucose feed moved
        MT glomerular bed cell water out moved
        MT glomerular bed fatty acid feed moved
        MT glomerular bed glucose feed moved
        MT gut bed cell water out moved
        MT gut bed fatty acid feed moved
        MT gut bed glucose feed moved
        MT liver bed cell water out moved
        MT liver bed fatty acid feed moved
        MT liver bed glucose feed moved
        MT muscle bed cell water out moved
        MT muscle bed fatty acid feed moved
        MT muscle bed glucose feed moved
        MT peritubular bed cell water out moved
        MT peritubular bed fatty acid feed moved
        MT peritubular bed glucose feed moved
        MT pulmonary bed cell water out moved
        MT pulmonary bed fatty acid feed moved
        MT pulmonary bed glucose feed moved
        MT skin bed cell water out moved
        MT skin bed fatty acid feed moved
        MT skin bed glucose feed moved
    "#]]
    .assert_eq(&report);
    let figures = watch.figures(20);
    let mut report = String::new();
    for (book, at_payer, _) in RULE {
        let (worst, _) = worst_change(start, &certificate, book, at_payer);
        if let Some((_, figures)) = figures.iter().find(|(of, _)| *of == book && !at_payer) {
            writeln!(report, "{book:?}: {}", line(worst, figures)).expect("a string takes it");
        }
    }
    report
}

type Figure = (f64, f64);

const RULE: [(Book, bool, [Figure; 3]); 11] = [
    (Book::Energy, false, [(9e3, 1e4), (9e2, 1e3), (2e2, 3e2)]),
    (
        Book::Element(Element::Carbon),
        false,
        [(6e2, 7e2), (2e1, 3e1), (9.0, 1e1)],
    ),
    (
        Book::Element(Element::Oxygen),
        false,
        [(9e3, 1e4), (3e1, 4e1), (6.0, 7.0)],
    ),
    (
        Book::Element(Element::Hydrogen),
        false,
        [(1e-4, 2e-4), (1e-5, 2e-5), (1e-6, 2e-6)],
    ),
    (
        Book::Element(Element::Hydrogen),
        true,
        [(0.3, 0.4), (3e-2, 4e-2), (1e-2, 2e-2)],
    ),
    (
        Book::Of(Species::NitrogenGas),
        false,
        [(1e4, 2e4), (1e1, 2e1), (7.0, 8.0)],
    ),
    (
        Book::Of(Species::Water),
        false,
        [(4e-2, 5e-2), (3e-3, 4e-3), (2e-3, 3e-3)],
    ),
    (
        Book::Of(Species::Sodium),
        false,
        [(1e-3, 2e-3), (1e-4, 2e-4), (8e-6, 9e-6)],
    ),
    (
        Book::Of(Species::Chloride),
        false,
        [(1e-3, 2e-3), (1e-4, 2e-4), (9e-6, 1e-5)],
    ),
    (
        Book::Of(Species::Potassium),
        false,
        [(8e-5, 9e-5), (7e-6, 8e-6), (4e-7, 5e-7)],
    ),
    (
        Book::Of(Species::Vasopressin),
        false,
        [(5e-2, 6e-2), (1e-3, 2e-3), (3e-4, 4e-4)],
    ),
];
const WIDEST: Figure = (1e1, 2e1);

fn holds(body: &CompiledBody, (book, at_payer): (Book, bool), quantity: QuantityId) -> bool {
    let combination = &body.conserved_basis().combinations()[quantity.index()];
    match (book, combination.book()) {
        (Book::Energy, Book::Of(Species::Energy)) => true,
        (Book::Element(Element::Hydrogen), Book::Element(Element::Hydrogen)) => {
            paid_by_a_works_payer(body, combination.terms()) == at_payer
        }
        (row, of) => row == of,
    }
}

fn exactly(figure: f64) -> [u64; 2] {
    Compensated::of(&Measure::<Dimensionless>::new(figure).expect("finite")).bits()
}

#[test]
fn each_figure_of_rule_is_the_bodys_policy_bit_for_bit() {
    let body = whole_body().expect("compiled");
    let window = NonZeroU64::new(10 * SECOND).expect("a window");
    let policy = body_certificate(&body, SlidingWindows::new(window, 60 * SECOND).expect("windows"))
        .expect("a policy");
    assert_body_rule(&body, &policy);
}

fn assert_body_rule(body: &CompiledBody, policy: &CertificatePolicy) {
    let (windows, rates) = policy.edges().expect("windows between breath edges");
    assert_eq!(windows.widest_ns() as f64, WIDEST.1 * SECOND as f64);
    assert_eq!(
        (windows.settling_ns(), windows.length_ns().get()),
        (20 * SECOND, 10 * SECOND),
        "the settling time and the window's length"
    );
    let mut held = [0; RULE.len()];
    for quantity in body.conserved_basis().ids() {
        let rate = rates.iter().find(|rate| rate.quantity() == quantity);
        let row = (RULE.iter().enumerate())
            .find(|(_, (book, at_payer, _))| holds(body, (*book, *at_payer), quantity));
        let Some((index, &(book, _, [(_, drift), (_, settled), (_, settling)]))) = row else {
            assert!(
                rate.is_none(),
                "{quantity:?} is rated and no row of RULE holds it"
            );
            continue;
        };
        held[index] += 1;
        let rate = rate.unwrap_or_else(|| panic!("{quantity:?} of {book:?} is not rated"));
        assert_eq!(
            (
                policy.drift(quantity).map(|bound| bound.bits()),
                rate.settled().bits(),
                rate.settling().map(|bound| bound.bits())
            ),
            (Some(exactly(drift)), exactly(settled), Some(exactly(settling))),
            "{quantity:?} of {book:?}"
        );
    }
    let payers = RULE
        .iter()
        .position(|&(_, at_payer, _)| at_payer)
        .expect("the payers' row");
    assert_eq!(
        held[payers], 2,
        "the coronary bed's hydrogen and the muscle bed's"
    );
    assert!(held.iter().all(|&count| count > 0), "{held:?}");
}

#[test]
fn rule_detects_a_change_to_either_breath_window_time() {
    struct Fixed;
    fn bound(book: Book, payer: bool) -> Option<f64> {
        let (_, _, [(_, fixed), ..]) = RULE.iter().find(|(of, at, _)| (*of, *at) == (book, payer))?;
        Some(*fixed)
    }
    fn species_book(species: Species) -> Book {
        if species == Species::Energy {
            Book::Energy
        } else {
            Book::Of(species)
        }
    }
    impl DriftTable for Fixed {
        fn names(&self, species: Species) -> bool {
            bound(species_book(species), false).is_some()
        }
        fn drift<Q: Substance>(&self) -> Result<Measure<Q>, AdmissionError> {
            Measure::new(bound(species_book(Q::SPECIES), false).expect("a named row"))
        }
        fn element(
            &self,
            element: Element,
            sites: &[SiteId],
        ) -> Result<Option<Measure<Atoms>>, AdmissionError> {
            let payer = element == Element::Hydrogen
                && sites
                    .iter()
                    .all(|site| matches!(site.key(), "coronary bed" | "muscle bed"));
            bound(Book::Element(element), payer).map(Measure::new).transpose()
        }
    }

    let body = whole_body().expect("compiled");
    let window = NonZeroU64::new(10 * SECOND).expect("a window");
    let sliding = SlidingWindows::new(window, 60 * SECOND).expect("windows");
    let stated = body_certificate(&body, sliding).expect("a policy");
    let (edges, rates) = stated.edges().expect("windows between breath edges");
    let build = |settling, length, rated: &[EdgeRate]| {
        let times = EdgeWindows::new(edges.clock(), settling, length, edges.widest_ns());
        certificate_with(&body, sliding, &Fixed)
            .expect("the fixed bounds")
            .between_edges(times, rated.to_vec())
            .expect("the body's rated rows")
    };
    let (settling, length) = (edges.settling_ns(), edges.length_ns());
    assert_eq!(build(settling, length, rates).digest(), stated.digest());
    assert_body_rule(&body, &stated);
    let omitted = Leak::Vasopressin.quantity(&body);
    let one_fewer: Vec<_> = rates
        .iter()
        .copied()
        .filter(|rate| rate.quantity() != omitted)
        .collect();
    assert_eq!(one_fewer.len() + 1, rates.len());
    let missing = build(settling, length, &one_fewer);
    assert!(
        std::panic::catch_unwind(|| assert_body_rule(&body, &missing)).is_err(),
        "RULE missed the omitted row"
    );

    let mut missed = Vec::new();
    for (name, altered) in [
        ("settling time", build(settling + 1, length, rates)),
        ("window length", build(settling, length.saturating_add(1), rates)),
    ] {
        assert_ne!(altered.digest(), stated.digest());
        if std::panic::catch_unwind(|| assert_body_rule(&body, &altered)).is_ok() {
            missed.push(name);
        }
    }
    assert!(
        missed.is_empty(),
        "RULE accepted an extra nanosecond in {missed:?}"
    );
}

#[test]
fn the_held_packet_moves_with_the_breath_rate_it_is_given() {
    let body = whole_body().expect("compiled");
    let held = |rate: Measure<Rate<Dimensionless>>| {
        let packet = body_packet_with_the_drive_scale_held(&body, rate).expect("a packet");
        body.bind(packet).expect("bound").packet_hash().digest()
    };
    let rhythm = body_breath().expect("the breath").rate;
    assert_ne!(held(finite(QUARTER)), held(rhythm));
}

fn certified(start: &Start, minute: &Session<'_, Running>) -> Certificate {
    let reached = minute.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds();
    let window = NonZeroU64::new(10 * SECOND).expect("a window");
    let windows = SlidingWindows::new(window, reached).expect("windows");
    let policy = body_certificate(&start.body, windows).expect("a policy");
    certify(&minute.conservation(), &policy).unwrap_or_else(|failed| {
        panic!(
            "{:?} {:?} {:?}: changed {:.6e} against {:?}",
            failed.check(),
            failed.quantity(),
            failed.window(),
            lead(failed.residual()),
            failed.allowance().map(lead)
        )
    })
}

fn worst_change(
    start: &Start,
    certificate: &Certificate,
    book: Book,
    at_payer: bool,
) -> (f64, Option<Interval>) {
    (certificate.results().iter())
        .filter(|result| holds(&start.body, (book, at_payer), result.quantity()))
        .map(|result| (lead(result.worst_drift()).abs(), Some(result.drift_window())))
        .fold((0.0, None), |most, x| if x.0 > most.0 { x } else { most })
}

fn line(worst: f64, figures: &Figures) -> String {
    let Figures {
        from_admission,
        steady,
        inside_a_breath,
    } = figures;
    format!(
        "worst {worst:.4e}, from admission {from_admission:.4e}, steady {steady:.4e}, inside a breath {inside_a_breath:.4e}"
    )
}

const MAIN_CLOSINGS: &str = "aortic valve shuts 0.285464778 s into the beat, leg veins 0.285689472 s\n";

/// No rest law reads the drive scale, so both admissions shut each valve at main's instants, and every row but the
/// scale and every parameter admission calibrates, matched by key and binding, admits within admission's tolerance of
/// the controlled run's.
fn the_held_body_admits_where_the_controlled_body_does(start: &Start, held: &Start) -> String {
    let [controlled, holding] = [start, held].map(|each| closings(&each.provenance));
    assert_eq!([controlled.as_str(), holding.as_str()], [MAIN_CLOSINGS; 2]);
    let tolerance = *start.provenance.tolerance();
    let scale = Chemoreflex::of(start).scale;
    let (ours, theirs) = (start.session(), held.session());
    let (ours, theirs) = (ours.record().accepted(), theirs.record().accepted());
    let mut most = (0.0, 0.0, None);
    for row in (start.body.rows().iter()).filter(|row| row.id() != scale) {
        let (x, y) = (lead(ours.value(row.id())), lead(theirs.value(row.id())));
        // Read at a unit size, which is stricter than admission's for the rows the packet sizes larger.
        let share = (x - y).abs() / tolerance.at(x, 1.0);
        if share > most.0 {
            most = (share, (x - y).abs(), Some(row.kind()));
        }
    }
    let (share, difference, at) = most;
    assert!(
        share <= 1.0,
        "the held admission's {at:?} is {difference} from the controlled one's, {share} of admission's tolerance"
    );
    let calibrated = |each: &Start| -> Vec<ValueRecord> {
        (each.provenance.values().iter())
            .filter(|record| matches!(record.provenance, ValueProvenance::Calibrated { .. }))
            .copied()
            .collect()
    };
    let (fitted, refitted) = (calibrated(start), calibrated(held));
    assert_eq!(
        fitted.len(),
        refitted.len(),
        "the two admissions calibrate different parameters"
    );
    let mut most = (0.0, 0.0, 0.0, None);
    for record in &fitted {
        let again = (refitted.iter())
            .find(|again| again.name == record.name)
            .unwrap_or_else(|| panic!("the held admission does not calibrate {:?}", record.name));
        let (x, y) = (lead(record.value), lead(again.value));
        let share = (x - y).abs() / tolerance.at(x, 1.0);
        if share > most.0 {
            most = (share, (x - y).abs(), (x - y).abs() / x.abs(), Some(record.name));
        }
    }
    let (fitted_share, fitted_difference, relative, fitted_at) = most;
    assert!(
        fitted_share <= 1.0,
        "the held admission calibrates {fitted_at:?} {fitted_difference} from the controlled one, {fitted_share} of \
         admission's tolerance"
    );
    format!(
        "controlled admission: {controlled}held admission: {holding}largest difference over every row but the drive scale {difference:.3e}, {share:.3e} of admission's tolerance, at {at:?}; over the {} calibrated parameters {fitted_difference:.3e}, {relative:.3e} of its value and {fitted_share:.3e} of admission's tolerance, at {fitted_at:?}\n",
        fitted.len()
    )
}

fn the_held_minute(held: &Start) -> (Session<'_, Running>, Watch) {
    let (mut minute, mut watch) = (Minute::default(), Watch::new(held));
    let session = held.session();
    watch.see(&session);
    let session = counted_until(held, session, WALK[1] * SECOND, &mut minute, &mut watch);
    (session, watch)
}

fn the_held_runs_drive_scale_is_its_rest_at_every_step(held: &Start, watch: &Watch) -> String {
    let rest = parameter(held, "ns.drive-scale-rest").bits();
    let off: Vec<f64> = (watch.moments.iter())
        .filter(|moment| moment.sensed.scale.bits() != rest)
        .map(|moment| moment.at as f64 / SECOND as f64)
        .collect();
    assert!(
        off.is_empty(),
        "the held run's drive scale leaves its rest at {off:?} s"
    );
    format!(
        "held run: the drive scale is its rest's bits at all {} accepted steps\n",
        watch.moments.len()
    )
}

/// By 30 s the drive scale has stood over one for ten seconds, so the controlled run has ventilated more than the held
/// run long enough to bring its carbon dioxide under the held run's.
const WALK: [u64; 2] = [30, 60];

fn tidal(watch: &Watch, breath: &[usize]) -> f64 {
    let (shallow, deep) = (watch.moments[breath[0]..=breath[1]].iter())
        .map(|moment| moment.volume)
        .fold((f64::INFINITY, f64::NEG_INFINITY), widen);
    deep - shallow
}

struct Walked {
    breaths: usize,
    length: f64,
    ventilation: f64,
    alveolar: [f64; 2],
    scale: f64,
    deviations: [f64; 2],
    arms: [f64; 2],
}

fn walked(watch: &Watch, [from, to]: [u64; 2]) -> Walked {
    let inside = |at: &usize| (from * SECOND..=to * SECOND).contains(&watch.moments[*at].at);
    let edges: Vec<usize> = watch.edges().into_iter().filter(inside).collect();
    let (from, to) = (edges[0], edges[edges.len() - 1]);
    let span = (watch.moments[to].at - watch.moments[from].at) as f64 / SECOND as f64;
    let tidal: f64 = (edges.windows(2)).map(|pair| tidal(watch, pair)).sum();
    let mean = |of: &dyn Fn(&Sensed) -> f64| watch.mean(from, to, |moment| of(&moment.sensed));
    let arm = |at: usize| mean(&|sensed| watch.reflex.arms(sensed)[at]);
    Walked {
        breaths: edges.len() - 1,
        length: span / (edges.len() - 1) as f64,
        ventilation: tidal / span,
        alveolar: [0, 1].map(|gas| watch.mean(from, to, |moment| moment.alveolar[gas])),
        scale: mean(&|sensed| lead(sensed.scale)),
        deviations: [
            mean(&|sensed| sensed.acidity - sensed.set_points[0]),
            mean(&|sensed| sensed.measure - sensed.set_points[1]),
        ],
        arms: [arm(0), arm(1)],
    }
}

/// One quantity's means in two runs over the same stretch of the clock, so both end together whatever each run's
/// breath. The stretch cuts a breath at each end. Putting the mean of the whole breath beside a cut piece in place of
/// the piece's samples moves that run's mean by at most the piece's share of the stretch times the farthest any of its
/// samples sits from that mean: `reach` sums this over both ends of both runs, so a margin past it holds however the
/// cut breaths are counted.
struct Compared {
    means: [f64; 2],
    spans: [f64; 2],
    reach: f64,
}
impl Compared {
    fn margin(&self) -> f64 {
        self.means[0] - self.means[1]
    }
}

fn compared(ours: &Watch, theirs: &Watch, [from, to]: [u64; 2], of: impl Fn(&Moment) -> f64) -> Compared {
    let one = |watch: &Watch| {
        let moments = &watch.moments;
        let first = (moments.iter())
            .position(|moment| moment.at >= from * SECOND)
            .expect("the stretch's start");
        let last = (moments.iter())
            .rposition(|moment| moment.at <= to * SECOND)
            .expect("the stretch's end");
        let edges: Vec<usize> = (watch.edges().into_iter())
            .filter(|edge| (first..=last).contains(edge))
            .collect();
        assert!(edges.len() >= 2, "no whole breath from {from} s to {to} s");
        let span = |a: usize, b: usize| (moments[b].at - moments[a].at) as f64;
        let cut = |[a, b]: [usize; 2], [whole, next]: [usize; 2]| {
            let beside = watch.mean(whole, next, &of);
            let farthest = (a..=b)
                .map(|at| (of(&moments[at]) - beside).abs())
                .fold(0.0, larger);
            span(a, b) * farthest
        };
        let n = edges.len();
        let reach = cut([first, edges[0]], [edges[0], edges[1]])
            + cut([edges[n - 1], last], [edges[n - 2], edges[n - 1]]);
        let whole = span(first, last);
        (watch.mean(first, last, &of), whole / SECOND as f64, reach / whole)
    };
    let [(ours, our_span, our_reach), (theirs, their_span, their_reach)] = [ours, theirs].map(one);
    Compared {
        means: [ours, theirs],
        spans: [our_span, their_span],
        reach: our_reach + their_reach,
    }
}

/// The breath's own under-ventilation is the perturbation. Each link is judged by means over whole breaths, since the
/// scale's ripple with the beat, 0.74%, is more than the central arm moves it in a minute, 0.2%. The arterial link is
/// the exception: the held run's whole breaths end 4.4 s before the controlled run's while its arterial carbon dioxide
/// climbs about 0.01 a breath, so the link is judged over the clock, and from 25 s and 35 s, the two starts that flipped
/// it over whole breaths.
fn every_link_moves_its_reader_the_way_the_loop_needs(
    start: &Start,
    controlled: &Watch,
    held: &Watch,
) -> String {
    let period = (lead(parameter(start, "ns.inspiration-length"))
        + lead(parameter(start, "ns.expiration-length")))
        / lead(parameter(start, "ns.breath-rate"));
    let (ours, theirs) = (walked(controlled, WALK), walked(held, WALK));
    let arterial = |from: u64| compared(controlled, held, [from, WALK[1]], |moment| moment.sensed.pressure);
    let (judged, [earlier, later]) = (arterial(WALK[0]), [WALK[0] - 5, WALK[0] + 5].map(arterial));
    let oxygen = compared(controlled, held, WALK, |moment| moment.sensed.oxygen).means;
    let saturation = compared(controlled, held, WALK, |moment| moment.sensed.saturation).means;
    let [central, peripheral] = ours.deviations;
    let links = [
        (
            "under-ventilation to the central sensor",
            format!("acidity {central:+.3e} from its set point"),
            central < 0.0,
        ),
        (
            "under-ventilation to the peripheral sensor",
            format!("x_p {peripheral:+.3e} from its set point"),
            peripheral > 0.0,
        ),
        (
            "the sensors to the drive scale",
            format!(
                "{:.6}, its central arm {:+.3e} and peripheral arm {:+.3e}",
                ours.scale, ours.arms[0], ours.arms[1]
            ),
            ours.scale > 1.0 && ours.arms.iter().all(|arm| *arm > 0.0),
        ),
        (
            "the drive scale to the breath",
            format!(
                "{:.4} s a breath against the period {period:.4} s, held {:.4} s",
                ours.length, theirs.length
            ),
            ours.length < period && (theirs.length - period).abs() <= 1e-3,
        ),
        (
            "the breath to ventilation",
            format!(
                "{:.2} a second against held {:.2}",
                ours.ventilation, theirs.ventilation
            ),
            ours.ventilation > theirs.ventilation,
        ),
        (
            "ventilation to alveolar carbon dioxide",
            format!(
                "{:.4} against held {:.4}; alveolar oxygen {:.4} against held {:.4}",
                ours.alveolar[1], theirs.alveolar[1], ours.alveolar[0], theirs.alveolar[0]
            ),
            ours.alveolar[1] < theirs.alveolar[1],
        ),
        (
            "alveolar to arterial carbon dioxide",
            format!(
                "over the clock from {} s to {} s, {:.4} s and {:.4} s, {:.4} against held {:.4}, by {:+.3e} where the cut breaths reach {:.3e}; from {} s by {:+.3e}, from {} s by {:+.3e}; arterial oxygen pressure {:.4} against held {:.4}, saturation {:.6} against held {:.6}",
                WALK[0],
                WALK[1],
                judged.spans[0],
                judged.spans[1],
                judged.means[0],
                judged.means[1],
                judged.margin(),
                judged.reach,
                WALK[0] - 5,
                earlier.margin(),
                WALK[0] + 5,
                later.margin(),
                oxygen[0],
                oxygen[1],
                saturation[0],
                saturation[1]
            ),
            judged.margin() < -judged.reach && earlier.margin() < 0.0 && later.margin() < 0.0,
        ),
    ];
    let mut report = format!(
        "sign walk over whole breaths from {} s to {} s: {} controlled breaths, {} held\n",
        WALK[0], WALK[1], ours.breaths, theirs.breaths
    );
    for (link, reading, _) in &links {
        writeln!(report, "{link}: {reading}").expect("a string takes it");
    }
    let wrong: Vec<&str> = (links.iter())
        .filter(|(_, _, right)| !right)
        .map(|(link, _, _)| *link)
        .collect();
    assert!(wrong.is_empty(), "{report}wrong way: {wrong:?}");
    report
}

/// Sleep enters 30 s after the anchor, where the awake walk starts. No accepted head falls on 30 s and an input stamped
/// before the head it is enqueued at is refused, so the branch resumes the awake run's last head before 30 s, which
/// costs half a minute of the body where the admitted checkpoint would cost a whole one.
const ASLEEP: u64 = 30;
const WAKES: u64 = 150;
const ENDS: u64 = 300;

/// The branch's watch takes the awake run's moments up to the branch point, so it holds the branch's history from
/// admission, as the branch's certificate does. Its core is its own, so its solves stay out of the awake run's count.
fn the_sleeping_branch<'s>(
    start: &'s Start,
    saved: &Checkpoint,
    core: &'s ng_solver::Core,
    awake: &Watch,
) -> (Session<'s, Running>, Watch, u64) {
    let mut session =
        Session::resume(saved, &start.identity, &start.body, core, &start.links).expect("resumed");
    let head = session.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds();
    assert!(
        head < ASLEEP * SECOND,
        "the branch starts {head} ns after the anchor"
    );
    for (id, at, sleep) in [
        (&b"asleep"[..], ASLEEP, Sleep::Nrem),
        (b"awake again", WAKES, Sleep::Awake),
    ] {
        let event = BoundaryEvent::new(
            InputId::new(Digest::of_canonical(id)),
            SimTime::from_ns(anchor_moment().nanoseconds() + at * SECOND),
            BoundaryInput::Sleep(sleep),
        );
        let input =
            AuthorizedInput::from_command_receipt(Digest::of_canonical(b"a test's own command"), event);
        session.enqueue(input).expect("enqueued");
    }
    let mut watch = Watch::new(start);
    watch.moments = (awake.moments.iter())
        .filter(|moment| moment.at < head)
        .cloned()
        .collect();
    watch.see(&session);
    let session = counted_until(
        start,
        session,
        WALK[1] * SECOND,
        &mut Minute::default(),
        &mut watch,
    );
    (session, watch, head)
}

/// What the breath pieces cut at a stretch's two ends can move a run's ventilation over its whole breaths there. A
/// piece counted at the ventilation of any breath that overlaps the stretch moves the mean by at most the piece's
/// share of the stretch times the farthest such a breath sits from the mean. The minute ends inside a breath, which is
/// taken to breathe within the range of the breaths before it.
fn ventilation_reach(watch: &Watch, [from, to]: [u64; 2], mean: f64) -> f64 {
    let at = |index: usize| watch.moments[index].at;
    let edges = watch.edges();
    let farthest = (edges.windows(2))
        .filter(|breath| at(breath[1]) > from * SECOND && at(breath[0]) < to * SECOND)
        .map(|breath| {
            let length = (at(breath[1]) - at(breath[0])) as f64 / SECOND as f64;
            (tidal(watch, breath) / length - mean).abs()
        })
        .fold(0.0, larger);
    let inside: Vec<u64> = (edges.into_iter())
        .map(at)
        .filter(|edge| (from * SECOND..=to * SECOND).contains(edge))
        .collect();
    let cut = (inside[0] - from * SECOND) + (to * SECOND - inside[inside.len() - 1]);
    cut as f64 / ((to - from) * SECOND) as f64 * farthest
}

/// Sleep's two signs against the awake run from the same head, judged as the awake walk judges its arterial link:
/// ventilation over each run's whole breaths with what the cut pieces can move it, arterial carbon dioxide over the
/// clock, and both again from five seconds earlier and later. The stretch starts ten seconds after sleep enters,
/// because the arteries lag the lung and the longer breath moves their swing out of step with the awake run's: the
/// sleeping run's arterial carbon dioxide still reads under the awake run's at single moments to 38 s, inside the
/// second sleeping breath, and stays over it from 39 s.
const SLEPT: [u64; 2] = [40, 60];

fn sleep_lowers_ventilation_and_raises_arterial_carbon_dioxide(
    asleep: &Watch,
    awake: &Watch,
    head: u64,
) -> String {
    let judged = |from: u64| {
        let stretch = [from, SLEPT[1]];
        let [ours, theirs] = [asleep, awake].map(|watch| walked(watch, stretch));
        let reach = ventilation_reach(asleep, stretch, ours.ventilation)
            + ventilation_reach(awake, stretch, theirs.ventilation);
        let arterial = compared(asleep, awake, stretch, |moment| moment.sensed.pressure);
        (ours, theirs, reach, arterial)
    };
    let (ours, theirs, reach, arterial) = judged(SLEPT[0]);
    let [earlier, later] = [SLEPT[0] - 5, SLEPT[0] + 5]
        .map(judged)
        .map(|(ours, theirs, _, arterial)| (ours.ventilation - theirs.ventilation, arterial.margin()));
    let lower = ours.ventilation - theirs.ventilation;
    let report = format!(
        "asleep from {ASLEEP} s, branched at {:.6} s; over whole breaths from {} s to {} s, {} breaths asleep and {} awake\n\
         asleep the drive scale {:.6} and the breath {:.4} s, awake {:.6} and {:.4} s\n\
         ventilation {:.2} a second asleep against {:.2} awake, by {lower:+.3e} where the cut pieces reach {reach:.3e}; from {} s by {:+.3e}, from {} s by {:+.3e}\n\
         arterial carbon dioxide over the clock, {:.4} s and {:.4} s, {:.4} asleep against {:.4} awake, by {:+.3e} where the cut breaths reach {:.3e}; from {} s by {:+.3e}, from {} s by {:+.3e}\n\
         alveolar carbon dioxide {:.4} asleep against {:.4} awake\n",
        head as f64 / SECOND as f64,
        SLEPT[0],
        SLEPT[1],
        ours.breaths,
        theirs.breaths,
        ours.scale,
        ours.length,
        theirs.scale,
        theirs.length,
        ours.ventilation,
        theirs.ventilation,
        SLEPT[0] - 5,
        earlier.0,
        SLEPT[0] + 5,
        later.0,
        arterial.spans[0],
        arterial.spans[1],
        arterial.means[0],
        arterial.means[1],
        arterial.margin(),
        arterial.reach,
        SLEPT[0] - 5,
        earlier.1,
        SLEPT[0] + 5,
        later.1,
        ours.alveolar[1],
        theirs.alveolar[1]
    );
    assert!(
        lower < -reach && earlier.0 < 0.0 && later.0 < 0.0,
        "{report}sleep does not lower ventilation"
    );
    assert!(
        arterial.margin() > arterial.reach && earlier.1 > 0.0 && later.1 > 0.0,
        "{report}sleep does not raise arterial carbon dioxide"
    );
    report
}

/// The branch's certificate covers its whole history, the awake stretch and the transition with the sleeping stretch,
/// since its windows have no start. Each worst change names its window, and the watch's figures from sleep's stamp on
/// set the sleeping stretch apart.
fn the_sleeping_branchs_breathing_books(
    start: &Start,
    branch: &Session<'_, Running>,
    watch: &Watch,
) -> String {
    let certificate = certified(start, branch);
    let figures = watch.figures(ASLEEP);
    let seconds = |at: SimTime| (at.nanoseconds() - anchor_moment().nanoseconds()) as f64 / SECOND as f64;
    let mut report = String::new();
    for (book, at_payer, _) in RULE {
        if let Some((_, figures)) = figures.iter().find(|(of, _)| *of == book && !at_payer) {
            let (worst, window) = worst_change(start, &certificate, book, at_payer);
            let window = window.expect("a window");
            writeln!(
                report,
                "asleep {book:?}: worst {worst:.4e} from {:.3} s to {:.3} s; from {ASLEEP} s any ten seconds {:.4e}, inside a breath {:.4e}",
                seconds(window.start()),
                seconds(window.end()),
                figures.steady,
                figures.inside_a_breath
            )
            .expect("a string takes it");
        }
    }
    report
}

const WOKEN: [u64; 2] = [WAKES + 10, ENDS];

fn the_sleeping_branch_wakes_and_runs_on<'s>(
    start: &Start,
    asleep: Session<'s, Running>,
    watch: &mut Watch,
    awake: &Watch,
) -> (Session<'s, Running>, String) {
    let reference = counted_until(start, asleep, ENDS * SECOND, &mut Minute::default(), watch);
    let [woken, slept] = [WOKEN, SLEPT].map(|stretch| walked(watch, stretch));
    let theirs = walked(awake, SLEPT);
    let report = format!(
        "awake again from {WAKES} s to {ENDS} s; over whole breaths from {} s, {} breaths, the drive scale {:.6} and the breath {:.4} s, against {:.6} and {:.4} s awake and {:.6} and {:.4} s asleep\n",
        WOKEN[0],
        woken.breaths,
        woken.scale,
        woken.length,
        theirs.scale,
        theirs.length,
        slept.scale,
        slept.length
    );
    assert!(
        woken.scale > theirs.scale && woken.length < theirs.length,
        "{report}waking does not lift the drive back over the awake run's"
    );
    (reference, report)
}

fn seconds(ns: u64) -> f64 {
    (ns - anchor_moment().nanoseconds()) as f64 / SECOND as f64
}

fn within(refused: &mut Vec<String>, name: String, measured: f64, (below, figure): Figure) {
    if !(below < 2.0 * measured && 2.0 * measured <= figure) {
        refused.push(format!(
            "{name}: twice {measured:.4e} is not over {below:e} and at most {figure:e}"
        ));
    }
}

/// The certificate keeps one window a combination, so the rates are read from the record's sealed edges, on the
/// windows the evaluator opens.
fn the_reference_run_passes_the_bodys_policy_and_rule_holds_each_figure_to_it(
    start: &Start,
    reference: &Session<'_, Running>,
    watch: &Watch,
) -> String {
    let certificate = certified(start, reference);
    let rhythm = certificate.rhythm().expect("windows between breath edges");
    let (length, settling) = (rhythm.windows().length_ns().get(), rhythm.windows().settling_ns());
    let view = reference.conservation();
    let span = view.span().expect("a span");
    let (clock, _) = (view.rhythms())
        .find(|&(_, phase)| phase == rhythm.windows().clock())
        .expect("the breath's rhythm");
    let members: Vec<Vec<QuantityId>> = (RULE.iter())
        .map(|&(book, at_payer, _)| {
            let held = |&quantity: &QuantityId| holds(&start.body, (book, at_payer), quantity);
            view.basis().ids().filter(held).collect()
        })
        .collect();
    let order = view.in_time_order().expect("the history in time order");
    let mut edges: Vec<(u64, Vec<Vec<f64>>)> = Vec::new();
    for segment in order.from(span.start()) {
        let (_, segment) = segment.expect("a link");
        if segment.ends_on_edge(clock) {
            let interval = segment.interval();
            let at =
                |quantity: &QuantityId| lead(segment.inventory(*quantity).on(interval).expect("its own")[1]);
            let values = members.iter().map(|row| row.iter().map(at).collect()).collect();
            edges.push((interval.end().nanoseconds(), values));
        }
    }
    let mut worst = vec![[(0.0, 0, 0); 2]; RULE.len()];
    let mut judged = 0;
    for (open, (opened, before)) in edges.iter().enumerate() {
        let Some((closed, after)) = edges[open + 1..]
            .iter()
            .find(|(closed, _)| closed - opened >= length)
        else {
            continue;
        };
        judged += 1;
        let stretch = usize::from(opened - span.start().nanoseconds() >= settling);
        let long = (closed - opened) as f64 / SECOND as f64;
        for (held, (before, after)) in worst.iter_mut().zip(before.iter().zip(after)) {
            for (before, after) in before.iter().zip(after) {
                let rate = (after - before).abs() / long;
                if rate > held[stretch].0 {
                    held[stretch] = (rate, *opened, *closed);
                }
            }
        }
    }
    assert_eq!(judged, rhythm.judged(), "the windows the evaluator opens");
    let gaps = edges.windows(2).map(|pair| (pair[1].0 - pair[0].0, pair[1].0));
    let (gap, ending) = gaps.fold((0, 0), |most, gap| if gap.0 > most.0 { gap } else { most });
    let gap = gap as f64 / SECOND as f64;
    let widest = rhythm.widest();
    let mut report = format!(
        "reference run: {} breath edges from {:.3} s to {:.3} s, {judged} windows between edges, the widest gap {gap:.4} s to {:.3} s, the widest stretch {:.4} s to {:.3} s\n",
        edges.len(),
        seconds(edges[0].0),
        seconds(edges[edges.len() - 1].0),
        seconds(ending),
        widest.length_ns() as f64 / SECOND as f64,
        seconds(widest.end().nanoseconds())
    );
    let mut refused = Vec::new();
    within(&mut refused, "the widest stretch".into(), gap, WIDEST);
    let figures = watch.figures(20);
    for (&(book, at_payer, [drift, settled, early]), [before, after]) in RULE.iter().zip(&worst) {
        let (ten, window) = worst_change(start, &certificate, book, at_payer);
        let window = window.expect("a window");
        let breath = (figures.iter())
            .find(|(of, _)| *of == book && !at_payer)
            .map(|(_, figures)| figures.inside_a_breath);
        let name = format!("{book:?}{}", if at_payer { " at a payer" } else { "" });
        let inside = breath.map_or(String::new(), |breath| format!(", inside a breath {breath:.4e}"));
        writeln!(
            report,
            "{name}: ten seconds {ten:.4e} from {:.3} s{inside}; between edges {:.4e} a second from {:.3} s to {:.3} s, before the settling time {:.4e} from {:.3} s to {:.3} s",
            seconds(window.start().nanoseconds()),
            after.0,
            seconds(after.1),
            seconds(after.2),
            before.0,
            seconds(before.1),
            seconds(before.2)
        )
        .expect("a string takes it");
        within(
            &mut refused,
            format!("{name}'s drift"),
            breath.map_or(ten, |breath| larger(ten, breath)),
            drift,
        );
        within(&mut refused, format!("{name}'s edge rate"), after.0, settled);
        within(&mut refused, format!("{name}'s settling rate"), before.0, early);
    }
    assert!(refused.is_empty(), "{report}{}", refused.join("\n"));
    report
}

fn the_held_runs_breathing_books(held: &Start, minute: &Session<'_, Running>, watch: &Watch) -> String {
    let certificate = certified(held, minute);
    let figures = watch.figures(20);
    let mut report = String::new();
    for (book, at_payer, _) in RULE {
        if let Some((_, figures)) = figures.iter().find(|(of, _)| *of == book && !at_payer) {
            let (worst, _) = worst_change(held, &certificate, book, at_payer);
            writeln!(report, "held {book:?}: {}", line(worst, figures)).expect("a string takes it");
        }
    }
    report
}

const QUARTER: f64 = 0.25;
const QUARTER_ENDS: u64 = 60;
const LEAK_ENDS: u64 = 40;
const LOOSE: f64 = 1e30;
const LEAK_NOTE: &str = "synthetic: a test's leak";
const NET_BAND: f64 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Leak {
    Oxygen,
    Carbon,
    NitrogenGas,
    Vasopressin,
}
impl Leak {
    const ALL: [Leak; 4] = [Leak::Oxygen, Leak::Carbon, Leak::NitrogenGas, Leak::Vasopressin];
    fn book(self) -> Book {
        match self {
            Leak::Oxygen => Book::Element(Element::Oxygen),
            Leak::Carbon => Book::Element(Element::Carbon),
            Leak::NitrogenGas => Book::Of(Species::NitrogenGas),
            Leak::Vasopressin => Book::Of(Species::Vasopressin),
        }
    }
    fn size(self) -> f64 {
        match self {
            Leak::Oxygen => 6e1,
            Leak::Carbon => 5e1,
            Leak::NitrogenGas => 3e1,
            Leak::Vasopressin => -3e-3,
        }
    }
    /// Each route's rate is sized so its row nets its size over the window it is refused in. The lungs breathe most of
    /// the nitrogen gas in their gas out within a breath, so its route is near six times its size. That gas lowers
    /// oxygen's row by about a third of each unit and carbon's by about a fortieth, so carbon dioxide's route takes
    /// back carbon's share, and oxygen gas's route makes up what oxygen's row lacks of its size after the two oxygen
    /// atoms each carbon dioxide brings.
    fn route(self) -> (f64, &'static str) {
        match self {
            Leak::Oxygen => (5.82, "oxygen gas into the arteries' blood"),
            Leak::Carbon => (5.39e1, "carbon dioxide into the arteries' blood"),
            Leak::NitrogenGas => (1.73e2, "nitrogen gas into the pulmonary bed's gas"),
            Leak::Vasopressin => (self.size(), "vasopressin into the central veins' blood"),
        }
    }
    fn settled(self, body: &CompiledBody, windows: SlidingWindows) -> Compensated {
        let (policy, quantity) = (
            body_certificate(body, windows).expect("the body's policy"),
            self.quantity(body),
        );
        let (_, rates) = policy.edges().expect("windows between breath edges");
        (rates.iter().find(|rate| rate.quantity() == quantity))
            .unwrap_or_else(|| panic!("the body rates {self:?}'s {quantity:?}"))
            .settled()
    }
    fn alone(self, body: &CompiledBody, windows: SlidingWindows) -> CertificatePolicy {
        let (quantity, rate) = (self.quantity(body), lead(self.settled(body, windows)));
        let rate = match self {
            Leak::Oxygen | Leak::Carbon => EdgeRate::element(quantity, finite(rate), None),
            Leak::NitrogenGas => EdgeRate::new::<NitrogenGas>(quantity, finite(rate), None),
            Leak::Vasopressin => EdgeRate::new::<Vasopressin>(quantity, finite(rate), None),
        }
        .expect("its own dimension");
        let loose = certificate_with(body, windows, &Loose).expect("a loose policy");
        loose
            .between_edges(edge_windows(body), vec![rate])
            .expect("its rates")
    }
    fn quantity(self, body: &CompiledBody) -> QuantityId {
        let mut held = (body.conserved_basis().ids()).filter(|quantity| quantity.book() == self.book());
        match (held.next(), held.next()) {
            (Some(quantity), None) => quantity,
            _ => panic!("{self:?}'s row is not one combination"),
        }
    }
}

struct Loose;
impl DriftTable for Loose {
    fn names(&self, _: Species) -> bool {
        true
    }
    fn drift<Q: Substance>(&self) -> Result<Measure<Q>, AdmissionError> {
        Measure::new(LOOSE)
    }
    fn element(&self, _: Element, _: &[SiteId]) -> Result<Option<Measure<Atoms>>, AdmissionError> {
        Measure::new(LOOSE).map(Some)
    }
}

fn finite<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn room(at: u64, humidity: f64) -> BoundaryEvent {
    let id = InputId::new(Digest::of_canonical(&humidity.to_bits().to_be_bytes()));
    let input = BoundaryInput::Room(Room {
        air: finite(307.0),
        radiant: finite(308.0),
        humidity: finite(humidity),
        air_speed: finite(0.0),
        pressure: finite(765.0),
        oxygen: finite(0.21),
        carbon_dioxide: finite(0.0004),
        clothing: finite(0.9),
    });
    BoundaryEvent::new(id, SimTime::from_ns(anchor_moment().nanoseconds() + at), input)
}

fn leaking_packet(body: &CompiledBody, rate: f64) -> ParameterPacket<SyntheticFixture> {
    let mut packet = body_packet_with_the_drive_scale_held(body, finite(rate)).expect("a packet");
    let (key, at) = (ParameterKey::new, Binding::Body);
    let route = |leak: Leak| leak.route().0;
    let stated = [
        packet.synthetic(
            key("bd.oxygen-leak"),
            at,
            finite::<Rate<Oxygen>>(route(Leak::Oxygen)),
            LEAK_NOTE,
        ),
        packet.synthetic(
            key("bd.carbon-dioxide-leak"),
            at,
            finite::<Rate<CarbonDioxide>>(route(Leak::Carbon)),
            LEAK_NOTE,
        ),
        packet.synthetic(
            key("bd.nitrogen-gas-leak"),
            at,
            finite::<Rate<NitrogenGas>>(route(Leak::NitrogenGas)),
            LEAK_NOTE,
        ),
        packet.synthetic(
            key("bd.vasopressin-leak"),
            at,
            finite::<Rate<Vasopressin>>(route(Leak::Vasopressin)),
            LEAK_NOTE,
        ),
    ];
    for stated in stated {
        stated.expect("once");
    }
    packet
}

fn at_the_edges(run: &Session<'_, Running>, clock: Slot, quantities: &[QuantityId]) -> Vec<(u64, Vec<f64>)> {
    let view = run.conservation();
    let span = view.span().expect("a span");
    let (rhythm, _) = (view.rhythms())
        .find(|&(_, phase)| phase == clock)
        .expect("the breath's rhythm");
    let order = view.in_time_order().expect("the history in time order");
    let mut edges = Vec::new();
    for segment in order.from(span.start()) {
        let (_, segment) = segment.expect("a link");
        if segment.ends_on_edge(rhythm) {
            let interval = segment.interval();
            let at =
                |quantity: &QuantityId| lead(segment.inventory(*quantity).on(interval).expect("its own")[1]);
            edges.push((interval.end().nanoseconds(), quantities.iter().map(at).collect()));
        }
    }
    edges
}

fn sliding(run: &Session<'_, Running>) -> SlidingWindows {
    let reached = run.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds();
    SlidingWindows::new(NonZeroU64::new(10 * SECOND).expect("a window"), reached).expect("windows")
}

/// The body's windows between breath edges, which no run's length moves.
fn edge_windows(body: &CompiledBody) -> EdgeWindows {
    let policy = body_certificate(body, SlidingWindows::new(NonZeroU64::MIN, 1).expect("windows"))
        .expect("the body's policy");
    let (windows, _) = policy.edges().expect("windows between breath edges");
    windows
}

fn settled_edge(windows: EdgeWindows, edges: &[(u64, Vec<f64>)]) -> usize {
    (edges.iter())
        .position(|(at, _)| at - anchor_moment().nanoseconds() >= windows.settling_ns())
        .expect("an edge after the settling time")
}

fn nets(
    windows: EdgeWindows,
    control: &[(u64, Vec<f64>)],
    leaking: &[(u64, Vec<f64>)],
) -> (Interval, [usize; 2], Vec<f64>) {
    let open = settled_edge(windows, leaking);
    let close = (open + 1..leaking.len())
        .find(|&close| leaking[close].0 - leaking[open].0 >= windows.length_ns().get())
        .expect("the window closes");
    for edge in [open, close] {
        let apart = leaking[edge].0.abs_diff(control[edge].0);
        assert!(
            apart < MILLISECOND,
            "edge {edge} of the two runs is {apart} ns apart"
        );
    }
    let window = Interval::new(
        SimTime::from_ns(leaking[open].0),
        SimTime::from_ns(leaking[close].0),
    )
    .expect("a window");
    let long = window.length_ns() as f64 / SECOND as f64;
    let changed = |run: &[(u64, Vec<f64>)], index: usize| run[close].1[index] - run[open].1[index];
    let nets = (0..leaking[open].1.len())
        .map(|index| (changed(leaking, index) - changed(control, index)) / long)
        .collect();
    (window, [open, close], nets)
}

fn at_a_quarter_each_leak_is_refused_between_edges_against_its_drift() -> String {
    let start = Start::leaking(QUARTER);
    assert_eq!(lead(parameter(&start, "ns.breath-rate")), QUARTER);
    let quantities = Leak::ALL.map(|leak| leak.quantity(&start.body));
    let windows = edge_windows(&start.body);
    let control = {
        let run = until(start.session(), LEAK_ENDS * SECOND);
        for leak in Leak::ALL {
            let alone = leak.alone(&start.body, sliding(&run));
            if let Err(failed) = certify(&run.conservation(), &alone) {
                panic!("the run with no leak is refused by {leak:?}'s own policy: {failed:?}");
            }
        }
        at_the_edges(&run, windows.clock(), &quantities)
    };
    let (from, to) = (
        &control[settled_edge(windows, &control)],
        &control[control.len() - 1],
    );
    let drifts =
        (0..quantities.len()).map(|row| (to.1[row] - from.1[row]) / ((to.0 - from.0) as f64 / SECOND as f64));
    let mut session = start.session();
    let opens = room(windows.settling_ns(), 1.0);
    let input = AuthorizedInput::from_command_receipt(Digest::of_canonical(b"a test's own command"), opens);
    session.enqueue(input).expect("enqueued");
    let leaking = until(session, LEAK_ENDS * SECOND);
    let edges = at_the_edges(&leaking, windows.clock(), &quantities);
    let (window, [open, close], nets) = nets(windows, &control, &edges);
    assert!(window.start().nanoseconds() - anchor_moment().nanoseconds() >= windows.settling_ns());
    let mut report = format!(
        "leaking body at {QUARTER} a second: with its room shut it passes each leak's own policy over {LEAK_ENDS} s; its rows' mean change from {:.3} s to {:.3} s\n",
        seconds(from.0),
        seconds(to.0)
    );
    let mut off = Vec::new();
    for ((leak, drift), net) in Leak::ALL.into_iter().zip(drifts).zip(nets) {
        assert!(
            drift * leak.size() < 0.0,
            "{leak:?} leaks {:+e} a second and the run with no leak moves {drift:+.4e} a second",
            leak.size()
        );
        if (net - leak.size()).abs() > NET_BAND * leak.size().abs() {
            off.push(format!(
                "{leak:?} nets {net:+.4e} a second against its size {:+e}",
                leak.size()
            ));
        }
        let (rate, into) = leak.route();
        writeln!(
            report,
            "{:?} moves {drift:+.4e} a second; its leak, {rate:+e} a second of {into}, nets {net:+.4e} from {:.3} s to {:.3} s against its size {:+e}",
            leak.book(),
            seconds(window.start().nanoseconds()),
            seconds(window.end().nanoseconds()),
            leak.size()
        )
        .expect("a string takes it");
    }
    assert!(off.is_empty(), "{report}{}", off.join("\n"));
    let long = window.length_ns() as f64 / SECOND as f64;
    for (at, leak) in Leak::ALL.into_iter().enumerate() {
        let alone = leak.alone(&start.body, sliding(&leaking));
        let failure = certify(&leaking.conservation(), &alone).expect_err("the leak is refused");
        assert_eq!(
            (failure.check(), failure.quantity(), failure.window()),
            (CertificateCheck::EdgeDrift, Some(quantities[at]), Some(window)),
            "{leak:?}"
        );
        let moved = edges[close].1[at] - edges[open].1[at];
        let rate = lead(leak.settled(&start.body, sliding(&leaking)));
        let (change, allowance) = (
            lead(failure.residual()),
            lead(failure.allowance().expect("an allowance")),
        );
        assert!(
            (allowance - rate * long).abs() <= 1e-12 * allowance,
            "{leak:?}'s allowance {allowance:e} is not its rate {rate:e} over {long} s"
        );
        assert!(
            (change - moved.abs()).abs() <= 1e-9 * change && moved * leak.size() > 0.0 && change > allowance,
            "{leak:?} changed {change:e} against {allowance:e}, and {moved:+e} at its edges"
        );
        writeln!(
            report,
            "{:?} refused between edges from {:.3} s to {:.3} s: changed {moved:+.4e} against {allowance:.4e}",
            leak.book(),
            seconds(window.start().nanoseconds()),
            seconds(window.end().nanoseconds()),
        )
        .expect("a string takes it");
    }
    report
}

fn at_a_quarter_the_run_with_no_leak_passes_the_bodys_policy() -> String {
    let start = Start::held_at(finite(QUARTER));
    assert_eq!(lead(parameter(&start, "ns.breath-rate")), QUARTER);
    let run = until(start.session(), QUARTER_ENDS * SECOND);
    let certificate = certified(&start, &run);
    let judged = certificate.rhythm().expect("windows between edges").judged();
    format!(
        "held run at {QUARTER} a second: passes the body's policy over {QUARTER_ENDS} s, {judged} windows between edges\n"
    )
}

#[test]
fn at_a_quarter_the_body_passes_its_policy_and_its_certificate_refuses_each_leak_between_edges() {
    let mut report = at_a_quarter_each_leak_is_refused_between_edges_against_its_drift();
    report += &at_a_quarter_the_run_with_no_leak_passes_the_bodys_policy();
    expect![[r#"
        leaking body at 0.25 a second: with its room shut it passes each leak's own policy over 40 s; its rows' mean change from 20.001 s to 36.001 s
        Element(Oxygen) moves -4.3066e0 a second; its leak, +5.82e0 a second of oxygen gas into the arteries' blood, nets +5.9996e1 from 20.001 s to 32.001 s against its size +6e1
        Element(Carbon) moves -3.5103e0 a second; its leak, +5.39e1 a second of carbon dioxide into the arteries' blood, nets +4.9961e1 from 20.001 s to 32.001 s against its size +5e1
        Of(NitrogenGas) moves -6.8914e-1 a second; its leak, +1.73e2 a second of nitrogen gas into the pulmonary bed's gas, nets +2.9977e1 from 20.001 s to 32.001 s against its size +3e1
        Of(Vasopressin) moves +5.1485e-5 a second; its leak, -3e-3 a second of vasopressin into the central veins' blood, nets -2.9826e-3 from 20.001 s to 32.001 s against its size -3e-3
        Element(Oxygen) refused between edges from 20.001 s to 32.001 s: changed +6.6923e2 against 4.8001e2
        Element(Carbon) refused between edges from 20.001 s to 32.001 s: changed +5.5736e2 against 3.6000e2
        Of(NitrogenGas) refused between edges from 20.001 s to 32.001 s: changed +3.5103e2 against 2.4000e2
        Of(Vasopressin) refused between edges from 20.001 s to 32.001 s: changed -3.5207e-2 against 2.4000e-2
        held run at 0.25 a second: passes the body's policy over 60 s, 11 windows between edges
    "#]].assert_eq(&report);
}

fn to_the_breaths_end<'s>(start: &Start, mut session: Session<'s, Running>) -> Session<'s, Running> {
    let phase = (start.body.rows().iter())
        .find(|row| matches!(row.kind(), RowKind::Memory(slot) if slot.law.owner == "NS" && slot.law.local == "breath phase"))
        .expect("the breath's phase")
        .id();
    loop {
        let (before, at) = (
            lead(session.record().accepted().value(phase)),
            session.record().accepted().time().nanoseconds(),
        );
        session = until(session, at + 1 - anchor_moment().nanoseconds());
        if lead(session.record().accepted().value(phase)) < before {
            return session;
        }
    }
}

fn each_gas_closes_on_the_airway_the_room_and_the_membrane_over_two_whole_breaths(
    start: &Start,
    minute: Session<'_, Running>,
) -> String {
    let body = &start.body;
    let columns = every_column(body);
    let (airway, lung) = (
        site::<TissueSite>("conducting airway"),
        site::<CapillaryBed>("pulmonary bed"),
    );
    let held = |session: &Session<'_, Running>, gas| -> f64 {
        let view = session.record().accepted();
        let in_gas = |kind| matches!(kind, RowKind::Amount { site, species, space: Space::Gas } if (site == airway || site == lung) && species == gas);
        (body.rows().iter())
            .filter(|row| in_gas(row.kind()))
            .map(|row| lead(view.value(row.id())))
            .sum()
    };
    let gases = [
        Species::Oxygen,
        Species::CarbonDioxide,
        Species::NitrogenGas,
        Species::Water,
    ];
    let edge = to_the_breaths_end(start, minute);
    let from = edge.record().accepted().time().nanoseconds();
    let before = gases.map(|gas| held(&edge, gas));
    let session = to_the_breaths_end(start, to_the_breaths_end(start, edge));
    let to = session.record().accepted().time().nanoseconds();
    let mut report = format!(
        "two breaths from {:.3} s to {:.3} s\n",
        (from - anchor_moment().nanoseconds()) as f64 / SECOND as f64,
        (to - anchor_moment().nanoseconds()) as f64 / SECOND as f64
    );
    let after = gases.map(|gas| held(&session, gas));
    let (early, late) = (extents(&session, &columns, from), extents(&session, &columns, to));
    for (at, gas) in gases.into_iter().enumerate() {
        let gas_at =
            |bed: SiteId| move |site, species, space| site == bed && species == gas && space == Space::Gas;
        let (mut inspired, mut expired, mut diffused, mut along) = (0.0, 0.0, 0.0, 0.0);
        for &column in &columns {
            let (into_airway, into_lung) = (
                count(body, column, gas_at(airway)),
                count(body, column, gas_at(lung)),
            );
            if into_airway == 0.0 && into_lung == 0.0 {
                continue;
            }
            let moved = late[column.index()] - early[column.index()];
            let placed = &body.columns()[column.index()];
            let receipt = placed.receipt().map(|receipt| receipt.boundary());
            assert_eq!(
                placed.origin().owner,
                "RS",
                "{:?} moves {gas:?} in the airway's gas",
                placed.origin()
            );
            match (receipt, into_airway, into_lung) {
                (Some(_), 1.0, 0.0) => inspired += moved,
                (Some(_), -1.0, 0.0) => expired += moved,
                (None, -1.0, 1.0) => along += moved,
                (None, 0.0, -1.0) => diffused += moved,
                (None, 0.0, 1.0) => diffused -= moved,
                other => panic!("{:?} moves {gas:?} as {other:?}", placed.origin()),
            }
        }
        let gained = after[at] - before[at];
        let gross = [inspired, expired, diffused, along]
            .map(f64::abs)
            .into_iter()
            .fold(0.0, |most, x| if x > most { x } else { most });
        assert!(gross > 0.0, "{gas:?} moved nowhere");
        assert!(
            (gained - (inspired - expired - diffused)).abs() <= 1e-9 * gross,
            "{gas:?}: in {inspired}, out {expired}, across {diffused}, along {along}, gained {gained}"
        );
        writeln!(
            report,
            "{gas:?}: in {inspired:.4}, out {expired:.4}, across {diffused:.4}, along {along:.4}, gained {gained:.4e}"
        )
        .expect("a string takes it");
    }
    report
}

fn the_first_second_of_the_body_on_ng_solver_stays_where_it_was(start: &Start) {
    let at = |key| Context::Site(site::<VesselSegment>(key));
    let (ventricle, tubule) = (
        site::<VesselSegment>("left ventricle"),
        site::<VesselSegment>("renal tubule"),
    );
    let (bed, arteries) = (
        site::<CapillaryBed>("glomerular bed"),
        site::<VesselSegment>("systemic arteries"),
    );
    let aortic = Slot {
        law: LawName {
            owner: "CV",
            local: "valves",
        },
        at: Context::Edge {
            from: ventricle,
            to: arteries,
        },
    };
    let kinds = [
        (
            "arterial pressure",
            field("CV", "blood pressure", at("systemic arteries")),
        ),
        (
            "right atrial pressure",
            field("CV", "blood pressure", at("right atrium")),
        ),
        (
            "portal vein pressure",
            field("CV", "blood pressure", at("portal vein")),
        ),
        (
            "leg vein pressure",
            field("CV", "blood pressure", at("leg veins")),
        ),
        ("heart rate", field("CV", "rate", Context::Body)),
        (
            "sympathetic traffic",
            field("NS", "sympathetic traffic", Context::Body),
        ),
        (
            "filtration",
            field(
                "RN",
                "filtration",
                Context::Edge {
                    from: bed,
                    to: tubule,
                },
            ),
        ),
        ("urine", field("Boundary", "urine", Context::Body)),
        ("wall power", field("CV", "wall power", Context::Body)),
        ("aortic flow", RowKind::Memory(aortic)),
    ];
    let mut session = start.session();
    let mut trace = String::new();
    for tenth in 1..=10 {
        session = until(session, tenth * 100 * MILLISECOND);
        let view = session.record().accepted();
        let held = (view.time().nanoseconds() - anchor_moment().nanoseconds()) as f64 / 1e6;
        write!(trace, "{held:>11.6} ms").expect("a string takes it");
        for (name, kind) in kinds {
            let row = start
                .body
                .rows()
                .iter()
                .find(|row| row.kind() == kind)
                .expect("a row");
            write!(trace, "  {name} {:.6}", lead(view.value(row.id()))).expect("a string takes it");
        }
        trace.push('\n');
    }
    let record = session.record();
    let ratio = Edge::at(start, &start.session()).resistance() / CORONARY_RESISTANCE;
    let pin = format!(
        "packet {}\nseal {}\nsteps {}\nviscosity over its reference at the start {ratio:.9}\n",
        hex(start.identity.packet().digest()),
        hex(record.seal().digest()),
        record.revision()
    );
    expect![[r#"
        packet c7ec85aee0622edf508da2a0ada7da12c2d77fe49c97c9e790583cbcdc006ef4
        seal 0aa9eb4ea3ac09101fa9655857cc9d1cabd48408c1c29d0562d9948266390b5f
        steps 198
        viscosity over its reference at the start 1.000092324
    "#]]
    .assert_eq(&pin);
    expect![[r#"
         100.404686 ms  arterial pressure 134.595436  right atrial pressure 2.899838  portal vein pressure 6.243627  leg vein pressure 3.563185  heart rate 1.063911  sympathetic traffic 0.786252  filtration 1.670550  urine 0.015786  wall power 24546.873073  aortic flow 0.000000
         201.522035 ms  arterial pressure 126.975733  right atrial pressure 3.333237  portal vein pressure 6.374803  leg vein pressure 3.722203  heart rate 1.081155  sympathetic traffic 0.085799  filtration 2.108908  urine 0.015786  wall power 15542.100771  aortic flow 39.654803
         302.706352 ms  arterial pressure 118.697020  right atrial pressure 3.744205  portal vein pressure 6.158129  leg vein pressure 3.904980  heart rate 1.047384  sympathetic traffic 0.016483  filtration 2.342035  urine 0.015786  wall power 38216.953770  aortic flow 0.000000
         403.022757 ms  arterial pressure 109.417396  right atrial pressure 3.252423  portal vein pressure 5.910982  leg vein pressure 4.252114  heart rate 1.004767  sympathetic traffic 0.012387  filtration 2.405432  urine 0.015786  wall power 14003.499405  aortic flow 0.000000
         504.258003 ms  arterial pressure 100.951074  right atrial pressure 1.517484  portal vein pressure 5.658394  leg vein pressure 3.951843  heart rate 0.970402  sympathetic traffic 0.023270  filtration 2.343725  urine 0.015786  wall power 11997.268313  aortic flow 0.000000
         608.059522 ms  arterial pressure 93.050732  right atrial pressure 0.922101  portal vein pressure 5.367241  leg vein pressure 3.258118  heart rate 0.947688  sympathetic traffic 0.075204  filtration 2.189160  urine 0.015786  wall power 8625.224992  aortic flow 0.000000
         708.059522 ms  arterial pressure 86.067502  right atrial pressure 0.844003  portal vein pressure 5.145296  leg vein pressure 2.684889  heart rate 0.940766  sympathetic traffic 0.262970  filtration 1.993594  urine 0.015786  wall power 6296.317052  aortic flow 0.000000
         808.059522 ms  arterial pressure 79.633379  right atrial pressure 1.012081  portal vein pressure 5.135669  leg vein pressure 2.440226  heart rate 0.952637  sympathetic traffic 0.642936  filtration 1.781198  urine 0.015786  wall power 4912.445667  aortic flow 0.000000
         908.059522 ms  arterial pressure 73.719445  right atrial pressure 1.262543  portal vein pressure 5.411434  leg vein pressure 2.534952  heart rate 0.982910  sympathetic traffic 0.904771  filtration 1.567550  urine 0.015786  wall power 3946.273758  aortic flow 0.000000
        1000.440947 ms  arterial pressure 68.693980  right atrial pressure 1.495867  portal vein pressure 5.777187  leg vein pressure 2.727024  heart rate 1.021751  sympathetic traffic 0.977546  filtration 1.378176  urine 0.015786  wall power 3075.457520  aortic flow 0.000000
    "#]].assert_eq(&trace);
}

/// The coronary bed's inflow edge has no tone and no fall, so only viscosity and its bed's debt move its resistance
/// from the packet's.
const CORONARY_RESISTANCE: f64 = 1.4 / 0.1;
const CORONARY_DEBT: (f64, f64) = (0.5, 8e-3);

struct Edge {
    hematocrit: f64,
    viscosity: f64,
    drop: f64,
    flow: f64,
    debt: f64,
}
impl Edge {
    fn at(start: &Start, session: &Session<'_, Running>) -> Self {
        let (arteries, coronary, veins) = (
            site::<VesselSegment>("systemic arteries"),
            site::<CapillaryBed>("coronary bed"),
            site::<VesselSegment>("central veins"),
        );
        let view = session.record().accepted();
        let value = |kind: RowKind| {
            let row = start
                .body
                .rows()
                .iter()
                .find(|row| row.kind() == kind)
                .expect("a row");
            lead(view.value(row.id()))
        };
        let pressure = |at| value(field("CV", "blood pressure", Context::Site(at)));
        let lumen = |species| {
            value(RowKind::Amount {
                site: veins,
                species,
                space: Space::Lumen,
            })
        };
        let edge = Context::Edge {
            from: arteries,
            to: coronary,
        };
        Self {
            hematocrit: lumen(Species::Hemoglobin) / lumen(Species::Water) / 5.0,
            viscosity: value(field("HC", "viscosity", Context::Body)),
            drop: pressure(arteries) - pressure(coronary),
            flow: value(field("CV", "circulation", edge)),
            debt: value(field("MT", "oxygen_debt_per_bed", Context::Site(coronary))),
        }
    }
    fn kept(&self) -> f64 {
        let ((share, half), owed) = (CORONARY_DEBT, self.debt);
        (half + (owed - share * owed)) / (owed + half)
    }
    fn resistance(&self) -> f64 {
        self.drop / self.flow / self.kept()
    }
}

/// Dividing out the drop and the share the bed's debt keeps leaves viscosity's part of the edge's flow. The body's gases
/// are read at the start of a beat before the bolus and after it, and between the two they change only by what the
/// room, the burns and the urine move, so the bolus carries none.
fn a_water_bolus_dilutes_the_blood_and_leaves_its_gases_where_they_were(start: &Start) {
    let mut session = start.session();
    let before = Edge::at(start, &session);
    let water = WaterDelivery {
        boundary: "gut water",
        amount: Measure::new(400.0).expect("finite"),
        ends: SimTime::from_ns(anchor_moment().nanoseconds() + 2 * SECOND),
    };
    let at = SimTime::from_ns(anchor_moment().nanoseconds() + SECOND);
    let event = BoundaryEvent::new(
        InputId::new(Digest::of_canonical(b"water bolus")),
        at,
        BoundaryInput::Water(water),
    );
    let input = AuthorizedInput::from_command_receipt(Digest::of_canonical(b"a test's own command"), event);
    session.enqueue(input).expect("enqueued");
    let arterial = |session: &Session<'_, Running>| {
        [Species::Hemoglobin, Species::Oxygen]
            .map(|species| concentration(start, session, "systemic arteries", species))
    };
    let before_arterial = arterial(&session);
    let mut openings: Vec<(u64, u64, [f64; 2])> = Vec::new();
    let session = beats(start, session, 3 * SECOND, |session, opened| {
        if let Some(closed) = opened {
            let at = session.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds();
            let gases = [Species::Oxygen, Species::CarbonDioxide].map(|gas| body_gas(start, session, gas));
            openings.push((at, closed, gases));
        }
    });
    let starts = onsets(
        &openings
            .iter()
            .map(|(at, closed, _)| (*at, *closed))
            .collect::<Vec<_>>(),
    );
    let gases = |at: u64| {
        openings
            .iter()
            .find(|opening| opening.0 == at)
            .map(|opening| opening.2)
    };
    let (first, last) = (
        *(starts.iter().find(|at| **at < 2 * SECOND)).expect("a beat starts before the bolus ends"),
        *(starts.iter().find(|at| **at >= 2 * SECOND)).expect("a beat starts after the bolus"),
    );
    let (held, delivered) = (
        gases(first).expect("read at its start"),
        gases(last).expect("read at its start"),
    );
    let body = &start.body;
    let columns = every_column(body);
    let [from, to] = [first, last].map(|at| extents(&session, &columns, anchor_moment().nanoseconds() + at));
    let expected: Vec<(usize, ColumnId)> = (columns.iter())
        .filter(|column| {
            let placed = &body.columns()[column.index()];
            matches!(placed.origin().owner, "RS" | "MT") || placed.origin().local == "urine"
        })
        .map(|column| (column.index(), *column))
        .collect();
    for (gas, (held, delivered)) in [Species::Oxygen, Species::CarbonDioxide]
        .iter()
        .zip(held.iter().zip(&delivered))
    {
        let moved: f64 = (expected.iter())
            .map(|(index, column)| {
                count(body, *column, |_, species, _| species == *gas) * (to[*index] - from[*index])
            })
            .sum();
        assert!(
            (delivered - held - moved).abs() <= 1e-9 * held,
            "the body's {gas:?} went from {held} to {delivered} over the bolus, where the room, the burns and the urine moved {moved}"
        );
    }
    let session = until(session, 30 * SECOND);
    let after_arterial = arterial(&session);
    assert!(
        after_arterial[0] < before_arterial[0] && after_arterial[1] < before_arterial[1],
        "arterial hemoglobin and oxygen content went from {before_arterial:?} to {after_arterial:?}"
    );
    let after = Edge::at(start, &session);
    let (ratio, resisted) = (
        after.viscosity / before.viscosity,
        after.resistance() / before.resistance(),
    );
    let report = format!(
        "hematocrit {} to {}, viscosity {} to {}, resistance ratio {resisted}, drop {} to {}, flow {} to {}, debt {} to {}",
        before.hematocrit,
        after.hematocrit,
        before.viscosity,
        after.viscosity,
        before.drop,
        after.drop,
        before.flow,
        after.flow,
        before.debt,
        after.debt
    );
    assert!(after.hematocrit < before.hematocrit && ratio < 1.0, "{report}");
    assert!((resisted - ratio).abs() < 1e-9, "{report}");
    let through_viscosity = after.drop / before.drop / ratio / (after.kept() / before.kept());
    assert!(
        after.flow > before.flow && (after.flow / before.flow - through_viscosity).abs() < 1e-9,
        "{report}"
    );
}

const QUOTIENT: f64 = 0.75;

fn every_column(body: &CompiledBody) -> Vec<ColumnId> {
    let mut columns: Vec<ColumnId> = (body.instances().iter())
        .filter_map(|instance| match instance.target() {
            Target::Column(column) => Some(column),
            Target::Row(_) => None,
        })
        .collect();
    columns.sort();
    columns.dedup();
    assert_eq!(columns.len(), body.columns().len(), "a column no law moves");
    columns
}

fn extents(session: &Session<'_, Running>, columns: &[ColumnId], at: u64) -> Vec<f64> {
    for segment in session.conservation().newest_first() {
        let segment = segment.expect("a segment");
        let interval = segment.interval();
        if interval.start().nanoseconds() < at && at <= interval.end().nanoseconds() {
            let piece = Interval::new(interval.start(), SimTime::from_ns(at)).expect("a piece");
            let mut extent = vec![0.0; columns.len()];
            for column in columns {
                let (forward, reverse) = segment.exchange(*column);
                let reached = |curve: Curve<'_>| lead(curve.on(piece).expect("inside its segment")[1]);
                extent[column.index()] = reached(forward) - reached(reverse);
            }
            return extent;
        }
    }
    panic!("no segment holds {at}")
}

fn count(body: &CompiledBody, column: ColumnId, on: impl Fn(SiteId, Species, Space) -> bool) -> f64 {
    (body.columns()[column.index()].terms().iter())
        .filter(|(row, _)| match body.rows()[row.index()].kind() {
            RowKind::Amount { site, species, space } => on(site, species, space),
            RowKind::Memory(_) | RowKind::Algebraic(_) => false,
        })
        .map(|&(_, count)| count as f64)
        .sum()
}

fn burns(body: &CompiledBody, column: ColumnId) -> bool {
    let origin = body.columns()[column.index()].origin();
    origin.owner == "MT"
        && ["glucose burn", "fatty acid burn", "glycolysis", "lactate burn"].contains(&origin.local)
}

fn at_bed(body: &CompiledBody, column: ColumnId, bed: SiteId) -> bool {
    body.columns()[column.index()].context() == Context::Site(bed)
}

fn held_at(start: &Start, session: &Session<'_, Running>, bed: SiteId, gas: Species) -> f64 {
    let view = session.record().accepted();
    (start.body.rows().iter())
        .filter(|row| matches!(row.kind(), RowKind::Amount { site, species, .. } if site == bed && species == gas))
        .map(|row| lead(view.value(row.id())))
        .sum()
}

fn bed_heat(start: &Start, session: &Session<'_, Running>) -> f64 {
    let view = session.record().accepted();
    (start.body.rows().iter())
        .filter(|row| match row.kind() {
            RowKind::Amount {
                site,
                species: Species::Energy,
                space: Space::Thermal,
            } => site.key().ends_with(" bed"),
            _ => false,
        })
        .map(|row| lead(view.value(row.id())))
        .sum()
}

#[derive(Debug)]
struct Burned {
    oxygen: f64,
    lung_bed_oxygen: f64,
    carbon_dioxide: f64,
    lung_bed_carbon_dioxide: f64,
    energy: f64,
    to_core: f64,
    work: f64,
}

fn burned(body: &CompiledBody, columns: &[ColumnId], from: &[f64], to: &[f64]) -> Burned {
    let lung = site::<CapillaryBed>("pulmonary bed");
    let bed_thermal = |site: SiteId, species, space| {
        species == Species::Energy && space == Space::Thermal && site.key().ends_with(" bed")
    };
    let core = site::<TissueSite>("body");
    let core_thermal = |site: SiteId, species: Species, space: Space| {
        site == core && species == Species::Energy && space == Space::Thermal
    };
    let mut burned = Burned {
        oxygen: 0.0,
        lung_bed_oxygen: 0.0,
        carbon_dioxide: 0.0,
        lung_bed_carbon_dioxide: 0.0,
        energy: 0.0,
        to_core: 0.0,
        work: 0.0,
    };
    for column in columns {
        let moved = to[column.index()] - from[column.index()];
        let of = |gas: Species| count(body, *column, |_, species, _| species == gas);
        if burns(body, *column) {
            let (oxygen, made) = (-of(Species::Oxygen) * moved, of(Species::CarbonDioxide) * moved);
            burned.oxygen += oxygen;
            burned.carbon_dioxide += made;
            burned.energy += of(Species::Energy) * moved;
            if at_bed(body, *column, lung) {
                burned.lung_bed_oxygen += oxygen;
                burned.lung_bed_carbon_dioxide += made;
            }
        } else if pays_work(body, *column) {
            burned.work -= count(body, *column, bed_thermal) * moved;
        } else if count(body, *column, bed_thermal) < 0.0 {
            burned.to_core += count(body, *column, core_thermal) * moved;
        }
    }
    burned
}

fn pays_work(body: &CompiledBody, column: ColumnId) -> bool {
    ["stroke work", "wall work"].contains(&body.columns()[column.index()].origin().local)
}

#[test]
fn the_heat_report_credits_each_column_only_what_it_puts_in_the_core() {
    let body = whole_body().expect("compiles");
    let columns = every_column(&body);
    let core = site::<TissueSite>("body");
    let (mut works, mut transfers) = (0, 0);
    for column in &columns {
        let from = vec![0.0; columns.len()];
        let mut to = from.clone();
        to[column.index()] = 1.0;
        let credited = burned(&body, &columns, &from, &to).to_core;
        let into_core = count(&body, *column, |site, species, space| {
            site == core && species == Species::Energy && space == Space::Thermal
        });
        let from_beds = count(&body, *column, |site, species, space| {
            site.key().ends_with(" bed") && species == Species::Energy && space == Space::Thermal
        });
        if pays_work(&body, *column) {
            assert_eq!(credited, 0.0, "a work column is credited {credited} at the core");
            works += 1;
        } else if !burns(&body, *column) && from_beds < 0.0 {
            assert_eq!(
                credited, into_core,
                "a bed's heat column is credited {credited} for {into_core}"
            );
            transfers += 1;
        }
    }
    assert_eq!(works, 22);
    assert!(
        transfers >= 11,
        "{transfers} columns move a bed's heat to the core"
    );
}

/// One admission serves both: the first breaths, read over the first eleven seconds, and one whole beat's heat.
#[test]
fn the_whole_body_breathes_from_its_rest_and_its_beat_heat_report_counts_what_the_core_receives() {
    let start = Start::admitted();
    let mut watch = Watch::new(&start);
    let mut session = start.session();
    watch.see(&session);
    while session.record().accepted().time().nanoseconds() < anchor_moment().nanoseconds() + 11 * SECOND {
        let at = session.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds();
        session = until(session, at + 1);
        watch.see(&session);
        let after = session.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds();
        if at < SECOND && after >= SECOND {
            assert!(
                watch.moments.iter().any(|moment| moment.phase > 0.0),
                "no breath has started by the first second"
            );
        }
    }
    each_breath_swings_the_alveolar_gases_and_venous_return(&watch);
    the_whole_beat_heat_report_counts_what_the_core_receives_from_the_beds(&start);
}

/// Research 11's airway water, `V' (rho_sat - rho_insp)` with the expired gas at its vapour limit, at the packet's
/// values: `V'` the mixing exchange's alveolar ventilation, `M / 2` for two mixed exchanges in series, and each density
/// a share of the dry gas over water's vapour volume, `P_H2O / (P_B - P_H2O)` at the limit and `h P_r / (P_B - h P_r)`
/// from the room. The lining moves exactly that.
const MIXING: f64 = 3_240.0;
const WATER_VOLUME: f64 = 24_000.0;
const ROOM_WATER: f64 = 0.5 * 20.0 / (700.0 - 0.5 * 20.0);

fn the_lining_moves_research_11s_form_over_the_first_minute(start: &Start, minute: &Session<'_, Running>) {
    let body = &start.body;
    let columns = every_column(body);
    let lung = site::<CapillaryBed>("pulmonary bed");
    let water = |space| move |site, species, held| site == lung && species == Species::Water && held == space;
    let lining = (columns.iter().copied())
        .find(|column| {
            count(body, *column, water(Space::Interstitium)) == -1.0
                && count(body, *column, water(Space::Gas)) == 1.0
        })
        .expect("the lining");
    let to = minute.record().accepted().time().nanoseconds();
    let moved = extents(minute, &columns, to)[lining.index()];
    let seconds = (to - anchor_moment().nanoseconds()) as f64 / SECOND as f64;
    let form = MIXING / 2.0 * ((700.0 - DRY) / DRY - ROOM_WATER) / WATER_VOLUME * seconds;
    assert!(
        (moved - form).abs() <= 1e-9 * form,
        "over {seconds} s the lining moved {moved} water, research 11's form {form}"
    );
}

fn the_whole_beat_heat_report_counts_what_the_core_receives_from_the_beds(start: &Start) {
    let mut openings = Vec::new();
    let session = beats(start, start.session(), 3 * SECOND, |session, opened| {
        if let Some(closed) = opened {
            openings.push((session.record().accepted().time().nanoseconds(), closed));
        }
    });
    let onsets = onsets(&openings);
    assert!(onsets.len() >= 2, "no complete beat in {openings:?}");
    let body = &start.body;
    let columns = every_column(body);
    let from = extents(&session, &columns, onsets[0]);
    let to = extents(&session, &columns, onsets[1]);
    let core = site::<TissueSite>("body");
    let mut received = 0.0;
    for &column in &columns {
        let from_beds = count(body, column, |site, species, space| {
            site.key().ends_with(" bed") && species == Species::Energy && space == Space::Thermal
        });
        if from_beds < 0.0 {
            let into_core = count(body, column, |site, species, space| {
                site == core && species == Species::Energy && space == Space::Thermal
            });
            received += into_core * (to[column.index()] - from[column.index()]);
        }
    }
    assert!(received > 0.0);
    let reported = burned(body, &columns, &from, &to).to_core;
    assert!(
        (reported - received).abs() <= 1e-9 * received,
        "the whole-beat check reports {reported} of bed heat at the core, which received {received}"
    );
}

fn only_the_core_store_takes_the_beds_heat_but_its_burns_and_the_work_it_pays(
    body: &CompiledBody,
    columns: &[ColumnId],
) {
    let core = site::<TissueSite>("body");
    for column in (columns.iter()).filter(|column| !burns(body, **column) && !pays_work(body, **column)) {
        let thermal = |at: &dyn Fn(SiteId) -> bool| {
            count(body, *column, |site, species, space| {
                species == Species::Energy && space == Space::Thermal && at(site)
            })
        };
        let from_beds = thermal(&|site| site.key().ends_with(" bed"));
        if from_beds != 0.0 {
            let terms = body.columns()[column.index()].terms().len();
            assert_eq!(
                (terms, thermal(&|site| site == core)),
                (2, -from_beds),
                "{:?} takes a bed's heat elsewhere",
                body.columns()[column.index()].origin()
            );
        }
    }
}

fn outside_the_gas(start: &Start, session: &Session<'_, Running>, gas: Species) -> f64 {
    let view = session.record().accepted();
    (start.body.rows().iter())
        .filter(|row| matches!(row.kind(), RowKind::Amount { species, space, .. } if species == gas && space != Space::Gas))
        .map(|row| lead(view.value(row.id())))
        .sum()
}

/// Over whole beats the flow is the water the left ventricle loses while its valve stands open, because the
/// trapezoid of the valve's flow at the ends of steps reads it 0.6% high. The lung's lymph takes water and no gas from
/// the pulmonary bed to the central veins, so the pulmonary artery carries that water too, and the lung's own bed
/// takes its oxygen from its plasma before the pulmonary vein, so the arteries carry the other beds' burn. The flow
/// times each gas's content difference then meets what the ledger says the beds' burns used and made, to LEDGER, the
/// body's settling over nine beats, and to what the blood and tissues gained over them: nine beats are not whole
/// breaths, and the breath swings the gas the blood holds. The lung's exchange meets the burns and that gain.
fn the_flow_times_the_oxygen_content_difference_meets_the_oxygen_the_beds_burned_over_whole_beats(
    start: &Start,
) {
    let (muscle, lung_bed, payer) = (
        site::<CapillaryBed>("muscle bed"),
        site::<CapillaryBed>("pulmonary bed"),
        site::<CapillaryBed>("coronary bed"),
    );
    let paying_heat = RowKind::Amount {
        site: payer,
        species: Species::Energy,
        space: Space::Thermal,
    };
    let inflow = field(
        "CV",
        "circulation",
        Context::Edge {
            from: site::<VesselSegment>("systemic arteries"),
            to: muscle,
        },
    );
    let lymph = field(
        "FL",
        "lymph",
        Context::Edge {
            from: site::<CapillaryBed>("pulmonary bed"),
            to: site::<VesselSegment>("central veins"),
        },
    );
    let ventricle = RowKind::Amount {
        site: site::<VesselSegment>("left ventricle"),
        species: Species::Water,
        space: Space::Lumen,
    };
    struct Sample {
        at: u64,
        contents: [f64; 5],
        ejecting: bool,
        ventricle: f64,
        inflow: f64,
        stored: f64,
        heat: f64,
        paying: f64,
        stores: [f64; 2],
    }
    let mut samples: Vec<Sample> = Vec::new();
    let mut openings: Vec<(u64, u64)> = Vec::new();
    let session = until(start.session(), 50 * SECOND);
    let session = beats(start, session, 60 * SECOND, |session, opened| {
        let at = session.record().accepted().time().nanoseconds();
        if let Some(closed) = opened {
            openings.push((at, closed));
        }
        let read = |key, gas| concentration(start, session, key, gas);
        samples.push(Sample {
            at,
            contents: [
                read("systemic arteries", Species::Oxygen),
                read("pulmonary artery", Species::Oxygen),
                read("systemic arteries", Species::CarbonDioxide),
                read("pulmonary artery", Species::CarbonDioxide),
                value(start, session, lymph),
            ],
            ejecting: aortic_flow(start, session) > 0.0,
            ventricle: value(start, session, ventricle),
            inflow: value(start, session, inflow),
            stored: held_at(start, session, muscle, Species::Oxygen),
            heat: bed_heat(start, session),
            paying: value(start, session, paying_heat),
            stores: [Species::Oxygen, Species::CarbonDioxide].map(|gas| outside_the_gas(start, session, gas)),
        });
    });
    let onsets = onsets(&openings);
    let (first, last) = (onsets[0], onsets[onsets.len() - 1]);
    let held = (last - first) as f64 / 1e9;
    let (mut means, mut ejected) = ([0.0; 5], 0.0);
    for pair in samples.windows(2) {
        let (then, now) = (&pair[0], &pair[1]);
        if then.at >= first && now.at <= last {
            let length = (now.at - then.at) as f64 / 1e9;
            for (mean, (then, now)) in means.iter_mut().zip(then.contents.iter().zip(&now.contents)) {
                *mean += 0.5 * length * (then + now) / held;
            }
            if then.ejecting || now.ejecting {
                ejected += then.ventricle - now.ventricle;
            }
        }
    }
    let [
        arterial_oxygen,
        venous_oxygen,
        arterial_carbon_dioxide,
        venous_carbon_dioxide,
        lymph,
    ] = means;
    let (flow, returned) = (ejected / held, ejected / held + lymph);
    let body = &start.body;
    let columns = every_column(body);
    let sample = |at: u64| samples.iter().find(|sample| sample.at == at).expect("a sample");
    let (beginning, end) = (
        extents(&session, &columns, first),
        extents(&session, &columns, last),
    );
    let beats = burned(body, &columns, &beginning, &end);
    let (oxygen, carbon_dioxide) = (beats.oxygen / held, beats.carbon_dioxide / held);
    let systemic = (
        (beats.oxygen - beats.lung_bed_oxygen) / held,
        (beats.carbon_dioxide - beats.lung_bed_carbon_dioxide) / held,
    );
    let fick = [
        (
            "oxygen",
            flow * arterial_oxygen - returned * venous_oxygen,
            systemic.0,
        ),
        (
            "carbon dioxide",
            returned * venous_carbon_dioxide - flow * arterial_carbon_dioxide,
            systemic.1,
        ),
    ];
    let quotient = beats.carbon_dioxide / beats.oxygen;
    let stored = sample(last).heat - sample(first).heat;
    let at_onset = sample(first).inflow;
    let moved = (samples.iter())
        .filter(|sample| first < sample.at && sample.at <= onsets[1])
        .max_by(|a, b| {
            (a.inflow - at_onset)
                .abs()
                .total_cmp(&(b.inflow - at_onset).abs())
        })
        .expect("a step inside the first beat");
    let (mut used, mut net) = (0.0, 0.0);
    let after = extents(&session, &columns, moved.at);
    for column in &columns {
        let crossed = after[column.index()] - beginning[column.index()];
        let oxygen = count(body, *column, |site, species, _| {
            site == muscle && species == Species::Oxygen
        });
        if burns(body, *column) {
            used -= oxygen * crossed;
        } else {
            net += oxygen * crossed;
        }
    }
    let change = moved.stored - sample(first).stored;
    let gained = [0, 1].map(|gas| (sample(last).stores[gas] - sample(first).stores[gas]) / held);
    for ((gas, taken, rate), gained) in fick.into_iter().zip(gained) {
        assert!(
            (taken - rate).abs() <= LEDGER * rate + gained.abs(),
            "over {} beats flow {flow} and lymph {lymph} move {taken} of {gas} a second against {rate}, and the blood \
             and tissues gained {gained} a second",
            onsets.len() - 1
        );
    }
    assert!(
        (quotient - QUOTIENT).abs() <= 1e-12,
        "the burns made {quotient} carbon dioxide an oxygen"
    );
    let urine: f64 = (columns.iter())
        .filter(|column| body.columns()[column.index()].origin().local == "urine")
        .map(|column| {
            let leaving = count(body, *column, |_, species, _| species == Species::CarbonDioxide);
            leaving * (end[column.index()] - beginning[column.index()]) / held
        })
        .sum();
    for (gas, rate) in [
        (Species::Oxygen, oxygen + gained[0]),
        (Species::CarbonDioxide, carbon_dioxide - gained[1] + urine),
    ] {
        let lung: f64 = (columns.iter())
            .filter(|column| body.columns()[column.index()].origin().owner == Respiratory::CARD)
            .map(|column| {
                let at_lung = count(body, *column, |site, species, space| {
                    site == lung_bed && species == gas && space == Space::Plasma
                });
                ((end[column.index()] - beginning[column.index()]) * at_lung).abs() / held
            })
            .sum();
        assert!(
            (lung - rate).abs() <= LEDGER * rate,
            "the ledger's {gas:?} exchange at the lung over the beats is {lung} a second against {rate}"
        );
    }
    assert!(
        (moved.inflow - at_onset).abs() > 0.01 * at_onset.abs(),
        "the muscle bed's inflow held at {} over its window",
        moved.inflow
    );
    assert!(
        (net - (used + change)).abs() <= 1e-9 * (net.abs() + used.abs()),
        "the muscle bed took {net} of oxygen against {used} used and {change} stored"
    );
    only_the_core_store_takes_the_beds_heat_but_its_burns_and_the_work_it_pays(body, &columns);
    assert!(
        (beats.to_core - (beats.energy - beats.work - stored)).abs() <= 1e-9 * beats.energy,
        "the core store took {} of heat against {} of fuel energy less {} of work and {stored} the beds kept",
        beats.to_core,
        beats.energy,
        beats.work
    );
    the_coronary_beds_fuel_pays_the_stroke_work_over_whole_beats(body, &columns, [&beginning, &end], held);
    let kept = sample(last).paying - sample(first).paying;
    let (mut heat, rest) = (0.0, lead(body_start_guess(paying_heat).expect("a finite guess")));
    for pair in samples
        .windows(2)
        .filter(|pair| pair[0].at >= first && pair[1].at <= last)
    {
        heat += 0.5 * (pair[0].paying + pair[1].paying) * (pair[1].at - pair[0].at) as f64 / 1e9 / held;
    }
    assert!(
        (heat - rest).abs() <= AT_REST * rest,
        "the coronary bed holds {heat} of heat over whole beats against its rest of {rest}"
    );
    let paid = paid_by(body, &columns, payer, [&beginning, &end]);
    assert!(
        (paid.to_core - (paid.energy - paid.work - kept)).abs() <= 1e-6 * paid.energy,
        "the coronary bed's heat took {paid:?} and kept {kept}"
    );
}

#[derive(Debug)]
struct Paid {
    oxygen: f64,
    energy: f64,
    work: f64,
    to_core: f64,
}

fn paid_by(body: &CompiledBody, columns: &[ColumnId], bed: SiteId, [from, to]: [&[f64]; 2]) -> Paid {
    let mut paid = Paid {
        oxygen: 0.0,
        energy: 0.0,
        work: 0.0,
        to_core: 0.0,
    };
    let heat = |site, species, space| site == bed && species == Species::Energy && space == Space::Thermal;
    for column in columns {
        let moved = to[column.index()] - from[column.index()];
        if burns(body, *column) && at_bed(body, *column, bed) {
            paid.oxygen -= count(body, *column, |_, species, _| species == Species::Oxygen) * moved;
            paid.energy += count(body, *column, |_, species, _| species == Species::Energy) * moved;
        } else if pays_work(body, *column) {
            paid.work -= count(body, *column, heat) * moved;
        } else if !burns(body, *column) {
            paid.to_core -= count(body, *column, heat) * moved;
        }
    }
    paid
}

fn the_coronary_beds_fuel_pays_the_stroke_work_over_whole_beats(
    body: &CompiledBody,
    columns: &[ColumnId],
    window: [&[f64]; 2],
    held: f64,
) {
    let payer = site::<CapillaryBed>("coronary bed");
    let burned = paid_by(body, columns, payer, window).oxygen / held;
    assert!(
        (burned - PAYER_DEMAND).abs() <= PAYER_MEAN * PAYER_DEMAND,
        "the coronary bed burns {burned} oxygen a second over whole beats against {PAYER_DEMAND}"
    );
    for column in columns {
        let column = &body.columns()[column.index()];
        assert!(
            column
                .receipt()
                .is_none_or(|receipt| receipt.boundary() != "chemical energy"),
            "{:?} ends at chemical energy",
            column.origin()
        );
    }
    for (local, bed) in [("stroke work", "coronary bed"), ("wall work", "muscle bed")] {
        let from = site::<CapillaryBed>(bed);
        for column in columns
            .iter()
            .filter(|column| body.columns()[column.index()].origin().local == local)
        {
            let taken = count(body, *column, |site, species, space| {
                site == from && species == Species::Energy && space == Space::Thermal
            });
            assert_eq!(taken, -1.0, "a {local} column takes no heat from the {bed}");
        }
    }
}
const LEDGER: f64 = 1e-4;

const DEMANDS: [(&str, f64); 11] = [
    ("pulmonary bed", 1.0),
    ("gut bed", 20.0),
    ("muscle bed", 25.0),
    ("glomerular bed", 3.0),
    ("peritubular bed", 10.0),
    ("brain bed", 8.0),
    ("coronary bed", 8.0),
    ("liver bed", 15.0),
    ("fat bed", 2.0),
    ("skin bed", 2.0),
    ("bone bed", 2.0),
];
const AT_DEMAND: f64 = 2e-4;
/// The coronary bed's burn ripples with the heart's work within a beat, so it meets its stated demand only over whole
/// beats, to this share.
const PAYER_MEAN: f64 = 1e-3;
const PAYER_DEMAND: f64 = 8.0;
const AT_REST: f64 = 2e-4;
const AT_REST_LACTATE: f64 = 1e-9;

const SPECIATION_FORMS: f64 = 1e-9;
const SOLUBILITY: f64 = 0.001;
const HEME: f64 = 4.0;
const P50: f64 = 25.0;
const HILL: [f64; 3] = [2.5, 1.0, 10.0];
const STANDARD_ACIDITY: f64 = 7.5;
const BY_ACIDITY: [f64; 3] = [-1.0, 0.5, -0.25];
const STANDARD_CARBON_DIOXIDE: [f64; 2] = [30.0, 10.0];
const BY_CARBON_DIOXIDE: [f64; 2] = [0.1, 0.01];
const CARBON_DIOXIDE_SOLUBILITY: f64 = 0.02;
const PK: f64 = 6.0;
const STRONG_ION_OFFSET: f64 = 50.0;
const WEAK_ACIDS: [[f64; 3]; 2] = [[1.0, 10.0, 60.0], [1.0, 1.0, 6.0]];
const HEMOGLOBIN_CHARGE: [f64; 2] = [5.0, 30.0];
const CARBAMINO: [f64; 3] = [4.0, 10.0, 10.0];
const CHARGES: [(Species, f64); 9] = [
    (Species::Sodium, 1.0),
    (Species::Potassium, 1.0),
    (Species::Magnesium, 2.0),
    (Species::Calcium, 2.0),
    (Species::Chloride, -1.0),
    (Species::Lactate, -1.0),
    (Species::Ketones, -1.0),
    (Species::DietBase, -1.0),
    (Species::FixedAcid, -1.0),
];

fn ln(x: f64) -> f64 {
    let mut y = 0.0;
    for _ in 0..200 {
        y -= 1.0 - x / exp(y);
    }
    y
}

/// Each bed's four speciation rows at the admitted state against blood chemistry's forms at the packet's synthetic
/// values, read from the bed's plasma amounts and the other rows: bicarbonate from the charge balance, acidity from
/// `10^(pH - pK) = HCO3 / (C_T - HCO3 - carbamino)`, and the oxygen partial pressure and saturation from the curve.
fn each_beds_speciation_meets_its_forms_at_the_admitted_state(start: &Start) {
    let admitted = start.session();
    for (key, _) in DEMANDS {
        let bed = site::<CapillaryBed>(key);
        let row = |local| value(start, &admitted, field("BC", local, Context::Site(bed)));
        let held = |species| {
            let kind = RowKind::Amount {
                site: bed,
                species,
                space: Space::Plasma,
            };
            value(start, &admitted, kind)
        };
        let water = held(Species::Water);
        let per = |species| held(species) / water;
        let (pressure, saturation) = (row("oxygen partial pressure"), row("oxygen saturation"));
        let (bicarbonate, acidity) = (row("bicarbonate"), row("acidity"));
        let carrier = per(Species::Hemoglobin);
        let strong: f64 = CHARGES
            .iter()
            .map(|(species, charge)| charge * per(*species))
            .sum();
        let weak: f64 = (WEAK_ACIDS.iter())
            .map(|[held, slope, offset]| held * (slope * acidity - offset))
            .sum::<f64>()
            + carrier * (HEMOGLOBIN_CHARGE[0] * acidity - HEMOGLOBIN_CHARGE[1]);
        let dissolved = bicarbonate / exp((acidity - PK) * core::f64::consts::LN_10);
        let reach = CARBAMINO[1] + CARBAMINO[2] * saturation;
        let carbamino = CARBAMINO[0] * carrier * dissolved / (dissolved + reach);
        let left = per(Species::CarbonDioxide) - bicarbonate - carbamino;
        let d = acidity - STANDARD_ACIDITY;
        let u =
            (dissolved / CARBON_DIOXIDE_SOLUBILITY - STANDARD_CARBON_DIOXIDE[0]) / STANDARD_CARBON_DIOXIDE[1];
        let shifted = P50 * (1.0 + BY_ACIDITY[0] * d + BY_ACIDITY[1] * d * d + BY_ACIDITY[2] * d * d * d);
        let half = shifted * (1.0 + BY_CARBON_DIOXIDE[0] * u + BY_CARBON_DIOXIDE[1] * u * u);
        let hill = HILL[0] - HILL[1] * exp(-pressure / HILL[2]);
        let bound = (per(Species::Oxygen) - SOLUBILITY * pressure) / (HEME * carrier);
        let forms = [
            ("bicarbonate", bicarbonate, strong + STRONG_ION_OFFSET - weak),
            (
                "acidity",
                exp((acidity - PK) * core::f64::consts::LN_10),
                bicarbonate / left,
            ),
            (
                "oxygen partial pressure",
                pressure,
                half * exp(ln(bound / (1.0 - bound)) / hill),
            ),
            (
                "oxygen saturation",
                saturation,
                1.0 / (1.0 + exp(-hill * ln(pressure / half))),
            ),
        ];
        for (local, held, form) in forms {
            assert!(
                (held - form).abs() <= SPECIATION_FORMS * form.abs(),
                "the {key} admits its {local} at {held} against its form's {form}"
            );
        }
    }
}

fn each_bed_burns_its_demand_and_rests_where_the_packet_puts_it(start: &Start) {
    let body = &start.body;
    let columns = every_column(body);
    let stepped = until(start.session(), 1);
    let view = stepped.conservation();
    let head = view.head().expect("a step");
    let interval = head.interval();
    let seconds = interval.length_ns() as f64 / 1e9;
    let gained = |curve: Curve<'_>| {
        let ends = curve.on(interval).expect("the head's interval");
        lead(ends[1]) - lead(ends[0])
    };
    let admitted = start.session();
    for (key, demand) in DEMANDS {
        let bed = site::<CapillaryBed>(key);
        let used: f64 = (columns.iter())
            .filter(|column| burns(body, **column) && at_bed(body, **column, bed))
            .map(|column| {
                let (forward, reverse) = head.exchange(*column);
                let oxygen = count(body, *column, |_, species, _| species == Species::Oxygen);
                -oxygen * (gained(forward) - gained(reverse)) / seconds
            })
            .sum();
        assert!(
            key == "coronary bed" || (used - demand).abs() <= AT_DEMAND * demand,
            "the {key} burns {used} oxygen a second against its demand of {demand}"
        );
        for (species, space) in [
            (Species::Glucose, Space::Cells),
            (Species::FattyAcids, Space::Cells),
            (Species::Water, Space::Cells),
            (Species::Energy, Space::Thermal),
        ] {
            let kind = RowKind::Amount {
                site: bed,
                species,
                space,
            };
            let held = value(start, &admitted, kind);
            let rest = lead(body_start_guess(kind).expect("a finite guess"));
            let ripples = key == "coronary bed" && species == Species::Energy;
            assert!(
                ripples || (held - rest).abs() <= AT_REST * rest.abs(),
                "the {key} admits {held} of {species:?} in {space:?} against its rest of {rest}"
            );
        }
        let lactate = RowKind::Amount {
            site: bed,
            species: Species::Lactate,
            space: Space::Cells,
        };
        let debt = field("MT", "oxygen_debt_per_bed", Context::Site(bed));
        let silent = matches!(key, "peritubular bed" | "pulmonary bed");
        let (share, half) = (if silent { 0.0 } else { 0.5 }, demand * 1e-3);
        for (when, session) in [("admitted", &admitted), ("a step on", &stepped)] {
            let (held, owed) = (value(start, session, lactate), value(start, session, debt));
            assert!(
                held.abs() <= AT_REST_LACTATE && owed.abs() <= 3.0 * AT_REST_LACTATE,
                "the {key} {when} holds {held} lactate in its cells and owes {owed} oxygen"
            );
            let whole = owed + half;
            let kept = (half + (owed - share * owed)) / whole;
            assert!(
                (kept - 1.0).abs() <= 4.0 * f64::EPSILON,
                "the {key}'s inflow {when} keeps {kept} of its resistance at a debt of {owed}"
            );
        }
    }
}

struct Ticks(Cell<u64>);
impl WallClock for Ticks {
    fn elapsed_ns(&self) -> Option<u64> {
        self.0.set(self.0.get() + 90 * MILLISECOND);
        Some(self.0.get())
    }
    fn source(&self) -> ClockSource {
        ClockSource::Scripted
    }
}

/// Its own solve, so the run test's mutant rows never pay for the gates.
#[test]
fn the_whole_body_passes_every_replay_gate() {
    let case = BodyCase::new().expect("solved");
    let step = 10 * MILLISECOND;
    let (first, second) = (
        move || ng_solver::Core::new(step),
        move || ng_solver::Core::new(step / 2),
    );
    let finest = case.body().longest_step_ns() / 16;
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let reference = move || {
        ReferenceIntegrator::new(finest)
            .with_scales(ErrorScales::uniform(scale(1e9), scale(1.0)).expect("positive"))
            .with_floor(NonZeroU64::new(100_000).expect("a floor"))
    };
    let clock = Ticks(Cell::new(0));
    let plan = ReplayPlan {
        clock: &clock,
        simulated_per_wall: 1,
        chunks: vec![70 * MILLISECOND, 130 * MILLISECOND + 1],
        resume_every: 1000,
        chance: Some(ChancePlan {
            second_policy: &second,
            reference: &reference,
            event_tolerance_ns: 100_000,
        }),
    };
    let evidence = replay_gates(&case, &first, &plan).expect("every gate");
    let store = MemoryStore::default();
    let core = first();
    let withheld = case.admit_withheld(&core, &store).expect("admitted");
    assert_eq!(withheld.record().identity().digest(), evidence.run());
    assert_eq!(hex(withheld.record().identity().packet().digest()), PACKET);
    assert_eq!(hex(withheld.record().seal().digest()), ADMITTED_SEAL);
    let fence = anchor_moment().nanoseconds() + SECOND;
    let events = case.events().expect("the inputs");
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|event| event.at().nanoseconds() == fence));
    let stream = LawName {
        owner: Musculoskeletal::CARD,
        local: "stir",
    };
    let breathing = LawName {
        owner: Nervous::CARD,
        local: "breath dice",
    };
    for gate in [
        ReplayGate::Pace,
        ReplayGate::Resume,
        ReplayGate::LateAdmission,
        ReplayGate::Rewind,
        ReplayGate::ExtraEvaluation,
        ReplayGate::StepPolicy,
        ReplayGate::NonPerturbation,
    ] {
        let of = evidence.of(gate);
        assert_eq!((of.gate(), of.run()), (gate, evidence.run()));
        assert!(of.runs().len() >= 2, "{gate:?} compared {} runs", of.runs().len());
        let [breath, stir] = of.fired() else {
            panic!("{gate:?} fired {:?}", of.fired());
        };
        assert_eq!(
            [
                (breath.stream(), breath.ordinal()),
                (stir.stream(), stir.ordinal())
            ],
            [(breathing, 0), (stream, 0)],
            "{gate:?}"
        );
        assert!(
            breath.at().nanoseconds() < stir.at().nanoseconds() && stir.at().nanoseconds() < fence,
            "{gate:?} started the breath at {:?} and stirred at {:?}",
            breath.at(),
            stir.at()
        );
    }
    assert_eq!(evidence.of(ReplayGate::Pace).clock(), Some(ClockSource::Scripted));
    assert_eq!(evidence.of(ReplayGate::StepPolicy).tolerance_ns(), Some(100_000));
}

#[test]
fn the_whole_body_admits_runs_its_first_minute_and_holds_its_pins() {
    let start = Start::admitted();
    each_bed_burns_its_demand_and_rests_where_the_packet_puts_it(&start);
    each_beds_speciation_meets_its_forms_at_the_admitted_state(&start);
    let before = start.core.solves.get();
    let (minute, counted, watch, branch) = the_first_minute_moves_no_held_pool_and_saves_a_branch(&start);
    let counts = Counts {
        rows: start.body.rows().len(),
        jacobian_nonzeros: start.body.sparsity().unknowns().len(),
        steps: minute.record().revision(),
        solves: start.core.solves.get() - before,
    };
    let mut report = each_breath_swings_the_alveolar_gases_and_venous_return(&watch);
    report += &the_pulmonary_plasma_stays_under_capacity_at_the_highest_alveolar_oxygen(&watch);
    report += &the_work_of_breathing_follows_the_pump_from_the_stated_mean(&watch);
    report += &the_alveolar_water_share_over_whole_breaths(&watch);
    the_body_admits_at_06_55(&start);
    report += &the_drive_scale_at_admission_and_over_the_first_beat(&start);
    the_first_second_of_the_body_on_ng_solver_stays_where_it_was(&start);
    report += &the_certificate_passes_on_ten_second_windows_over_the_first_minute_with_its_held_support(
        &start, &minute, &watch,
    );
    {
        // The held run's own admission and store go once its tests are done, before the sleeping branch runs on.
        let held = Start::held();
        report += &the_held_body_admits_where_the_controlled_body_does(&start, &held);
        let (held_minute, held_watch) = the_held_minute(&held);
        report += &the_held_runs_drive_scale_is_its_rest_at_every_step(&held, &held_watch);
        report += &the_held_runs_breathing_books(&held, &held_minute, &held_watch);
        report += &every_link_moves_its_reader_the_way_the_loop_needs(&start, &watch, &held_watch);
    }
    let core = ng_solver::Core::new(LONGEST_STEP);
    let (asleep, mut asleep_watch, head) = the_sleeping_branch(&start, &branch, &core, &watch);
    report += &sleep_lowers_ventilation_and_raises_arterial_carbon_dioxide(&asleep_watch, &watch, head);
    report += &the_sleeping_branchs_breathing_books(&start, &asleep, &asleep_watch);
    let (reference, woken) = the_sleeping_branch_wakes_and_runs_on(&start, asleep, &mut asleep_watch, &watch);
    report += &woken;
    report += &the_reference_run_passes_the_bodys_policy_and_rule_holds_each_figure_to_it(
        &start,
        &reference,
        &asleep_watch,
    );
    drop(reference);
    the_lining_moves_research_11s_form_over_the_first_minute(&start, &minute);
    report += &each_gas_closes_on_the_airway_the_room_and_the_membrane_over_two_whole_breaths(&start, minute);
    expect![[r#"
        before the first breath edge: alveolar oxygen 101.294, carbon dioxide 28.356, venous return 77.29
        breath from 5.006 s: alveolar oxygen 100.752 swinging 3.062, carbon dioxide 28.705 swinging 1.990, venous return's beat means 74.07 to 84.51 around 77.81, tidal volume 11021.5
        breath from 10.037 s: alveolar oxygen 100.410 swinging 3.057, carbon dioxide 28.844 swinging 1.931, venous return's beat means 73.96 to 84.47 around 77.61, tidal volume 11026.1
        breath from 15.074 s: alveolar oxygen 100.133 swinging 3.150, carbon dioxide 28.907 swinging 1.945, venous return's beat means 73.98 to 84.41 around 77.77, tidal volume 11016.8
        breath from 20.091 s: alveolar oxygen 99.916 swinging 3.201, carbon dioxide 28.930 swinging 1.965, venous return's beat means 74.00 to 84.39 around 78.08, tidal volume 11002.1
        breath from 25.079 s: alveolar oxygen 99.749 swinging 3.214, carbon dioxide 28.935 swinging 1.961, venous return's beat means 74.01 to 84.38 around 78.36, tidal volume 10988.0
        breath from 30.039 s: alveolar oxygen 99.621 swinging 3.186, carbon dioxide 28.932 swinging 1.938, venous return's beat means 74.12 to 84.39 around 78.39, tidal volume 10975.2
        breath from 34.975 s: alveolar oxygen 99.519 swinging 3.089, carbon dioxide 28.928 swinging 1.909, venous return's beat means 74.10 to 84.41 around 78.27, tidal volume 10958.6
        breath from 39.892 s: alveolar oxygen 99.435 swinging 3.017, carbon dioxide 28.924 swinging 1.920, venous return's beat means 74.07 to 84.33 around 78.13, tidal volume 10949.3
        breath from 44.795 s: alveolar oxygen 99.366 swinging 3.019, carbon dioxide 28.922 swinging 1.969, venous return's beat means 74.07 to 83.91 around 77.64, tidal volume 10942.6
        breath from 49.687 s: alveolar oxygen 99.308 swinging 3.131, carbon dioxide 28.919 swinging 2.015, venous return's beat means 74.08 to 83.26 around 76.74, tidal volume 10938.5
        breath from 54.570 s: alveolar oxygen 99.259 swinging 3.238, carbon dioxide 28.918 swinging 2.047, venous return's beat means 74.10 to 83.61 around 75.80, tidal volume 10935.4
        highest alveolar oxygen 102.524 (4.101 times P50) at 2.024 s: plasma oxygen pressure 100.886 (4.035 times P50), bound 0.971026 of capacity
        work of breathing: stated 5331.3, over two whole breaths from 49.687 s 5367.6
        alveolar water over whole breaths from 20.091 s: 1.02166 of the saturated vapour share
        drive scale at admission 1.007406222041; over the first beat, to 0.998962223 s, 0.999997735913 by the trapezoid and 1.000013756971 from the set points
        Energy: worst 1.5287e3, from admission 7.0243e0, steady 1.5296e3, inside a breath 7.6119e2
        Element(Carbon): worst 6.6534e1, from admission 3.8811e1, steady 5.0087e1, inside a breath 2.0096e2
        Element(Oxygen): worst 1.8721e3, from admission 1.8458e3, steady 1.0175e3, inside a breath 4.4957e3
        Element(Hydrogen): worst 3.1838e-5, from admission 7.8286e-6, steady 3.1872e-5, inside a breath 1.5646e-5
        Of(NitrogenGas): worst 3.4950e3, from admission 3.4590e3, steady 2.0239e3, inside a breath 8.6200e3
        Of(Water): worst 1.9184e-2, from admission 1.9184e-2, steady 1.5519e-2, inside a breath 2.0007e-2
        Of(Vasopressin): worst 3.4251e-3, from admission 5.4883e-4, steady 3.2924e-3, inside a breath 2.0938e-2
        controlled admission: aortic valve shuts 0.285464778 s into the beat, leg veins 0.285689472 s
        held admission: aortic valve shuts 0.285464778 s into the beat, leg veins 0.285689472 s
        largest difference over every row but the drive scale 8.052e-10, 8.260e-3 of admission's tolerance, at Some(Amount { site: SiteId { key: "systemic arteries", class: VesselSegment }, space: Lumen, species: Vasopressin }); over the 3 calibrated parameters 6.651e-12, 2.431e-14 of its value and 2.345e-4 of admission's tolerance, at Some(Parameter { key: ParameterKey { key: "rn.sodium-capacity" }, binding: Edge { from: SiteId { key: "renal tubule", class: VesselSegment }, to: SiteId { key: "peritubular bed", class: CapillaryBed } } })
        held run: the drive scale is its rest's bits at all 11680 accepted steps
        held Energy: worst 1.4583e3, from admission 8.4422e0, steady 1.4594e3, inside a breath 7.3025e2
        held Element(Carbon): worst 4.6100e1, from admission 3.8271e1, steady 4.3161e1, inside a breath 2.0145e2
        held Element(Oxygen): worst 1.8508e3, from admission 1.8470e3, steady 5.2877e1, inside a breath 4.4965e3
        held Element(Hydrogen): worst 3.3523e-5, from admission 7.7904e-6, steady 3.3547e-5, inside a breath 1.6847e-5
        held Of(NitrogenGas): worst 3.4629e3, from admission 3.4604e3, steady 7.7951e1, inside a breath 8.6213e3
        held Of(Water): worst 1.9155e-2, from admission 1.9154e-2, steady 1.1407e-2, inside a breath 2.0097e-2
        held Of(Vasopressin): worst 3.0454e-3, from admission 3.9430e-4, steady 2.0234e-3, inside a breath 2.0981e-2
        sign walk over whole breaths from 30 s to 60 s: 6 controlled breaths, 5 held
        under-ventilation to the central sensor: acidity -3.120e-4 from its set point
        under-ventilation to the peripheral sensor: x_p +1.649e-2 from its set point
        the sensors to the drive scale: 1.020160, its central arm +1.847e-3 and peripheral arm +1.831e-2
        the drive scale to the breath: 4.9012 s a breath against the period 5.0000 s, held 5.0001 s
        the breath to ventilation: 2234.14 a second against held 2201.44
        ventilation to alveolar carbon dioxide: 28.9237 against held 28.9602; alveolar oxygen 99.4185 against held 99.3669
        alveolar to arterial carbon dioxide: over the clock from 30 s to 60 s, 29.9997 s and 29.9986 s, 30.0006 against held 30.0094, by -8.739e-3 where the cut breaths reach 2.618e-3; from 25 s by -6.643e-3, from 35 s by -1.141e-2; arterial oxygen pressure 90.1228 against held 90.1065, saturation 0.958942 against held 0.958907
        asleep from 30 s, branched at 29.991136 s; over whole breaths from 40 s to 60 s, 3 breaths asleep and 3 awake
        asleep the drive scale 0.873901 and the breath 5.7215 s, awake 1.023836 and 4.8836 s
        ventilation 1982.52 a second asleep against 2239.92 awake, by -2.574e2 where the cut pieces reach 5.728e0; from 35 s by -2.627e2, from 45 s by -2.535e2
        arterial carbon dioxide over the clock, 19.9994 s and 19.9995 s, 30.1595 asleep against 30.0140 awake, by +1.456e-1 where the cut breaths reach 1.103e-2; from 35 s by +1.173e-1, from 45 s by +1.774e-1
        alveolar carbon dioxide 29.3352 asleep against 28.9195 awake
        asleep Energy: worst 3.4862e3 from 50.001 s to 60.001 s; from 30 s any ten seconds 3.4873e3, inside a breath 2.0459e3
        asleep Element(Carbon): worst 3.3745e2 from 34.193 s to 44.193 s; from 30 s any ten seconds 3.3751e2, inside a breath 2.1640e2
        asleep Element(Oxygen): worst 4.2660e3 from 31.808 s to 41.808 s; from 30 s any ten seconds 4.2625e3, inside a breath 4.6606e3
        asleep Element(Hydrogen): worst 5.3436e-5 from 49.832 s to 59.832 s; from 30 s any ten seconds 5.3468e-5, inside a breath 3.1299e-5
        asleep Of(NitrogenGas): worst 8.3456e3 from 34.110 s to 44.110 s; from 30 s any ten seconds 8.3450e3, inside a breath 8.9322e3
        asleep Of(Water): worst 1.9347e-2 from 37.572 s to 47.572 s; from 30 s any ten seconds 1.9305e-2, inside a breath 2.1802e-2
        asleep Of(Vasopressin): worst 1.4045e-2 from 35.356 s to 45.356 s; from 30 s any ten seconds 1.4187e-2, inside a breath 2.5728e-2
        awake again from 150 s to 300 s; over whole breaths from 160 s, 29 breaths, the drive scale 1.056587 and the breath 4.7322 s, against 1.023836 and 4.8836 s awake and 0.873901 and 5.7215 s asleep
        reference run: 59 breath edges from 5.006 s to 298.949 s, 56 windows between edges, the widest gap 5.8282 s to 35.874 s, the widest stretch 5.8282 s to 35.874 s
        Energy: ten seconds 4.6473e3 from 80.509 s, inside a breath 2.5697e3; between edges 4.6118e2 a second from 81.305 s to 92.444 s, before the settling time 1.3626e2 from 15.074 s to 25.079 s
        Element(Carbon): ten seconds 3.3745e2 from 34.193 s, inside a breath 2.1640e2; between edges 1.0729e1 a second from 30.046 s to 41.678 s, before the settling time 4.8577e0 from 5.006 s to 15.074 s
        Element(Oxygen): ten seconds 4.2660e3 from 31.808 s, inside a breath 4.6606e3; between edges 1.8695e1 a second from 147.671 s to 161.716 s, before the settling time 3.4607e0 from 5.006 s to 15.074 s
        Element(Hydrogen): ten seconds 6.9592e-5 from 65.552 s, inside a breath 3.8705e-5; between edges 6.8165e-6 a second from 64.491 s to 75.718 s, before the settling time 8.6078e-7 from 5.006 s to 15.074 s
        Element(Hydrogen) at a payer: ten seconds 1.9341e-1 from 158.590 s; between edges 1.5782e-2 a second from 157.159 s to 170.908 s, before the settling time 9.2136e-3 from 5.006 s to 15.074 s
        Of(NitrogenGas): ten seconds 8.3456e3 from 34.110 s, inside a breath 8.9322e3; between edges 5.9562e0 a second from 35.874 s to 47.442 s, before the settling time 3.6760e0 from 15.074 s to 25.079 s
        Of(Water): ten seconds 2.0430e-2 from 289.751 s, inside a breath 2.1843e-2; between edges 1.5622e-3 a second from 284.657 s to 298.949 s, before the settling time 1.1556e-3 from 15.074 s to 25.079 s
        Of(Sodium): ten seconds 8.2853e-4 from 290.001 s; between edges 7.8217e-5 a second from 284.657 s to 298.949 s, before the settling time 4.3812e-6 from 15.074 s to 25.079 s
        Of(Chloride): ten seconds 8.6996e-4 from 290.001 s; between edges 8.2127e-5 a second from 284.657 s to 298.949 s, before the settling time 4.6002e-6 from 15.074 s to 25.079 s
        Of(Potassium): ten seconds 4.1427e-5 from 290.001 s; between edges 3.9108e-6 a second from 284.657 s to 298.949 s, before the settling time 2.1906e-7 from 15.074 s to 25.079 s
        Of(Vasopressin): ten seconds 1.4045e-2 from 35.356 s, inside a breath 2.5728e-2; between edges 8.3907e-4 a second from 142.172 s to 152.617 s, before the settling time 1.7876e-4 from 15.074 s to 25.079 s
        two breaths from 64.316 s to 74.045 s
        Oxygen: in 4560.2375, out 3637.1692, across 899.2353, along 922.1879, gained 2.3833e1
        CarbonDioxide: in 8.6862, out 679.7044, across -667.3050, along -671.3639, gained -3.7132e0
        NitrogenGas: in 17146.4932, out 17140.9561, across 0.0000, along -0.9528, gained 5.5371e0
        Water: in 0.0000, out 0.0407, across -0.0410, along -0.0407, gained 3.1313e-4
    "#]].assert_eq(&report);
    let mut minute = format!(
        "steps {}, solves {}: ordinary {}, landing {}\n",
        counts.steps,
        counts.solves,
        counted.attempts,
        counts.solves - counted.attempts
    );
    for (reason, refusals) in &counted.refused {
        writeln!(minute, "refused {refusals} {reason}").expect("a string takes it");
    }
    expect![[r#"
        steps 11666, solves 14397: ordinary 12343, landing 2054
        refused 76 Accuracy Algebraic(Slot { law: LawName { owner: "CV", local: "blood pressure" }, at: Site(SiteId { key: "left ventricle", class: VesselSegment }) })
        refused 243 Accuracy Memory(Slot { law: LawName { owner: "CV", local: "valves" }, at: Edge { from: SiteId { key: "left atrium", class: VesselSegment }, to: SiteId { key: "left ventricle", class: VesselSegment } } })
        refused 297 Accuracy Memory(Slot { law: LawName { owner: "CV", local: "valves" }, at: Edge { from: SiteId { key: "left ventricle", class: VesselSegment }, to: SiteId { key: "systemic arteries", class: VesselSegment } } })
        refused 61 IterationLimit
    "#]]
    .assert_eq(&minute);
    assert_eq!(over_ceilings(counts, CEILINGS), Vec::<&str>::new(), "{counts:?}");
    a_water_bolus_dilutes_the_blood_and_leaves_its_gases_where_they_were(&start);
    the_flow_times_the_oxygen_content_difference_meets_the_oxygen_the_beds_burned_over_whole_beats(&start);
}
