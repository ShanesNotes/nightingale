use core::cell::Cell;
use core::fmt::Write;
use core::num::NonZeroU64;
use expect_test::expect;
use ng_contract::*;
use ng_record::{Checkpoint, Running, Session};
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;
const SECOND: u64 = 1_000 * MILLISECOND;
const PACKET: &str = "d3411f7ce491109f9a58f3c9f660848dc06e1c2813e574de2b0dee550b14ebe9";
const ADMITTED_SEAL: &str = "9040a97ac5572486cee34c0c76c03a915e26659b1b25369e73f0ccb4d94485b7";

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

fn the_body_admits_at_06_55(start: &Start) {
    let at = start.session().record().accepted().time().nanoseconds();
    assert_eq!(at, (6 * 3_600 + 55 * 60) * SECOND);
    assert_eq!(hex(start.identity.packet().digest()), PACKET);
    assert_eq!(hex(start.session().record().seal().digest()), ADMITTED_SEAL);
}

fn no_held_pool_moves_at_admission_or_over_the_first_minute(start: &Start) -> Session<'_, Running> {
    let pools = start.held_pools();
    assert_eq!(pools.len(), 409);
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
    let policy = arena_certificate(&start.body, windows).expect("a policy");
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
        steps 11794, quantities 480, held routes 62
        EN vasopressin clearance moved
        EN vasopressin secretion moved
        GI gut chloride moved
        GI gut potassium moved
        GI gut sodium moved
        GI gut water moved
        MS skin water moved
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
    let pin = format!(
        "packet {}\nseal {}\nsteps {}\n",
        hex(start.identity.packet().digest()),
        hex(record.seal().digest()),
        record.revision()
    );
    expect![[r#"
        packet d3411f7ce491109f9a58f3c9f660848dc06e1c2813e574de2b0dee550b14ebe9
        seal 4a7fde4c8e7a5769dea090dfc1ad6c6572173609f1bf8ad0d89d7c1f00ecaac1
        steps 198
    "#]]
    .assert_eq(&pin);
    expect![[r#"
         100.432883 ms  arterial pressure 134.633468  right atrial pressure 3.008818  portal vein pressure 6.258075  leg vein pressure 3.596946  heart rate 1.063945  sympathetic traffic 0.786091  filtration 1.670059  urine 0.020000  wall power 24549.774592  aortic flow 0.000000
         200.972493 ms  arterial pressure 127.073929  right atrial pressure 3.567014  portal vein pressure 6.406726  leg vein pressure 3.850298  heart rate 1.081239  sympathetic traffic 0.086528  filtration 2.109293  urine 0.020000  wall power 15618.327492  aortic flow 39.437189
         300.346342 ms  arterial pressure 119.006854  right atrial pressure 4.057817  portal vein pressure 6.216568  leg vein pressure 4.180947  heart rate 1.048357  sympathetic traffic 0.016674  filtration 2.344810  urine 0.020000  wall power 39547.568657  aortic flow 0.000000
         403.707336 ms  arterial pressure 109.434896  right atrial pressure 3.675895  portal vein pressure 5.988828  leg vein pressure 4.614348  heart rate 1.004394  sympathetic traffic 0.012228  filtration 2.415998  urine 0.020000  wall power 13948.549809  aortic flow 0.000000
         504.102217 ms  arterial pressure 101.045105  right atrial pressure 2.024984  portal vein pressure 5.767247  leg vein pressure 4.365019  heart rate 0.970296  sympathetic traffic 0.022846  filtration 2.358743  urine 0.020000  wall power 12015.672581  aortic flow 0.000000
         608.059522 ms  arterial pressure 93.141401  right atrial pressure 1.494983  portal vein pressure 5.498156  leg vein pressure 3.709240  heart rate 0.947449  sympathetic traffic 0.073838  filtration 2.207971  urine 0.020000  wall power 8662.306473  aortic flow 0.000000
         708.059522 ms  arterial pressure 86.170598  right atrial pressure 1.466172  portal vein pressure 5.280771  leg vein pressure 3.176864  heart rate 0.940358  sympathetic traffic 0.258647  filtration 2.015384  urine 0.020000  wall power 6352.024723  aortic flow 0.000000
         808.059522 ms  arterial pressure 79.751643  right atrial pressure 1.672377  portal vein pressure 5.262452  leg vein pressure 2.969114  heart rate 0.951979  sympathetic traffic 0.637062  filtration 1.805023  urine 0.020000  wall power 4974.935647  aortic flow 0.000000
         908.059522 ms  arterial pressure 73.855007  right atrial pressure 1.951321  portal vein pressure 5.528065  leg vein pressure 3.094512  heart rate 0.982017  sympathetic traffic 0.902234  filtration 1.592550  urine 0.020000  wall power 4010.466126  aortic flow 0.000000
        1000.179868 ms  arterial pressure 68.861027  right atrial pressure 2.199760  portal vein pressure 5.891259  leg vein pressure 3.309528  heart rate 1.020599  sympathetic traffic 0.976716  filtration 1.404098  urine 0.020000  wall power 3214.808695  aortic flow 0.000000
    "#]].assert_eq(&trace);
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
}
