use ng_contract::*;
use ng_record::{AuthorizedInput, Checkpoint, CheckpointError, HistoryStore, Running, Session, Step};
use ng_validate::*;

const SECOND: u64 = 1_000_000_000;
const MILLISECOND: u64 = 1_000_000;

#[derive(Default)]
struct Links {
    links: Vec<(Digest, Vec<u8>)>,
}
impl HistoryStore for Links {
    fn resolve(&self, seal: &Digest) -> Option<Vec<u8>> {
        let at = self.links.binary_search_by_key(seal, |link| link.0).ok()?;
        Some(self.links[at].1.clone())
    }
    fn keep(&mut self, seal: Digest, link: Vec<u8>) -> Result<(), CheckpointError> {
        if let Err(at) = self.links.binary_search_by_key(&seal, |link| link.0) {
            self.links.insert(at, (seal, link));
        }
        Ok(())
    }
}

fn reference(step: u64) -> ReferenceIntegrator {
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    ReferenceIntegrator::new(step)
        .with_scales(ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive"))
}

/// One admission costs about as much as everything else here, so one solve serves every check, each from a checkpoint
/// of the admitted start.
struct Solved {
    body: CompiledBody,
    core: ReferenceIntegrator,
    provenance: InitializationProvenance,
    saved: Checkpoint,
    identity: RunIdentity,
    links: Links,
}
impl Solved {
    fn new(step: u64) -> Self {
        let body = arena_body().expect("compiled");
        let core = reference(step);
        let (saved, identity, provenance, links) = {
            let (packet, lens) = (
                arena_packet(&body).expect("a packet"),
                arena_lens(&body).expect("a lens"),
            );
            let port = BoundaryPort::new(Vec::new()).expect("no event");
            let admitted = admit_at(&body, packet, port, &core, &lens, anchor_moment()).expect("admitted");
            let mut links = Links::default();
            let saved = admitted.session.checkpoint(&mut links).expect("a checkpoint");
            let identity = admitted.session.record().identity().clone();
            (saved, identity, admitted.provenance, links)
        };
        Self {
            body,
            core,
            provenance,
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
            ..
        } = self;
        Session::resume(saved, identity, body, core, links).expect("resumed")
    }
    fn row(&self, kind: RowKind) -> RowId {
        let rows = self.body.rows();
        let found = rows.iter().find(|row| row.kind() == kind);
        found.unwrap_or_else(|| panic!("no row {kind:?}")).id()
    }
    fn value(&self, session: &Session<'_, Running>, kind: RowKind) -> f64 {
        number(session.record().accepted().value(self.row(kind)))
    }
    fn mean_over<'s>(
        &self,
        session: Session<'s, Running>,
        until: u64,
        kind: RowKind,
    ) -> (Session<'s, Running>, f64) {
        let (mut sum, mut count) = (0.0, 0.0);
        let session = steps(session, until, |session| {
            sum += self.value(session, kind);
            count += 1.0;
        });
        (session, sum / count)
    }
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = || Measure::new(0.0).expect("finite");
    let at = Position::new(origin(), origin(), origin());
    AnatomyBuilder::new().site::<C>(key, at).id()
}

fn vessel(key: &'static str) -> SiteId {
    site::<VesselSegment>(key)
}
fn bed(key: &'static str) -> SiteId {
    site::<CapillaryBed>(key)
}
fn junction(key: &'static str) -> SiteId {
    site::<Junction>(key)
}
fn algebraic(owner: &'static str, local: &'static str, at: Context) -> RowKind {
    let law = LawName { owner, local };
    RowKind::Algebraic(Slot { law, at })
}
fn memory(owner: &'static str, local: &'static str, at: Context) -> RowKind {
    let law = LawName { owner, local };
    RowKind::Memory(Slot { law, at })
}
fn pressure(site: SiteId) -> RowKind {
    algebraic("CV", "blood pressure", Context::Site(site))
}
fn amount(site: SiteId, space: Space, species: Species) -> RowKind {
    RowKind::Amount { site, space, species }
}
fn number(held: Compensated) -> f64 {
    f64::from_bits(held.bits()[0])
}
fn pressure_channel() -> RowKind {
    pressure(vessel("systemic arteries"))
}
fn rate_row() -> RowKind {
    algebraic("CV", "rate", Context::Body)
}
fn sympathetic() -> RowKind {
    algebraic("NS", "sympathetic traffic", Context::Body)
}
fn dissipated() -> RowKind {
    algebraic("CV", "dissipated power", Context::Body)
}
fn valve_flow(from: SiteId, to: SiteId) -> RowKind {
    memory("CV", "valves", Context::Edge { from, to })
}

fn steps<'s>(
    mut session: Session<'s, Running>,
    until: u64,
    mut each: impl FnMut(&Session<'_, Running>),
) -> Session<'s, Running> {
    let end = anchor_moment().nanoseconds() + until;
    while session.record().accepted().time().nanoseconds() < end {
        let next = SimTime::from_ns(session.record().accepted().time().nanoseconds() + 1);
        session = match run_until(session, next) {
            Ok(reached) => reached,
            Err(stopped) => panic!("{:?}", stopped.fault()),
        };
        each(&session);
    }
    session
}

fn input(after: u64, tag: &[u8], payload: BoundaryInput) -> AuthorizedInput {
    let at = SimTime::from_ns(anchor_moment().nanoseconds() + after);
    let event = BoundaryEvent::new(InputId::new(Digest::of_canonical(tag)), at, payload);
    AuthorizedInput::from_command_receipt(Digest::of_canonical(b"a test's own command"), event)
}

fn stand(session: &mut Session<'_, Running>, after: u64) {
    let input = posture_at(after, b"stand", 1.0, Legs::Standing);
    session.enqueue(input).expect("enqueued");
}

fn the_slice_compiles_binds_and_lays_out_its_ring(solved: &Solved) -> Result<(), InitializationFailure> {
    let body = &solved.body;
    let rows = body.rows();
    let count = |kind: fn(&RowKind) -> bool| rows.iter().filter(|row| kind(&row.kind())).count();
    assert_eq!(count(|kind| matches!(kind, RowKind::Amount { .. })), 155);
    assert_eq!(count(|kind| matches!(kind, RowKind::Memory(_))), 9);
    assert_eq!(count(|kind| matches!(kind, RowKind::Algebraic(_))), 76);
    assert_eq!(body.columns().len(), 256);
    assert_eq!(body.longest_step_ns(), 10 * MILLISECOND);
    assert_eq!(body.held().count(), OUTSIDE_SLICE);
    let (packet, lens) = (arena_packet(body)?, arena_lens(body)?);
    let parameters = body.bind(packet)?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(anchor_moment()).reading(&lens);
    let system = body.initialization(parameters, port, request)?;
    assert_eq!((system.points(), system.rows()), (100, rows.len()));
    Ok(())
}

fn fitted(solved: &Solved, key: &'static str, binding: Binding) -> ValueRecord {
    let name = ValueName::Parameter {
        key: ParameterKey::new(key),
        binding,
    };
    let values = solved.provenance.values();
    *values
        .iter()
        .find(|record| record.name == name)
        .expect("a fitted value")
}

fn initialization_meets_every_fixture_anchor_with_recorded_provenance(solved: &Solved) {
    let provenance = &solved.provenance;
    let (arteries, kidney) = (vessel("systemic arteries"), bed("glomerular bed"));
    let tubule = vessel("renal tubule");
    let reclaim = bed("peritubular bed");
    for (record, anchor) in [
        (fitted(solved, "cv.intrinsic-rate", Binding::Body), "heart rate"),
        (
            fitted(
                solved,
                "cv.resistance",
                Binding::Edge {
                    from: arteries,
                    to: kidney,
                },
            ),
            "mean arterial pressure",
        ),
        (
            fitted(
                solved,
                "rn.sodium-capacity",
                Binding::Edge {
                    from: tubule,
                    to: reclaim,
                },
            ),
            "filtration",
        ),
    ] {
        assert_eq!(
            record.provenance,
            ValueProvenance::Calibrated {
                anchor: AnchorId::new(anchor)
            }
        );
    }
    let (_, worst) = provenance.worst().expect("a body with rows");
    assert!(number(worst) <= 1.0);
    let start = solved.session();
    assert_eq!(start.record().accepted().time(), anchor_moment());
    let started = |kind| {
        let name = ValueName::Start(kind);
        let record = provenance.values().iter().find(|record| record.name == name);
        record.expect("a start value").provenance
    };
    assert_eq!(started(pressure_channel()), ValueProvenance::Solved);
    let held = amount(bed("leg bed"), Space::Cells, Species::Water);
    assert_eq!(started(held), ValueProvenance::Fixture);
}

/// A run on another grid settles on its own orbit, so these read near the anchors and not on them. `admit` holds each
/// to its tolerance on the ring's own points.
fn the_first_beat_of_a_run_reads_near_every_anchor(solved: &Solved) {
    let (run, rate) = solved.mean_over(solved.session(), SECOND, rate_row());
    assert!((rate - 1.0).abs() < 0.05, "{rate}");
    let (_, pressure) = solved.mean_over(run, 2 * SECOND, pressure_channel());
    assert!((pressure - 100.0).abs() < 10.0, "{pressure}");
}

fn water_ions_moieties_and_energy_close_over_the_first_accepted_steps(solved: &Solved) {
    let run = solved.session();
    let store = amount(site::<TissueSite>("body"), Space::Elastic, Species::Energy);
    let sum = algebraic("CV", "elastic energy", Context::Body);
    let (mut firings, mut mode_changes) = (0, 0);
    let mut last = run.record().accepted().modes().to_vec();
    let run = steps(run, 2 * SECOND, |session| {
        let closure = (solved.value(session, store) - solved.value(session, sum)).abs();
        assert!(closure <= 1e-6 * solved.value(session, store), "{closure}");
        assert!(solved.value(session, dissipated()) >= 0.0);
        let modes = session.record().accepted().modes();
        if modes != last {
            mode_changes += 1;
            firings += modes.iter().zip(&last).filter(|(now, then)| now != then).count();
            last = modes.to_vec();
        }
    });
    assert!(firings >= 8, "{firings} firings in {mode_changes} changes");
    let view = run.record().conservation();
    let segments: Vec<_> = view.segments().collect();
    let (first, final_one) = (segments[0], segments[segments.len() - 1]);
    let book = |quantity: QuantityId| {
        let (start, end) = (first.interval(), final_one.interval());
        let read = |curve: ng_record::Curve<'_>, piece, at: usize| curve.on(piece).expect("own segment")[at];
        let moved =
            read(final_one.inventory(quantity), end, 1).minus(read(first.inventory(quantity), start, 0));
        let inflow = read(final_one.inflow(quantity), end, 1).minus(read(first.inflow(quantity), start, 0));
        let outflow =
            read(final_one.outflow(quantity), end, 1).minus(read(first.outflow(quantity), start, 0));
        (moved, inflow, outflow)
    };
    let mut seen = Vec::new();
    for quantity in view.basis().ids() {
        let (moved, inflow, outflow) = book(quantity);
        let gross = inflow.plus(outflow);
        let held = moved.minus(inflow.minus(outflow)).abs();
        let allowed = 1e-12 + number(gross) * 1e-12;
        assert!(number(held) <= allowed, "{quantity:?} {held:?} {allowed}");
        seen.push(quantity.book());
    }
    for species in [
        Species::Water,
        Species::Sodium,
        Species::Potassium,
        Species::Chloride,
        Species::Albumin,
        Species::Vasopressin,
        Species::Norepinephrine,
        Species::Acetylcholine,
        Species::Energy,
    ] {
        assert!(seen.contains(&Book::Of(species)), "{species:?}");
    }
    assert!(seen.contains(&Book::Charge));
    let held = [
        amount(bed("leg bed"), Space::Cells, Species::Water),
        amount(
            site::<EnclosedSpace>("pleural space"),
            Space::Fluid,
            Species::Water,
        ),
    ];
    let start = solved.session();
    for kind in held {
        assert_eq!(
            solved.value(&run, kind).to_bits(),
            solved.value(&start, kind).to_bits()
        );
    }
}

/// A wrong map reads zero or names the wrong row, and a reversed law closes every conserved total and still admits, so
/// each link of each loop must move its reader the way the loop needs on the admitted start. A stage's row is its value
/// less its law, so a law that rises with what it reads shows a falling slope.
fn every_link_of_every_loop_moves_the_row_that_reads_it_the_right_way(
    solved: &Solved,
) -> Result<(), InitializationFailure> {
    let Solved { body, .. } = solved;
    let (packet, lens) = (arena_packet(body)?, arena_lens(body)?);
    let parameters = body.bind(packet)?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(anchor_moment()).reading(&lens);
    let system = body.initialization(parameters, port, request)?;
    let mut workspace = system.workspace();
    let start = solved.session();
    for (slot, row) in body.rows().iter().enumerate() {
        workspace.unknowns_mut()[slot] = number(start.record().accepted().value(row.id()));
    }
    let (_, block) = system.stage(&workspace, 0)?;
    let view = block.view();
    let slope = |reader: RowKind, read: RowKind| {
        let (reader, read) = (solved.row(reader).index(), solved.row(read).index());
        let entries = view.row_offsets()[reader]..view.row_offsets()[reader + 1];
        let found = entries
            .into_iter()
            .find(|&entry| view.columns()[entry].index() == read);
        found.map_or(0.0, |entry| view.values()[entry])
    };
    let body_at = Context::Body;
    let (arteries, veins) = (vessel("systemic arteries"), vessel("central veins"));
    let (left, atrium) = (vessel("left ventricle"), vessel("left atrium"));
    let right = vessel("right ventricle");
    let (sinoatrial, myocardial) = (junction("sinoatrial junction"), junction("myocardial junction"));
    let (leg, lung, gut, kidney) = (
        bed("leg bed"),
        bed("pulmonary bed"),
        bed("splanchnic bed"),
        bed("glomerular bed"),
    );
    let (tubule, duct, reclaim) = (
        vessel("renal tubule"),
        vessel("collecting duct"),
        bed("peritubular bed"),
    );
    let (arteriolar, venous) = (junction("arteriolar junction"), junction("venous junction"));
    let stretch = algebraic("CV", "carotid_sinus_stretch", body_at);
    let arch = algebraic("CV", "aortic_arch_stretch", body_at);
    let filter = memory("NS", "afferent filter", body_at);
    let set_point = memory("NS", "baroreflex set point", body_at);
    let vagal = algebraic("NS", "vagal traffic", body_at);
    let tone = algebraic("RN", "afferent_tone", body_at);
    let effect = |junction| algebraic("Receptor", "effect norepinephrine", Context::Site(junction));
    let vagal_effect = algebraic("Receptor", "effect acetylcholine", Context::Site(sinoatrial));
    let v2 = algebraic("Receptor", "effect vasopressin", Context::Site(reclaim));
    let flow = |from, to| algebraic("CV", "circulation", Context::Edge { from, to });
    let valve = |from, to| memory("CV", "valves", Context::Edge { from, to });
    let starling = |bed| algebraic("FL", "starling", Context::Site(bed));
    let tissue = |bed| algebraic("FL", "interstitial pressure", Context::Site(bed));
    let capillary = |bed| algebraic("CV", "capillary_pressure", Context::Site(bed));
    let lymph = |bed| {
        let edge = Context::Edge { from: bed, to: veins };
        algebraic("FL", "lymph", edge)
    };
    let filtered = algebraic(
        "RN",
        "filtration",
        Context::Edge {
            from: kidney,
            to: tubule,
        },
    );
    let lumen = |space| algebraic("RN", "lumen pressure", Context::Site(space));
    let plasma = |site, species| amount(site, Space::Plasma, species);
    let blood = |site, species| amount(site, Space::Lumen, species);
    let urine = |site, species| amount(site, Space::Lumen, species);
    let pleura = algebraic(
        "RS",
        "enclosure_pressure",
        Context::Site(site::<EnclosedSpace>("pleural space")),
    );
    let abdomen = algebraic(
        "GI",
        "enclosure_pressure",
        Context::Site(site::<EnclosedSpace>("abdominal space")),
    );
    let cleft = |site| amount(site, Space::Cleft, Species::Norepinephrine);
    let stored = |site| amount(site, Space::Vesicle, Species::Norepinephrine);
    let bound = |site| amount(site, Space::Bound, Species::Norepinephrine);
    let bound_v2 = amount(reclaim, Space::Bound, Species::Vasopressin);
    let tissue_albumin = |bed| amount(bed, Space::Interstitium, Species::Albumin);
    let (rises, falls) = (1.0, -1.0);
    for (reader, read, way) in [
        (stretch, pressure(arteries), rises),
        (arch, pressure(arteries), rises),
        (filter, stretch, rises),
        (filter, arch, rises),
        (set_point, filter, rises),
        (sympathetic(), filter, falls),
        (sympathetic(), set_point, rises),
        (vagal, filter, rises),
        (stored(arteriolar), sympathetic(), falls),
        (cleft(arteriolar), stored(arteriolar), rises),
        (stored(arteriolar), cleft(arteriolar), rises),
        (bound(arteriolar), cleft(arteriolar), rises),
        (effect(arteriolar), bound(arteriolar), rises),
        (flow(arteries, leg), effect(arteriolar), falls),
        (flow(arteries, gut), effect(arteriolar), falls),
        (pressure(vessel("leg veins")), effect(venous), rises),
        (pressure(vessel("splanchnic veins")), effect(venous), rises),
        (pressure(left), effect(myocardial), rises),
        (pressure(right), effect(myocardial), rises),
        (rate_row(), effect(sinoatrial), rises),
        (rate_row(), vagal_effect, falls),
        (flow(arteries, kidney), effect(junction("renal junction")), falls),
        (tone, amount(tubule, Space::Lumen, Species::Sodium), falls),
        (tone, amount(tubule, Space::Lumen, Species::Water), rises),
        (flow(arteries, kidney), tone, rises),
        (filtered, pressure(kidney), rises),
        (filtered, lumen(tubule), falls),
        (filtered, plasma(kidney, Species::Albumin), falls),
        (starling(leg), capillary(leg), rises),
        (starling(leg), tissue(leg), falls),
        (starling(leg), plasma(leg, Species::Albumin), falls),
        (tissue_albumin(reclaim), plasma(reclaim, Species::Albumin), rises),
        (lymph(leg), tissue(leg), rises),
        (lymph(lung), tissue(lung), rises),
        (lymph(gut), tissue(gut), rises),
        (lymph(kidney), tissue(kidney), rises),
        (lymph(reclaim), tissue(reclaim), rises),
        (tissue(lung), pleura, rises),
        (tissue(gut), abdomen, rises),
        (pressure(lung), pleura, rises),
        (pressure(gut), abdomen, rises),
        (
            blood(veins, Species::Vasopressin),
            blood(veins, Species::Sodium),
            rises,
        ),
        (
            blood(veins, Species::Vasopressin),
            blood(veins, Species::Water),
            falls,
        ),
        (bound_v2, plasma(reclaim, Species::Vasopressin), rises),
        (bound_v2, plasma(reclaim, Species::Water), falls),
        (v2, bound_v2, rises),
        (urine(duct, Species::Water), v2, falls),
        (valve(atrium, left), pressure(left), falls),
        (valve(atrium, left), pressure(atrium), rises),
    ] {
        let moved = -slope(reader, read) * way;
        assert!(moved > 0.0, "{reader:?} reads {read:?}: {moved}");
    }
    for (reader, read) in [
        (tissue(leg), pleura),
        (tissue(leg), abdomen),
        (tissue(kidney), pleura),
    ] {
        assert_eq!(slope(reader, read), 0.0, "{reader:?} reads {read:?}");
    }
    Ok(())
}

/// A shut valve with no head across it and no flow stays shut, or a still body would fire at every step. The shut law holds
/// the flow where it stands, and an opened law would move it.
fn a_shut_valve_with_no_head_stays_shut(solved: &Solved) -> Result<(), InitializationFailure> {
    let Solved { body, .. } = solved;
    let (packet, lens) = (arena_packet(body)?, arena_lens(body)?);
    let parameters = body.bind(packet)?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(anchor_moment()).reading(&lens);
    let mut system = body.initialization(parameters, port, request)?;
    let mut workspace = system.workspace();
    let (arteries, left) = (vessel("systemic arteries"), vessel("left ventricle"));
    let size = system.rows();
    let flow = solved
        .row(memory(
            "CV",
            "valves",
            Context::Edge {
                from: left,
                to: arteries,
            },
        ))
        .index();
    for point in 0..2 {
        workspace.unknowns_mut()[point * size + solved.row(pressure(left)).index()] = 50.0;
        workspace.unknowns_mut()[point * size + solved.row(pressure(arteries)).index()] = 50.0;
        workspace.unknowns_mut()[point * size + flow] = -5.0;
    }
    system.sweep(&workspace, 0..1)?;
    let (residuals, _) = system.stage(&workspace, 1)?;
    assert_eq!(residuals[flow], 0.0);
    Ok(())
}

fn posture_at(after: u64, tag: &[u8], trunk: f64, legs: Legs) -> AuthorizedInput {
    let posture = Posture::new(Measure::new(trunk).expect("finite"), legs).expect("in range");
    input(after, tag, BoundaryInput::Posture(posture))
}

/// Standing drops the blood into the legs, the sinus feels less stretch, and the reflex raises traffic and rate within
/// seconds, with no law that reads posture but the heads. Each sensor feels the column up to its own height.
fn the_body_answers_a_stand_through_its_own_baroreflex(solved: &Solved) {
    let (flat, flat_traffic) = solved.mean_over(solved.session(), 6 * SECOND, sympathetic());
    let flat_rate = solved.value(&flat, rate_row());
    let mut standing = solved.session();
    stand(&mut standing, 1200 * MILLISECOND);
    let (standing, traffic) = solved.mean_over(standing, 6 * SECOND, sympathetic());
    assert!(traffic > 1.2 * flat_traffic, "{traffic} against {flat_traffic}");
    assert!(
        solved.value(&standing, rate_row()) > flat_rate,
        "the rate follows the traffic"
    );
    let leg = vessel("leg veins");
    let water = amount(leg, Space::Lumen, Species::Water);
    assert!(solved.value(&standing, water) > solved.value(&flat, water) + 10.0);
    let (sinus, arch) = (
        algebraic("CV", "carotid_sinus_stretch", Context::Body),
        algebraic("CV", "aortic_arch_stretch", Context::Body),
    );
    assert_eq!(solved.value(&flat, sinus), solved.value(&flat, arch));
    let gap = solved.value(&standing, arch) - solved.value(&standing, sinus);
    assert!(
        (gap - 0.105).abs() < 1e-9,
        "{gap}: 70 per metre over the 0.15 m between them, on a reference of 100"
    );
}

/// What a route delivers reaches the blood: each payload a meal carries, from outside the contract.
fn a_meal_delivers_sodium_potassium_and_chloride(solved: &Solved) {
    fn delivered<Q>(boundary: &'static str, amount: f64) -> Delivered<Q> {
        let ends = SimTime::from_ns(anchor_moment().nanoseconds() + 2500 * MILLISECOND);
        let amount = Measure::new(amount).expect("finite");
        Delivered {
            boundary,
            amount,
            ends,
        }
    }
    let total = |session: &Session<'_, Running>, species: Species| {
        let held =
            |row: &&Row| matches!(row.kind(), RowKind::Amount { species: found, .. } if found == species);
        let rows = solved.body.rows().iter().filter(held);
        rows.map(|row| number(session.record().accepted().value(row.id())))
            .sum::<f64>()
    };
    let (quiet, _) = solved.mean_over(solved.session(), 3 * SECOND, rate_row());
    let mut fed = solved.session();
    for (tag, payload) in [
        (
            &b"sodium"[..],
            BoundaryInput::Sodium(delivered("gut sodium", 50.0)),
        ),
        (
            b"potassium",
            BoundaryInput::Potassium(delivered("gut potassium", 2.0)),
        ),
        (
            b"chloride",
            BoundaryInput::Chloride(delivered("gut chloride", 52.0)),
        ),
    ] {
        fed.enqueue(input(1200 * MILLISECOND, tag, payload))
            .expect("enqueued");
    }
    let (fed, _) = solved.mean_over(fed, 3 * SECOND, rate_row());
    for (species, amount) in [
        (Species::Sodium, 50.0),
        (Species::Potassium, 2.0),
        (Species::Chloride, 52.0),
    ] {
        let more = total(&fed, species) - total(&quiet, species);
        assert!(
            (more / amount - 1.0).abs() < 0.01,
            "{species:?}: {more} of {amount}"
        );
    }
}

/// A short window is a bolus. The wall's gain has no receipt, so a push this hard takes the heat past zero, and the step
/// reports that and the body runs on.
fn a_bolus_keeps_the_body_running_and_the_step_reports_the_heat(solved: &Solved) {
    let ends = SimTime::from_ns(anchor_moment().nanoseconds() + 2200 * MILLISECOND);
    let water = WaterDelivery {
        boundary: "gut water",
        amount: Measure::new(400.0).expect("finite"),
        ends,
    };
    let mut run = solved.session();
    run.enqueue(input(1200 * MILLISECOND, b"bolus", BoundaryInput::Water(water)))
        .expect("enqueued");
    let end = anchor_moment().nanoseconds() + 3 * SECOND;
    let mut reported = 0;
    while run.record().accepted().time().nanoseconds() < end {
        let mut attempt = run.step();
        run = loop {
            match attempt {
                Step::Accepted(next, receipt) => {
                    reported += receipt.excursions().len();
                    break next;
                }
                Step::Refused(retrying) => attempt = retrying.retry(),
                Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
            }
        };
    }
    assert!(reported > 0);
}

/// Once the wall has let go, the heat the store sheds is what the flows lose on the way, up to the steps' own error, so
/// the rate that closes the store is a real account of dissipation and no free number. While it lets go the store sheds
/// what the stiffer wall held as well.
fn the_heat_shed_in_late_diastole_is_what_the_flows_lose(solved: &Solved) {
    let edges: Vec<(RowKind, SiteId, SiteId)> = (solved.body.rows().iter())
        .filter_map(|row| match row.kind() {
            RowKind::Algebraic(Slot {
                law,
                at: Context::Edge { from, to },
            }) if law.local == "circulation" => Some((row.kind(), from, to)),
            RowKind::Memory(Slot {
                law,
                at: Context::Edge { from, to },
            }) if law.local == "valves" => Some((row.kind(), from, to)),
            _ => None,
        })
        .collect();
    assert_eq!(edges.len(), 17);
    let activation = memory("CV", "activation", Context::Body);
    let (mut heat, mut losses, mut read) = (0.0, 0.0, 0);
    steps(solved.session(), 3 * SECOND, |session| {
        if solved.value(session, activation) < 1e-3 {
            read += 1;
            heat += solved.value(session, dissipated());
            for &(flow, from, to) in &edges {
                let head = solved.value(session, pressure(from)) - solved.value(session, pressure(to));
                losses += solved.value(session, flow) * head;
            }
        }
    });
    assert!(read > 500, "{read}");
    assert!((heat / losses - 1.0).abs() < 0.1, "{heat} against {losses}");
}

fn a_checkpoint_of_a_run_mid_beat_holds_its_modes_and_runs_on_the_same_bits(solved: &Solved) {
    let Solved {
        body, core, identity, ..
    } = solved;
    let mut run = steps(solved.session(), 1250 * MILLISECOND, |_| {});
    let mut links = Links::default();
    let saved = run.checkpoint(&mut links).expect("a checkpoint");
    stand(&mut run, 1300 * MILLISECOND);
    let reached = steps(run, 1400 * MILLISECOND, |_| {});
    let mut resumed = Session::resume(&saved, identity, body, core, &links).expect("resumed");
    stand(&mut resumed, 1300 * MILLISECOND);
    let again = steps(resumed, 1400 * MILLISECOND, |_| {});
    assert_eq!(
        reached.record().accepted().modes(),
        again.record().accepted().modes()
    );
    for row in body.rows() {
        let (one, other) = (
            reached.record().accepted().value(row.id()),
            again.record().accepted().value(row.id()),
        );
        assert_eq!(one.bits(), other.bits(), "{:?}", row.kind());
    }
}

/// The means from the twentieth second to the thirtieth, and how far beat means swing there.
fn settled(solved: &Solved, posture: Vec<AuthorizedInput>) -> ([f64; 5], f64) {
    let output = valve_flow(vessel("left ventricle"), vessel("systemic arteries"));
    let filtration = algebraic(
        "RN",
        "filtration",
        Context::Edge {
            from: bed("glomerular bed"),
            to: vessel("renal tubule"),
        },
    );
    let mut run = solved.session();
    for input in posture {
        run.enqueue(input).expect("enqueued");
    }
    let run = steps(run, 20 * SECOND, |_| {});
    let mut then = run.record().accepted().time().nanoseconds();
    let (mut sums, mut beat, mut length, mut last) = ([0.0; 5], 0.0, 0.0, 0.0);
    let (mut least, mut most, mut total, mut whole) = (f64::INFINITY, 0.0_f64, 0.0, false);
    steps(run, 30 * SECOND, |session| {
        let now = session.record().accepted().time().nanoseconds();
        let span = (now - then) as f64;
        then = now;
        total += span;
        let reading = [pressure_channel(), rate_row(), output, filtration, sympathetic()]
            .map(|kind| solved.value(session, kind));
        for (sum, value) in sums.iter_mut().zip(reading) {
            *sum += span * value;
        }
        beat += span * reading[0];
        length += span;
        let phase = solved.value(session, memory("CV", "phase", Context::Body));
        if phase < last {
            let mean = beat / length;
            if whole {
                least = if mean < least { mean } else { least };
                most = if mean > most { mean } else { most };
            }
            (beat, length, whole) = (0.0, 0.0, true);
        }
        last = phase;
    });
    (sums.map(|sum| sum / total), most - least)
}

/// What the contract's 07:00 case checks, by direction, because magnitudes wait for a reviewed packet: at heart level
/// the pressure settles above supine, the rate rises, stroke volume and output fall, and filtration falls a little.
/// The reflex holds that with traffic to spare and does not hunt, so a body past that point has room left.
fn reads_the_way_the_contract_checks(name: &str, flat: ([f64; 5], f64), posture: ([f64; 5], f64)) {
    let ([flat_pressure, flat_rate, flat_output, flat_filtered, flat_traffic], flat_swing) = flat;
    let ([pressure, rate, output, filtered, traffic], swing) = posture;
    let (stroke, flat_stroke) = (output / rate, flat_output / flat_rate);
    let read = format!(
        "{name}: pressure {pressure} of {flat_pressure}, rate {rate} of {flat_rate}, output {output} of {flat_output}, \
         filtration {filtered} of {flat_filtered}, traffic {traffic} of {flat_traffic}, swings {swing} and {flat_swing}"
    );
    assert!(pressure > flat_pressure, "{read}");
    assert!(rate > flat_rate, "{read}");
    assert!(output < flat_output && stroke < flat_stroke, "{read}");
    assert!(
        filtered < flat_filtered && filtered > 0.95 * flat_filtered,
        "{read}"
    );
    assert!(traffic > flat_traffic && traffic < 0.8, "{read}");
    assert!(swing < 1.0 && flat_swing < 1.0, "{read}");
}

fn a_ramp_to_sitting() -> Vec<AuthorizedInput> {
    (0..=20u64)
        .map(|step| {
            let legs = if step < 10 {
                Legs::Horizontal
            } else {
                Legs::SeatedDependent
            };
            let at = 5 * SECOND + step * SECOND / 2;
            posture_at(at, &[b's', step as u8], step as f64 / 20.0, legs)
        })
        .collect()
}

#[test]
fn a_stand_and_a_sit_settle_the_way_the_contract_checks() {
    let solved = Solved::new(5 * MILLISECOND);
    let flat = settled(&solved, Vec::new());
    let stand = vec![posture_at(5 * SECOND, b"stand", 1.0, Legs::Standing)];
    reads_the_way_the_contract_checks("standing", flat, settled(&solved, stand));
    reads_the_way_the_contract_checks("seated", flat, settled(&solved, a_ramp_to_sitting()));
}

#[test]
fn the_slice_compiles_initializes_closes_and_regulates_itself() {
    let solved = Solved::new(MILLISECOND);
    the_slice_compiles_binds_and_lays_out_its_ring(&solved).expect("it holds");
    initialization_meets_every_fixture_anchor_with_recorded_provenance(&solved);
    the_first_beat_of_a_run_reads_near_every_anchor(&solved);
    water_ions_moieties_and_energy_close_over_the_first_accepted_steps(&solved);
    every_link_of_every_loop_moves_the_row_that_reads_it_the_right_way(&solved).expect("it holds");
    a_shut_valve_with_no_head_stays_shut(&solved).expect("it holds");
    the_body_answers_a_stand_through_its_own_baroreflex(&solved);
    a_meal_delivers_sodium_potassium_and_chloride(&solved);
    a_bolus_keeps_the_body_running_and_the_step_reports_the_heat(&solved);
    the_heat_shed_in_late_diastole_is_what_the_flows_lose(&solved);
    a_checkpoint_of_a_run_mid_beat_holds_its_modes_and_runs_on_the_same_bits(&solved);
}
