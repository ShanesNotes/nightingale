use core::cell::Cell;
use core::fmt::Write;
use core::num::NonZeroU64;
use expect_test::expect;
use ng_contract::*;
use ng_record::{AuthorizedInput, Checkpoint, Running, Session};
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;
const SECOND: u64 = 1_000 * MILLISECOND;
const PACKET: &str = "e9943a87f779d37f8514aa50f1017961ccab29f66400152357e8b617b7af8e40";
const ADMITTED_SEAL: &str = "05f7ad106481da6115a56e9a34b78a0c2cf803473240159d8afec90fa6e60d6e";

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

fn no_held_pool_moves_at_admission_or_over_the_first_minute(start: &Start) -> Session<'_, Running> {
    let pools = start.held_pools();
    assert_eq!(pools.len(), 365);
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
    for mark in 1..=6 {
        session = until(session, 10 * mark * SECOND);
        for ((_, kind), (now, then)) in pools.iter().zip(amounts(&session).iter().zip(&admitted)) {
            assert_eq!(now, then, "{kind:?} moved by {} s", 10 * mark);
        }
    }
    session
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
        steps 11794, quantities 436, held routes 86
        EN vasopressin clearance moved
        EN vasopressin secretion moved
        GI gut chloride moved
        GI gut potassium moved
        GI gut sodium moved
        GI gut water moved
        MS skin water moved
        MT bone bed carbon dioxide made moved
        MT bone bed oxygen use moved
        MT brain bed carbon dioxide made moved
        MT brain bed oxygen use moved
        MT coronary bed carbon dioxide made moved
        MT coronary bed oxygen use moved
        MT fat bed carbon dioxide made moved
        MT fat bed oxygen use moved
        MT glomerular bed carbon dioxide made moved
        MT glomerular bed oxygen use moved
        MT gut bed carbon dioxide made moved
        MT gut bed oxygen use moved
        MT liver bed carbon dioxide made moved
        MT liver bed oxygen use moved
        MT lung carbon dioxide moved
        MT lung oxygen moved
        MT muscle bed carbon dioxide made moved
        MT muscle bed oxygen use moved
        MT peritubular bed carbon dioxide made moved
        MT peritubular bed oxygen use moved
        MT pulmonary bed carbon dioxide made moved
        MT pulmonary bed oxygen use moved
        MT skin bed carbon dioxide made moved
        MT skin bed oxygen use moved
    "#]]
    .assert_eq(&report);
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
        packet e9943a87f779d37f8514aa50f1017961ccab29f66400152357e8b617b7af8e40
        seal 7e30e7a5df836cc73bf6e338f34e8d530f8b3e5aa690b44f36fecd4c688de52f
        steps 198
        viscosity over its reference at the start 1.000000413
    "#]]
    .assert_eq(&pin);
    expect![[r#"
         100.432831 ms  arterial pressure 134.633292  right atrial pressure 3.008802  portal vein pressure 6.258052  leg vein pressure 3.596931  heart rate 1.063945  sympathetic traffic 0.786091  filtration 1.670060  urine 0.020000  wall power 24549.627276  aortic flow 0.000000
         200.971783 ms  arterial pressure 127.073846  right atrial pressure 3.566989  portal vein pressure 6.406698  leg vein pressure 3.850276  heart rate 1.081239  sympathetic traffic 0.086531  filtration 2.109289  urine 0.020000  wall power 15618.233755  aortic flow 39.436322
         300.343558 ms  arterial pressure 119.007062  right atrial pressure 4.057781  portal vein pressure 6.216546  leg vein pressure 4.180908  heart rate 1.048358  sympathetic traffic 0.016675  filtration 2.344802  urine 0.020000  wall power 39549.054176  aortic flow 0.000000
         403.707420 ms  arterial pressure 109.434879  right atrial pressure 3.675871  portal vein pressure 5.988799  leg vein pressure 4.614320  heart rate 1.004394  sympathetic traffic 0.012228  filtration 2.415994  urine 0.020000  wall power 13948.485600  aortic flow 0.000000
         504.102519 ms  arterial pressure 101.045104  right atrial pressure 2.024956  portal vein pressure 5.767216  leg vein pressure 4.364992  heart rate 0.970296  sympathetic traffic 0.022847  filtration 2.358741  urine 0.020000  wall power 12015.605952  aortic flow 0.000000
         608.059522 ms  arterial pressure 93.141457  right atrial pressure 1.494962  portal vein pressure 5.498127  leg vein pressure 3.709215  heart rate 0.947449  sympathetic traffic 0.073838  filtration 2.207971  urine 0.020000  wall power 8662.269616  aortic flow 0.000000
         708.059522 ms  arterial pressure 86.170674  right atrial pressure 1.466155  portal vein pressure 5.280744  leg vein pressure 3.176841  heart rate 0.940358  sympathetic traffic 0.258646  filtration 2.015385  urine 0.020000  wall power 6352.014084  aortic flow 0.000000
         808.059522 ms  arterial pressure 79.751725  right atrial pressure 1.672362  portal vein pressure 5.262427  leg vein pressure 2.969095  heart rate 0.951979  sympathetic traffic 0.637060  filtration 1.805026  urine 0.020000  wall power 4974.942873  aortic flow 0.000000
         908.059522 ms  arterial pressure 73.855082  right atrial pressure 1.951307  portal vein pressure 5.528042  leg vein pressure 3.094495  heart rate 0.982016  sympathetic traffic 0.902233  filtration 1.592553  urine 0.020000  wall power 4010.478457  aortic flow 0.000000
        1000.179870 ms  arterial pressure 68.861095  right atrial pressure 2.199746  portal vein pressure 5.891237  leg vein pressure 3.309513  heart rate 1.020599  sympathetic traffic 0.976716  filtration 1.404101  urine 0.020000  wall power 3214.816905  aortic flow 0.000000
    "#]].assert_eq(&trace);
}

/// The coronary bed's inflow edge has no tone, no feedback share and no fall, so its pressure drop over its flow is
/// the packet's resistance times viscosity over its reference.
const CORONARY_RESISTANCE: f64 = 1.4 / 0.1;

struct Edge {
    hematocrit: f64,
    viscosity: f64,
    drop: f64,
    flow: f64,
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
        }
    }
    fn resistance(&self) -> f64 {
        self.drop / self.flow
    }
}

/// A 400 mL water bolus into the gut lowers the hematocrit, viscosity and the edge's resistance, and the edge's flow is
/// its drop over the packet's resistance times viscosity's ratio, so the run tells viscosity's part of the flow from
/// the drop's. The body's gases are read at the start of a beat before the bolus and after it, so the beat's swing in
/// them does not enter.
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
        "hematocrit {} to {}, viscosity {} to {}, resistance ratio {resisted}, drop {} to {}, flow {} to {}",
        before.hematocrit,
        after.hematocrit,
        before.viscosity,
        after.viscosity,
        before.drop,
        after.drop,
        before.flow,
        after.flow
    );
    assert!(after.hematocrit < before.hematocrit && ratio < 1.0, "{report}");
    assert!((resisted - ratio).abs() < 1e-9, "{report}");
    let through_viscosity = after.drop / before.drop / ratio;
    assert!(
        after.flow > before.flow && (after.flow / before.flow - through_viscosity).abs() < 1e-9,
        "{report}"
    );
}

const USE: f64 = 96.0;
const MADE: f64 = 0.75 * USE;
/// The lung's own bed takes its use from its plasma before the pulmonary vein, so the arteries carry the rest.
const LUNG_BED_USE: f64 = 1.0;
const SYSTEMIC: f64 = (USE - LUNG_BED_USE) / USE;

fn exchanged(session: &Session<'_, Running>, quantity: QuantityId, at: u64, outflow: bool) -> f64 {
    for segment in session.conservation().newest_first() {
        let segment = segment.expect("a segment");
        let interval = segment.interval();
        let (from, to) = (interval.start().nanoseconds(), interval.end().nanoseconds());
        if from < at && at <= to {
            let curve = if outflow {
                segment.outflow(quantity)
            } else {
                segment.inflow(quantity)
            };
            let piece = Interval::new(interval.start(), SimTime::from_ns(at)).expect("a piece");
            return lead(curve.on(piece).expect("inside its segment")[1]);
        }
    }
    panic!("no segment holds {at}")
}

/// The flow is the water the left ventricle loses while its valve stands open, because the trapezoid of the valve's
/// flow at the ends of steps reads it 0.6% high. The lung's lymph takes water and no gas from the pulmonary bed to the
/// central veins, so the pulmonary artery carries that water too. Both gases then meet their held rates to LEDGER, the
/// body's settling over nine beats.
fn the_flow_times_the_oxygen_and_carbon_dioxide_content_differences_meets_their_held_rates_over_whole_beats(
    start: &Start,
) {
    let mut samples: Vec<(u64, [f64; 5], bool, f64)> = Vec::new();
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
    let mut openings: Vec<(u64, u64)> = Vec::new();
    let session = until(start.session(), 50 * SECOND);
    let session = beats(start, session, 60 * SECOND, |session, opened| {
        let at = session.record().accepted().time().nanoseconds();
        if let Some(closed) = opened {
            openings.push((at, closed));
        }
        let read = |key, gas| concentration(start, session, key, gas);
        samples.push((
            at,
            [
                read("systemic arteries", Species::Oxygen),
                read("pulmonary artery", Species::Oxygen),
                read("systemic arteries", Species::CarbonDioxide),
                read("pulmonary artery", Species::CarbonDioxide),
                value(start, session, lymph),
            ],
            aortic_flow(start, session) > 0.0,
            value(start, session, ventricle),
        ));
    });
    let onsets = onsets(&openings);
    let (first, last) = (onsets[0], onsets[onsets.len() - 1]);
    let held = (last - first) as f64 / 1e9;
    let (mut means, mut ejected) = ([0.0; 5], 0.0);
    for pair in samples.windows(2) {
        let ((from, then, was_ejecting, before), (to, now, ejecting, after)) = (pair[0], pair[1]);
        if from >= first && to <= last {
            let length = (to - from) as f64 / 1e9;
            for (mean, (then, now)) in means.iter_mut().zip(then.iter().zip(&now)) {
                *mean += 0.5 * length * (then + now) / held;
            }
            if was_ejecting || ejecting {
                ejected += before - after;
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
    let fick = [
        (
            "oxygen",
            flow * arterial_oxygen - returned * venous_oxygen,
            SYSTEMIC * USE,
        ),
        (
            "carbon dioxide",
            returned * venous_carbon_dioxide - flow * arterial_carbon_dioxide,
            SYSTEMIC * MADE,
        ),
    ];
    for (gas, taken, rate) in fick {
        assert!(
            (taken - rate).abs() <= LEDGER * rate,
            "over {} beats flow {flow} and lymph {lymph} move {taken} of {gas} a second against {rate}",
            onsets.len() - 1
        );
    }
    let basis = start.body.conserved_basis();
    for (gas, outflow, rate) in [
        (Species::Oxygen, false, USE),
        (Species::CarbonDioxide, true, MADE),
    ] {
        let lung = (basis.ids())
            .filter(|quantity| quantity.book() == Book::Of(gas))
            .map(|quantity| {
                let moved = exchanged(&session, quantity, last, outflow)
                    - exchanged(&session, quantity, first, outflow);
                moved / held
            })
            .fold(0.0, |most, moved| if moved > most { moved } else { most });
        assert!(
            (lung - rate).abs() <= LEDGER * rate,
            "the ledger's {gas:?} exchange at the lung over the beats is {lung} a second against {rate}"
        );
    }
}
const LEDGER: f64 = 1e-4;

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
    the_body_admits_at_06_55(&start);
    the_first_second_of_the_body_on_ng_solver_stays_where_it_was(&start);
    let before = start.core.solves.get();
    let minute = no_held_pool_moves_at_admission_or_over_the_first_minute(&start);
    the_certificate_passes_on_ten_second_windows_over_the_first_minute_with_its_held_support(&start, &minute);
    let counts = Counts {
        rows: start.body.rows().len(),
        jacobian_nonzeros: start.body.sparsity().unknowns().len(),
        steps: minute.record().revision(),
        solves: start.core.solves.get() - before,
    };
    assert_eq!(over_ceilings(counts, CEILINGS), Vec::<&str>::new(), "{counts:?}");
    a_water_bolus_dilutes_the_blood_and_leaves_its_gases_where_they_were(&start);
    the_flow_times_the_oxygen_and_carbon_dioxide_content_differences_meets_their_held_rates_over_whole_beats(
        &start,
    );
}
