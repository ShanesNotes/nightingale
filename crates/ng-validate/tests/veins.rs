use ng_contract::*;
use ng_record::{AuthorizedInput, Running, Session};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;
const SHARE: f64 = 0.05;
const WIDTH: f64 = 1.0;
const ALBUMIN_VOLUME: f64 = 0.05;
const INTERVALS: usize = 400;
/// Each marked vein a test stages, its elastance and its unstressed volume. The staged workspace reads the junction's
/// effect as one, so the leg veins, which a nerve reaches, stand at 566 - 200 = 366 mL.
const VEINS: [(&str, f64, f64); 2] = [("central veins", 0.1, 300.0), ("leg veins", 0.4, 366.0)];

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

fn staged(
    body: &CompiledBody,
    key: &'static str,
    volumes: &[f64],
) -> Result<Vec<Vec<f64>>, InitializationFailure> {
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
    let mut each = Vec::with_capacity(volumes.len());
    for volume in volumes {
        for slot in (0..stages).filter(|slot| slot % body.rows().len() == water) {
            workspace.unknowns_mut()[slot] = volume - ALBUMIN_VOLUME * held;
        }
        each.push(
            system
                .stage_residuals(&workspace, workspace.legs_of(0)[0])?
                .to_vec(),
        );
    }
    Ok(each)
}

fn residuals_at(
    body: &CompiledBody,
    key: &'static str,
    volume: f64,
) -> Result<Vec<f64>, InitializationFailure> {
    Ok(staged(body, key, &[volume])?.remove(0))
}

/// Below `V_u` a marked vein's pressure, less what lies outside it, is `E share (V_u / n) (1 - (V_u / V)^n)` up to
/// the soft floor's and the soft limit's widths. A stage's row is its value less its law, so the law's change
/// between two volumes is the residual's fall, and the outside pressure cancels. At `V_u` the stressed volume is
/// `(1 - share) w / 2`.
#[test]
fn below_its_unstressed_volume_a_marked_vein_follows_the_closed_form() -> Result<(), InitializationFailure> {
    let body = arena_body()?;
    for (key, elastance, unstressed) in VEINS {
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

/// How far the smoothing at `V_u` lets the store's slope stand off the pressure. The stretched part's slope is
/// `E s s'`, not `E s`, which is `E w^2 / (4 sqrt(x^2 + w^2))` less at `x = V - V_u`. The collapsed part reads its
/// potential at a smoothed volume, which moves it by at most `share (V_u / V)^(n + 1)` times as much: the margin here
/// is twice that.
fn margin(elastance: f64, unstressed: f64, volume: f64) -> f64 {
    let (apart, ratio) = (volume - unstressed, (unstressed / volume).max(1.0));
    let collapsed = 2.0 * SHARE * ratio * ratio * ratio.sqrt();
    elastance * WIDTH * WIDTH * (1.0 + collapsed) / (4.0 * (apart * apart + WIDTH * WIDTH).sqrt())
}

struct Filled {
    grown: f64,
    work: f64,
    band: f64,
}

/// A vein's part of the store at `volume` less its part at `V_u`, read from the elastic energy row, which is its value
/// less the sum of the blood sites' parts. Beside it, the work the vein's own pressure does from `V_u`, by Simpson's
/// rule over the staged pressure row, and the margin summed the same way.
fn filled(
    body: &CompiledBody,
    key: &'static str,
    elastance: f64,
    unstressed: f64,
    volume: f64,
) -> Result<Filled, InitializationFailure> {
    let energy = index(body, algebraic("CV", "elastic energy", Context::Body));
    let row = index(body, pressure(vessel(key)));
    let step = (volume - unstressed) / INTERVALS as f64;
    let volumes: Vec<f64> = (0..=INTERVALS).map(|at| unstressed + step * at as f64).collect();
    let staged = staged(body, key, &volumes)?;
    let at_rest = elastance * (1.0 - SHARE) * WIDTH / 2.0;
    let (mut work, mut band) = (0.0, 0.0);
    for (at, (volume, residuals)) in volumes.iter().zip(&staged).enumerate() {
        let weight = match at {
            0 | INTERVALS => 1.0,
            odd if odd % 2 == 1 => 4.0,
            _ => 2.0,
        };
        work += weight * (staged[0][row] - residuals[row] + at_rest);
        band += weight * margin(elastance, unstressed, *volume);
    }
    Ok(Filled {
        grown: staged[0][energy] - staged[INTERVALS][energy],
        work: work * step / 3.0,
        band: (band * step / 3.0).abs(),
    })
}

/// Below `V_u` a marked vein's wall holds the work its pressure did to empty it, the integral of `E s` from `V_u`.
#[test]
fn below_its_unstressed_volume_a_marked_veins_store_is_the_work_of_its_pressure()
-> Result<(), InitializationFailure> {
    let body = arena_body()?;
    for (key, elastance, unstressed) in VEINS {
        for volume in [unstressed / 2.0, unstressed / 4.0] {
            let Filled { grown, work, band } = filled(&body, key, elastance, unstressed, volume)?;
            assert!(
                (grown - work).abs() <= band,
                "{key} at {volume} mL: the store grew {grown} against the work {work}, margin {band}"
            );
        }
    }
    Ok(())
}

/// Across `V_u` the store grows by the work of the pressure on both sides, and its slope, read between two staged
/// volumes, is that pressure, so the store below joins the store above with its slope.
#[test]
fn at_its_unstressed_volume_a_marked_veins_store_and_its_slope_join_the_store_above()
-> Result<(), InitializationFailure> {
    let body = arena_body()?;
    let energy = index(&body, algebraic("CV", "elastic energy", Context::Body));
    let apart = 0.05;
    for (key, elastance, unstressed) in VEINS {
        let row = index(&body, pressure(vessel(key)));
        for volume in [unstressed - 10.0, unstressed + 10.0] {
            let Filled { grown, work, band } = filled(&body, key, elastance, unstressed, volume)?;
            assert!(
                (grown - work).abs() <= band,
                "{key} at {volume} mL: the store grew {grown} against the work {work}, margin {band}"
            );
            let near = staged(&body, key, &[unstressed, volume - apart, volume, volume + apart])?;
            let slope = (near[1][energy] - near[3][energy]) / (2.0 * apart);
            let wall = near[0][row] - near[2][row] + elastance * (1.0 - SHARE) * WIDTH / 2.0;
            let off = margin(elastance, unstressed, volume);
            assert!(
                (slope - wall).abs() <= off,
                "{key} at {volume} mL: the store's slope {slope} against the pressure {wall}, margin {off}"
            );
        }
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
