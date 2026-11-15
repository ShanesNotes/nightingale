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

fn junction(key: &'static str) -> SiteId {
    site::<Junction>(key)
}

#[test]
#[ignore]
fn probe57() {
    use std::io::Write;
    let out = std::env::var("NC57_OUT").expect("NC57_OUT");
    let trunk: f64 = std::env::var("NC57_TRUNK").expect("trunk").parse().expect("number");
    let until: u64 = std::env::var("NC57_UNTIL").expect("until").parse().expect("number");
    let mut file = std::io::BufWriter::new(std::fs::File::create(out).expect("file"));
    let store = RowKind::Amount { site: site::<TissueSite>("body"), space: Space::Elastic, species: Species::Energy };
    let thermal = RowKind::Amount { site: site::<TissueSite>("body"), space: Space::Thermal, species: Species::Energy };
    let sum = algebraic("CV", "elastic energy", Context::Body);
    let power = algebraic("CV", "wall power", Context::Body);
    let effect = algebraic("Receptor", "effect norepinephrine", Context::Site(junction("venous junction")));
    let veins = ["leg veins", "splanchnic veins", "central veins", "pulmonary vein"];
    writeln!(file, "t\tpower\tstore\tsum\tthermal\teffect\t{}", veins.iter().map(|v| format!("{v} water\t{v} albumin\t{v} pressure")).collect::<Vec<_>>().join("\t")).unwrap();
    tilted(trunk, until * SECOND, |body, session| {
        let t = (session.record().accepted().time().nanoseconds() - anchor_moment().nanoseconds()) as f64 / 1e9;
        let mut line = format!("{t:.9}");
        for kind in [power, store, sum, thermal, effect] {
            line += &format!("\t{:.17e}", value(body, session, kind));
        }
        for v in veins {
            for kind in [lumen(vessel(v), Species::Water), lumen(vessel(v), Species::Albumin), pressure(vessel(v))] {
                line += &format!("\t{:.17e}", value(body, session, kind));
            }
        }
        writeln!(file, "{line}").unwrap();
    });
}
