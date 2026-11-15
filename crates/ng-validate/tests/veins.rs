use ng_contract::*;
use ng_record::{AuthorizedInput, Running, Session};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const SHARE: f64 = 0.05;
const WIDTH: f64 = 1.0;
const ALBUMIN_VOLUME: f64 = 0.05;

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
    assert!(
        least > 0.5 * first,
        "the leg veins fell from {first} to {least} mL"
    );
}

/// The leg veins sit in no enclosure, so their pressure is `E s`. No requirement bounds heat or muscle work, so the
/// test holds neither.
#[test]
fn a_vein_below_its_unstressed_volume_lowers_its_pressure_as_it_empties_and_the_ledger_closes() {
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
    let (mut collapsed, mut emptied) = (None, (f64::INFINITY, 0.0));
    tilted(-0.2, 11 * SECOND, |body, session| {
        let held = value(body, session, store);
        let closure = (held - value(body, session, sum)).abs();
        assert!(held > 0.0 && closure <= 1e-6 * held, "{held} {closure}");
        let (pressure, volume) = (value(body, session, leg), value(body, session, water));
        if pressure < 0.0 {
            collapsed.get_or_insert((volume, pressure));
            if volume < emptied.0 {
                emptied = (volume, pressure);
            }
        }
    });
    let (start, start_pressure) = collapsed.expect("the leg veins went below their unstressed volume");
    let (least, pressure) = emptied;
    assert!(
        least < start - 10.0 && pressure < start_pressure,
        "{start} {start_pressure} {least} {pressure}"
    );
}

fn residuals_at(
    body: &CompiledBody,
    key: &'static str,
    volume: f64,
) -> Result<Vec<f64>, InitializationFailure> {
    let (packet, lens) = (arena_packet(body)?, arena_lens(body)?);
    let parameters = body.bind(packet)?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(anchor_moment()).reading(&lens);
    let system = body.initialization(parameters, port, request)?;
    let mut workspace = system.workspace();
    let at = vessel(key);
    let (water, albumin) = (
        index(body, lumen(at, Species::Water)),
        index(body, lumen(at, Species::Albumin)),
    );
    let held = workspace.unknowns_mut()[albumin];
    let stages = workspace.fitted_slots().start;
    for slot in (0..stages).filter(|slot| slot % body.rows().len() == water) {
        workspace.unknowns_mut()[slot] = volume - ALBUMIN_VOLUME * held;
    }
    Ok(system
        .stage_residuals(&workspace, workspace.legs_of(0)[0])?
        .to_vec())
}

/// Below `V_u` a marked vein's pressure, less what lies outside it, is `E share (V_u / n) (1 - (V_u / V)^n)` up to
/// the soft floor's and the soft limit's widths. A stage's row is its value less its law, so the law's change
/// between two volumes is the residual's fall, and the outside pressure cancels. At `V_u` the stressed volume is
/// `(1 - share) w / 2`. The staged workspace reads the junction's effect as one, so the leg veins, which a nerve
/// reaches, stand at `V_u` 566 - 200 = 366 mL.
#[test]
fn below_its_unstressed_volume_a_marked_vein_follows_the_closed_form() -> Result<(), InitializationFailure> {
    let body = arena_body()?;
    for (key, elastance, unstressed) in [("central veins", 0.1, 300.0), ("leg veins", 0.4, 366.0)] {
        let row = index(&body, pressure(vessel(key)));
        let rest = residuals_at(&body, key, unstressed)?[row];
        for volume in [unstressed / 2.0, unstressed / 4.0] {
            let wall =
                rest - residuals_at(&body, key, volume)?[row] + elastance * (1.0 - SHARE) * WIDTH / 2.0;
            let ratio: f64 = unstressed / volume;
            let shrunk = (unstressed / 1.5) * (1.0 - ratio * ratio.sqrt());
            let closed = elastance * SHARE * shrunk;
            let (low, high) = (
                -elastance * SHARE * WIDTH * WIDTH / (4.0 * -shrunk),
                elastance * WIDTH * WIDTH / (4.0 * (unstressed - volume)),
            );
            let gap = wall - closed;
            assert!(
                gap > low - 1e-12 && gap < high + 1e-12,
                "{key} at {volume} mL: {wall} against {closed}"
            );
        }
    }
    let (elastance, unstressed) = (1.0, 700.0);
    let row = index(&body, pressure(vessel("systemic arteries")));
    let rest = residuals_at(&body, "systemic arteries", unstressed)?[row];
    let wall =
        rest - residuals_at(&body, "systemic arteries", unstressed / 2.0)?[row] + elastance * WIDTH / 2.0;
    let floor = elastance * smooth_max(-unstressed / 2.0, WIDTH);
    assert!((wall - floor).abs() < 1e-12, "{wall} against the floor {floor}");
    Ok(())
}

/// The elastic energy row is its value less the sum of `E s^2 / 2` over the blood sites, so its fall between two
/// staged volumes of one vein is the growth of that vein's part, which the pressure row's `E s` gives.
#[test]
fn below_its_unstressed_volume_a_marked_veins_part_of_the_store_grows() -> Result<(), InitializationFailure> {
    let body = arena_body()?;
    let energy = index(&body, algebraic("CV", "elastic energy", Context::Body));
    let row = index(&body, pressure(vessel("central veins")));
    let (elastance, unstressed) = (0.1, 300.0);
    let rest = residuals_at(&body, "central veins", unstressed)?;
    let at_rest = elastance * (1.0 - SHARE) * WIDTH / 2.0;
    let mut last = 0.0;
    for volume in [unstressed / 2.0, unstressed / 4.0] {
        let staged = residuals_at(&body, "central veins", volume)?;
        let wall = rest[row] - staged[row] + at_rest;
        let grown = rest[energy] - staged[energy];
        let expected = (wall * wall - at_rest * at_rest) / (2.0 * elastance);
        assert!(
            grown > last && (grown - expected).abs() <= 1e-9 * expected,
            "at {volume} mL: the store's part grew {grown} against {expected}"
        );
        last = grown;
    }
    Ok(())
}

#[test]
fn a_marked_vein_with_no_volume_is_refused_by_the_collapse() -> Result<(), InitializationFailure> {
    let body = arena_body()?;
    let refused = residuals_at(&body, "central veins", -1.0);
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
