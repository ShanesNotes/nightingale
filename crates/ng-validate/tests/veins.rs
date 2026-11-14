use ng_contract::*;
use ng_record::{AuthorizedInput, Running, Session};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const SHARE: f64 = 0.05;
const WIDTH: f64 = 1.0;

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = || Measure::new(0.0).expect("finite");
    let at = Position::new(origin(), origin(), origin());
    AnatomyBuilder::new().site::<C>(key, at).id()
}
fn vessel(key: &'static str) -> SiteId {
    site::<VesselSegment>(key)
}
fn algebraic(owner: &'static str, local: &'static str, at: Context) -> RowKind {
    let law = LawName { owner, local };
    RowKind::Algebraic(Slot { law, at })
}
fn pressure(site: SiteId) -> RowKind {
    algebraic("CV", "blood pressure", Context::Site(site))
}
fn lumen(site: SiteId, species: Species) -> RowKind {
    RowKind::Amount {
        site,
        space: Space::Lumen,
        species,
    }
}
fn index(body: &CompiledBody, kind: RowKind) -> usize {
    let found = body.rows().iter().find(|row| row.kind() == kind);
    found.unwrap_or_else(|| panic!("no row {kind:?}")).id().index()
}
fn value(body: &CompiledBody, session: &Session<'_, Running>, kind: RowKind) -> f64 {
    let row = body.rows()[index(body, kind)].id();
    f64::from_bits(session.record().accepted().value(row).bits()[0])
}
fn smooth_max(a: f64, width: f64) -> f64 {
    0.5 * (a + (a * a + width * width).sqrt())
}

fn head_down(trunk: f64) -> AuthorizedInput {
    let at = SimTime::from_ns(anchor_moment().nanoseconds() + SECOND);
    let posture = Posture::new(Measure::new(trunk).expect("finite"), Legs::Horizontal).expect("in range");
    let event = BoundaryEvent::new(
        InputId::new(Digest::of_canonical(b"head down")),
        at,
        BoundaryInput::Posture(posture),
    );
    AuthorizedInput::from_command_receipt(Digest::of_canonical(b"a test's own command"), event)
}

/// Admits the slice on the halved core, tilts it head-down at 1 s and hands each accepted step to `each` until `until`.
fn tilted(trunk: f64, until: u64, mut each: impl FnMut(&CompiledBody, &Session<'_, Running>)) {
    let (store, core) = (MemoryStore::default(), ng_solver::Core::new(10 * MILLISECOND));
    let body = arena_body().expect("compiled");
    let (packet, lens) = (
        arena_packet(&body).expect("a packet"),
        arena_lens(&body).expect("a lens"),
    );
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let admitted = admit_at(&body, packet, port, &core, &lens, anchor_moment(), &store).expect("admitted");
    let mut session = admitted.session;
    session.enqueue(head_down(trunk)).expect("enqueued");
    let end = anchor_moment().nanoseconds() + until;
    while session.record().accepted().time().nanoseconds() < end {
        let next = SimTime::from_ns(session.record().accepted().time().nanoseconds() + 1);
        session = match run_until(session, next) {
            Ok(reached) => reached,
            Err(stopped) => {
                let at = stopped.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds();
                panic!("{:?} after {at} ns", stopped.fault())
            }
        };
        each(&body, &session);
    }
}

/// Native-contract 33's fault: with no collapse at the veins the leg veins empty at trunk -0.2 and the halved core
/// refuses every step at 29.5 s.
#[test]
fn head_down_at_trunk_minus_a_fifth_runs_past_thirty_seconds_on_the_halved_core() {
    let leg = lumen(vessel("leg veins"), Species::Water);
    let (mut first, mut least) = (None, f64::INFINITY);
    tilted(-0.2, 31 * SECOND, |body, session| {
        let water = value(body, session, leg);
        first.get_or_insert(water);
        least = if water < least { water } else { least };
    });
    let first = first.expect("a step");
    assert!(least > 0.5 * first, "the leg veins fell from {first} to {least} mL");
}

/// Below `V_u` the store holds `E s^2 / 2` at a vein, positive and growing as it empties, and the walls' ledger still
/// closes. The leg veins sit in no enclosure, so their pressure is `E s`. What the store gains past its receipts is
/// muscle work and what it sheds is heat: printed, because no requirement bounds them.
#[test]
fn a_vein_below_its_unstressed_volume_holds_a_growing_store_and_the_ledger_closes() {
    let elastance = 0.4;
    let (leg, water) = (
        pressure(vessel("leg veins")),
        lumen(vessel("leg veins"), Species::Water),
    );
    let store = RowKind::Amount {
        site: site::<TissueSite>("body"),
        space: Space::Elastic,
        species: Species::Energy,
    };
    let sum = algebraic("CV", "elastic energy", Context::Body);
    let power = algebraic("CV", "wall power", Context::Body);
    let (mut then, mut heat, mut muscle) = (anchor_moment().nanoseconds(), 0.0, 0.0);
    let (mut collapsed, mut emptied) = (None, (f64::INFINITY, 0.0));
    tilted(-0.2, 11 * SECOND, |body, session| {
        let held = value(body, session, store);
        let closure = (held - value(body, session, sum)).abs();
        assert!(held > 0.0 && closure <= 1e-6 * held, "{held} {closure}");
        let now = session.record().accepted().time().nanoseconds();
        let span = (now - then) as f64 / 1e9;
        then = now;
        let net = value(body, session, power);
        let shed = smooth_max(net, 100.0);
        heat += span * shed;
        muscle += span * (shed - net);
        let (stressed, volume) = (value(body, session, leg) / elastance, value(body, session, water));
        if stressed < 0.0 {
            let part = elastance * stressed * stressed / 2.0;
            collapsed.get_or_insert((volume, part));
            if volume < emptied.0 {
                emptied = (volume, part);
            }
        }
    });
    let (start, start_part) = collapsed.expect("the leg veins went below their unstressed volume");
    let (least, part) = emptied;
    println!(
        "leg veins below V_u: {start} mL holding {start_part}, then {least} mL holding {part}; heat {heat}, \
         muscle work {muscle}"
    );
    assert!(least < start - 10.0 && part > start_part, "{start} {start_part} {least} {part}");
}

/// The residual of one vessel's pressure row, with its water set so its volume is `volume`, on the start the
/// initializer guesses.
fn residual_at(body: &CompiledBody, key: &'static str, volume: f64) -> Result<f64, InitializationFailure> {
    let (packet, lens) = (arena_packet(body)?, arena_lens(body)?);
    let parameters = body.bind(packet)?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(anchor_moment()).reading(&lens);
    let system = body.initialization(parameters, port, request)?;
    let mut workspace = system.workspace();
    let at = vessel(key);
    let (water, albumin, row) = (
        index(body, lumen(at, Species::Water)),
        index(body, lumen(at, Species::Albumin)),
        index(body, pressure(at)),
    );
    let held = workspace.unknowns_mut()[albumin];
    let stages = workspace.fitted_slots().start;
    for slot in (0..stages).filter(|slot| slot % body.rows().len() == water) {
        workspace.unknowns_mut()[slot] = volume - 0.05 * held;
    }
    let residuals = system.stage_residuals(&workspace, workspace.legs_of(0)[0])?;
    Ok(residuals[row])
}

/// Below `V_u` a marked vein's pressure, less what lies outside it, is `E share (V_u / n) (1 - (V_u / V)^n)` up to
/// the soft floor's and the soft limit's widths, and at an unmarked vessel it is the floor of main. The power 1.5 is
/// `r sqrt(r)`. A stage's row is its value less its law, so the law's change between two volumes is the residual's
/// fall, and the outside pressure cancels. At `V_u` the stressed volume is `(1 - share) w / 2`.
#[test]
fn below_its_unstressed_volume_a_marked_vein_follows_the_closed_form() -> Result<(), InitializationFailure> {
    let body = arena_body()?;
    let (elastance, unstressed) = (0.1, 300.0);
    let rest = residual_at(&body, "central veins", unstressed)?;
    for volume in [unstressed / 2.0, unstressed / 4.0] {
        let wall = rest - residual_at(&body, "central veins", volume)? + elastance * (1.0 - SHARE) * WIDTH / 2.0;
        let ratio = unstressed / volume;
        let shrunk = (unstressed / 1.5) * (1.0 - ratio * ratio.sqrt());
        let closed = elastance * SHARE * shrunk;
        let (low, high) = (
            -elastance * SHARE * WIDTH * WIDTH / (4.0 * -shrunk),
            elastance * WIDTH * WIDTH / (4.0 * (unstressed - volume)),
        );
        let gap = wall - closed;
        assert!(gap > low - 1e-12 && gap < high + 1e-12, "at {volume} mL: {wall} against {closed}");
    }
    let (elastance, unstressed) = (1.0, 700.0);
    let rest = residual_at(&body, "systemic arteries", unstressed)?;
    let wall = rest - residual_at(&body, "systemic arteries", unstressed / 2.0)? + elastance * WIDTH / 2.0;
    let floor = elastance * smooth_max(-unstressed / 2.0, WIDTH);
    assert!((wall - floor).abs() < 1e-12, "{wall} against the floor {floor}");
    Ok(())
}

/// The collapse divides by the volume and takes the logarithm of `V_u / V`, so a marked vein with no volume left, or
/// with an unstressed volume at or below zero, is refused by the term's own name.
#[test]
fn a_marked_vein_with_no_volume_is_refused_by_the_collapse() -> Result<(), InitializationFailure> {
    let body = arena_body()?;
    let refused = residual_at(&body, "central veins", -1.0);
    let collapse = LawName {
        owner: "CV",
        local: "vein collapse",
    };
    assert!(
        matches!(refused, Err(InitializationFailure::Undefined { law }) if law == collapse),
        "{refused:?}"
    );
    Ok(())
}
