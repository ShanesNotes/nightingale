//! Bytes the base f43d49e8 wrote, which every later build must load: checkpoints, store links, a fault's export
//! and observer checkpoints of three courses, with what the base did next. A test appended to tests/observation.rs
//! in a detached worktree of the base printed them; a test reads no file, so they are inline.
use core::num::NonZeroU64;
use ng_contract::*;
use ng_observe::*;
use ng_record::*;
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;

fn bytes(hexed: &str) -> Vec<u8> {
    let digit = |c: u8| {
        (c as char)
            .to_digit(16)
            .map(|d| d as u8)
            .unwrap_or_else(|| panic!("hex digit {c}"))
    };
    (hexed.as_bytes().as_chunks::<2>().0.iter())
        .map(|pair| digit(pair[0]) * 16 + digit(pair[1]))
        .collect()
}

fn digest(hexed: &str) -> Digest {
    Digest::from_bytes(bytes(hexed).try_into().expect("thirty-two bytes"))
}

fn checkpoint(hexed: &str) -> Checkpoint {
    Checkpoint::from_bytes(bytes(hexed))
}

fn relabelled(hexed: &str, magic: &[u8; 8]) -> Checkpoint {
    let mut saved = bytes(hexed);
    saved[..8].copy_from_slice(magic);
    Checkpoint::from_bytes(saved)
}

fn branch(name: &[u8]) -> BranchId {
    BranchId::new(Digest::of_canonical(name))
}

#[derive(Default)]
struct Store {
    links: Vec<(Digest, Vec<u8>)>,
}
impl Store {
    fn of(links: &[(&str, &str)]) -> Self {
        Self {
            links: (links.iter())
                .map(|(seal, link)| (digest(seal), bytes(link)))
                .collect(),
        }
    }
}
impl HistoryStore for Store {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        (self.links.iter())
            .find(|link| link.0 == *seal)
            .map(|link| link.1.clone())
    }
    fn keep(&mut self, seal: Digest, link: Vec<u8>) -> Result<(), CheckpointError> {
        self.links.push((seal, link));
        Ok(())
    }
}

fn at(seconds: u64) -> SimTime {
    SimTime::from_ns(seconds * SECOND)
}

fn measure<U>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn count(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("not zero")
}

fn water_case() -> TwoReservoirs {
    let start = [measure(500_500.0), measure(499_500.0)];
    let delivery = Delivery {
        id: b"input",
        from: at(40),
        to: at(60),
        amount: measure(100.0),
    };
    let case = TwoReservoirs::delivering(start, vec![delivery]).expect("a body");
    case.until(at(200))
}

fn lab(case: &TwoReservoirs) -> ObservationLens {
    let kind = |row: RowId| case.body().rows()[row.index()].kind();
    let [a, b] = case
        .rows()
        .map(|row| ChannelSpec::new("a synthetic method", kind(row)));
    let channels = vec![
        a.averaged(count(5)).erring(measure::<Water>(2.0)),
        b.delayed(30 * SECOND),
    ];
    case.body().lens(channels, count(3 * SECOND)).expect("a lens")
}

fn seed() -> ObservationSeed {
    ObservationSeed::new([5, 6, 7, 8])
}

struct FaultsAt {
    inner: ReferenceIntegrator,
    at: SimTime,
}
impl CoupledSolver for FaultsAt {
    fn resolution(&self) -> DeclaredResolution {
        self.inner.resolution()
    }
    fn next_step(&self, accepted: AcceptedStateView<'_>) -> u64 {
        self.inner.next_step(accepted)
    }
    fn solve(&self, problem: &CoupledProblem<'_>, ticket: &StepTicket) -> Result<StepProposal, TrialFailure> {
        if ticket.interval().start() == self.at {
            return Err(ticket.mismatch().into());
        }
        self.inner.solve(problem, ticket)
    }
}

fn faulting() -> FaultsAt {
    FaultsAt {
        inner: ReferenceIntegrator::new(8 * SECOND),
        at: at(68),
    }
}

enum Next<'m> {
    Accepted(Session<'m, Running>),
    Stopped(Session<'m, Faulted>),
}

fn step(session: Session<'_, Running>) -> Next<'_> {
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, _) => return Next::Accepted(session),
            Step::Refused(session) => session.retry(),
            Step::Faulted(stopped) => return Next::Stopped(stopped),
        };
    }
}

fn accepted(session: Session<'_, Running>) -> Session<'_, Running> {
    match step(session) {
        Next::Accepted(session) => session,
        Next::Stopped(stopped) => panic!("{:?}", stopped.fault()),
    }
}

fn read(
    observer: &mut Observer<'_>,
    session: &Session<'_, Running>,
    lens: &ObservationLens,
) -> Vec<Observation> {
    let view = session.observation(lens).expect("this body's lens");
    observer.advance(&view).expect("the next revision")
}

fn named(digests: RecordDigests) -> [Digest; 8] {
    let RecordDigests {
        revision: _,
        time: _,
        state,
        continuation,
        segment,
        modes,
        streams,
        applied,
        waiting,
        ledger,
    } = digests;
    [
        state,
        continuation,
        segment,
        modes,
        streams,
        applied,
        waiting,
        ledger,
    ]
}

fn identity_of(case: &dyn ReplayCase, core: &dyn CoupledSolver) -> RunIdentity {
    case.admit(core).expect("admitted").record().identity().clone()
}

#[test]
fn a_checkpoint_main_wrote_resumes_and_goes_on_as_main_did() {
    let case = water_case();
    let (core, fresh) = (faulting(), faulting());
    let identity = identity_of(&case, &core);
    assert_eq!(identity.digest(), digest(WATER_IDENTITY));
    let zero = checkpoint(WATER_CHECKPOINT_0);
    let mut empty = Store::default();
    let opened = Session::resume(&zero, &identity, case.body(), &fresh, &empty).expect("resumed");
    assert_eq!(opened.record().seal().digest(), digest(WATER_SEALS[0]));
    assert_eq!(
        opened.checkpoint(&mut empty).expect("saved").as_bytes(),
        zero.as_bytes()
    );

    let mut store = Store::of(&WATER_LINKS);
    let saved = checkpoint(WATER_CHECKPOINT_6);
    let mut session = Session::resume(&saved, &identity, case.body(), &fresh, &store).expect("resumed");
    assert_eq!(session.record().seal().digest(), digest(WATER_SEALS[6]));
    assert_eq!(
        session.checkpoint(&mut store).expect("saved").as_bytes(),
        saved.as_bytes()
    );
    let stopped = loop {
        session = match step(session) {
            Next::Accepted(next) => next,
            Next::Stopped(stopped) => break stopped,
        };
        let revision = session.record().revision();
        assert_eq!(
            session.record().seal().digest(),
            digest(WATER_SEALS[revision as usize]),
            "revision {revision}"
        );
    };
    assert_eq!(stopped.record().revision(), 9);
    let report = stopped.export(&mut store).expect("exported");
    assert_eq!(report.last_accepted().as_bytes(), bytes(WATER_EXPORT_9));
}

#[test]
fn a_run_resumed_from_main_rewinds_to_a_later_checkpoint_main_wrote() {
    let case = water_case();
    let (core, fresh) = (faulting(), faulting());
    let identity = identity_of(&case, &core);
    let earlier = checkpoint(WATER_CHECKPOINT_3);
    let mut session = Session::resume(
        &earlier,
        &identity,
        case.body(),
        &core,
        &Store::of(&WATER_LINKS[..3]),
    )
    .expect("resumed");
    while session.record().revision() < 7 {
        session = accepted(session);
    }
    let later = checkpoint(WATER_CHECKPOINT_6);
    let fork = session.rewind(&later, branch(b"at six")).expect("on this branch");
    let loaded =
        Session::resume(&later, &identity, case.body(), &fresh, &Store::of(&WATER_LINKS)).expect("resumed");
    assert_eq!(fork.record().seal().digest(), digest(WATER_SEALS[6]));
    assert_eq!(fork.record().digests(), loaded.record().digests());
    assert_eq!(accepted(fork).record().seal().digest(), digest(WATER_SEALS[7]));
    let back = session
        .rewind(&earlier, branch(b"at three"))
        .expect("on this branch");
    assert_eq!(accepted(back).record().seal().digest(), digest(WATER_SEALS[4]));
}

#[test]
fn a_fault_main_exported_goes_on_only_through_recover() {
    let case = water_case();
    let (core, fresh) = (faulting(), faulting());
    let identity = identity_of(&case, &core);
    let store = Store::of(&WATER_LINKS);
    let export = checkpoint(WATER_EXPORT_9);
    let refused = Session::resume(&export, &identity, case.body(), &fresh, &store);
    assert_eq!(refused.err(), Some(CheckpointError::Latched));

    let mut session = Session::resume(
        &checkpoint(WATER_CHECKPOINT_6),
        &identity,
        case.body(),
        &core,
        &store,
    )
    .expect("resumed");
    let stopped = loop {
        session = match step(session) {
            Next::Accepted(next) => next,
            Next::Stopped(stopped) => break stopped,
        };
    };
    let recovered = stopped
        .recover(&export, branch(b"past the fault"))
        .expect("on this branch");
    assert_eq!(recovered.record().seal().digest(), digest(WATER_SEALS[9]));
    assert!(matches!(step(recovered), Next::Stopped(_)));
    let earlier = (stopped.recover(&checkpoint(WATER_CHECKPOINT_3), branch(b"before the fault")))
        .expect("on this branch");
    assert_eq!(accepted(earlier).record().seal().digest(), digest(WATER_SEALS[4]));
}

#[test]
fn an_observer_main_checkpointed_reads_on_beside_the_run_it_watched() {
    let case = water_case();
    let lens = lab(&case);
    let (core, fresh) = (faulting(), faulting());
    let identity = identity_of(&case, &core);
    let store = Store::of(&WATER_LINKS);
    let saved = ObserverCheckpoint::from_bytes(bytes(WATER_OBSERVER_6));
    let mut observer = Observer::resume(&lens, seed(), &saved).expect("resumed");
    let mut session = Session::resume(
        &checkpoint(WATER_CHECKPOINT_6),
        &identity,
        case.body(),
        &fresh,
        &store,
    )
    .expect("resumed");
    let repeated = session.observation(&lens).expect("this body's lens");
    assert!(matches!(
        observer.advance(&repeated),
        Err(ObservationRefusal::Gap { .. })
    ));
    assert_eq!(observer.checkpoint().as_bytes(), saved.as_bytes());

    let mut admitted = case.admit(&core).expect("admitted");
    while admitted.record().revision() < 7 {
        admitted = accepted(admitted);
    }
    let unrelated = admitted.observation(&lens).expect("this body's lens");
    assert!(matches!(
        observer.advance(&unrelated),
        Err(ObservationRefusal::Gap { .. })
    ));
    assert_eq!(observer.checkpoint().as_bytes(), saved.as_bytes());

    let unread = ObserverCheckpoint::from_bytes(bytes(WATER_OBSERVER_UNREAD));
    let mut attaching = Observer::resume(&lens, seed(), &unread).expect("resumed");
    read(&mut attaching, &session, &lens);

    let mut stream = Digest::of_canonical(b"prior shape");
    loop {
        session = match step(session) {
            Next::Accepted(next) => next,
            Next::Stopped(_) => break,
        };
        stream = chained(stream, &read(&mut observer, &session, &lens));
    }
    assert_eq!(stream, digest(WATER_STREAM_AFTER_6));
}

#[test]
fn a_run_admitted_now_seals_in_blake2b_and_reaches_mains_state_ledger_and_observations() {
    let case = water_case();
    let lens = lab(&case);
    let (core, fresh) = (faulting(), faulting());
    let mut store = Store::default();
    let mut session = case.admit(&core).expect("admitted");
    let identity = session.record().identity().clone();
    let mut observer = Observer::new(&lens, seed());
    let mut stream = chained(
        Digest::of_canonical(b"prior shape"),
        &read(&mut observer, &session, &lens),
    );
    let zero = session.checkpoint(&mut store).expect("saved");
    assert_eq!(&zero.as_bytes()[..8], b"ngcheck2");
    assert_ne!(session.record().seal().digest(), digest(WATER_SEALS[0]));
    let stopped = loop {
        session = match step(session) {
            Next::Accepted(next) => next,
            Next::Stopped(stopped) => break stopped,
        };
        stream = chained(stream, &read(&mut observer, &session, &lens));
    };
    assert_eq!(stream, digest(WATER_STREAM_WHOLE));
    let digests = stopped.record().digests();
    assert_eq!((digests.revision, digests.time), (9, at(68)));
    assert_eq!(named(digests), WATER_DIGESTS.map(digest));
    assert_ne!(stopped.record().seal().digest(), digest(WATER_SEALS[9]));
    let report = stopped.export(&mut store).expect("exported");
    assert_eq!(&report.last_accepted().as_bytes()[..8], b"ngcheck2");
    let resumed = Session::resume(&zero, &identity, case.body(), &fresh, &store).expect("resumed");
    assert_eq!(
        resumed.record().digests(),
        case.admit(&core).expect("admitted").record().digests()
    );
}

#[test]
fn a_magic_that_names_the_other_hash_is_refused() {
    let case = water_case();
    let (core, fresh) = (faulting(), faulting());
    let identity = identity_of(&case, &core);
    let main = Store::of(&WATER_LINKS);
    let refused = |saved: &Checkpoint, store: &Store| {
        Session::resume(saved, &identity, case.body(), &fresh, store).err()
    };
    let swapped = relabelled(WATER_CHECKPOINT_6, b"ngcheck2");
    assert!(matches!(
        refused(&swapped, &main),
        Some(CheckpointError::HistoryAltered { .. })
    ));
    let swapped = relabelled(WATER_CHECKPOINT_0, b"ngcheck2");
    assert!(matches!(
        refused(&swapped, &main),
        Some(CheckpointError::HistoryAltered { .. })
    ));
    let unknown = relabelled(WATER_CHECKPOINT_6, b"ngcheck3");
    assert_eq!(refused(&unknown, &main), Some(CheckpointError::UnknownFormat));

    let mut store = Store::default();
    let mut session = case.admit(&core).expect("admitted");
    while session.record().revision() < 3 {
        session = accepted(session);
    }
    let mut now = session.checkpoint(&mut store).expect("saved").as_bytes().to_vec();
    now[..8].copy_from_slice(b"ngcheck1");
    let swapped = Checkpoint::from_bytes(now);
    assert!(matches!(
        refused(&swapped, &store),
        Some(CheckpointError::HistoryAltered { .. })
    ));
}

#[test]
fn a_fork_refuses_a_checkpoint_of_the_other_format_even_at_revision_zero() {
    let case = water_case();
    let (core, fresh) = (faulting(), faulting());
    let identity = identity_of(&case, &core);
    let main_zero = checkpoint(WATER_CHECKPOINT_0);
    let admitted = case.admit(&core).expect("admitted");
    let refused = admitted.rewind(&main_zero, branch(b"main's start")).err();
    assert_eq!(refused, Some(CheckpointError::NotOnThisBranch));

    let now_zero = admitted.checkpoint(&mut Store::default()).expect("saved");
    let loaded =
        Session::resume(&main_zero, &identity, case.body(), &fresh, &Store::default()).expect("resumed");
    let refused = loaded.rewind(&now_zero, branch(b"this build's start")).err();
    assert_eq!(refused, Some(CheckpointError::NotOnThisBranch));
    assert!(loaded.rewind(&main_zero, branch(b"its own start")).is_ok());
}

struct Course<'c> {
    case: &'c dyn ReplayCase,
    core: &'c dyn CoupledSolver,
    fresh: &'c dyn CoupledSolver,
    identity: &'c str,
    saved: &'c str,
    links: &'c [(&'c str, &'c str)],
    seals: &'c [&'c str],
    digests: [&'c str; 8],
}

#[test]
fn checkpoints_main_wrote_with_a_continuation_and_with_a_firing_resume_as_main_did() {
    let water = water_case();
    let (solver, fresh_solver) = (ng_solver::Core::new(8 * SECOND), ng_solver::Core::new(8 * SECOND));
    let chance = ChanceCase::ectopy().expect("a body");
    let (reference, fresh_reference) = (ReferenceIntegrator::new(SECOND), ReferenceIntegrator::new(SECOND));
    let courses = [
        Course {
            case: &water,
            core: &solver,
            fresh: &fresh_solver,
            identity: SOLVER_IDENTITY,
            saved: SOLVER_CHECKPOINT_3,
            links: &SOLVER_LINKS,
            seals: &SOLVER_SEALS_AFTER,
            digests: SOLVER_DIGESTS,
        },
        Course {
            case: &chance,
            core: &reference,
            fresh: &fresh_reference,
            identity: CHANCE_IDENTITY,
            saved: CHANCE_CHECKPOINT_7,
            links: &CHANCE_LINKS,
            seals: &CHANCE_SEALS_AFTER,
            digests: CHANCE_DIGESTS,
        },
    ];
    for course in courses {
        let identity = identity_of(course.case, course.core);
        assert_eq!(identity.digest(), digest(course.identity));
        let store = Store::of(course.links);
        let saved = checkpoint(course.saved);
        let mut session =
            (Session::resume(&saved, &identity, course.case.body(), course.fresh, &store)).expect("resumed");
        for seal in course.seals {
            session = accepted(session);
            assert_eq!(session.record().seal().digest(), digest(seal));
        }
        assert_eq!(named(session.record().digests()), course.digests.map(digest));
    }
}
const WATER_IDENTITY: &str = "a1c6ba75453ca162c2f45bb119873c60aa461d311d3b5ba20434a4b0f99c6b25";
const WATER_SEALS: [&str; 10] = [
    "cc271c2eef942faa880aa857bdd7e44896ef41dba4f523f2d05f7508b0175e93",
    "c32e4e0da8b1a991028d2469750636895a29675370b1c3e4c2e16ba768a86207",
    "5a83bb92b4f4c1d154fbefc4c4b097dc3e26efaf998855705d3564438720a24d",
    "2719991efcdd4e65541b5c1813106c2a9dc5a6edabe89154ed14f9a884cabc81",
    "0e95a2483f4f41c2d29dff997d21d5adc6c8cb6b40ea1ed951027378092ae096",
    "a4be339b38754f285c6e927991bf349e8bd4260a3acd5418bf32b68c3406c6a2",
    "92cbc304481ce075deea0a2ac1ff29dc5a88e39123137b9a2a462e9513d4bd32",
    "f158cedbe01f9cd67531801f6cd3c28770add5a8215323a483a2bd4aa95fbdef",
    "7f284c991333021b28f8102e8d57322520f02098320d029e793fd98b506a1ded",
    "2382e330e8739c4b4f6ab24464312a35cb82c937654f8ff55f563473ec5e5db2",
];
const WATER_CHECKPOINT_0: &str = concat!(
    "6e67636865636b317eec50e01192ec46e1ad708a19df68054e7b5a9de08e5a533e0464b8d9e8150200000000",
    "0000022ba1c6ba75453ca162c2f45bb119873c60aa461d311d3b5ba20434a4b0f99c6b250000000000000000",
    "cc271c2eef942faa880aa857bdd7e44896ef41dba4f523f2d05f7508b0175e93000000000000000000000000",
    "000001c300000000000000136e672d636f6e747261637420696d616765203100000000000000000000000000",
    "000002411e8c50000000000000000000000000411e7cb0000000000000000000000000000000000000000000",
    "00000000000000000000000000000000000000000000000000000000000000a88b7bcf57bfea7e58366db917",
    "ca9c5fd0dafc1ce80c51bc6427ea54e75116e300000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000001c96c6d5be8d08a12e7b5cdc1b207fa6b2430974c86",
    "803d8891675e76fd992c2000000009502f9000000000000000000000000000000000086578746572696f7240",
    "590000000000000000000df8475800000000000000000000000000000000000000000000000001bf6870b53f",
    "d7c4e2b637b73fd906a4e2584c26516a6033c97fd4aac7bb0eb775a16338610d4e12a796c536301a21e90758",
    "99055e98f9dd1f2dad6b4805c3c92c00000000000000033ff00000000000003ff00000000000003f50624dd2",
    "f1a9fc0000000000000002411e8c5000000000411e7cb00000000000000000000000023ff00000000000003f",
    "f00000000000000000000000000000a1c6ba75453ca162c2f45bb119873c60aa461d311d3b5ba20434a4b0f9",
    "9c6b2500000000000000000000000000000001412e848000000000000000000000000077797bb700bdb46ebb",
    "d4c246f6554d9cc633d75ad39c870f07440f238d632e1d",
);
const WATER_CHECKPOINT_3: &str = concat!(
    "6e67636865636b315c01f852005d978c315a7e1bf3b3964fb3846eaeaee56a15d37843cbacb02bd300000000",
    "0000027ba1c6ba75453ca162c2f45bb119873c60aa461d311d3b5ba20434a4b0f99c6b250000000000000003",
    "2719991efcdd4e65541b5c1813106c2a9dc5a6edabe89154ed14f9a884cabc81000000000000000000000000",
    "0000021300000000000000136e672d636f6e747261637420696d6167652031000000059682f0000000000000",
    "000002411e8bf2a1b5402c3dbcabe000000000411e7d0d5e4abfd4bdbcabe000000000000000000000000000",
    "00000000000000000000000000000000000000000000000000000000000000a88b7bcf57bfea7e58366db917",
    "ca9c5fd0dafc1ce80c51bc6427ea54e75116e300000003b9aca0000000000000000000000000000000000100",
    "000003b9aca000000000059682f0000000000000000002411e8c114282749e3d96870000000000411e7ceebd",
    "7d8b62bd968700000000000000000000000002411e8bf2a1b5402c3dbcabe000000000411e7d0d5e4abfd4bd",
    "bcabe0000000000000000000000001c96c6d5be8d08a12e7b5cdc1b207fa6b2430974c86803d8891675e76fd",
    "992c2000000009502f9000000000000000000000000000000000086578746572696f72405900000000000000",
    "00000df84758000000000000000000000000000000000000000003b9aca000bf6870b53fd7c4e2b637b73fd9",
    "06a4e2584c26516a6033c97fd4aac7bb0eb775a16338610d4e12a796c536301a21e9075899055e98f9dd1f2d",
    "ad6b4805c3c92c00000000000000033ff00000000000003ff00000000000003f50624dd2f1a9fc0000000000",
    "000002411e8c5000000000411e7cb00000000000000000000000023ff00000000000003ff000000000000000",
    "00000000000000a1c6ba75453ca162c2f45bb119873c60aa461d311d3b5ba20434a4b0f99c6b250000000000",
    "0000000000000000000001412e848000000000000000000000000077797bb700bdb46ebbd4c246f6554d9cc6",
    "33d75ad39c870f07440f238d632e1d",
);
const WATER_CHECKPOINT_6: &str = concat!(
    "6e67636865636b3189b0f5326fb509fa8c738613f79c761082922398c5df3176c0f02057d1155d2500000000",
    "00000283a1c6ba75453ca162c2f45bb119873c60aa461d311d3b5ba20434a4b0f99c6b250000000000000006",
    "92cbc304481ce075deea0a2ac1ff29dc5a88e39123137b9a2a462e9513d4bd32000000000000000000000000",
    "0000021b00000000000000136e672d636f6e747261637420696d61676520310000000b2d05e0000000000000",
    "000002411e8c38ac1c2a033dbfbe0000000000411e7d6753e3d5fdbdbfbe0000000000000000000000000000",
    "00000000000000000000000000000000000000000000000000000000000000a88b7bcf57bfea7e58366db917",
    "ca9c5fd0dafc1ce80c51bc6427ea54e75116e300000009502f90000000000000000000000000000000000100",
    "000009502f90000000000b2d05e0000000000000000002411e8bb6d208ec483da47a4000000000411e7d492d",
    "f713b8bda47a40000000000000000000000002411e8c38ac1c2a033dbfbe0000000000411e7d6753e3d5fdbd",
    "bfbe00000000000000000000000001c96c6d5be8d08a12e7b5cdc1b207fa6b2430974c86803d8891675e76fd",
    "992c2000000009502f9000000000000000000000000000000000086578746572696f72405900000000000000",
    "00000df847580000000000000000010000000000000001000000000000000000000009502f9000bf6870b53f",
    "d7c4e2b637b73fd906a4e2584c26516a6033c97fd4aac7bb0eb775a16338610d4e12a796c536301a21e90758",
    "99055e98f9dd1f2dad6b4805c3c92c00000000000000033ff00000000000003ff00000000000003f50624dd2",
    "f1a9fc0000000000000002411e8c5000000000411e7cb00000000000000000000000023ff00000000000003f",
    "f00000000000000000000000000000a1c6ba75453ca162c2f45bb119873c60aa461d311d3b5ba20434a4b0f9",
    "9c6b2500000000000000000000000000000001412e848000000000000000000000000077797bb700bdb46ebb",
    "d4c246f6554d9cc633d75ad39c870f07440f238d632e1d",
);
const WATER_EXPORT_9: &str = concat!(
    "6e67636865636b31ad341663b01a09a0299b82f700308c3484c0a5b5dedc6c634e281d9ea579efaa00000000",
    "0000027ba1c6ba75453ca162c2f45bb119873c60aa461d311d3b5ba20434a4b0f99c6b250000000000000009",
    "2382e330e8739c4b4f6ab24464312a35cb82c937654f8ff55f563473ec5e5db2000000000000000100000000",
    "0000021300000000000000136e672d636f6e747261637420696d61676520310000000fd51da8000000000000",
    "000002411e8cdae0ce54bb3daa1ba000000000411e7db51f31ab45bdaa1ba000000000000000000000000000",
    "00000000000000000000000000000000000000000000000000000000000000a88b7bcf57bfea7e58366db917",
    "ca9c5fd0dafc1ce80c51bc6427ea54e75116e30000000df84758000000000000000000000000000000000100",
    "00000df84758000000000fd51da8000000000000000002411e8cfa0637ab2dbdb439d000000000411e7d95f9",
    "c854d33db439d0000000000000000000000002411e8cdae0ce54bb3daa1ba000000000411e7db51f31ab45bd",
    "aa1ba0000000000000000000000001c96c6d5be8d08a12e7b5cdc1b207fa6b2430974c86803d8891675e76fd",
    "992c2000000009502f9000000000000000000000000000000000086578746572696f72405900000000000000",
    "00000df8475800000000000000000100000000000000000000000df8475800bf6870b53fd7c4e2b637b73fd9",
    "06a4e2584c26516a6033c97fd4aac7bb0eb775a16338610d4e12a796c536301a21e9075899055e98f9dd1f2d",
    "ad6b4805c3c92c00000000000000033ff00000000000003ff00000000000003f50624dd2f1a9fc0000000000",
    "000002411e8c5000000000411e7cb00000000000000000000000023ff00000000000003ff000000000000000",
    "00000000000000a1c6ba75453ca162c2f45bb119873c60aa461d311d3b5ba20434a4b0f99c6b250000000000",
    "0000000000000000000001412e848000000000000000000000000077797bb700bdb46ebbd4c246f6554d9cc6",
    "33d75ad39c870f07440f238d632e1d",
);
const WATER_OBSERVER_6: &str = concat!(
    "6e67776174636831717c72209d833890cf556dd5e6c3a5cb851cc112b3f00dd5ec26803f5bff2d1286097afc",
    "e027444b4188beae091a4630fd8bc5e6fef08657951e760cbee7aeb0000000000000000192cbc304481ce075",
    "deea0a2ac1ff29dc5a88e39123137b9a2a462e9513d4bd320000000000000000411e8c38ac1c2a033dbfbe00",
    "0000000000000000000000010000000000000000000000000000000000000000000000000000000000000000",
    "0000000000000000000000000000000a00000004e3b29200411e7d01e1fdcc29bd9020500000000000000005",
    "9682f000411e7d0d5e4abfd4bdbcabe0000000000000000649534e00411e7d18ac1b39acbdbbe73c00000000",
    "00000006fc23ac00411e7d23f9ebb384bdbb22980000000000000007aef40a00411e7d2f387c11483d9db330",
    "000000000000000861c46800411e7d3a588c36e6bdb26e5000000000000000091494c600411e7d45789c5c83",
    "3db3b6940000000000000009c7652400411e7d50b77244493d58aa00000000000000000a7a358200411e7d5c",
    "05ab0d23bdaf5b58000000000000000b2d05e000411e7d6753e3d5fdbdbfbe0000000000",
);
const WATER_OBSERVER_UNREAD: &str = concat!(
    "6e67776174636831ac98e92dbabca083b9783ecfd1d731a8cb8ccf1bddc9f75d333883ba5ff9830c86097afc",
    "e027444b4188beae091a4630fd8bc5e6fef08657951e760cbee7aeb000000000000000000000000000000000",
    "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    "00000000000000000000000000000000000000000000000000000000",
);
const WATER_LINKS: [(&str, &str); 6] = [
    (
        "c32e4e0da8b1a991028d2469750636895a29675370b1c3e4c2e16ba768a86207",
        concat!(
            "cc271c2eef942faa880aa857bdd7e44896ef41dba4f523f2d05f7508b0175e933847387b9deeeeb93e952560",
            "79d361a32e474814534384600296ca3000f3c903000000000000000000000001dcd65000412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "000000000000000000000000000000000000000000000000000000000000000000000000401f9ebbf3dfc290",
            "bcc00000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "5a83bb92b4f4c1d154fbefc4c4b097dc3e26efaf998855705d3564438720a24d",
        concat!(
            "c32e4e0da8b1a991028d2469750636895a29675370b1c3e4c2e16ba768a86207cda799236abb89055c665fba",
            "4b989954401e6584f314caa084056e626186103e00000001dcd6500000000003b9aca000412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000401f9ebbf3dfc290bcc0000000000000402f5ebec5b0f4bc",
            "3cd00000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "2719991efcdd4e65541b5c1813106c2a9dc5a6edabe89154ed14f9a884cabc81",
        concat!(
            "5a83bb92b4f4c1d154fbefc4c4b097dc3e26efaf998855705d3564438720a24d53fc59dad75cef5b66f2859f",
            "0fceccda32153bb7ea36cf9b676df5eac71f2f0300000003b9aca000000000059682f000412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000402f5ebec5b0f4bc3cd000000000000040375792aff4e354",
            "3cc00000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "0e95a2483f4f41c2d29dff997d21d5adc6c8cb6b40ea1ed951027378092ae096",
        concat!(
            "2719991efcdd4e65541b5c1813106c2a9dc5a6edabe89154ed14f9a884cabc817ae129bddc2cb3fa8ccb7ce1",
            "d2317a6c72250e086752e69a8fd285235ecee972000000059682f0000000000773594000412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "000000000000000000000000000000000000000040375792aff4e3543cc0000000000000403ee0c85684e560",
            "3ce00000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "a4be339b38754f285c6e927991bf349e8bd4260a3acd5418bf32b68c3406c6a2",
        concat!(
            "0e95a2483f4f41c2d29dff997d21d5adc6c8cb6b40ea1ed951027378092ae09610b19b41753406c54206e6d7",
            "2baa4f92a7cdea6cb8c2462401c35fb50438c963000000077359400000000009502f9000412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000403ee0c85684e5603ce0000000000000404325bee276fae1",
            "3cec0000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "92cbc304481ce075deea0a2ac1ff29dc5a88e39123137b9a2a462e9513d4bd32",
        concat!(
            "a4be339b38754f285c6e927991bf349e8bd4260a3acd5418bf32b68c3406c6a257c7774265ff279943ef1b19",
            "2dc13ff13371118a0b596e4a886400625ee3c2e600000009502f90000000000b2d05e000412e848000000000",
            "0000000000000000412e84d00000000000000000000000000000000000000000000000000000000040440000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000040440000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000404325bee276fae13cec0000000000004046ea7c7abf9021",
            "00000000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
];
const WATER_DIGESTS: [&str; 8] = [
    "db9e5d00222618ea7ed02f67819ced11832665a0a7a4b03e4a9f4d035ce21c43",
    "0311647f7bb84facb68a8c93a5a968a12153c93fb82c40b9fed686ad1988abca",
    "3dda4bf111b3f43df52075428c85f646b961839d91dcc05b9b20a67d6939cb68",
    "32233292dfaf316ed0dfc6c4b3caf9938f430d36cc2f00db5e96fa19c8d51a5f",
    "5cacdd2ba9f61b20e549284c7d8b4516403793e7e2032020a4d40df40fa39d47",
    "dce03cecfcf179e28ff98d78d8c825bd31ea49722ec7a0d5a3778ff6431ed8aa",
    "5f082c678c10735da3eca62c0845c41577c4c795d714ee466845c1b378533e07",
    "bb16314c7064935be82504eaf88101c4442a7c97d1e7fa970598682073bf93bf",
];
const WATER_STREAM_WHOLE: &str = "67357de59fd948e6e3a2c961119d629430417a5a7eb82b5818af4f77f0cf123c";
const WATER_STREAM_AFTER_6: &str = "fd694728b90a14a4c63801351757491d6ea6b29a79262df4b51a0d90b8f180bf";
const SOLVER_IDENTITY: &str = "a8a55892a037646c0a00c65594dfc1abd4588ab1ee431afdc30ec1329e84c375";
const SOLVER_CHECKPOINT_3: &str = concat!(
    "6e67636865636b312c49828825dab638669e40577d4303eac8ac88a1c168ef3d0edce2dc168d488300000000",
    "0000029ba8a55892a037646c0a00c65594dfc1abd4588ab1ee431afdc30ec1329e84c3750000000000000003",
    "d0d7856bb065601b4fa3beee62988220cfe4276ce616d1122cac6924b32365c6000000000000000000000000",
    "0000023300000000000000136e672d636f6e747261637420696d6167652031000000059682f0000000000000",
    "000002411e8bf2a1b5402c3dbcabe000000000411e7d0d5e4abfd4bdbcabe000000000000000000000000000",
    "00000000000000000000000000000000000000000000000000000000000000debcb6a9cfb965f1ab7ea2229b",
    "c7905cacc5350f587dbcdd4c134dadca1fbab50000000000000000000000000000000400000001dcd650003f",
    "10018c0da2650f411e8c01e27d5b50411e7cfe1d82a4b0000000000000000100000003b9aca0000000000596",
    "82f0000000000000000002411e8c114282749e3d96870000000000411e7ceebd7d8b62bd9687000000000000",
    "00000000000002411e8bf2a1b5402c3dbcabe000000000411e7d0d5e4abfd4bdbcabe0000000000000000000",
    "000001c96c6d5be8d08a12e7b5cdc1b207fa6b2430974c86803d8891675e76fd992c2000000009502f900000",
    "0000000000000000000000000000086578746572696f7240590000000000000000000df84758000000000000",
    "000000000000000000000000000003b9aca000bf6870b53fd7c4e2b637b73fd906a4e2584c26516a6033c97f",
    "d4aac7bb0eb7753be3f8eea9cdd5fdc3f736c95eecff3362fa2eb0e8b3773b7b14c56c8934af920000000000",
    "0000033ff00000000000003ff00000000000003f50624dd2f1a9fc0000000000000002411e8c500000000041",
    "1e7cb00000000000000000000000023ff00000000000003ff00000000000000000000000000000a8a55892a0",
    "37646c0a00c65594dfc1abd4588ab1ee431afdc30ec1329e84c3750000000000000000000000000000000141",
    "2e848000000000000000000000000061225494a926c1f9a613e0b68a96e8128c0c0f4fafcbf4b73eddee225d",
    "145287",
);
const SOLVER_LINKS: [(&str, &str); 3] = [
    (
        "862bed5d27ac08285a2f65b1d228b755528ffb09b93a688f1b03df55800f5a83",
        concat!(
            "7645e145ec6de74f32d50e7feab717d06ea9ec7776f5ceecbb28ba30a55e895a9f9e46aaa46365808a685429",
            "ecfc4f0f9c0375a9a0d310ad7246cd062b01d180000000000000000000000001dcd65000412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "000000000000000000000000000000000000000000000000000000000000000000000000401f9ebbf3dfc290",
            "bcc00000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "c045e477c1abd7415721e3f47dbd70f905a1b166fb18f115fabfabc6f99484c0",
        concat!(
            "862bed5d27ac08285a2f65b1d228b755528ffb09b93a688f1b03df55800f5a838858b8a90c436f82e9626802",
            "8d07db080e24abe60b208c9016f0e9961c27cb3000000001dcd6500000000003b9aca000412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000401f9ebbf3dfc290bcc0000000000000402f5ebec5b0f4bc",
            "3cd00000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "d0d7856bb065601b4fa3beee62988220cfe4276ce616d1122cac6924b32365c6",
        concat!(
            "c045e477c1abd7415721e3f47dbd70f905a1b166fb18f115fabfabc6f99484c03f88811e65ede32b04d4ca5d",
            "602f882b5202b2bfe9d3b56bf7fc1154062378ed00000003b9aca000000000059682f000412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000402f5ebec5b0f4bc3cd000000000000040375792aff4e354",
            "3cc00000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
];
const SOLVER_SEALS_AFTER: [&str; 3] = [
    "44dadabc339af4b01befc248a330e229608569b5a35c3a38e887b978bc125beb",
    "e0265b1949e29724ea852d7cf29dec0b0372cc5a57cc9a78208cf2a13890ca39",
    "cc3f48cf48ee4fde38af5d703a96a0f617a08bf73551d50076d70d3d98fde04b",
];
const SOLVER_DIGESTS: [&str; 8] = [
    "f522865bc85d52505e9f8b4e312d7a63831dadca280973e0bf1d15dcde0cda29",
    "f44981fdd735d3f81267e88efd6be99b5dae932f0dc210628075467ab1ee65a1",
    "4df410119350404ee1d6a0fe225903c4574b128c7e5f2eb0c09e881cd5a9f695",
    "32233292dfaf316ed0dfc6c4b3caf9938f430d36cc2f00db5e96fa19c8d51a5f",
    "5cacdd2ba9f61b20e549284c7d8b4516403793e7e2032020a4d40df40fa39d47",
    "4047983876882802329ec69dce7a00b8af10033f62330bfe12de014d42d8ec64",
    "5f082c678c10735da3eca62c0845c41577c4c795d714ee466845c1b378533e07",
    "17a911744fe9df11072a2b0b443ddfd6d3508333b00fd87b7dd39e5400833af0",
];
const CHANCE_IDENTITY: &str = "24201c9444b8b3c7aefa9123fe01e4056569e2b9006c35dde243709044aaf74c";
const CHANCE_CHECKPOINT_7: &str = concat!(
    "6e67636865636b3130ee2f1592683e8024f51ffca21e695906f72c434b3736c7773f1ab6a519611b00000000",
    "0000037b24201c9444b8b3c7aefa9123fe01e4056569e2b9006c35dde243709044aaf74c0000000000000007",
    "3681d2cacce69795f2e5beb258d99c37690f73ae8c132dfc7e3122c1d21bf656000000000000000000000000",
    "0000031300000000000000136e672d636f6e747261637420696d616765203100000001867655de0000000000",
    "000002411e8c35fad8726dbdb9abbc00000000411e7cca05278d933db9abbc00000000000000000000000300",
    "0000000000000000000000000000003ff0000000000000000000000000000000000000000000010000000000",
    "00000000000000000000020000000000000001000000000000000000000000000000014023bef94353cb85e6",
    "b086984331b24dc1c24ae424f2e0ae6014c8992e08f5340000000165a0bc0000000000000000000000000000",
    "0000010000000165a0bc0000000001867655de0000000000000005411e8c3827c0158abda5a1e80000000041",
    "1e7cc7d83fea763da5a1e8000000003fd26a5806ca97e4000000000000000000000000000000000000000000",
    "0000003ff000000000000000000000000000000000000000000005411e8c35fad8726dbdb9abbc0000000041",
    "1e7cca05278d933db9abbc000000003fd4019748cb1fe9000000000000000000000000000000000000000000",
    "0000003ff0000000000000000000000000000000000000000000017faa2b0cb51d95b0ff5b33272101993313",
    "2b175e1590985b8725df6206e2853e00000006fc23ac00000000000000000000000000000000086578746572",
    "696f7240590000000000000000000df8475800000000000000000000000000000000000000000165a0bc0050",
    "46533a022e3e1d81e32df83040cc50920ad3cd9965b6c66b4a9a6b24305c01dcd911828fcbcda487cc6e837e",
    "86637c27148acf1ff0546e51b50099b87918d2000000000000000a3ff00000000000003ff00000000000003f",
    "a999999999999a3f50624dd2f1a9fc3f40624dd2f1a9fc3fe000000000000040240000000000003fc9999999",
    "99999a3f847ae147ae147b3ff00000000000000000000000000005411e8c5000000000411e7cb00000000000",
    "0000000000000000000000000000003ff000000000000000000000000000053ff00000000000003ff0000000",
    "0000003ff00000000000003ff00000000000003ff00000000000000000000000000001000000000000000124",
    "201c9444b8b3c7aefa9123fe01e4056569e2b9006c35dde243709044aaf74c00000000000000000000000000",
    "000001412e8480000000000000000000000000570b91864cf7aad52ff1b1c0554e73d04ca8071352041c9a35",
    "8f0df24d5ff375",
);
const CHANCE_LINKS: [(&str, &str); 7] = [
    (
        "0c8c05cc840754ffc0b7878181227473aea790ae6af64563b0055ace8d3d2789",
        concat!(
            "6a930088385d63ba6d217091802799d9077dd31a727d502a9be476620013a6c2e677a2d65b7b9ed38b502a9c",
            "3740b73e9ab25e876fbe6cabf63b38042e089e040000000000000000000000003b9aca00412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000003feff3ba76088313",
            "00000000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "b6088398fe586f45c91bf09afecade547029226641625526e5a47fb2513bf042",
        concat!(
            "0c8c05cc840754ffc0b7878181227473aea790ae6af64563b0055ace8d3d2789180be8ea0901dfc4c5e97ad8",
            "aa11074789a652917c2c43a069e90b883c10b721000000003b9aca000000000077359400412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "00000000000000000000000000000000000000003feff3ba7608831300000000000000003fffeb8f96626b86",
            "bca00000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "6802705636ac66666c523767e017c392087292c942125cddd582d93233af755c",
        concat!(
            "b6088398fe586f45c91bf09afecade547029226641625526e5a47fb2513bf042ee28068498f4c91953c74fb3",
            "8a08406c085196aab0cb08ad6ab2570f567aef99000000007735940000000000b2d05e00412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "00000000000000000000000000000000000000003fffeb8f96626b86bca00000000000004007ea8d9f8377cf",
            "3c900000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "0da4f7eed5a8f45c7ee0c255ad34dcbf05a7d1b4db3197632c46fbc4fba55a55",
        concat!(
            "6802705636ac66666c523767e017c392087292c942125cddd582d93233af755cd5837bfd09f31ffb2c9c635a",
            "12fd33c858df75ebe81d91c058e43eb4dd97bfc400000000b2d05e0000000000ee6b2800412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "00000000000000000000000000000000000000004007ea8d9f8377cf3c90000000000000400fdb422fdde5e4",
            "bca00000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "7b35b5b89b01e7b7f7d449fe80550b28adcff3e1d93110d8897ab88288d55c75",
        concat!(
            "0da4f7eed5a8f45c7ee0c255ad34dcbf05a7d1b4db3197632c46fbc4fba55a557015c97b07cb60ddf2a84423",
            "dd2695dc52745b7e6e3afc6e4ce54dc851ddcde200000000ee6b2800000000012a05f200412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000400fdb422fdde5e4bca00000000000004013e3f3c84a6667",
            "bcb80000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "07ff75140710158c522bf49e69d2951ea10b896ac9b218e93ff696328283c5a4",
        concat!(
            "7b35b5b89b01e7b7f7d449fe80550b28adcff3e1d93110d8897ab88288d55c75e05264692d899c5a056680eb",
            "20557b9e47121b88117871eb85f95f3a5d279c80000000012a05f2000000000165a0bc00412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "00000000000000000000000000000000000000004013e3f3c84a6667bcb80000000000004017d83fea762b44",
            "bca80000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
    (
        "3681d2cacce69795f2e5beb258d99c37690f73ae8c132dfc7e3122c1d21bf656",
        concat!(
            "07ff75140710158c522bf49e69d2951ea10b896ac9b218e93ff696328283c5a4235c2c8379d0fa8972985ac3",
            "8889f19b20911f3e31a001fc8fd7c814ea4fd3470000000165a0bc0000000001867655de412e848000000000",
            "0000000000000000412e84800000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "00000000000000000000000000000000000000004017d83fea762b44bca8000000000000401a05278d9366af",
            "bc900000000000000000000000000000000000000000000000000000000000000000000000000000",
        ),
    ),
];
const CHANCE_SEALS_AFTER: [&str; 3] = [
    "24fd7714188fd4975dea61ec46a159c3731febf8f1f05dfc08af71d249ad3e18",
    "e4f0924f188a8a7040082b6b9936b255a5fdcb6b1e101fa01d3c31752a9d141b",
    "342cf5ba3d540fd3ebcc8b15e993fd9f0bbaa697259f4c339e643ac54f046b1d",
];
const CHANCE_DIGESTS: [&str; 8] = [
    "27891330dadd4a7a916d0f2e0aaa002e2203f6fbc87d661b63e1917f395c6485",
    "268a80e8d8c37b230a781acf485b533a4b771d6997317c6961957f72687a7d61",
    "1e0c3416690d321266957213c5f55885bc938d5239648d4c3dd21802a669ea2d",
    "19cf275e0ccbdf4f4e0a12fbf6211ce44d35b548cde40e0fe15274de457aa924",
    "836fad37ba0d48cfa58758c71e2c381f77f0d5131be291bd4156fa76a542f87d",
    "61495ea10689af74f18d2cb585d3cdcd203d4c50d88841e16c5c11d3ab57eb97",
    "c0dbaac603be4a3bbed664fb7b41ba97aed36facadcec14cb9fc8edc82e778bd",
    "53aa3543c51bee9c6798bb85bd7fbaeb78dddc0b4d24069373b28adf127effcb",
];
