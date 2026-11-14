use core::fmt::Write;
use core::num::NonZeroU64;
use expect_test::expect;
use ng_contract::*;
use ng_record::{Checkpoint, Running, Session};
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;
const SECOND: u64 = 1_000 * MILLISECOND;

/// The body's admission costs minutes, so one solve serves every check, each from a checkpoint of the admitted start.
struct Start {
    body: CompiledBody,
    core: ng_solver::Core,
    saved: Checkpoint,
    identity: RunIdentity,
    links: MemoryStore,
}
impl Start {
    fn admitted() -> Self {
        let body = whole_body().expect("compiled");
        let (core, links) = (ng_solver::Core::new(10 * MILLISECOND), MemoryStore::default());
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
        let guess = Compensated::of(&body_start_guess(*kind).expect("a finite guess"));
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
        steps 11797, quantities 480, held routes 62
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
        packet 65794c8df1131031c5659b9de0a949fcad72dbf2228e22d07483a66040a02929
        seal c28be00fec6e1426f20c983236d6c99aad39f8d15f8e822ac90b01dc1423036b
        steps 198
    "#]]
    .assert_eq(&pin);
    expect![[r#"
         100.432883 ms  arterial pressure 134.633470  right atrial pressure 3.008813  heart rate 1.063945  sympathetic traffic 0.786091  filtration 1.670059  urine 0.020000  wall power 24549.795335  aortic flow 0.000000
         200.972488 ms  arterial pressure 127.073930  right atrial pressure 3.567000  heart rate 1.081239  sympathetic traffic 0.086528  filtration 2.109294  urine 0.020000  wall power 15618.341703  aortic flow 39.437187
         300.302619 ms  arterial pressure 119.011127  right atrial pressure 4.057603  heart rate 1.048376  sympathetic traffic 0.016681  filtration 2.344745  urine 0.020000  wall power 39572.451620  aortic flow 0.000000
         403.704249 ms  arterial pressure 109.435147  right atrial pressure 3.675946  heart rate 1.004395  sympathetic traffic 0.012228  filtration 2.416000  urine 0.020000  wall power 13948.571425  aortic flow 0.000000
         504.099297 ms  arterial pressure 101.045318  right atrial pressure 2.024952  heart rate 0.970297  sympathetic traffic 0.022846  filtration 2.358747  urine 0.020000  wall power 12015.683838  aortic flow 0.000000
         608.059522 ms  arterial pressure 93.141382  right atrial pressure 1.494959  heart rate 0.947449  sympathetic traffic 0.073837  filtration 2.207971  urine 0.020000  wall power 8662.222415  aortic flow 0.000000
         708.059522 ms  arterial pressure 86.170580  right atrial pressure 1.466167  heart rate 0.940358  sympathetic traffic 0.258647  filtration 2.015383  urine 0.020000  wall power 6351.984080  aortic flow 0.000000
         808.059522 ms  arterial pressure 79.751627  right atrial pressure 1.672380  heart rate 0.951978  sympathetic traffic 0.637062  filtration 1.805023  urine 0.020000  wall power 4974.924985  aortic flow 0.000000
         908.059522 ms  arterial pressure 73.854992  right atrial pressure 1.951324  heart rate 0.982016  sympathetic traffic 0.902234  filtration 1.592549  urine 0.020000  wall power 4010.470565  aortic flow 0.000000
        1000.179885 ms  arterial pressure 68.861012  right atrial pressure 2.199761  heart rate 1.020599  sympathetic traffic 0.976716  filtration 1.404097  urine 0.020000  wall power 3214.815454  aortic flow 0.000000
    "#]].assert_eq(&trace);
}

#[test]
fn the_whole_body_admits_runs_its_first_minute_and_holds_its_pins() {
    let start = Start::admitted();
    the_body_admits_at_06_55(&start);
    the_first_second_of_the_body_on_ng_solver_stays_where_it_was(&start);
    let minute = no_held_pool_moves_at_admission_or_over_the_first_minute(&start);
    the_certificate_passes_on_ten_second_windows_over_the_first_minute_with_its_held_support(&start, &minute);
}
