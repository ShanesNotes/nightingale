mod certificate_oracle;

use certificate_oracle::{Ledger, certified};
use core::cell::{Cell, RefCell};
use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::{
    AuthorizedInput, BranchId, CheckpointError, ConservationRefusal, HistoryStore, Running, Session, Step,
    StepReceipt,
};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const HOUR: u64 = 3_600 * SECOND;
const DAY: u64 = 24 * HOUR;
const PAIR_PRECISION: f64 = f64::EPSILON * f64::EPSILON / 2.0;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at(ns: u64) -> SimTime {
    SimTime::from_ns(ns)
}

fn interval(from: u64, to: u64) -> Interval {
    Interval::new(at(from), at(to)).expect("an interval")
}

fn delivery(id: &'static [u8], from_s: u64, to_s: u64, rate: f64) -> Delivery {
    Delivery {
        id,
        from: at(from_s * SECOND),
        to: at(to_s * SECOND),
        amount: measure(rate * (to_s - from_s) as f64),
    }
}

fn accepted(step: Step<'_>) -> (Session<'_, Running>, StepReceipt) {
    match step {
        Step::Accepted(session, receipt) => (session, receipt),
        Step::Refused(session) => panic!("refused: {:?}", session.refused()),
        Step::Faulted(session) => panic!("faulted: {:?}", session.fault()),
    }
}

fn step_folding<'m>(
    mut session: Session<'m, Running>,
    end: SimTime,
    folds: &mut [&mut CertificateFold<'m, '_>],
) -> Session<'m, Running> {
    while session.record().accepted().time() < end {
        let mut attempt = session.step();
        session = loop {
            match attempt {
                Step::Accepted(next, _) => {
                    for fold in folds.iter_mut() {
                        fold.advance(&next).expect("follows the previous accepted step");
                    }
                    break next;
                }
                Step::Refused(retrying) => attempt = retrying.retry(),
                Step::Faulted(stopped) => panic!("faulted: {:?}", stopped.fault()),
            }
        };
    }
    session
}

fn assert_fold_matches_certify(
    session: &Session<'_, Running>,
    fold: &CertificateFold<'_, '_>,
    ledger: &Ledger,
    label: &str,
) {
    let expected = certified(&session.conservation(), ledger);
    let actual = fold.certificate();
    match (actual, expected) {
        (Ok(got), Ok(want)) => {
            assert_eq!(
                format!("{got:?}"),
                format!("{want:?}"),
                "{label}: passing certificate"
            );
        }
        (Err(got), Err(want)) => {
            assert_eq!(
                (
                    got.check(),
                    got.quantity(),
                    got.window(),
                    got.residual().bits(),
                    got.allowance().map(Compensated::bits),
                ),
                (
                    want.check(),
                    want.quantity(),
                    want.window(),
                    want.residual().bits(),
                    want.allowance().map(Compensated::bits),
                ),
                "{label}: failing certificate"
            );
        }
        (got, want) => panic!("{label}: fold gave {got:?}, certify gave {want:?}"),
    }
}

fn drifts_then_opens() -> TwoReservoirs {
    let held = 1e18;
    let bulk = Delivery {
        id: b"bulk",
        from: at(0),
        to: at(600 * SECOND),
        amount: measure(600.0),
    };
    let tiny = Delivery {
        id: b"below the pair's precision",
        from: at(2 * HOUR),
        to: at(4 * HOUR),
        amount: measure(held * PAIR_PRECISION / 4.0),
    };
    TwoReservoirs::delivering([measure(held), measure(held)], vec![bulk, tiny]).expect("a body")
}

#[test]
fn the_fold_matches_certify_and_the_oracle_bit_for_bit_across_window_shapes() {
    let rested = || [measure(1_000.0), measure(1_000.0)];
    let shifted = TwoReservoirs::delivering(
        [measure(500_500.0), measure(499_500.0)],
        vec![
            delivery(b"baseline", 0, 96_400, 1.0),
            delivery(b"high", 96_400, 116_400, 5.0),
        ],
    )
    .expect("a body");
    let cases = [
        (
            "rested-tie",
            TwoReservoirs::delivering(rested(), Vec::new()).expect("a body"),
            60 * SECOND,
            HOUR,
            4 * HOUR,
            4 * HOUR,
        ),
        (
            "one-window-past-end",
            TwoReservoirs::delivering(
                rested(),
                vec![delivery(b"a", 0, 3_600, 1.0), delivery(b"b", 3_600, 3_720, 5.0)],
            )
            .expect("a body"),
            60 * SECOND,
            HOUR,
            HOUR,
            HOUR + 120 * SECOND,
        ),
        ("shifted-starts", shifted, 70 * SECOND, DAY, 2 * DAY, 2 * DAY),
        (
            "closure-over-drift",
            drifts_then_opens(),
            60 * SECOND,
            HOUR,
            4 * HOUR,
            4 * HOUR,
        ),
    ];
    for (name, case, step_ns, window_ns, coverage_ns, end_ns) in cases {
        let (core, store) = (ReferenceIntegrator::new(step_ns), MemoryStore::default());
        let session = case.admit(&core, &store).expect("admitted");
        let basis = session.conservation().basis();
        let loose = Ledger::sliding(basis, window_ns, coverage_ns, 1e-24, 1e-9, 1e12);
        let tight = Ledger::sliding(basis, window_ns, coverage_ns, 1e-24, 1e-9, 1.0);
        let drift_only = Ledger::sliding(basis, window_ns, coverage_ns, 1e300, 1e-9, 1.0);
        let mut f_loose = CertificateFold::open(&session, &loose.policy).expect("opened");
        let mut f_tight = CertificateFold::open(&session, &tight.policy).expect("opened");
        let mut f_drift = CertificateFold::open(&session, &drift_only.policy).expect("opened");
        let session = step_folding(
            session,
            at(end_ns),
            &mut [&mut f_loose, &mut f_tight, &mut f_drift],
        );
        assert_fold_matches_certify(&session, &f_loose, &loose, &format!("{name}/loose"));
        assert_fold_matches_certify(&session, &f_tight, &tight, &format!("{name}/tight"));
        assert_fold_matches_certify(&session, &f_drift, &drift_only, &format!("{name}/drift"));
    }

    let (span, length, step) = (6 * HOUR, 3 * HOUR + 50 * MILLISECOND, 100 * MILLISECOND);
    let case = TwoReservoirs::delivering([measure(600.0), measure(424.0)], Vec::new())
        .expect("a body")
        .until(at(span));
    let (core, store) = (ng_solver::Core::new(step), MemoryStore::default());
    let session = case.admit(&core, &store).expect("admitted");
    let mut ledger = Ledger::sliding(session.conservation().basis(), length, span, 0.0, 1e-12, 1e9);
    ledger.policy = ledger
        .policy
        .admitting_row_rounding(case.body(), 7)
        .expect("a policy");
    ledger.admit_row_rounding(case.body(), 7);
    let mut fold = CertificateFold::open(&session, &ledger.policy).expect("opened");
    let session = step_folding(session, case.end(), &mut [&mut fold]);
    assert_fold_matches_certify(&session, &fold, &ledger, "row-rounding");

    let body = arena_body().expect("compiled");
    let packet = arena_packet(&body).expect("a packet");
    let lens = arena_lens(&body).expect("a lens");
    let core = ng_solver::Core::new(10 * MILLISECOND);
    let store = MemoryStore::default();
    let origin = anchor_moment();
    let port = BoundaryPort::new(Vec::new()).expect("no events");
    let session = admit_at(&body, packet, port, &core, &lens, origin, &store)
        .expect("admitted")
        .session;
    let basis = session.conservation().basis();
    let sliding = Ledger::sliding(basis, SECOND, 2 * SECOND, 0.0, 1e-12, 1e4);
    let one_window = Ledger::sliding(basis, 2 * SECOND, 2 * SECOND, 0.0, 1e-12, 1e4);
    let mut f_sliding = CertificateFold::open(&session, &sliding.policy).expect("opened");
    let mut f_one = CertificateFold::open(&session, &one_window.policy).expect("opened");
    let end = at(origin.nanoseconds() + 2 * SECOND);
    let session = step_folding(session, end, &mut [&mut f_sliding, &mut f_one]);
    assert_fold_matches_certify(&session, &f_sliding, &sliding, "slice/sliding");
    assert_fold_matches_certify(&session, &f_one, &one_window, "slice/one-window");
}

fn breathing_body() -> CompiledBody {
    let origin = || Position::new(measure(0.0), measure(0.0), measure(0.0));
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let vessel = map.site::<VesselSegment>("vessel", origin());
    let bed = map.site::<CapillaryBed>("bed", origin());
    map.contain(root.id(), vessel.id());
    map.contain(root.id(), bed.id());
    map.connect(Network::Vascular, vessel.id(), bed.id());
    map.connect(Network::Vascular, bed.id(), vessel.id());
    let mut schema = Schema::new(map.finish().expect("an anatomy")).expect("a schema");
    schema.leave_out(Space::Plasma, "gas only");
    schema.leave_out(Space::Cells, "gas only");
    schema.leave_out(Space::Lumen, "gas only");
    schema.leave_out(Space::Interstitium, "gas only");
    schema.carry::<NitrogenGas, CapillaryBed>(Space::Gas);
    schema
        .boundary::<NitrogenGas>("nitrogen", BoundaryDirection::IntoBody, bed.id(), Space::Gas)
        .expect("a boundary");
    let class = RateClass::new(RateBand::Wave, 3_600 * SECOND).expect("a class");
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let phase = laws.state::<Dimensionless>("phase", Admissible::nonnegative());
        let pace =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.pace"), Admissible::positive());
        let rate = laws.kernel(|mut k| {
            let value = k.parameter(&pace);
            k.finish(value, laws.every_mode(), class)
        });
        laws.memory_rate(&phase, rate);
        let threshold =
            laws.parameter::<Dimensionless>(ParameterKey::new("bd.threshold"), Admissible::positive());
        let [tick, tock] = laws.modes("beat", ["tick", "tock"]);
        for (from, to) in [(&tick, &tock), (&tock, &tick)] {
            let past = laws.kernel(|mut k| {
                let (now, threshold) = (k.state(&phase), k.parameter(&threshold));
                let gap = k.subtract(now, threshold);
                k.finish(gap, from.only(), class)
            });
            laws.event(Guard::new(past, Crossing::Rising, from.to(to)?).resetting(&phase));
        }
        laws.rhythm(&phase);
        let BoundaryRoute { route, .. } = ports.route::<NitrogenGas>("nitrogen")?;
        let drift = laws.parameter::<Rate<NitrogenGas>>(ParameterKey::new("bd.drift"), Admissible::finite());
        let leak = laws.kernel(|mut k| {
            let value = k.parameter(&drift);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, leak));
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

fn breathing_packet(period_s: f64, drift: f64) -> ParameterPacket<SyntheticFixture> {
    let origin = Position::new(measure(0.0), measure(0.0), measure(0.0));
    let bed = AnatomyBuilder::new().site::<CapillaryBed>("bed", origin).id();
    let mut packet = ParameterPacket::new();
    packet
        .synthetic(
            ParameterKey::new("bd.pace"),
            Binding::Body,
            measure::<Rate<Dimensionless>>(1.0 / period_s),
            "synthetic",
        )
        .expect("once");
    packet
        .synthetic(
            ParameterKey::new("bd.threshold"),
            Binding::Body,
            measure::<Dimensionless>(1.0),
            "synthetic",
        )
        .expect("once");
    packet
        .synthetic(
            ParameterKey::new("bd.drift"),
            Binding::Body,
            measure::<Rate<NitrogenGas>>(drift),
            "synthetic",
        )
        .expect("once");
    packet
        .start(
            RowKind::Amount {
                site: bed,
                space: Space::Gas,
                species: Species::NitrogenGas,
            },
            measure::<NitrogenGas>(4_000.0),
            "synthetic",
        )
        .expect("once");
    let phase = LawName {
        owner: "Boundary",
        local: "phase",
    };
    packet
        .start(
            RowKind::Memory(phase.into()),
            measure::<Dimensionless>(0.0),
            "synthetic",
        )
        .expect("once");
    packet
        .start_mode(
            LawName {
                owner: "Boundary",
                local: "beat",
            },
            "tick",
            "synthetic",
        )
        .expect("once");
    packet
}

#[test]
fn the_fold_matches_certify_at_every_intermediate_head_including_rhythm_tail_rollback() {
    let body = breathing_body();
    let clock = Slot::from(LawName {
        owner: "Boundary",
        local: "phase",
    });
    let (core, store) = (
        ReferenceIntegrator::new(250 * MILLISECOND),
        MemoryStore::default(),
    );
    let port = BoundaryPort::new(Vec::new()).expect("no events");
    let session = admit_fixture(&body, breathing_packet(4.0, 1.0), port, &core, &store).expect("admitted");
    let basis = session.conservation().basis();
    let nitrogen = basis.ids().next().expect("nitrogen gas");
    let ledger = Ledger::sliding(basis, 10 * SECOND, 16 * SECOND, 1e-6, 1e-9, 1e9).between_edges(
        clock,
        (12 * SECOND, 10 * SECOND, 10 * SECOND),
        &[(nitrogen, 5.0, None)],
    );
    let one_window = Ledger::sliding(basis, 10 * SECOND, 10 * SECOND, 1e-6, 1e-9, 1e9).between_edges(
        clock,
        (12 * SECOND, 10 * SECOND, 10 * SECOND),
        &[(nitrogen, 5.0, None)],
    );
    let mut fold = CertificateFold::open(&session, &ledger.policy).expect("opened");
    let mut f_one = CertificateFold::open(&session, &one_window.policy).expect("opened");

    let session = step_folding(session, at(16 * SECOND), &mut [&mut fold, &mut f_one]);
    let stopped = fold.certificate().expect_err("tail [0, 16s] exceeds widest 10s");
    assert_eq!(
        (stopped.check(), stopped.window()),
        (CertificateCheck::RhythmStopped, Some(interval(0, 16 * SECOND)))
    );
    assert_fold_matches_certify(&session, &fold, &ledger, "rhythm/16s-stopped-tail");
    assert_fold_matches_certify(&session, &f_one, &one_window, "rhythm-one/16s-stopped-tail");

    let session = step_folding(session, at(24 * SECOND), &mut [&mut fold, &mut f_one]);
    let passed = fold
        .certificate()
        .expect("snapshot at 16s did not mutate stopped or widest");
    let rhythm = passed.rhythm().expect("rhythm result");
    assert_eq!(rhythm.last_close(), Some(at(24 * SECOND)));
    assert_eq!(rhythm.widest(), interval(0, 4 * SECOND));
    assert_eq!(rhythm.judged(), 1);
    assert_fold_matches_certify(&session, &fold, &ledger, "rhythm/24s-recovered");
    assert_fold_matches_certify(&session, &f_one, &one_window, "rhythm-one/24s-recovered");

    let session = step_folding(session, at(32 * SECOND), &mut [&mut fold, &mut f_one]);
    assert_fold_matches_certify(&session, &fold, &ledger, "rhythm/32s");
    assert_fold_matches_certify(&session, &f_one, &one_window, "rhythm-one/32s");
    assert_eq!(f_one.pieces(), 1, "one-window fixed check stays at one piece");
}

#[test]
fn the_fold_refuses_a_foreign_run_a_foreign_branch_a_skipped_step_and_a_repeated_step() {
    let case = TwoReservoirs::closed().expect("a body");
    let core = ReferenceIntegrator::new(60 * SECOND);
    let store = MemoryStore::default();
    let session_a = case.admit(&core, &store).expect("admitted");
    let session_b = admit_seeded(
        case.body(),
        case.packet().expect("packet"),
        BoundaryPort::new(Vec::new()).expect("port"),
        &core,
        Seed::new([9, 9, 9, 9]),
        &store,
    )
    .expect("admitted");
    let basis = session_a.conservation().basis();
    let ledger = Ledger::sliding(basis, 120 * SECOND, 180 * SECOND, 1e-24, 1e-9, 1.0);
    let mut fold = CertificateFold::open(&session_a, &ledger.policy).expect("opened");
    let origin = session_a.record().seal().digest();

    let (session_b, _) = accepted(session_b.step());
    assert_eq!(
        fold.advance(&session_b),
        Err(ConservationRefusal::OtherRun),
        "another run is refused"
    );
    assert_eq!(fold.seal().base, origin);

    let (session_a1, r1) = accepted(session_a.step());
    let saved_1 = session_a1.checkpoint().expect("checkpoint at rev 1");
    let (session_a2, _) = accepted(session_a1.step());
    assert_eq!(
        fold.advance(&session_a2),
        Err(ConservationRefusal::Gap {
            expected: origin,
            found: Some(r1.seal().digest()),
        }),
        "a skipped step is refused"
    );

    let session_a = Session::resume(
        &saved_1,
        session_a2.record().identity(),
        case.body(),
        &core,
        &store,
    )
    .expect("resumed at rev 1");
    fold.advance(&session_a).expect("rev 1 follows origin");
    let seal_1 = session_a.record().seal().digest();
    assert_eq!(
        fold.advance(&session_a),
        Err(ConservationRefusal::Gap {
            expected: seal_1,
            found: Some(origin),
        }),
        "a repeated revision is refused"
    );

    let branch = BranchId::new(Digest::of_canonical(b"child-branch"));
    let child = session_a.rewind(&saved_1, branch).expect("rewound");
    let mut child_fold = CertificateFold::open(&child, &ledger.policy).expect("opened on child");
    let saved_b1 = session_b.checkpoint().expect("checkpoint on run b");
    let child_b = session_b.rewind(&saved_b1, branch).expect("rewound run b");
    let (child_b, _) = accepted(child_b.step());
    assert_eq!(
        child_fold.advance(&child_b),
        Err(ConservationRefusal::OtherRun),
        "another run sharing the same caller branch id is still refused as OtherRun"
    );
    let (child, _) = accepted(child.step());
    assert_eq!(
        fold.advance(&child),
        Err(ConservationRefusal::OtherRun),
        "a child branch with the same previous seal is refused"
    );

    let (session_a, _) = accepted(session_a.step());
    fold.advance(&session_a)
        .expect("earlier refusals left the fold ready for rev 2");
    let (session_a, _) = accepted(session_a.step());
    fold.advance(&session_a).expect("rev 3");
    assert_fold_matches_certify(&session_a, &fold, &ledger, "after-refusals");
}

struct FaultsAt<'c> {
    inner: &'c ReferenceIntegrator,
    at: SimTime,
    armed: Cell<bool>,
}
impl CoupledSolver for FaultsAt<'_> {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if self.armed.get() && ticket.interval().start() == self.at {
            return Err(ticket.refuse(NumericalRefusal::IterationLimit));
        }
        self.inner.solve(problem, ticket)
    }
}

#[test]
fn resume_rewind_and_recover_rebuild_the_fold_from_the_verified_prefix_and_isolate_child_branches() {
    let case = TwoReservoirs::closed().expect("a body");
    let core = ReferenceIntegrator::new(60 * SECOND).with_floor(NonZeroU64::new(30 * SECOND).expect("floor"));
    let store = MemoryStore::default();
    let session = case.admit(&core, &store).expect("admitted");
    let basis = session.conservation().basis();
    let ledger = Ledger::sliding(basis, HOUR, 4 * HOUR, 1e-24, 1e-9, 10.0);

    let mut parent_fold = CertificateFold::open(&session, &ledger.policy).expect("opened");
    let parent = step_folding(session, at(2 * HOUR), &mut [&mut parent_fold]);
    let checkpoint_2h = parent.checkpoint().expect("checkpoint at 2h");
    let identity = parent.record().identity().clone();

    let resumed = Session::resume(&checkpoint_2h, &identity, case.body(), &core, &store).expect("resumed");
    let mut resumed_fold = CertificateFold::open(&resumed, &ledger.policy).expect("rebuilt");
    let resumed = step_folding(resumed, at(4 * HOUR), &mut [&mut resumed_fold]);
    assert_fold_matches_certify(&resumed, &resumed_fold, &ledger, "resumed-clean");
    drop(parent);

    let open = TwoReservoirs::delivering([measure(1_000.0), measure(1_000.0)], Vec::new()).expect("a body");
    let session = open.admit(&core, &store).expect("admitted");
    let open_ledger = Ledger::sliding(session.conservation().basis(), HOUR, 4 * HOUR, 1e-24, 1e-9, 10.0);
    let mut p_fold = CertificateFold::open(&session, &open_ledger.policy).expect("opened");
    let mut parent = step_folding(session, at(2 * HOUR), &mut [&mut p_fold]);
    let saved_2h = parent.checkpoint().expect("checkpoint at 2h");
    let late_event = BoundaryEvent::new(
        InputId::new(Digest::of_canonical(b"parent-drift")),
        parent.record().earliest_stamp(),
        BoundaryInput::Water(WaterDelivery {
            boundary: "exterior",
            amount: measure(500.0),
            ends: at(3 * HOUR),
        }),
    );
    parent
        .enqueue(AuthorizedInput::from_command_receipt(
            Digest::of_canonical(b"cmd"),
            late_event,
        ))
        .expect("enqueued");
    let parent = step_folding(parent, at(4 * HOUR), &mut [&mut p_fold]);
    assert_eq!(
        p_fold.certificate().expect_err("parent drifted").check(),
        CertificateCheck::Drift
    );
    assert_fold_matches_certify(&parent, &p_fold, &open_ledger, "parent-drifted");

    let child_branch = BranchId::new(Digest::of_canonical(b"clean-child"));
    let child = parent.rewind(&saved_2h, child_branch).expect("rewound");
    let mut child_fold = CertificateFold::open(&child, &open_ledger.policy).expect("rebuilt");
    let child = step_folding(child, at(4 * HOUR), &mut [&mut child_fold]);
    assert!(
        child_fold.certificate().is_ok(),
        "child gets parent's clean prefix and none of parent's later windows"
    );
    assert_fold_matches_certify(&child, &child_fold, &open_ledger, "child-clean");

    let early_drift = TwoReservoirs::delivering(
        [measure(1_000.0), measure(1_000.0)],
        vec![delivery(b"early", 0, 1_800, 1.0)],
    )
    .expect("a body");
    let session = early_drift.admit(&core, &store).expect("admitted");
    let early_ledger = Ledger::sliding(session.conservation().basis(), HOUR, 4 * HOUR, 1e-24, 1e-9, 10.0);
    let mut e_fold = CertificateFold::open(&session, &early_ledger.policy).expect("opened");
    let parent = step_folding(session, at(2 * HOUR), &mut [&mut e_fold]);
    let saved = parent.checkpoint().expect("saved after drift");
    let parent = step_folding(parent, at(3 * HOUR), &mut [&mut e_fold]);
    let child = parent
        .rewind(&saved, BranchId::new(Digest::of_canonical(b"inherits-drift")))
        .expect("rewound");
    let mut c_fold = CertificateFold::open(&child, &early_ledger.policy).expect("rebuilt");
    let child = step_folding(child, at(4 * HOUR), &mut [&mut c_fold]);
    let failed = c_fold.certificate().expect_err("inherits prefix drift");
    assert_eq!(
        (failed.check(), failed.window()),
        (CertificateCheck::Drift, Some(interval(0, HOUR)))
    );
    assert_fold_matches_certify(&child, &c_fold, &early_ledger, "child-inherits-prefix-drift");

    let faulting = FaultsAt {
        inner: &core,
        at: at(2 * HOUR),
        armed: Cell::new(true),
    };
    let session = open.admit(&faulting, &store).expect("admitted");
    let mut f_fold = CertificateFold::open(&session, &open_ledger.policy).expect("opened");
    let session = step_folding(session, at(2 * HOUR), &mut [&mut f_fold]);
    let mut attempt = session.step();
    let mut stopped = loop {
        match attempt {
            Step::Refused(retrying) => attempt = retrying.retry(),
            Step::Faulted(stopped) => break stopped,
            Step::Accepted(..) => panic!("expected fault at 2h"),
        }
    };
    let report = stopped.export().expect("exported");
    faulting.armed.set(false);
    let recovered = stopped
        .recover(
            report.last_accepted(),
            BranchId::new(Digest::of_canonical(b"recovered-branch")),
        )
        .expect("recovered");
    let mut r_fold = CertificateFold::open(&recovered, &open_ledger.policy).expect("rebuilt");
    let recovered = step_folding(recovered, at(4 * HOUR), &mut [&mut r_fold]);
    assert!(r_fold.certificate().is_ok());
    assert_fold_matches_certify(&recovered, &r_fold, &open_ledger, "recovered-branch");
}

#[test]
fn a_window_at_24h_lag_is_judged_on_the_step_that_holds_its_end() {
    let step = 60 * SECOND;
    let case = TwoReservoirs::delivering(
        [measure(1_000.0), measure(1_000.0)],
        vec![delivery(
            b"late-burst",
            (36 * HOUR - step) / SECOND,
            (36 * HOUR) / SECOND,
            10.0,
        )],
    )
    .expect("a body");
    let (core, store) = (ReferenceIntegrator::new(step), MemoryStore::default());
    let session = case.admit(&core, &store).expect("admitted");
    let basis = session.conservation().basis();
    let one_day = Ledger::sliding(basis, DAY, DAY, 1e-24, 1e-9, 100.0);
    let sliding_25h = Ledger::sliding(basis, DAY, 25 * HOUR, 1e-24, 1e-9, 100.0);
    let two_days_passing = Ledger::sliding(basis, DAY, 2 * DAY, 1e-24, 1e-9, 1e6);
    let mut f_one = CertificateFold::open(&session, &one_day.policy).expect("opened");
    let mut f_mid = CertificateFold::open(&session, &sliding_25h.policy).expect("opened");
    let mut f_pass = CertificateFold::open(&session, &two_days_passing.policy).expect("opened");

    let session = step_folding(
        session,
        at(DAY - step),
        &mut [&mut f_one, &mut f_mid, &mut f_pass],
    );
    assert_eq!(f_one.pieces(), 0, "no window judged before 24h");
    assert_eq!(f_mid.pieces(), 0, "no sliding window judged before 24h");
    assert_eq!(
        f_one.certificate().expect_err("short of 24h").check(),
        CertificateCheck::Coverage
    );

    let (session, _) = accepted(session.step());
    assert_eq!(session.record().accepted().time(), at(DAY));
    f_one.advance(&session).expect("rev 1440");
    f_mid.advance(&session).expect("rev 1440");
    f_pass.advance(&session).expect("rev 1440");
    assert_eq!(
        f_one.pieces(),
        1,
        "one-window [0, 24h] judged on the step holding 24h"
    );
    assert!(f_one.certificate().is_ok(), "quiet through 24h");
    assert_fold_matches_certify(&session, &f_one, &one_day, "24h-lag/one-window");

    let (session, _) = accepted(session.step());
    f_one.advance(&session).expect("rev 1441");
    f_mid.advance(&session).expect("rev 1441");
    f_pass.advance(&session).expect("rev 1441");
    assert_eq!(f_mid.pieces(), 1, "first sliding piece judged on [24h, 24h+60s]");
    assert_eq!(f_one.pieces(), 1, "one-window stays at one piece");

    let session = step_folding(
        session,
        at(36 * HOUR - step),
        &mut [&mut f_one, &mut f_mid, &mut f_pass],
    );
    assert!(f_mid.certificate().is_ok(), "every 24h window before 36h passes");

    let session = step_folding(session, at(36 * HOUR), &mut [&mut f_one, &mut f_mid, &mut f_pass]);
    let drifted = f_mid
        .certificate()
        .expect_err("window starting 24h earlier fails on the step reaching 36h");
    assert_eq!(drifted.check(), CertificateCheck::Drift);
    assert_eq!(drifted.window().expect("a window").length_ns(), DAY);
    assert!(
        drifted.window().expect("a window").end() <= at(36 * HOUR)
            && drifted.window().expect("a window").start() >= at(12 * HOUR - step),
        "first drifting window closes inside the burst at 36h: {:?}",
        drifted.window()
    );
    assert_fold_matches_certify(&session, &f_mid, &sliding_25h, "24h-lag/at-36h");

    let session = step_folding(session, at(2 * DAY), &mut [&mut f_one, &mut f_mid, &mut f_pass]);
    assert_fold_matches_certify(&session, &f_mid, &sliding_25h, "24h-lag/two-days-drift");
    assert_fold_matches_certify(&session, &f_pass, &two_days_passing, "24h-lag/two-days-pass");
}

#[derive(Default)]
struct DroppingStore {
    kept: MemoryStore,
    lost: RefCell<Vec<Digest>>,
}
impl HistoryStore for DroppingStore {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        if self.lost.borrow().contains(seal) {
            return None;
        }
        self.kept.resolve(seal)
    }
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        self.kept.keep(seal, link)
    }
}

#[test]
fn the_fold_reproduces_coverage_before_a_lost_link_and_lost_link_during_run_rules() {
    let (case, store) = (drifts_then_opens(), DroppingStore::default());
    let core = ReferenceIntegrator::new(60 * SECOND);
    let session = case.admit(&core, &store).expect("admitted");
    let basis = session.conservation().basis();
    let cover_4h = Ledger::sliding(basis, HOUR, 4 * HOUR, 1e-24, 1e-9, 1.0);
    let cover_5h = Ledger::sliding(basis, HOUR, 5 * HOUR, 1e-24, 1e-9, 1.0);
    let mut f4 = CertificateFold::open(&session, &cover_4h.policy).expect("opened");
    let mut f5 = CertificateFold::open(&session, &cover_5h.policy).expect("opened");

    let session = step_folding(session, at(HOUR), &mut [&mut f4, &mut f5]);
    let lost = session.record().seal().digest();
    store.lost.borrow_mut().push(lost);
    let session = step_folding(session, at(4 * HOUR), &mut [&mut f4, &mut f5]);

    assert_eq!(
        f4.certificate().expect_err("lost link under 4h coverage").check(),
        CertificateCheck::History(CheckpointError::HistoryMissing { seal: lost })
    );
    assert_eq!(
        f5.certificate()
            .expect_err("coverage before lost link under 5h")
            .check(),
        CertificateCheck::Coverage
    );
    let rebuilt_4 = CertificateFold::open(&session, &cover_4h.policy).expect("opened");
    let rebuilt_5 = CertificateFold::open(&session, &cover_5h.policy).expect("opened");
    assert_eq!(
        rebuilt_4.certificate().expect_err("rebuilt 4h").check(),
        CertificateCheck::History(CheckpointError::HistoryMissing { seal: lost })
    );
    assert_eq!(
        rebuilt_5.certificate().expect_err("rebuilt 5h").check(),
        CertificateCheck::Coverage
    );

    let (case, store) = (TwoReservoirs::closed().expect("a body"), DroppingStore::default());
    let session = case.admit(&core, &store).expect("admitted");
    let ledger = Ledger::sliding(session.conservation().basis(), HOUR, 4 * HOUR, 1e-24, 1e-9, 1.0);
    let mut fold = CertificateFold::open(&session, &ledger.policy).expect("opened");
    let mut early_seals = Vec::new();
    let mut session = session;
    while session.record().accepted().time() < at(2 * HOUR + 5 * 60 * SECOND) {
        let receipt;
        (session, receipt) = accepted(session.step());
        fold.advance(&session).expect("advanced");
        if receipt.revision() <= 64 {
            early_seals.push(receipt.seal().digest());
        }
    }
    store.lost.borrow_mut().extend(early_seals);
    let session = step_folding(session, at(4 * HOUR), &mut [&mut fold]);
    let live_cert = fold
        .certificate()
        .expect("links dropped after trailing passed them are never read again");
    let clean_store = MemoryStore::default();
    let clean = case.admit(&core, &clean_store).expect("admitted");
    let Ok(clean) = run_until(clean, at(4 * HOUR)) else {
        panic!("clean run faulted");
    };
    let expected = certify(&clean.conservation(), &ledger.policy).expect("clean cert");
    assert_eq!(format!("{live_cert:?}"), format!("{expected:?}"));
    drop(session);

    let (case, store) = (TwoReservoirs::closed().expect("a body"), DroppingStore::default());
    let core = ReferenceIntegrator::new(SECOND);
    let mut session = case.admit(&core, &store).expect("admitted");
    let ledger = Ledger::sliding(
        session.conservation().basis(),
        150 * SECOND,
        300 * SECOND,
        1e-24,
        1e-9,
        1.0,
    );
    let mut fold = CertificateFold::open(&session, &ledger.policy).expect("opened");
    let mut seals = Vec::new();
    for _ in 0..130 {
        let receipt;
        (session, receipt) = accepted(session.step());
        fold.advance(&session).expect("advanced");
        seals.push(receipt.seal().digest());
    }
    let (rev_64, rev_128) = (seals[63], seals[127]);
    store.lost.borrow_mut().extend([rev_64, rev_128]);
    let _session = step_folding(session, at(300 * SECOND), &mut [&mut fold]);
    assert_eq!(
        fold.certificate().expect_err("oldest damaged page first").check(),
        CertificateCheck::History(CheckpointError::HistoryMissing { seal: rev_64 })
    );
}

fn step_folding_peak<'m>(
    mut session: Session<'m, Running>,
    end: SimTime,
    fold: &mut CertificateFold<'m, '_>,
) -> (Session<'m, Running>, usize) {
    let mut peak = fold.memory_bytes();
    while session.record().accepted().time() < end {
        let mut attempt = session.step();
        session = loop {
            match attempt {
                Step::Accepted(next, _) => {
                    fold.advance(&next).expect("follows the previous accepted step");
                    peak = peak.max(fold.memory_bytes());
                    break next;
                }
                Step::Refused(retrying) => attempt = retrying.retry(),
                Step::Faulted(stopped) => panic!("faulted: {:?}", stopped.fault()),
            }
        };
    }
    (session, peak)
}

#[test]
fn the_fold_holds_bounded_memory_over_48_hours_and_does_not_grow_after_one_window() {
    let case = TwoReservoirs::closed().expect("a body");
    let (core, store) = (ReferenceIntegrator::new(60 * SECOND), MemoryStore::default());
    let session = case.admit(&core, &store).expect("admitted");
    let basis = session.conservation().basis();
    let sliding = Ledger::sliding(basis, DAY, 2 * DAY, 1e-24, 1e-9, 1.0);
    let one = Ledger::sliding(basis, DAY, DAY, 1e-24, 1e-9, 1.0);
    let mut f_sliding = CertificateFold::open(&session, &sliding.policy).expect("opened");
    let mut f_one = CertificateFold::open(&session, &one.policy).expect("opened");

    let session = step_folding(session, at(DAY), &mut [&mut f_sliding, &mut f_one]);
    let one_after_close = f_one.memory_bytes();
    let (session, peak_first_quarter) = step_folding_peak(session, at(30 * HOUR), &mut f_sliding);
    let (session, peak_rest_of_48h) = step_folding_peak(session, at(2 * DAY), &mut f_sliding);
    assert!(
        peak_rest_of_48h <= peak_first_quarter,
        "peak memory must plateau after the first trailing page: 24h..30h={peak_first_quarter} B, 30h..48h={peak_rest_of_48h} B"
    );
    assert!(
        peak_rest_of_48h < 64 * 1024,
        "48h two-reservoir fold stayed under 64 KiB, measured {peak_rest_of_48h} B"
    );
    assert!(
        one_after_close < 4 * 1024,
        "one-window fold releases trailing marks after judging: {one_after_close} B"
    );
    assert_fold_matches_certify(&session, &f_sliding, &sliding, "48h-memory/sliding");

    let (core, store) = (ng_solver::Core::new(100 * MILLISECOND), MemoryStore::default());
    let session = case.admit(&core, &store).expect("admitted");
    let dense = Ledger::sliding(
        session.conservation().basis(),
        60 * SECOND,
        300 * SECOND,
        1e-24,
        1e-9,
        1.0,
    );
    let mut f_dense = CertificateFold::open(&session, &dense.policy).expect("opened");
    let (session, _) = step_folding_peak(session, at(60 * SECOND), &mut f_dense);
    let (session, peak_window_2) = step_folding_peak(session, at(120 * SECOND), &mut f_dense);
    let (session, peak_window_3_to_5) = step_folding_peak(session, at(300 * SECOND), &mut f_dense);
    assert!(
        peak_window_3_to_5 <= peak_window_2,
        "dense fold peak must plateau after one full rolling window: 60..120s={peak_window_2} B, 120..300s={peak_window_3_to_5} B"
    );
    assert!(
        peak_window_3_to_5 < 64 * 1024,
        "dense 3,000-step fold stayed under 64 KiB, measured {peak_window_3_to_5} B"
    );
    assert_fold_matches_certify(&session, &f_dense, &dense, "dense-3000-steps");

    let body = arena_body().expect("compiled");
    let packet = arena_packet(&body).expect("a packet");
    let lens = arena_lens(&body).expect("a lens");
    let core = ng_solver::Core::new(10 * MILLISECOND);
    let store = MemoryStore::default();
    let origin = anchor_moment();
    let session = admit_at(
        &body,
        packet,
        BoundaryPort::new(Vec::new()).expect("no events"),
        &core,
        &lens,
        origin,
        &store,
    )
    .expect("admitted")
    .session;
    let slice_policy = Ledger::sliding(
        session.conservation().basis(),
        SECOND,
        3 * SECOND,
        0.0,
        1e-12,
        1e4,
    );
    let mut f_slice = CertificateFold::open(&session, &slice_policy.policy).expect("opened");
    let (session, _) = step_folding_peak(session, at(origin.nanoseconds() + SECOND), &mut f_slice);
    let (session, peak_slice_2) =
        step_folding_peak(session, at(origin.nanoseconds() + 2 * SECOND), &mut f_slice);
    let (session, peak_slice_3) =
        step_folding_peak(session, at(origin.nanoseconds() + 3 * SECOND), &mut f_slice);
    assert!(
        peak_slice_3 <= peak_slice_2,
        "slice fold peak plateaued across rolling windows: 1..2s={peak_slice_2} B, 2..3s={peak_slice_3} B"
    );
    assert!(
        peak_slice_3 < 3 * 1024 * 1024,
        "slice fold stayed under 3 MiB, measured {peak_slice_3} B"
    );
    assert_fold_matches_certify(&session, &f_slice, &slice_policy, "slice-3s-memory");
}

#[test]
fn the_fold_checks_basis_and_undeclared_rhythm_before_any_step() {
    let body = breathing_body();
    let (core, store) = (
        ReferenceIntegrator::new(250 * MILLISECOND),
        MemoryStore::default(),
    );
    let session = admit_fixture(
        &body,
        breathing_packet(4.0, 1.0),
        BoundaryPort::new(Vec::new()).expect("no events"),
        &core,
        &store,
    )
    .expect("admitted");
    let basis = session.conservation().basis();
    let missing_clock = Slot::from(LawName {
        owner: "Boundary",
        local: "other",
    });
    let undeclared = Ledger::sliding(basis, 10 * SECOND, 10 * SECOND, 1e-6, 1e-9, 1e9).between_edges(
        missing_clock,
        (0, 10 * SECOND, 15 * SECOND),
        &[],
    );
    assert_eq!(
        CertificateFold::open(&session, &undeclared.policy)
            .err()
            .map(|failure| failure.check()),
        Some(CertificateCheck::UndeclaredRhythm)
    );

    let other_case = TwoReservoirs::closed().expect("a body");
    let other_policy = Ledger::sliding(
        other_case.body().conserved_basis(),
        10 * SECOND,
        10 * SECOND,
        1e-6,
        1e-9,
        1e9,
    );
    assert_eq!(
        CertificateFold::open(&session, &other_policy.policy)
            .err()
            .map(|failure| failure.check()),
        Some(CertificateCheck::Coverage)
    );

    let valid = Ledger::sliding(basis, 10 * SECOND, 10 * SECOND, 1e-6, 1e-9, 1e9);
    let fold = CertificateFold::open(&session, &valid.policy).expect("opened at admission");
    assert_eq!(fold.span(), None);
    assert_eq!(
        fold.certificate().expect_err("no step yet").check(),
        CertificateCheck::Coverage
    );
}
