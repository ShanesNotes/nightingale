use core::cell::RefCell;
use core::num::NonZeroU64;
use ng_contract::*;
use ng_record::{BranchId, Checkpoint, CheckpointError, HistoryStore, Running, Session, Step, Witness};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MINUTE: u64 = 60 * SECOND;
const HEADER: usize = 128;

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at(ns: u64) -> SimTime {
    SimTime::from_ns(ns)
}

fn delivery(id: &'static [u8], from_s: u64, to_s: u64, rate: f64) -> Delivery {
    Delivery {
        id,
        from: at(from_s * SECOND),
        to: at(to_s * SECOND),
        amount: measure(rate * (to_s - from_s) as f64),
    }
}

fn resealed(mut bytes: Vec<u8>) -> Checkpoint {
    let raw = Digest::of_canonical(&bytes[40..]);
    bytes[8..40].copy_from_slice(raw.as_bytes());
    Checkpoint::from_bytes(bytes)
}

fn trailer_offset(bytes: &[u8]) -> usize {
    let image_len = u64::from_be_bytes(*bytes[HEADER..].first_chunk().expect("image length")) as usize;
    let branch_start = HEADER + 8 + image_len;
    let forks = u64::from_be_bytes(*bytes[branch_start + 32..].first_chunk().expect("fork count")) as usize;
    let opening_start = branch_start + 32 + 8 + forks * 48;
    let opening_count =
        u64::from_be_bytes(*bytes[opening_start..].first_chunk().expect("opening count")) as usize;
    opening_start + 8 + opening_count * 16 + 32
}

fn policy_for(
    basis: &ConservedBasis,
    window_s: u64,
    coverage_s: u64,
    abs: f64,
    rel: f64,
    drift: f64,
) -> CertificatePolicy {
    let entries = basis
        .ids()
        .map(|quantity| {
            QuantityPolicy::new(
                quantity,
                measure::<Water>(abs),
                measure(rel),
                measure::<Water>(drift),
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .expect("water basis");
    let windows = SlidingWindows::new(
        NonZeroU64::new(window_s * SECOND).expect("positive"),
        coverage_s * SECOND,
    )
    .expect("valid windows");
    CertificatePolicy::new(basis, entries, ExpectedDrift::Zero, windows).expect("policy")
}

fn step_accepted<'m>(mut session: Session<'m, Running>) -> Session<'m, Running> {
    let mut attempt = session.step();
    session = loop {
        match attempt {
            Step::Accepted(next, _) => break next,
            Step::Refused(retrying) => attempt = retrying.retry(),
            Step::Faulted(stopped) => panic!("faulted: {:?}", stopped.fault()),
        }
    };
    session
}

struct TamperedWitnessStore<'a> {
    inner: &'a RollingStore,
    altered: Option<(Digest, Witness)>,
    missing: Option<Digest>,
}
impl HistoryStore for TamperedWitnessStore<'_> {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        self.inner.resolve(seal)
    }
    fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
        self.inner.keep(seal, link)
    }
    fn resolve_witness(&self, seal: &Digest) -> Option<Witness> {
        if self.missing == Some(*seal) {
            return None;
        }
        if let Some((target, replacement)) = self.altered
            && target == *seal
        {
            return Some(replacement);
        }
        self.inner.resolve_witness(seal)
    }
    fn keep_with_witness(&self, seal: Digest, link: &[u8], witness: Witness) -> Result<(), CheckpointError> {
        self.inner.keep_with_witness(seal, link, witness)
    }
}

struct FaultingCore {
    inner: ReferenceIntegrator,
    fault_after_s: u64,
}
impl CoupledSolver for FaultingCore {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.interval().end().nanoseconds() > self.fault_after_s * SECOND {
            return Err(ticket.mismatch().into());
        }
        self.inner.solve(problem, ticket)
    }
}

struct PerturbedCore {
    inner: ReferenceIntegrator,
    perturb_after_s: u64,
}
impl CoupledSolver for PerturbedCore {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        if accepted.time().nanoseconds() >= self.perturb_after_s * SECOND {
            self.inner.next_step(accepted) / 2
        } else {
            self.inner.next_step(accepted)
        }
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        self.inner.solve(problem, ticket)
    }
}

fn origin() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn breathing_body() -> CompiledBody {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let vessel = map.site::<VesselSegment>("vessel", origin());
    map.contain(root.id(), vessel.id());
    let bed = map.site::<CapillaryBed>("bed", origin()).id();
    map.contain(root.id(), bed);
    map.connect(Network::Vascular, vessel.id(), bed);
    map.connect(Network::Vascular, bed, vessel.id());
    let mut schema = Schema::new(map.finish().expect("map")).expect("schema");
    schema.leave_out(Space::Plasma, "bed only");
    schema.leave_out(Space::Cells, "bed only");
    schema.leave_out(Space::Lumen, "no blood");
    schema.carry::<Water, CapillaryBed>(Space::Interstitium);
    schema
        .boundary::<Water>("leak", BoundaryDirection::IntoBody, bed, Space::Interstitium)
        .expect("boundary");
    let class = RateClass::new(RateBand::Wave, 3_600 * SECOND).expect("class");
    assemble(schema, |mut body| {
        let (mut laws, ports) = body.boundary()?.parts();
        let phase = laws.state::<Dimensionless>("phase", Admissible::nonnegative());
        let pace =
            laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.pace"), Admissible::positive());
        let running = laws.kernel(|mut k| {
            let (pace, moving) = (k.parameter(&pace), k.room_air_speed());
            let value = k.scale(pace, moving);
            k.finish(value, laws.every_mode(), class)
        });
        laws.memory_rate(&phase, running);
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
        let BoundaryRoute { route, .. } = ports.route::<Water>("leak")?;
        let drift = laws.parameter::<Rate<Water>>(ParameterKey::new("bd.drift"), Admissible::finite());
        let rate = laws.kernel(|mut k| {
            let (drift, gate) = (k.parameter(&drift), k.room_humidity());
            let value = k.scale(drift, gate);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&route, rate));
        body.add(laws.finish());
        body.compile()
    })
    .expect("compiled")
}

fn room_input(leaking: bool, breathing: bool) -> BoundaryInput {
    let gate = |open: bool| measure(if open { 1.0 } else { 0.0 });
    BoundaryInput::Room(Room {
        air: measure(307.0),
        radiant: measure(308.0),
        humidity: gate(leaking),
        air_speed: gate(breathing),
        pressure: measure(765.0),
        oxygen: measure(0.21),
        carbon_dioxide: measure(0.0),
        clothing: measure(0.9),
    })
}

fn breathing_packet(body: &CompiledBody, leak_rate: f64) -> ParameterPacket<SyntheticFixture> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", origin());
    let vessel = map.site::<VesselSegment>("vessel", origin());
    map.contain(root.id(), vessel.id());
    let bed = map.site::<CapillaryBed>("bed", origin()).id();
    let _ = body;
    let mut packet = ParameterPacket::new();
    let named = |local: &'static str| LawName {
        owner: "Boundary",
        local,
    };
    packet
        .synthetic(
            ParameterKey::new("bd.pace"),
            Binding::Body,
            measure::<Rate<Dimensionless>>(0.5),
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
            measure::<Rate<Water>>(leak_rate),
            "synthetic",
        )
        .expect("once");
    packet
        .start(
            RowKind::Amount {
                site: bed,
                space: Space::Interstitium,
                species: Species::Water,
            },
            measure::<Water>(1_000.0),
            "synthetic",
        )
        .expect("once");
    packet
        .start(
            RowKind::Memory(named("phase").into()),
            measure::<Dimensionless>(0.0),
            "synthetic",
        )
        .expect("once");
    packet
        .start_mode(named("beat"), "tick", "synthetic")
        .expect("once");
    packet
}

#[test]
fn a_rolling_run_trims_during_execution_and_keeps_only_witnesses_and_checkpoints_after_passing() {
    let case = TwoReservoirs::delivering(
        [measure(500.0), measure(500.0)],
        vec![delivery(b"sip", 10, 30, 0.5), delivery(b"sip2", 140, 160, 0.25)],
    )
    .expect("case");
    let end = at(240 * SECOND);
    let policy = policy_for(case.body().conserved_basis(), 60, 240, 1e-9, 1e-12, 50.0);

    let untrimmed_core = ReferenceIntegrator::new(2 * SECOND);
    let untrimmed_store = MemoryStore::default();
    let untrimmed = run_until(
        case.admit(&untrimmed_core, &untrimmed_store).expect("admitted"),
        end,
    )
    .ok()
    .expect("completed");
    let want_cert = certify(&untrimmed.conservation(), &policy).expect("certified");

    let rolling_core = ReferenceIntegrator::new(2 * SECOND);
    let rolling_store = RollingStore::new();
    let mut session = case.admit(&rolling_core, &rolling_store).expect("admitted");
    rolling_store.record_checkpoint(&session).expect("rev 0 ckpt");
    let mut fold = CertificateFold::open(&session, &policy).expect("fold");
    let mut next_ckpt_ns = MINUTE;
    let mut peak_retained_links = 0usize;
    let mut saw_trimmed_during_run = false;
    while session.record().accepted().time() < end {
        session = step_accepted(session);
        fold.advance(&session).expect("advance");
        let t = session.record().accepted().time().nanoseconds();
        if t >= next_ckpt_ns {
            rolling_store.record_checkpoint(&session).expect("minute ckpt");
            next_ckpt_ns += MINUTE;
        }
        rolling_store.trim_before(fold.oldest_open_window_start(), fold.oldest_retained_revision());
        let live = rolling_store.listing();
        peak_retained_links = peak_retained_links.max(live.complete_link_revisions.len());
        if live.witness_count > 70 && live.complete_link_revisions.len() < live.witness_count {
            saw_trimmed_during_run = true;
        }
    }
    assert!(
        saw_trimmed_during_run,
        "rolling trim must drop old complete links during the run"
    );
    let total_steps = session.record().revision() as usize;
    assert!(
        total_steps > 100,
        "crosses the 64-link page boundary: {total_steps}"
    );
    assert!(
        peak_retained_links < total_steps,
        "retained peak {peak_retained_links} must stay below full {total_steps}"
    );
    let got_cert = fold.certificate().expect("fold passed");
    assert_eq!(format!("{got_cert:?}"), format!("{want_cert:?}"));

    rolling_store.finish();
    let listing = rolling_store.listing();
    assert_eq!(listing.witness_count, total_steps);
    assert_eq!(listing.witness_bytes, total_steps * 96);
    assert_eq!(
        listing.checkpoint_revisions.len(),
        5,
        "rev 0 + 4 minute checkpoints"
    );
    assert!(
        listing.complete_link_revisions.is_empty(),
        "passing finished run keeps zero complete links outside checkpoints"
    );
    assert_eq!(listing.complete_link_bytes, 0);
    assert!(listing.flight_recorder.is_none());

    let hash = session.record().record_hash();
    let witnesses = rolling_store.witnesses();
    for (idx, entry) in witnesses.iter().enumerate() {
        let rev = idx as u64 + 1;
        let w = rolling_store.resolve_witness(&entry.seal).expect("witness");
        assert_eq!(
            w.seal(hash, rev),
            entry.seal,
            "witness recomputes seal at rev {rev}"
        );
    }

    let trimmed_err = certify(&session.conservation(), &policy).expect_err("trimmed history refuses certify");
    assert!(
        matches!(
            trimmed_err.check(),
            CertificateCheck::History(CheckpointError::HistoryTrimmed { .. })
        ),
        "certify must report HistoryTrimmed without running a core, got {:?}",
        trimmed_err.check()
    );

    let last_trimmed_seal = witnesses[witnesses.len() - 2].seal;
    let good_w = rolling_store
        .resolve_witness(&last_trimmed_seal)
        .expect("witness");
    let mut flipped = *good_w.ledger().as_bytes();
    flipped[0] ^= 1;
    let bad_w = Witness::new(good_w.previous(), good_w.image(), Digest::from_bytes(flipped));
    let tampered = TamperedWitnessStore {
        inner: &rolling_store,
        altered: Some((last_trimmed_seal, bad_w)),
        missing: None,
    };
    let head_ckpt = rolling_store
        .checkpoint_at_or_before_revision(session.record().revision())
        .expect("ckpt");
    let resumed_tampered = Session::resume(
        &head_ckpt.checkpoint,
        session.record().identity(),
        case.body(),
        &rolling_core,
        &rolling_store,
    )
    .expect("resumed");
    let _ = resumed_tampered;
    let altered_err = ng_record::retained_span(
        case.body(),
        session.record().identity(),
        &head_ckpt.checkpoint,
        head_ckpt.seal,
        head_ckpt.revision,
        &tampered,
    )
    .err();
    assert_eq!(
        altered_err,
        Some(CheckpointError::HistoryAltered {
            seal: last_trimmed_seal
        })
    );

    let page_case = TwoReservoirs::delivering([measure(500.0), measure(500.0)], Vec::new()).expect("case");
    let page_policy = policy_for(page_case.body().conserved_basis(), 70, 140, 1e-9, 1e-12, 50.0);
    let page_core = ReferenceIntegrator::new(SECOND);
    let page_store = RollingStore::new();
    let mut page_session = page_case.admit(&page_core, &page_store).expect("admitted");
    page_store.record_checkpoint(&page_session).expect("rev 0");
    let mut page_fold = CertificateFold::open(&page_session, &page_policy).expect("fold");
    for step_idx in 1..=65 {
        page_session = step_accepted(page_session);
        page_fold.advance(&page_session).expect("advance");
        if step_idx == 65 {
            assert_eq!(
                page_fold.oldest_retained_revision(),
                Some(1),
                "oldest_retained_revision must return page 0's floor (1), not its newest revision (64)"
            );
            page_store.trim_before(None, page_fold.oldest_retained_revision());
            assert_eq!(
                page_store.listing().complete_link_revisions.first().copied(),
                Some(1)
            );
        }
    }
    while page_session.record().accepted().time() < at(140 * SECOND) {
        page_session = step_accepted(page_session);
        page_fold.advance(&page_session).expect("advance");
        page_store.trim_before(
            page_fold.oldest_open_window_start(),
            page_fold.oldest_retained_revision(),
        );
    }
    assert!(
        page_fold.certificate().is_ok(),
        "rolling trim must respect ConservationFeed's 64-link page floor"
    );
}

#[test]
fn a_trimmed_restore_checks_every_seal_head_link_and_opening_without_reparsing_discarded_ledgers() {
    let case = TwoReservoirs::delivering(
        [measure(500.0), measure(500.0)],
        vec![delivery(b"sip", 10, 20, 1.0)],
    )
    .expect("case");
    let end = at(180 * SECOND);
    let policy = policy_for(case.body().conserved_basis(), 60, 180, 1e-9, 1e-12, 100.0);
    let core = ReferenceIntegrator::new(3 * SECOND);
    let store = RollingStore::new();
    let session = case.admit(&core, &store).expect("admitted");
    let outcome = run_rolling(
        session,
        end,
        &policy,
        &store,
        RollingConfig {
            checkpoint_every_ns: NonZeroU64::new(30 * SECOND).expect("positive"),
        },
    )
    .expect("rolling");
    assert!(outcome.certificate.is_ok());
    assert!(outcome.listing.complete_link_revisions.is_empty());
    let RollingStatus::Completed(finished) = outcome.status else {
        panic!("expected completed");
    };
    let identity = finished.record().identity().clone();
    let saved = store
        .checkpoint_at_or_before_revision(finished.record().revision())
        .expect("saved ckpt");
    assert!(saved.revision > 10);

    store.reset_resolve_calls();
    let fresh = ReferenceIntegrator::new(3 * SECOND);
    let resumed = Session::resume(&saved.checkpoint, &identity, case.body(), &fresh, &store)
        .expect("trimmed restore succeeds from witnesses + embedded head link");
    assert_eq!(
        store.resolve_calls(),
        0,
        "trimmed restore must resolve and reparse zero discarded links"
    );
    assert_eq!(resumed.record().revision(), saved.revision);
    assert_eq!(resumed.record().seal().digest(), saved.seal);
    assert_eq!(
        resumed.conservation().span().expect("span").start(),
        at(0),
        "trimmed restore takes opening time from the record opening"
    );

    let earlier = store
        .checkpoint_at_or_before_revision(saved.revision - 5)
        .expect("earlier");
    assert!(earlier.revision > 0 && earlier.revision < saved.revision);
    store.reset_resolve_calls();
    let rewound = finished
        .rewind(
            &earlier.checkpoint,
            BranchId::new(Digest::of_canonical(b"trimmed-fork")),
        )
        .expect("rewind across trimmed chain");
    assert_eq!(
        store.resolve_calls(),
        0,
        "History::holds must walk witnesses without resolving discarded links"
    );
    assert_eq!(rewound.record().revision(), earlier.revision);
    assert_eq!(rewound.record().seal().digest(), earlier.seal);

    let target_seal = store.witness_seals()[2];
    let good_w = store.resolve_witness(&target_seal).expect("witness");
    let mut bad_prev = *good_w.previous().as_bytes();
    bad_prev[0] ^= 1;
    let bad_prev_w = Witness::new(Digest::from_bytes(bad_prev), good_w.image(), good_w.ledger());
    let mut bad_img = *good_w.image().as_bytes();
    bad_img[0] ^= 1;
    let bad_img_w = Witness::new(good_w.previous(), Digest::from_bytes(bad_img), good_w.ledger());
    let mut bad_ledger = *good_w.ledger().as_bytes();
    bad_ledger[0] ^= 1;
    let bad_ledger_w = Witness::new(good_w.previous(), good_w.image(), Digest::from_bytes(bad_ledger));

    for bad_w in [bad_prev_w, bad_img_w, bad_ledger_w] {
        let tampered = TamperedWitnessStore {
            inner: &store,
            altered: Some((target_seal, bad_w)),
            missing: None,
        };
        assert_eq!(
            Session::resume(&saved.checkpoint, &identity, case.body(), &fresh, &tampered).err(),
            Some(CheckpointError::HistoryAltered { seal: target_seal }),
            "tampered witness must fail restore"
        );
    }

    let mid_seal = store.witness_seals()[(earlier.revision as usize) + 1];
    let mid_good = store.resolve_witness(&mid_seal).expect("witness");
    let mut bad_mid_prev = *mid_good.previous().as_bytes();
    bad_mid_prev[0] ^= 1;
    let bad_mid_w = Witness::new(
        Digest::from_bytes(bad_mid_prev),
        mid_good.image(),
        mid_good.ledger(),
    );
    struct ToggleWitnessStore<'a> {
        inner: &'a RollingStore,
        altered: RefCell<Option<(Digest, Witness)>>,
    }
    impl HistoryStore for ToggleWitnessStore<'_> {
        fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
            self.inner.resolve(seal)
        }
        fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
            self.inner.keep(seal, link)
        }
        fn resolve_witness(&self, seal: &Digest) -> Option<Witness> {
            if let Some((target, replacement)) = *self.altered.borrow()
                && target == *seal
            {
                return Some(replacement);
            }
            self.inner.resolve_witness(seal)
        }
    }
    let toggled = ToggleWitnessStore {
        inner: &store,
        altered: RefCell::new(None),
    };
    let resumed_for_rewind =
        Session::resume(&saved.checkpoint, &identity, case.body(), &fresh, &toggled).expect("resumed");
    *toggled.altered.borrow_mut() = Some((mid_seal, bad_mid_w));
    assert_eq!(
        resumed_for_rewind
            .rewind(
                &earlier.checkpoint,
                BranchId::new(Digest::of_canonical(b"tampered-holds-fork"))
            )
            .err(),
        Some(CheckpointError::HistoryAltered { seal: mid_seal }),
        "History::holds must verify every witness seal while walking back"
    );

    let missing_store = TamperedWitnessStore {
        inner: &store,
        altered: None,
        missing: Some(target_seal),
    };
    assert_eq!(
        Session::resume(&saved.checkpoint, &identity, case.body(), &fresh, &missing_store).err(),
        Some(CheckpointError::HistoryMissing { seal: target_seal })
    );

    let raw = saved.checkpoint.as_bytes();
    let offset = trailer_offset(raw);

    let mut late_opened = raw.to_vec();
    let bad_time = saved.time.nanoseconds() + 1;
    late_opened[offset..offset + 8].copy_from_slice(&bad_time.to_be_bytes());
    assert_eq!(
        Session::resume(&resealed(late_opened), &identity, case.body(), &fresh, &store).err(),
        Some(CheckpointError::Malformed),
        "opened past head start must be refused"
    );

    let mut shifted_opened = earlier.checkpoint.as_bytes().to_vec();
    let e_offset = trailer_offset(&shifted_opened);
    shifted_opened[e_offset..e_offset + 8].copy_from_slice(&1_u64.to_be_bytes());
    assert_eq!(
        finished
            .rewind(
                &resealed(shifted_opened),
                BranchId::new(Digest::of_canonical(b"bad-opened-fork"))
            )
            .err(),
        Some(CheckpointError::Malformed),
        "rewind must cross-check opened against parent"
    );

    let mut bad_head_link = raw.to_vec();
    let last = bad_head_link.len() - 1;
    bad_head_link[last] ^= 1;
    assert_eq!(
        Session::resume(&resealed(bad_head_link), &identity, case.body(), &fresh, &store).err(),
        Some(CheckpointError::HistoryAltered { seal: saved.seal })
    );

    let mut empty_head_link = raw[..offset + 8].to_vec();
    empty_head_link.extend_from_slice(&0_u64.to_be_bytes());
    let body_len = (empty_head_link.len() - HEADER) as u64;
    empty_head_link[40..48].copy_from_slice(&body_len.to_be_bytes());
    assert_eq!(
        Session::resume(&resealed(empty_head_link), &identity, case.body(), &fresh, &store).err(),
        Some(CheckpointError::Malformed),
        "rev > 0 with empty head_link trailer must be Malformed"
    );

    let rev0 = store.checkpoint_at_revision(0).expect("rev 0");
    let mut rev0_with_trailer = rev0.checkpoint.as_bytes().to_vec();
    rev0_with_trailer.extend_from_slice(&0_u64.to_be_bytes());
    rev0_with_trailer.extend_from_slice(&1_u64.to_be_bytes());
    rev0_with_trailer.push(0xAA);
    let body_len = (rev0_with_trailer.len() - HEADER) as u64;
    rev0_with_trailer[40..48].copy_from_slice(&body_len.to_be_bytes());
    assert_eq!(
        Session::resume(
            &resealed(rev0_with_trailer),
            &identity,
            case.body(),
            &fresh,
            &store
        )
        .err(),
        Some(CheckpointError::Malformed),
        "rev == 0 with non-empty trailer must be Malformed"
    );

    for partial in [1usize, 7, 15, 17] {
        let mut garbage = raw[..offset].to_vec();
        garbage.extend(core::iter::repeat_n(0_u8, partial));
        let body_len = (garbage.len() - HEADER) as u64;
        garbage[40..48].copy_from_slice(&body_len.to_be_bytes());
        assert_eq!(
            Session::resume(&resealed(garbage), &identity, case.body(), &fresh, &store).err(),
            Some(CheckpointError::Malformed),
            "partial trailer of {partial} bytes inside stated body must be Malformed"
        );
    }

    let mut trailing = raw.to_vec();
    trailing.push(0);
    assert_eq!(
        Session::resume(
            &Checkpoint::from_bytes(trailing),
            &identity,
            case.body(),
            &fresh,
            &store
        )
        .err(),
        Some(CheckpointError::TrailingContent)
    );
}

#[test]
fn the_flight_recorder_keeps_the_failing_span_on_closure_drift_and_rhythm_failures_and_checks_it_locally() {
    let drift_case = TwoReservoirs::delivering(
        [measure(500.0), measure(500.0)],
        vec![delivery(b"burst", 70, 110, 2.0)],
    )
    .expect("drift case");
    let drift_policy = policy_for(drift_case.body().conserved_basis(), 40, 200, 1e-9, 1e-12, 10.0);
    let core = ReferenceIntegrator::new(2 * SECOND);
    let store = RollingStore::new();
    let session = drift_case.admit(&core, &store).expect("admitted");
    let outcome = run_rolling(
        session,
        at(200 * SECOND),
        &drift_policy,
        &store,
        RollingConfig {
            checkpoint_every_ns: NonZeroU64::new(20 * SECOND).expect("positive"),
        },
    )
    .expect("rolling");
    let fold_err = outcome.certificate.expect_err("drift must fail");
    assert_eq!(fold_err.check(), CertificateCheck::Drift);
    let fail_window = fold_err.window().expect("window");
    let fr = outcome.listing.flight_recorder.expect("flight recorder latched");
    assert_eq!(
        fr.trigger,
        FlightRecorderTrigger::CertificateFailure(CertificateCheck::Drift)
    );
    assert!(fr.from_checkpoint_time <= fail_window.start());
    assert!(
        fr.from_checkpoint_revision > 0,
        "latched after earlier checkpoints"
    );
    let expected_revs: Vec<u64> = (fr.from_checkpoint_revision + 1..=fr.last_head_revision).collect();
    assert_eq!(
        outcome.listing.complete_link_revisions, expected_revs,
        "only the flight recorder span remains after finish"
    );
    let RollingStatus::Completed(finished) = outcome.status else {
        panic!("completed");
    };
    let retained = store
        .retained_flight_recorder(drift_case.body(), finished.record().identity())
        .expect("verified span")
        .expect("present");
    let local_err = certify_flight_recorder(&retained, &drift_policy).expect_err("reproduces drift");
    assert_eq!(
        (
            local_err.check(),
            local_err.quantity(),
            local_err.window(),
            local_err.residual().bits()
        ),
        (
            fold_err.check(),
            fold_err.quantity(),
            fold_err.window(),
            fold_err.residual().bits()
        )
    );

    let closure_case = TwoReservoirs::delivering(
        [measure(1e18), measure(1e18)],
        vec![
            delivery(b"bulk", 0, 10, 1.0),
            Delivery {
                id: b"below precision",
                from: at(24 * SECOND),
                to: at(120 * SECOND),
                amount: measure(1e-18 * 96.0),
            },
        ],
    )
    .expect("closure case");
    let closure_policy = policy_for(closure_case.body().conserved_basis(), 20, 100, 0.0, 0.0, 1e9);
    let closure_store = RollingStore::new();
    let closure_core = ReferenceIntegrator::new(2 * SECOND);
    let closure_session = closure_case
        .admit(&closure_core, &closure_store)
        .expect("admitted");
    let closure_out = run_rolling(
        closure_session,
        at(100 * SECOND),
        &closure_policy,
        &closure_store,
        RollingConfig {
            checkpoint_every_ns: NonZeroU64::new(10 * SECOND).expect("positive"),
        },
    )
    .expect("rolling");
    let closure_err = closure_out.certificate.expect_err("closure must fail");
    assert_eq!(closure_err.check(), CertificateCheck::Closure);
    let closure_fr = closure_out.listing.flight_recorder.expect("fr latched");
    assert_eq!(
        closure_fr.trigger,
        FlightRecorderTrigger::CertificateFailure(CertificateCheck::Closure)
    );
    assert!(
        closure_out.drift_tally.is_none(),
        "a closure failure must never produce a drift tally"
    );
    let RollingStatus::Completed(closure_finished) = closure_out.status else {
        panic!("completed");
    };
    let closure_retained = closure_store
        .retained_flight_recorder(closure_case.body(), closure_finished.record().identity())
        .expect("verified span")
        .expect("present");
    let local_closure =
        certify_flight_recorder(&closure_retained, &closure_policy).expect_err("reproduces closure");
    assert_eq!(
        (
            local_closure.check(),
            local_closure.quantity(),
            local_closure.window(),
            local_closure.residual().bits()
        ),
        (
            closure_err.check(),
            closure_err.quantity(),
            closure_err.window(),
            closure_err.residual().bits()
        )
    );

    let breath = breathing_body();
    let basis = breath.conserved_basis();
    let water_id = basis.ids().next().expect("water");
    let clock = Slot::from(LawName {
        owner: "Boundary",
        local: "phase",
    });
    let edge_policy = policy_for(basis, 10, 40, 1e-6, 1e-9, 1e9)
        .between_edges(
            EdgeWindows::new(clock, 0, NonZeroU64::new(6 * SECOND).expect("pos"), 10 * SECOND),
            vec![EdgeRate::new(water_id, measure::<Rate<Water>>(1.0), None).expect("rate")],
        )
        .expect("edge policy");
    let events = vec![
        BoundaryEvent::new(
            InputId::new(Digest::of_canonical(b"start")),
            at(0),
            room_input(false, true),
        ),
        BoundaryEvent::new(
            InputId::new(Digest::of_canonical(b"leak")),
            at(14 * SECOND),
            room_input(true, true),
        ),
    ];
    let edge_store = RollingStore::new();
    let edge_core = ReferenceIntegrator::new(250_000_000);
    let edge_session = admit_fixture(
        &breath,
        breathing_packet(&breath, 5.0),
        BoundaryPort::new(events).expect("port"),
        &edge_core,
        &edge_store,
    )
    .expect("admitted");
    let edge_out = run_rolling(
        edge_session,
        at(40 * SECOND),
        &edge_policy,
        &edge_store,
        RollingConfig {
            checkpoint_every_ns: NonZeroU64::new(10 * SECOND).expect("positive"),
        },
    )
    .expect("rolling");
    let edge_err = edge_out.certificate.expect_err("edge drift fails");
    assert_eq!(edge_err.check(), CertificateCheck::EdgeDrift);
    let RollingStatus::Completed(edge_finished) = edge_out.status else {
        panic!("completed");
    };
    let edge_retained = edge_store
        .retained_flight_recorder(&breath, edge_finished.record().identity())
        .expect("verified")
        .expect("present");
    let local_edge =
        certify_flight_recorder(&edge_retained, &edge_policy).expect_err("reproduces edge drift");
    assert_eq!(
        (
            local_edge.check(),
            local_edge.quantity(),
            local_edge.window(),
            local_edge.residual().bits()
        ),
        (
            edge_err.check(),
            edge_err.quantity(),
            edge_err.window(),
            edge_err.residual().bits()
        )
    );
}

#[test]
fn the_flight_recorder_latches_on_engine_fault_store_failure_and_replay_divergence() {
    let case = TwoReservoirs::delivering([measure(500.0), measure(500.0)], Vec::new()).expect("case");
    let policy = policy_for(case.body().conserved_basis(), 20, 100, 1e-9, 1e-12, 100.0);

    let fault_core = FaultingCore {
        inner: ReferenceIntegrator::new(2 * SECOND),
        fault_after_s: 50,
    };
    let fault_store = RollingStore::new();
    let fault_session = case.admit(&fault_core, &fault_store).expect("admitted");
    let fault_id = fault_session.record().identity().clone();
    let fault_out = run_rolling(
        fault_session,
        at(100 * SECOND),
        &policy,
        &fault_store,
        RollingConfig {
            checkpoint_every_ns: NonZeroU64::new(10 * SECOND).expect("positive"),
        },
    )
    .expect("rolling");
    assert!(matches!(fault_out.status, RollingStatus::Faulted(_)));
    let fault_fr = fault_out.listing.flight_recorder.expect("fault latched fr");
    assert!(matches!(fault_fr.trigger, FlightRecorderTrigger::EngineFault(_)));
    assert!(
        fault_store
            .retained_flight_recorder(case.body(), &fault_id)
            .expect("valid span")
            .is_some()
    );

    let store_core = ReferenceIntegrator::new(2 * SECOND);
    let fail_store = RollingStore::new();
    let mut s = case.admit(&store_core, &fail_store).expect("admitted");
    let s_id = s.record().identity().clone();
    fail_store.record_checkpoint(&s).expect("rev 0");
    let mut fold = CertificateFold::open(&s, &policy).expect("fold");
    let mut next_ckpt = 20 * SECOND;
    let mut injected = false;
    while s.record().accepted().time() < at(80 * SECOND) {
        if !injected && s.record().accepted().time() >= at(36 * SECOND) {
            fail_store.fail_next_keep();
            injected = true;
        }
        s = step_accepted(s);
        fold.advance(&s).expect("advance");
        if s.record().accepted().time().nanoseconds() >= next_ckpt {
            let _ = fail_store.record_checkpoint(&s);
            next_ckpt += 20 * SECOND;
        }
        fail_store.trim_before(fold.oldest_open_window_start(), fold.oldest_retained_revision());
    }
    fail_store.finish();
    let store_listing = fail_store.listing();
    let store_fr = store_listing.flight_recorder.expect("store failure latched fr");
    assert!(matches!(
        store_fr.trigger,
        FlightRecorderTrigger::StoreFailure { .. }
    ));
    assert!(
        store_fr.from_checkpoint_time <= at(16 * SECOND),
        "a store failure keeps from the checkpoint before the window open when it failed: {store_fr:?}"
    );
    let second_latch = fail_store
        .latch_flight_recorder(
            FlightRecorderTrigger::ReplayDivergence,
            Some(at(60 * SECOND)),
            s.record().revision(),
            s.record().seal().digest(),
        )
        .expect("already latched");
    assert_eq!(
        second_latch.trigger, store_fr.trigger,
        "a later trigger must not overwrite the first flight recorder trigger"
    );
    assert_eq!(
        second_latch.from_checkpoint_revision, store_fr.from_checkpoint_revision,
        "a later trigger must not advance the first flight recorder start checkpoint"
    );
    assert_eq!(
        store_listing.complete_link_revisions,
        (store_fr.from_checkpoint_revision + 1..=store_fr.last_head_revision).collect::<Vec<_>>()
    );
    assert!(
        fail_store
            .retained_flight_recorder(case.body(), &s_id)
            .expect("valid span")
            .is_some()
    );

    let primary_core = ReferenceIntegrator::new(2 * SECOND);
    let shadow_core = PerturbedCore {
        inner: ReferenceIntegrator::new(2 * SECOND),
        perturb_after_s: 40,
    };
    let div_store = RollingStore::new();
    let shadow_store = MemoryStore::default();
    let mut primary = case.admit(&primary_core, &div_store).expect("admitted");
    let mut shadow = case.admit(&shadow_core, &shadow_store).expect("shadow admitted");
    let div_id = primary.record().identity().clone();
    div_store.record_checkpoint(&primary).expect("rev 0");
    let mut next_ckpt = 20 * SECOND;
    while primary.record().accepted().time() < at(80 * SECOND) {
        primary = step_accepted(primary);
        shadow = step_accepted(shadow);
        if primary.record().seal() != shadow.record().seal() {
            div_store.latch_flight_recorder(
                FlightRecorderTrigger::ReplayDivergence,
                Some(primary.record().accepted().time()),
                primary.record().revision(),
                primary.record().seal().digest(),
            );
        }
        if primary.record().accepted().time().nanoseconds() >= next_ckpt {
            div_store.record_checkpoint(&primary).expect("ckpt");
            next_ckpt += 20 * SECOND;
        }
    }
    div_store.finish();
    let div_listing = div_store.listing();
    let div_fr = div_listing.flight_recorder.expect("replay divergence latched fr");
    assert_eq!(div_fr.trigger, FlightRecorderTrigger::ReplayDivergence);
    assert_eq!(div_fr.from_checkpoint_time, at(40 * SECOND));
    assert!(
        div_store
            .retained_flight_recorder(case.body(), &div_id)
            .expect("valid span")
            .is_some()
    );
}

#[test]
fn a_revision_between_checkpoints_replays_forward_and_regeneration_checks_every_witness() {
    let case = TwoReservoirs::delivering(
        [measure(500.0), measure(500.0)],
        vec![delivery(b"sip", 12, 30, 0.5)],
    )
    .expect("case");
    let end = at(120 * SECOND);
    let policy = policy_for(case.body().conserved_basis(), 40, 120, 1e-9, 1e-12, 100.0);
    let core = ReferenceIntegrator::new(3 * SECOND);
    let store = RollingStore::new();
    let session = case.admit(&core, &store).expect("admitted");
    let outcome = run_rolling(
        session,
        end,
        &policy,
        &store,
        RollingConfig {
            checkpoint_every_ns: NonZeroU64::new(30 * SECOND).expect("positive"),
        },
    )
    .expect("rolling");
    let orig_cert = outcome.certificate.expect("passed");
    assert!(outcome.listing.complete_link_revisions.is_empty());
    let RollingStatus::Completed(finished) = outcome.status else {
        panic!("completed");
    };
    let identity = finished.record().identity().clone();
    let seals = store.witness_seals();
    let target_rev = 15u64;
    assert!(
        !outcome.listing.checkpoint_revisions.contains(&target_rev),
        "target revision sits strictly between checkpoints"
    );
    let prior_ckpt = store
        .checkpoint_at_or_before_revision(target_rev)
        .expect("prior ckpt");
    assert!(prior_ckpt.revision < target_rev);
    let fresh = ReferenceIntegrator::new(3 * SECOND);
    let witnesses_before = store.listing().witness_count;
    let between_sink = MemoryStore::default();
    let between = ReplayStore::new(&store, &between_sink);
    let from = prior_ckpt.revision as usize;
    let at_target = regenerate(
        case.body(),
        &fresh,
        &between,
        RegenerationSource {
            build: "nightingale-nc95-test",
            from_checkpoint: &prior_ckpt.checkpoint,
            expected: &identity,
            expected_seals: &seals[from..target_rev as usize],
        },
    )
    .expect("replayed between checkpoints from the checkpoint before it");
    assert_eq!(at_target.steps_verified, target_rev - prior_ckpt.revision);
    assert_eq!(at_target.session.record().revision(), target_rev);
    assert_eq!(
        at_target.session.record().seal().digest(),
        seals[(target_rev - 1) as usize]
    );

    let diverging = PerturbedCore {
        inner: ReferenceIntegrator::new(3 * SECOND),
        perturb_after_s: 0,
    };
    let diverging_sink = MemoryStore::default();
    let diverging_replay = ReplayStore::new(&store, &diverging_sink);
    let refused = regenerate(
        case.body(),
        &diverging,
        &diverging_replay,
        RegenerationSource {
            build: "nightingale-nc95-drifted",
            from_checkpoint: &prior_ckpt.checkpoint,
            expected: &identity,
            expected_seals: &seals[from..target_rev as usize],
        },
    )
    .err();
    assert!(
        matches!(&refused, Some(RegenerationError::Diverged(finding)) if finding.revision == prior_ckpt.revision + 1),
        "a replay between checkpoints on a core that leaves the chain is a finding at its first step: {refused:?}"
    );
    assert_eq!(
        store.listing().witness_count,
        witnesses_before,
        "a replay keeps nothing in the store it is checked against"
    );

    let rev0 = store.checkpoint_at_revision(0).expect("rev 0");
    let regen_store = MemoryStore::default();
    let regen_core = ReferenceIntegrator::new(3 * SECOND);
    let regen_replay = ReplayStore::new(&store, &regen_store);
    let regenerated = regenerate(
        case.body(),
        &regen_core,
        &regen_replay,
        RegenerationSource {
            build: "nightingale-nc95-test",
            from_checkpoint: &rev0.checkpoint,
            expected: &identity,
            expected_seals: &seals,
        },
    )
    .expect("regenerated");
    assert_eq!(regenerated.build, "nightingale-nc95-test");
    assert_eq!(regenerated.steps_verified, seals.len() as u64);
    assert_eq!(
        regenerated.session.record().digests(),
        finished.record().digests()
    );

    let parsed_count = regenerated
        .session
        .conservation()
        .newest_first()
        .collect::<Result<Vec<_>, _>>()
        .expect("every discarded ledger now parses back")
        .len();
    assert_eq!(parsed_count, seals.len());
    let regen_cert = certify(&regenerated.session.conservation(), &policy).expect("recertified");
    assert_eq!(format!("{regen_cert:?}"), format!("{orig_cert:?}"));

    let bad_core = PerturbedCore {
        inner: ReferenceIntegrator::new(3 * SECOND),
        perturb_after_s: 30,
    };
    let bad_store = MemoryStore::default();
    let bad_replay = ReplayStore::new(&store, &bad_store);
    let err = regenerate(
        case.body(),
        &bad_core,
        &bad_replay,
        RegenerationSource {
            build: "nightingale-nc95-drifted",
            from_checkpoint: &rev0.checkpoint,
            expected: &identity,
            expected_seals: &seals,
        },
    )
    .err()
    .expect("diverging build must report a finding");
    match err {
        RegenerationError::Diverged(finding) => {
            assert_eq!(finding.build, "nightingale-nc95-drifted");
            assert!(finding.revision > 1);
            assert_ne!(finding.actual_seal, finding.expected_seal);
            assert_ne!(finding.actual_witness, finding.expected_witness);
        }
        other => panic!("expected Diverged finding, got {other:?}"),
    }

    let missing_witnesses = TamperedWitnessStore {
        inner: &store,
        altered: None,
        missing: Some(seals[4]),
    };
    let missing_sink = MemoryStore::default();
    let missing_replay = ReplayStore::new(&missing_witnesses, &missing_sink);
    assert_eq!(
        regenerate(
            case.body(),
            &regen_core,
            &missing_replay,
            RegenerationSource {
                build: "nightingale-nc95-test",
                from_checkpoint: &rev0.checkpoint,
                expected: &identity,
                expected_seals: &seals,
            },
        )
        .err(),
        Some(RegenerationError::WitnessMissing { revision: 5 })
    );
}

#[test]
fn a_shared_rolling_store_serves_each_session_its_own_link_for_a_seal() {
    let case = TwoReservoirs::delivering(
        [measure(500.0), measure(500.0)],
        vec![delivery(b"sip", 4, 30, 0.5)],
    )
    .expect("case");
    let core = ReferenceIntegrator::new(2 * SECOND);
    let other = PerturbedCore {
        inner: ReferenceIntegrator::new(2 * SECOND),
        perturb_after_s: 0,
    };
    let store = RollingStore::new();
    let mut a = case.admit(&core, &store).expect("admitted");
    let mut seals = Vec::new();
    let mut at_five = None;
    for _ in 0..10 {
        a = step_accepted(a);
        seals.push(a.record().seal().digest());
        if a.record().revision() == 5 {
            at_five = Some(a.checkpoint().expect("ckpt"));
        }
    }
    let identity = a.record().identity().clone();
    let b =
        Session::resume(&at_five.expect("rev 5"), &identity, case.body(), &other, &store).expect("resumed");
    let b = step_accepted(b);
    assert_ne!(b.record().seal().digest(), seals[5], "b's revision 6 is not a's");
    assert_eq!(
        a.conservation()
            .newest_first()
            .collect::<Result<Vec<_>, _>>()
            .err(),
        None,
        "a's walk reads a's own links after b keeps its revision 6 in the same store"
    );
    assert_eq!(
        b.conservation()
            .newest_first()
            .collect::<Result<Vec<_>, _>>()
            .err(),
        None,
        "and b's walk reads b's"
    );
    store.trim_before(None, Some(7));
    assert_eq!(
        store.listing().complete_link_revisions,
        vec![7, 8, 9, 10],
        "a trim drops every session's links below its floor and only those"
    );
}

#[test]
fn an_engine_fault_keeps_the_links_of_the_window_open_at_the_fault() {
    let case = TwoReservoirs::delivering([measure(500.0), measure(500.0)], Vec::new()).expect("case");
    let policy = policy_for(case.body().conserved_basis(), 20, 100, 1e-9, 1e-12, 100.0);
    let fault_core = FaultingCore {
        inner: ReferenceIntegrator::new(2 * SECOND),
        fault_after_s: 50,
    };
    let store = RollingStore::new();
    let session = case.admit(&fault_core, &store).expect("admitted");
    let identity = session.record().identity().clone();
    let out = run_rolling(
        session,
        at(100 * SECOND),
        &policy,
        &store,
        RollingConfig {
            checkpoint_every_ns: NonZeroU64::new(10 * SECOND).expect("positive"),
        },
    )
    .expect("rolling");
    assert!(matches!(out.status, RollingStatus::Faulted(_)));
    let fr = out.listing.flight_recorder.clone().expect("latched");
    assert!(
        fr.from_checkpoint_revision < fr.last_head_revision,
        "the span reaches back past the head: {fr:?}"
    );
    assert!(
        fr.from_checkpoint_time.nanoseconds() <= 30 * SECOND,
        "the span starts at the checkpoint before the window open at the fault: {fr:?}"
    );
    assert_eq!(
        out.listing.complete_link_revisions,
        (fr.from_checkpoint_revision + 1..=fr.last_head_revision).collect::<Vec<_>>()
    );
    let retained = store
        .retained_flight_recorder(case.body(), &identity)
        .expect("valid")
        .expect("present");
    assert!(
        certify_flight_recorder(&retained, &policy).is_ok(),
        "the kept span holds whole windows that pass"
    );
}

#[test]
fn a_complete_closure_passing_run_rechecks_loosened_drift_from_worst_drift_and_requires_rerun_on_failure() {
    let case = TwoReservoirs::delivering(
        [measure(500.0), measure(500.0)],
        vec![delivery(b"meal", 30, 70, 1.0)],
    )
    .expect("case");
    let basis = case.body().conserved_basis();
    let water = basis.ids().next().expect("water");
    let tight = policy_for(basis, 40, 120, 1e-9, 1e-12, 10.0);
    let loose = policy_for(basis, 40, 120, 1e-9, 1e-12, 100.0);
    let still_tight = policy_for(basis, 40, 120, 1e-9, 1e-12, 20.0);

    let core = ReferenceIntegrator::new(2 * SECOND);
    let store = RollingStore::new();
    let session = case.admit(&core, &store).expect("admitted");
    let outcome = run_rolling(
        session,
        at(120 * SECOND),
        &tight,
        &store,
        RollingConfig {
            checkpoint_every_ns: NonZeroU64::new(30 * SECOND).expect("positive"),
        },
    )
    .expect("rolling");
    assert_eq!(
        outcome.certificate.expect_err("tight drift fails").check(),
        CertificateCheck::Drift
    );
    let tally = outcome
        .drift_tally
        .expect("complete closure-passing run keeps worst_drift tally even when drift failed");
    assert_eq!(tally.recheck(&tight, &loose), DriftRecheck::Passes);
    assert_eq!(
        tally.recheck(&tight, &still_tight),
        DriftRecheck::NeedsRerun { quantity: water }
    );

    let RollingStatus::Completed(finished) = outcome.status else {
        panic!("completed");
    };
    let rev0 = store.checkpoint_at_revision(0).expect("rev 0");
    let regen_store = MemoryStore::default();
    let regen_core = ReferenceIntegrator::new(2 * SECOND);
    let regen_replay = ReplayStore::new(&store, &regen_store);
    let all_seals = store.witness_seals();
    let regen = regenerate(
        case.body(),
        &regen_core,
        &regen_replay,
        RegenerationSource {
            build: "nc95-test",
            from_checkpoint: &rev0.checkpoint,
            expected: finished.record().identity(),
            expected_seals: &all_seals,
        },
    )
    .expect("regenerated");
    let loose_cert = certify(&regen.session.conservation(), &loose).expect("passes loosened bound");
    assert_eq!(
        certify(&regen.session.conservation(), &still_tight)
            .expect_err("still fails")
            .check(),
        CertificateCheck::Drift
    );

    assert_eq!(loose_cert.recheck_drift(&loose, &loose), DriftRecheck::Passes);
    assert_eq!(
        loose_cert.recheck_drift(&loose, &still_tight),
        DriftRecheck::NeedsRerun { quantity: water }
    );
    let wrong_prior = policy_for(basis, 40, 120, 1e-9, 1e-12, 101.0);
    assert_eq!(
        loose_cert.recheck_drift(&wrong_prior, &loose),
        DriftRecheck::IncompatiblePolicy
    );
    let diff_window = policy_for(basis, 50, 120, 1e-9, 1e-12, 100.0);
    assert_eq!(
        loose_cert.recheck_drift(&loose, &diff_window),
        DriftRecheck::IncompatiblePolicy
    );
    let diff_closure = policy_for(basis, 40, 120, 2e-9, 1e-12, 100.0);
    assert_eq!(
        loose_cert.recheck_drift(&loose, &diff_closure),
        DriftRecheck::IncompatiblePolicy
    );
    let diff_rel = policy_for(basis, 40, 120, 1e-9, 2e-12, 100.0);
    assert_eq!(
        loose_cert.recheck_drift(&loose, &diff_rel),
        DriftRecheck::IncompatiblePolicy
    );
    let with_rounding = policy_for(basis, 40, 120, 1e-9, 1e-12, 100.0)
        .admitting_row_rounding(case.body(), 7)
        .expect("rounding");
    assert_eq!(
        loose_cert.recheck_drift(&loose, &with_rounding),
        DriftRecheck::IncompatiblePolicy
    );
    let clock = Slot::from(LawName {
        owner: "Boundary",
        local: "phase",
    });
    let with_edges = policy_for(basis, 40, 120, 1e-9, 1e-12, 100.0)
        .between_edges(
            EdgeWindows::new(clock, 0, NonZeroU64::new(10 * SECOND).expect("pos"), 20 * SECOND),
            Vec::new(),
        )
        .expect("edges");
    assert_eq!(
        loose_cert.recheck_drift(&loose, &with_edges),
        DriftRecheck::IncompatiblePolicy
    );
}

#[test]
fn earlier_v2_checkpoints_without_trailer_still_load_and_continue() {
    let case = TwoReservoirs::delivering([measure(500.0), measure(500.0)], Vec::new()).expect("case");
    let core = ReferenceIntegrator::new(4 * SECOND);
    let store = MemoryStore::default();
    let mut session = case.admit(&core, &store).expect("admitted");
    for _ in 0..5 {
        session = step_accepted(session);
    }
    let identity = session.record().identity().clone();
    let saved = session.checkpoint().expect("saved");
    let raw = saved.as_bytes();
    let offset = trailer_offset(raw);
    assert!(
        offset < raw.len(),
        "current V2 rev > 0 checkpoint carries a trailer"
    );

    let mut legacy_v2 = raw[..offset].to_vec();
    let legacy_body_len = (legacy_v2.len() - HEADER) as u64;
    legacy_v2[40..48].copy_from_slice(&legacy_body_len.to_be_bytes());
    let legacy_ckpt = resealed(legacy_v2);

    let fresh = ReferenceIntegrator::new(4 * SECOND);
    let resumed = Session::resume(&legacy_ckpt, &identity, case.body(), &fresh, &store)
        .expect("legacy V2 checkpoint without trailer still loads against untrimmed store");
    assert_eq!(resumed.record().digests(), session.record().digests());
    assert_eq!(resumed.record().seal(), session.record().seal());
    let next = step_accepted(resumed);
    let resaved = next.checkpoint().expect("saved with trailer");
    assert!(trailer_offset(resaved.as_bytes()) < resaved.as_bytes().len());

    struct WitnessesOnly(RefCell<Vec<(Digest, Witness)>>);
    impl HistoryStore for WitnessesOnly {
        fn resolve(&self, _: &Digest) -> Option<Vec<u8>> {
            None
        }
        fn keep(&self, _: Digest, _: &[u8]) -> Result<(), CheckpointError> {
            Ok(())
        }
        fn resolve_witness(&self, seal: &Digest) -> Option<Witness> {
            self.0.borrow().iter().find(|(s, _)| s == seal).map(|(_, w)| *w)
        }
    }
    let rolling = RollingStore::new();
    let mut s2 = case.admit(&core, &rolling).expect("admitted");
    for _ in 0..5 {
        s2 = step_accepted(s2);
    }
    let entries: Vec<(Digest, Witness)> = rolling
        .witness_seals()
        .into_iter()
        .map(|s| (s, rolling.resolve_witness(&s).expect("w")))
        .collect();
    let trimmed_only = WitnessesOnly(RefCell::new(entries));
    assert!(matches!(
        Session::resume(&legacy_ckpt, &identity, case.body(), &fresh, &trimmed_only).err(),
        Some(CheckpointError::HistoryTrimmed { .. })
    ));
}

#[test]
fn a_retained_span_refuses_tampered_prefix_witnesses_trimmed_span_links_and_reproduces_stopped_rhythm() {
    let breath = breathing_body();
    let basis = breath.conserved_basis();
    let water_id = basis.ids().next().expect("water");
    let clock = Slot::from(LawName {
        owner: "Boundary",
        local: "phase",
    });
    let policy = policy_for(basis, 20, 40, 1e-6, 1e-9, 1e9)
        .between_edges(
            EdgeWindows::new(clock, 0, NonZeroU64::new(6 * SECOND).expect("pos"), 9 * SECOND),
            vec![EdgeRate::new(water_id, measure::<Rate<Water>>(100.0), None).expect("rate")],
        )
        .expect("policy");
    let events = vec![
        BoundaryEvent::new(
            InputId::new(Digest::of_canonical(b"go")),
            at(0),
            room_input(false, true),
        ),
        BoundaryEvent::new(
            InputId::new(Digest::of_canonical(b"hold")),
            at(18 * SECOND),
            room_input(false, false),
        ),
    ];
    let store = RollingStore::new();
    let core = ReferenceIntegrator::new(250_000_000);
    let session = admit_fixture(
        &breath,
        breathing_packet(&breath, 0.0),
        BoundaryPort::new(events).expect("port"),
        &core,
        &store,
    )
    .expect("admitted");
    let out = run_rolling(
        session,
        at(30 * SECOND),
        &policy,
        &store,
        RollingConfig {
            checkpoint_every_ns: NonZeroU64::new(5 * SECOND).expect("pos"),
        },
    )
    .expect("rolling");
    let RollingStatus::Completed(finished) = out.status else {
        panic!("completed");
    };
    let identity = finished.record().identity();
    let fr = out.listing.flight_recorder.expect("rhythm stopped latched fr");
    assert_eq!(
        fr.trigger,
        FlightRecorderTrigger::CertificateFailure(CertificateCheck::RhythmStopped)
    );
    assert!(
        fr.from_checkpoint_revision > 0,
        "flight recorder must anchor to a non-zero checkpoint before the stopped stretch"
    );
    let retained = store
        .retained_flight_recorder(&breath, identity)
        .expect("verified")
        .expect("present");
    let local_err = certify_flight_recorder(&retained, &policy).expect_err("reproduces stopped rhythm");
    assert_eq!(local_err.check(), CertificateCheck::RhythmStopped);

    let from_ckpt = store
        .checkpoint_at_revision(fr.from_checkpoint_revision)
        .expect("ckpt");
    let early_seal = store.witness_seals()[0];
    let good_w = store.resolve_witness(&early_seal).expect("w");
    let mut bad_bytes = *good_w.ledger().as_bytes();
    bad_bytes[0] ^= 1;
    let bad_prefix = TamperedWitnessStore {
        inner: &store,
        altered: Some((
            early_seal,
            Witness::new(good_w.previous(), good_w.image(), Digest::from_bytes(bad_bytes)),
        )),
        missing: None,
    };
    assert_eq!(
        ng_record::retained_span(
            &breath,
            identity,
            &from_ckpt.checkpoint,
            fr.last_head_seal,
            fr.last_head_revision,
            &bad_prefix,
        )
        .err(),
        Some(CheckpointError::HistoryAltered { seal: early_seal })
    );

    struct TrimmedSpanStore<'a>(&'a RollingStore, Digest);
    impl HistoryStore for TrimmedSpanStore<'_> {
        fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
            if *seal == self.1 {
                return None;
            }
            self.0.resolve(seal)
        }
        fn keep(&self, seal: Digest, link: &[u8]) -> Result<(), CheckpointError> {
            self.0.keep(seal, link)
        }
        fn resolve_witness(&self, seal: &Digest) -> Option<Witness> {
            self.0.resolve_witness(seal)
        }
    }
    let missing_in_span = TrimmedSpanStore(&store, fr.last_head_seal);
    assert_eq!(
        ng_record::retained_span(
            &breath,
            identity,
            &from_ckpt.checkpoint,
            fr.last_head_seal,
            fr.last_head_revision,
            &missing_in_span,
        )
        .err(),
        Some(CheckpointError::HistoryTrimmed {
            seal: fr.last_head_seal
        })
    );

    assert_eq!(
        ng_record::retained_span(
            &breath,
            identity,
            &from_ckpt.checkpoint,
            fr.from_checkpoint_seal,
            fr.from_checkpoint_revision - 1,
            &store,
        )
        .err(),
        Some(CheckpointError::NotOnThisBranch)
    );
}
