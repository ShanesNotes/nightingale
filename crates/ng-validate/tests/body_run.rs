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
const PACKET: &str = "23b5753243a212d29fbf2f6a1839ab10abe43c0e5595fa54280963fa43d034b5";
const ADMITTED_SEAL: &str = "2e7571c51824d489de808c044b5466f8243568968212eb9695d7616307710724";

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
}
impl Start {
    fn admitted() -> Self {
        let body = whole_body().expect("compiled");
        let core = Counted {
            core: ng_solver::Core::new(10 * MILLISECOND),
            solves: Cell::new(0),
        };
        let links = MemoryStore::default();
        let (saved, identity) = {
            let (packet, lens) = (
                body_packet(&body).expect("a packet"),
                body_lens(&body).expect("a lens"),
            );
            let port = BoundaryPort::new(Vec::new()).expect("no event");
            let admitted = admit_at(&body, packet, port, &core, &lens, anchor_moment(), &links);
            let session = admitted
                .unwrap_or_else(|refused| panic!("admission refused: {refused:?}"))
                .session;
            (
                session.checkpoint().expect("a checkpoint"),
                session.record().identity().clone(),
            )
        };
        Self {
            body,
            core,
            saved,
            identity,
            links,
        }
    }
    fn session(&self) -> Session<'_, Running> {
        let Self {
            body,
            core,
            saved,
            identity,
            links,
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
const BREATHING_BOOKS: [Book; 6] = [
    Book::Of(Species::Vasopressin),
    Book::Element(Element::Oxygen),
    Book::Element(Element::Carbon),
    Book::Element(Element::Hydrogen),
    Book::Of(Species::NitrogenGas),
    Book::Energy,
];
const DRY: f64 = 650.0;

/// The most an inventory moved over the ten seconds from admission, over any ten seconds from twenty seconds on, and
/// inside one whole breath from twenty seconds on.
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

fn paid_by_the_coronary_bed(body: &CompiledBody, terms: &[(RowId, i64)]) -> bool {
    terms.iter().all(|(row, _)| {
        matches!(body.rows()[row.index()].kind(), RowKind::Amount { site, .. } if site.key() == "coronary bed")
    })
}

struct Moment {
    at: u64,
    phase: f64,
    venous: f64,
    aortic: f64,
    alveolar: [f64; 2],
    plasma: [f64; 3],
    inventories: Vec<f64>,
}

struct Watch {
    phase: RowId,
    venous: RowId,
    aortic: RowId,
    lung: [RowId; 3],
    plasma: [RowId; 4],
    books: Vec<(Book, Vec<(RowId, i64)>)>,
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
            .find(|row| matches!(row.kind(), RowKind::Memory(slot) if slot.law.owner == "RS" && slot.law.local == "breath phase"))
            .expect("the breath's phase")
            .id();
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
            .filter(|combination| !paid_by_the_coronary_bed(body, combination.terms()))
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
            plasma: [
                plasma(Species::Oxygen),
                plasma(Species::Hemoglobin),
                plasma(Species::Water),
                find(field("BC", "oxygen partial pressure", Context::Site(lung))),
            ],
            books,
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
            plasma: [oxygen / water, carrier / water, pressure],
            inventories,
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
    fn figures(&self) -> Vec<(Book, Figures)> {
        let steady = (self.moments.iter())
            .position(|moment| moment.at >= 20 * SECOND)
            .expect("twenty seconds");
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
            "breath from {at:.3} s: alveolar oxygen {:.3} swinging {oxygen:.3}, carbon dioxide {:.3} swinging {carbon_dioxide:.3}, venous return's beat means {low:.2} to {high:.2} around {mean:.2}",
            means[0], means[1]
        )
        .expect("a string takes it");
    }
    report
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

fn no_held_pool_moves_at_admission_or_over_the_first_minute(
    start: &Start,
) -> (Session<'_, Running>, Minute, Watch) {
    let pools = start.held_pools();
    assert_eq!(pools.len(), 319);
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
    for mark in 1..=6 {
        session = counted_until(start, session, 10 * mark * SECOND, &mut minute, &mut watch);
        for ((_, kind), (now, then)) in pools.iter().zip(amounts(&session).iter().zip(&admitted)) {
            assert_eq!(now, then, "{kind:?} moved by {} s", 10 * mark);
        }
    }
    (session, minute, watch)
}

fn the_certificate_passes_on_ten_second_windows_over_the_first_minute_with_its_held_support(
    start: &Start,
    minute: &Session<'_, Running>,
    watch: &Watch,
) -> String {
    let reached = minute.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds();
    let window = NonZeroU64::new(10 * SECOND).expect("a window");
    let windows = SlidingWindows::new(window, reached).expect("windows");
    let policy = body_certificate(&start.body, windows).expect("a policy");
    let certificate = certify(&minute.conservation(), &policy).unwrap_or_else(|failed| {
        panic!(
            "{:?} {:?} {:?}",
            failed.check(),
            failed.quantity(),
            failed.window()
        )
    });
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
        steps 11671, quantities 399, held routes 89
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
    let payer = |terms: &[(RowId, i64)]| paid_by_the_coronary_bed(&start.body, terms);
    let rule = [
        (Book::Energy, false, 2e3, 3e3),
        (Book::Element(Element::Carbon), false, 4e2, 5e2),
        (Book::Element(Element::Oxygen), false, 8e3, 9e3),
        (Book::Element(Element::Hydrogen), false, 1e-4, 2e-4),
        (Book::Element(Element::Hydrogen), true, 0.1, 0.2),
        (Book::Of(Species::NitrogenGas), false, 1e4, 2e4),
        (Book::Of(Species::Water), false, 0.03, 0.04),
        (Book::Of(Species::Sodium), false, 3e-4, 4e-4),
        (Book::Of(Species::Chloride), false, 3e-4, 4e-4),
        (Book::Of(Species::Potassium), false, 1e-5, 2e-5),
        (Book::Of(Species::Vasopressin), false, 3e-2, 4e-2),
    ];
    let figures = watch.figures();
    let mut report = String::new();
    for (book, at_payer, below, bound) in rule {
        let worst = (certificate.results().iter())
            .filter(|result| {
                let combination = &start.body.conserved_basis().combinations()[result.quantity().index()];
                combination.book() == book && payer(combination.terms()) == at_payer
            })
            .map(|result| lead(result.worst_drift()).abs())
            .fold(0.0, |most, x| if x > most { x } else { most });
        let breathing = figures.iter().find(|(of, _)| *of == book && !at_payer);
        if let Some((
            _,
            Figures {
                from_admission: start,
                steady,
                inside_a_breath: swing,
            },
        )) = breathing
        {
            writeln!(
                report,
                "{book:?}: worst {worst:.4e}, from admission {start:.4e}, steady {steady:.4e}, inside a breath {swing:.4e}"
            )
            .expect("a string takes it");
        }
        let figure = breathing.map_or(worst, |(_, figures)| larger(worst, figures.inside_a_breath));
        assert!(
            below < 2.0 * figure && 2.0 * figure <= bound,
            "{book:?}: twice the worst ten-second change or the most a breath moves it, {}, rounds up to a figure other than {bound}",
            2.0 * figure
        );
    }
    report
}

fn to_the_breaths_end<'s>(start: &Start, mut session: Session<'s, Running>) -> Session<'s, Running> {
    let phase = (start.body.rows().iter())
        .find(|row| matches!(row.kind(), RowKind::Memory(slot) if slot.law.owner == "RS" && slot.law.local == "breath phase"))
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
    let gases = [Species::Oxygen, Species::CarbonDioxide, Species::NitrogenGas];
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
        packet 23b5753243a212d29fbf2f6a1839ab10abe43c0e5595fa54280963fa43d034b5
        seal 440eaab02b1da7a6660cc216bc9fab6f29997032a454748fff759bf2db0e67a0
        steps 198
        viscosity over its reference at the start 1.000234140
    "#]]
    .assert_eq(&pin);
    expect![[r#"
         100.403446 ms  arterial pressure 134.601269  right atrial pressure 2.843641  portal vein pressure 6.244676  leg vein pressure 3.539214  heart rate 1.063931  sympathetic traffic 0.786553  filtration 1.669107  urine 0.020000  wall power 24560.983051  aortic flow 0.000000
         201.433366 ms  arterial pressure 126.985705  right atrial pressure 3.291840  portal vein pressure 6.370534  leg vein pressure 3.684452  heart rate 1.081205  sympathetic traffic 0.086030  filtration 2.106929  urine 0.020000  wall power 15556.203956  aortic flow 39.628797
         302.696199 ms  arterial pressure 118.697846  right atrial pressure 3.714403  portal vein pressure 6.150610  leg vein pressure 3.872230  heart rate 1.047429  sympathetic traffic 0.016489  filtration 2.340250  urine 0.020000  wall power 38248.362634  aortic flow 0.000000
         402.779726 ms  arterial pressure 109.431891  right atrial pressure 3.229225  portal vein pressure 5.902154  leg vein pressure 4.229787  heart rate 1.004896  sympathetic traffic 0.012386  filtration 2.404005  urine 0.020000  wall power 14035.650432  aortic flow 0.000000
         504.160253 ms  arterial pressure 100.945499  right atrial pressure 1.494082  portal vein pressure 5.648199  leg vein pressure 3.930286  heart rate 0.970460  sympathetic traffic 0.023279  filtration 2.342866  urine 0.020000  wall power 12009.296776  aortic flow 0.000000
         608.059522 ms  arterial pressure 93.031450  right atrial pressure 0.901488  portal vein pressure 5.356791  leg vein pressure 3.237583  heart rate 0.947724  sympathetic traffic 0.075377  filtration 2.188546  urine 0.020000  wall power 8628.863519  aortic flow 0.000000
         708.059522 ms  arterial pressure 86.043565  right atrial pressure 0.826700  portal vein pressure 5.135960  leg vein pressure 2.666845  heart rate 0.940826  sympathetic traffic 0.263694  filtration 1.993062  urine 0.020000  wall power 6296.742215  aortic flow 0.000000
         808.059522 ms  arterial pressure 79.606036  right atrial pressure 0.997983  portal vein pressure 5.128166  leg vein pressure 2.425356  heart rate 0.952742  sympathetic traffic 0.644069  filtration 1.780424  urine 0.020000  wall power 4911.355677  aortic flow 0.000000
         908.059522 ms  arterial pressure 73.690079  right atrial pressure 1.251502  portal vein pressure 5.405636  leg vein pressure 2.522915  heart rate 0.983060  sympathetic traffic 0.905286  filtration 1.566187  urine 0.020000  wall power 3944.365530  aortic flow 0.000000
        1000.382092 ms  arterial pressure 68.667041  right atrial pressure 1.487235  portal vein pressure 5.772029  leg vein pressure 2.716771  heart rate 1.021902  sympathetic traffic 0.977673  filtration 1.376097  urine 0.020000  wall power 3072.862098  aortic flow 0.000000
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
        if at < SECOND && at + 1 >= SECOND {
            assert!(
                watch.moments.iter().any(|moment| moment.phase > 0.0),
                "no breath has started by the first second"
            );
        }
    }
    each_breath_swings_the_alveolar_gases_and_venous_return(&watch);
    the_whole_beat_heat_report_counts_what_the_core_receives_from_the_beds(&start);
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
        owner: Respiratory::CARD,
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
    let (minute, counted, watch) = no_held_pool_moves_at_admission_or_over_the_first_minute(&start);
    let counts = Counts {
        rows: start.body.rows().len(),
        jacobian_nonzeros: start.body.sparsity().unknowns().len(),
        steps: minute.record().revision(),
        solves: start.core.solves.get() - before,
    };
    let mut report = each_breath_swings_the_alveolar_gases_and_venous_return(&watch);
    report += &the_pulmonary_plasma_stays_under_capacity_at_the_highest_alveolar_oxygen(&watch);
    the_body_admits_at_06_55(&start);
    the_first_second_of_the_body_on_ng_solver_stays_where_it_was(&start);
    report += &the_certificate_passes_on_ten_second_windows_over_the_first_minute_with_its_held_support(
        &start, &minute, &watch,
    );
    report += &each_gas_closes_on_the_airway_the_room_and_the_membrane_over_two_whole_breaths(&start, minute);
    expect![[r#"
        before the first breath edge: alveolar oxygen 101.326, carbon dioxide 28.340, venous return 77.34
        breath from 5.000 s: alveolar oxygen 100.791 swinging 3.070, carbon dioxide 28.690 swinging 2.011, venous return's beat means 74.25 to 84.57 around 78.05
        breath from 10.000 s: alveolar oxygen 100.452 swinging 3.053, carbon dioxide 28.831 swinging 1.946, venous return's beat means 74.11 to 84.52 around 78.01
        breath from 15.000 s: alveolar oxygen 100.177 swinging 3.042, carbon dioxide 28.896 swinging 1.918, venous return's beat means 74.11 to 84.49 around 78.00
        breath from 20.000 s: alveolar oxygen 99.950 swinging 3.035, carbon dioxide 28.927 swinging 1.905, venous return's beat means 74.11 to 84.50 around 78.00
        breath from 25.000 s: alveolar oxygen 99.759 swinging 3.032, carbon dioxide 28.942 swinging 1.900, venous return's beat means 74.12 to 84.50 around 78.00
        breath from 30.000 s: alveolar oxygen 99.596 swinging 3.049, carbon dioxide 28.952 swinging 1.901, venous return's beat means 74.05 to 84.49 around 78.00
        breath from 35.000 s: alveolar oxygen 99.455 swinging 3.072, carbon dioxide 28.960 swinging 1.903, venous return's beat means 74.06 to 84.49 around 78.00
        breath from 40.001 s: alveolar oxygen 99.331 swinging 3.092, carbon dioxide 28.966 swinging 1.904, venous return's beat means 74.06 to 84.49 around 78.00
        breath from 45.001 s: alveolar oxygen 99.223 swinging 3.111, carbon dioxide 28.973 swinging 1.905, venous return's beat means 74.06 to 84.49 around 78.01
        breath from 50.001 s: alveolar oxygen 99.127 swinging 3.127, carbon dioxide 28.978 swinging 1.906, venous return's beat means 74.06 to 84.49 around 78.01
        highest alveolar oxygen 102.575 (4.103 times P50) at 1.088 s: plasma oxygen pressure 93.084 (3.723 times P50), bound 0.963989 of capacity
        Energy: worst 1.1289e3, from admission 5.9381e1, steady 1.1296e3, inside a breath 5.6486e2
        Element(Carbon): worst 4.7480e1, from admission 3.6194e1, steady 4.4205e1, inside a breath 2.0167e2
        Element(Oxygen): worst 1.8252e3, from admission 1.8235e3, steady 5.2037e1, inside a breath 4.4882e3
        Element(Hydrogen): worst 5.0585e-5, from admission 1.5862e-5, steady 5.0627e-5, inside a breath 2.5428e-5
        Of(NitrogenGas): worst 3.4121e3, from admission 3.4115e3, steady 7.5750e1, inside a breath 8.6038e3
        Of(Vasopressin): worst 2.1035e-3, from admission 1.3302e-4, steady 1.3759e-3, inside a breath 1.5313e-2
        two breaths from 60.001 s to 70.001 s
        Oxygen: in 4589.9436, out 3651.6821, across 950.4726, along 938.9561, gained -1.2211e1
        CarbonDioxide: in 8.7427, out 686.3418, across -678.3349, along -677.6465, gained 7.3587e-1
        NitrogenGas: in 17258.1880, out 17246.6866, across 0.0000, along 10.8382, gained 1.1501e1
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
        steps 11671, solves 14388: ordinary 12338, landing 2050
        refused 74 Accuracy Algebraic(Slot { law: LawName { owner: "CV", local: "blood pressure" }, at: Site(SiteId { key: "left ventricle", class: VesselSegment }) })
        refused 1 Accuracy Algebraic(Slot { law: LawName { owner: "CV", local: "elastic energy" }, at: Body })
        refused 1 Accuracy Amount { site: SiteId { key: "body", class: TissueSite }, space: Elastic, species: Energy }
        refused 1 Accuracy Amount { site: SiteId { key: "conducting airway", class: TissueSite }, space: Gas, species: CarbonDioxide }
        refused 239 Accuracy Memory(Slot { law: LawName { owner: "CV", local: "valves" }, at: Edge { from: SiteId { key: "left atrium", class: VesselSegment }, to: SiteId { key: "left ventricle", class: VesselSegment } } })
        refused 293 Accuracy Memory(Slot { law: LawName { owner: "CV", local: "valves" }, at: Edge { from: SiteId { key: "left ventricle", class: VesselSegment }, to: SiteId { key: "systemic arteries", class: VesselSegment } } })
        refused 58 IterationLimit
    "#]]
    .assert_eq(&minute);
    assert_eq!(over_ceilings(counts, CEILINGS), Vec::<&str>::new(), "{counts:?}");
    a_water_bolus_dilutes_the_blood_and_leaves_its_gases_where_they_were(&start);
    the_flow_times_the_oxygen_content_difference_meets_the_oxygen_the_beds_burned_over_whole_beats(&start);
}
