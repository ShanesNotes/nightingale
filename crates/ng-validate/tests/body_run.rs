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
const PACKET: &str = "328146d6e6ae5cdfad1d9065a5d8b6ea17fd3c924df9203b5ce8c44cd95f09d5";
const ADMITTED_SEAL: &str = "5cd995520edd11c20e82506e0c84a51e988f74c83ab3d9e31d1a250cd8355a90";

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
    }
    session
}

fn no_held_pool_moves_at_admission_or_over_the_first_minute(start: &Start) -> (Session<'_, Running>, Minute) {
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
    for mark in 1..=6 {
        session = counted_until(start, session, 10 * mark * SECOND, &mut minute);
        for ((_, kind), (now, then)) in pools.iter().zip(amounts(&session).iter().zip(&admitted)) {
            assert_eq!(now, then, "{kind:?} moved by {} s", 10 * mark);
        }
    }
    (session, minute)
}

fn the_certificate_passes_on_ten_second_windows_over_the_first_minute_with_its_held_support(
    start: &Start,
    minute: &Session<'_, Running>,
) {
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
        steps 11739, quantities 399, held routes 89
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
}

/// Over ten seconds after the first minute, each gas the airway carries closes on the respiratory card's routes: what
/// the room puts in less what it takes out and what the membrane moves into the pulmonary bed's plasma is what the
/// dead space and the alveolar unit gained.
fn each_gas_closes_on_the_airway_the_room_and_the_membrane_over_ten_seconds(
    start: &Start,
    minute: Session<'_, Running>,
) {
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
    let from = minute.record().accepted().time().nanoseconds();
    let before = gases.map(|gas| held(&minute, gas));
    let session = until(minute, from - anchor_moment().nanoseconds() + 10 * SECOND);
    let to = session.record().accepted().time().nanoseconds();
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
    }
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
        packet 328146d6e6ae5cdfad1d9065a5d8b6ea17fd3c924df9203b5ce8c44cd95f09d5
        seal 5001258e3b0d5293808ecc4bd873705e371fa86041d46dc69080e68881566ad9
        steps 196
        viscosity over its reference at the start 1.000234140
    "#]]
    .assert_eq(&pin);
    expect![[r#"
         100.431615 ms  arterial pressure 134.631135  right atrial pressure 2.998390  portal vein pressure 6.246882  leg vein pressure 3.586652  heart rate 1.063942  sympathetic traffic 0.786097  filtration 1.670136  urine 0.020000  wall power 24573.594635  aortic flow 0.000000
         201.075082 ms  arterial pressure 127.066077  right atrial pressure 3.554914  portal vein pressure 6.392338  leg vein pressure 3.838452  heart rate 1.081222  sympathetic traffic 0.086335  filtration 2.109658  urine 0.020000  wall power 15580.644495  aortic flow 39.550340
         300.773672 ms  arterial pressure 118.963316  right atrial pressure 4.046727  portal vein pressure 6.201253  leg vein pressure 4.169709  heart rate 1.048163  sympathetic traffic 0.016618  filtration 2.345385  urine 0.020000  wall power 39212.514690  aortic flow 0.000000
         403.586201 ms  arterial pressure 109.444515  right atrial pressure 3.654480  portal vein pressure 5.974840  leg vein pressure 4.600291  heart rate 1.004439  sympathetic traffic 0.012230  filtration 2.415940  urine 0.020000  wall power 13935.818085  aortic flow 0.000000
         503.972823 ms  arterial pressure 101.054947  right atrial pressure 2.010192  portal vein pressure 5.753185  leg vein pressure 4.349139  heart rate 0.970333  sympathetic traffic 0.022828  filtration 2.358777  urine 0.020000  wall power 12009.501295  aortic flow 0.000000
         608.059522 ms  arterial pressure 93.141287  right atrial pressure 1.482228  portal vein pressure 5.484073  leg vein pressure 3.693218  heart rate 0.947452  sympathetic traffic 0.073857  filtration 2.207845  urine 0.020000  wall power 8653.474167  aortic flow 0.000000
         708.059522 ms  arterial pressure 86.170700  right atrial pressure 1.455032  portal vein pressure 5.267650  leg vein pressure 3.162410  heart rate 0.940363  sympathetic traffic 0.258683  filtration 2.015290  urine 0.020000  wall power 6345.593911  aortic flow 0.000000
         808.059522 ms  arterial pressure 79.751898  right atrial pressure 1.662051  portal vein pressure 5.250661  leg vein pressure 2.956498  heart rate 0.951985  sympathetic traffic 0.637088  filtration 1.804989  urine 0.020000  wall power 4971.123428  aortic flow 0.000000
         908.059522 ms  arterial pressure 73.855379  right atrial pressure 1.941206  portal vein pressure 5.517491  leg vein pressure 3.083332  heart rate 0.982024  sympathetic traffic 0.902238  filtration 1.592580  urine 0.020000  wall power 4008.727387  aortic flow 0.000000
        1000.170544 ms  arterial pressure 68.861963  right atrial pressure 2.189564  portal vein pressure 5.881250  leg vein pressure 3.299054  heart rate 1.020603  sympathetic traffic 0.976713  filtration 1.404194  urine 0.020000  wall power 3215.302678  aortic flow 0.000000
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
/// are read at the start of a beat before the bolus and after it, so the beat's swing in them does not enter.
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
    let held = (starts.iter().find(|at| **at < 2 * SECOND)).and_then(|at| gases(*at));
    let delivered = (starts.iter().find(|at| **at >= 2 * SECOND)).and_then(|at| gases(*at));
    let (held, delivered) = (
        held.expect("a beat starts before the bolus ends"),
        delivered.expect("a beat starts after the bolus"),
    );
    for (gas, (held, delivered)) in ["oxygen", "carbon dioxide"]
        .iter()
        .zip(held.iter().zip(&delivered))
    {
        assert!(
            (delivered - held).abs() <= 1e-5 * held,
            "the body's {gas} went from {held} to {delivered} over the bolus, each at a beat's start"
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

#[test]
fn the_whole_beat_heat_report_counts_what_the_core_receives_from_the_beds() {
    let start = Start::admitted();
    let mut openings = Vec::new();
    let session = beats(&start, start.session(), 3 * SECOND, |session, opened| {
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

/// Over whole beats the flow is the water the left ventricle loses while its valve stands open, because the
/// trapezoid of the valve's flow at the ends of steps reads it 0.6% high. The lung's lymph takes water and no gas from
/// the pulmonary bed to the central veins, so the pulmonary artery carries that water too, and the lung's own bed
/// takes its oxygen from its plasma before the pulmonary vein, so the arteries carry the other beds' burn. The flow
/// times each gas's content difference then meets what the ledger says the beds' burns used and made, to LEDGER, the
/// body's settling over nine beats.
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
    for (gas, taken, rate) in fick {
        assert!(
            (taken - rate).abs() <= LEDGER * rate,
            "over {} beats flow {flow} and lymph {lymph} move {taken} of {gas} a second against {rate}",
            onsets.len() - 1
        );
    }
    assert!(
        (quotient - QUOTIENT).abs() <= 1e-12,
        "the burns made {quotient} carbon dioxide an oxygen"
    );
    for (gas, rate) in [
        (Species::Oxygen, oxygen),
        (Species::CarbonDioxide, carbon_dioxide),
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
        let (share, half) = (if key == "peritubular bed" { 0.0 } else { 0.5 }, demand * 1e-3);
        for (when, session) in [("admitted", &admitted), ("a step on", &stepped)] {
            let (held, owed) = (value(start, session, lactate), value(start, session, debt));
            assert!(
                held.abs() <= AT_REST_LACTATE && owed.abs() <= 3.0 * AT_REST_LACTATE,
                "the {key} {when} holds {held} lactate in its cells and owes {owed} oxygen"
            );
            let whole = owed + half;
            let kept = (half + (owed - share * owed)) / whole;
            assert_eq!(
                kept.to_bits(),
                1f64.to_bits(),
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
        let [stir] = of.fired() else {
            panic!("{gate:?} fired {:?}", of.fired());
        };
        assert_eq!((stir.stream(), stir.ordinal()), (stream, 0), "{gate:?}");
        assert!(
            stir.at().nanoseconds() < fence,
            "{gate:?} stirred at {:?}",
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
    the_body_admits_at_06_55(&start);
    the_first_second_of_the_body_on_ng_solver_stays_where_it_was(&start);
    let before = start.core.solves.get();
    let (minute, counted) = no_held_pool_moves_at_admission_or_over_the_first_minute(&start);
    the_certificate_passes_on_ten_second_windows_over_the_first_minute_with_its_held_support(&start, &minute);
    let counts = Counts {
        rows: start.body.rows().len(),
        jacobian_nonzeros: start.body.sparsity().unknowns().len(),
        steps: minute.record().revision(),
        solves: start.core.solves.get() - before,
    };
    each_gas_closes_on_the_airway_the_room_and_the_membrane_over_ten_seconds(&start, minute);
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
        steps 11739, solves 14452: ordinary 12404, landing 2048
        refused 60 Accuracy Algebraic(Slot { law: LawName { owner: "CV", local: "blood pressure" }, at: Site(SiteId { key: "left ventricle", class: VesselSegment }) })
        refused 243 Accuracy Memory(Slot { law: LawName { owner: "CV", local: "valves" }, at: Edge { from: SiteId { key: "left atrium", class: VesselSegment }, to: SiteId { key: "left ventricle", class: VesselSegment } } })
        refused 301 Accuracy Memory(Slot { law: LawName { owner: "CV", local: "valves" }, at: Edge { from: SiteId { key: "left ventricle", class: VesselSegment }, to: SiteId { key: "systemic arteries", class: VesselSegment } } })
        refused 61 IterationLimit
    "#]]
    .assert_eq(&minute);
    assert_eq!(over_ceilings(counts, CEILINGS), Vec::<&str>::new(), "{counts:?}");
    a_water_bolus_dilutes_the_blood_and_leaves_its_gases_where_they_were(&start);
    the_flow_times_the_oxygen_content_difference_meets_the_oxygen_the_beds_burned_over_whole_beats(&start);
}
