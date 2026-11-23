use crate::boundary::DriveFrame;
use crate::catalog::PerfusionTemperatures;
use crate::evaluate::Inputs;
use crate::*;

const RESISTANCE: f64 = 2.0;
const WIDTH: f64 = 4.0;
const REFERENCE: f64 = 1.0;
const UP: f64 = 9.0;
const DOWN: f64 = 3.0;
const SKIN_TEMPERATURE: f64 = 5.0;
const DILATOR: f64 = 0.5;
const CONSTRICTOR: f64 = 2.0;
const RATIO: &str = "skin log resistance ratio";
const SKIN_FACTOR_OPS: usize = 3;

#[derive(Clone, Copy, PartialEq)]
enum Ratio {
    Read,
    Unread,
    NoBed,
}

struct Sites {
    root: SiteId,
    skin: SiteId,
    bed: SiteId,
    liver: SiteId,
    artery: SiteId,
    vein: SiteId,
    junction: SiteId,
}

fn measure<U: Dimension>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn class() -> RateClass {
    RateClass::new(RateBand::Wave, 1_000_000_000).expect("a class")
}

fn cv(local: &'static str) -> LawName {
    LawName { owner: "CV", local }
}

fn map(ratio: Ratio) -> Result<(Anatomy, Sites), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at());
    let store = map.site::<TissueSite>("skin heat site", at());
    let artery = map.site::<VesselSegment>("artery", at());
    let vein = map.site::<VesselSegment>("vein", at());
    let junction = map.site::<Junction>("skin junction", at());
    let bed = map.site::<CapillaryBed>("skin bed", at());
    let liver = map.site::<CapillaryBed>("liver bed", at());
    for site in [store.id(), artery.id(), vein.id(), junction.id(), liver.id()] {
        map.contain(root.id(), site);
    }
    map.contain(store.id(), bed.id());
    match ratio {
        Ratio::NoBed => map.serves(&bed, Tissue::Muscle),
        _ => map.serves(&bed, Tissue::Skin),
    }
    map.serves(&liver, Tissue::Liver);
    for bed in [bed.id(), liver.id()] {
        map.connect(Network::Vascular, artery.id(), bed);
        map.connect(Network::Vascular, bed, vein.id());
    }
    map.connect(Network::Vascular, vein.id(), artery.id());
    map.supply(&junction, &[artery.id(), vein.id(), bed.id(), liver.id()]);
    let sites = Sites {
        root: root.id(),
        skin: store.id(),
        bed: bed.id(),
        liver: liver.id(),
        artery: artery.id(),
        vein: vein.id(),
        junction: junction.id(),
    };
    Ok((map.finish()?, sites))
}

fn publish<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>) {
    let class = class();
    let stores = ports.temperatures(mt);
    let still = mt.kernel(|mut k| {
        let value = k.zero::<Temperature>();
        k.finish(value, mt.every_mode(), class)
    });
    mt.define(&stores.core, still);
    let still = mt.site_kernel(|mut k| {
        let value = k.zero::<Temperature>();
        k.finish(value, mt.every_mode(), class)
    });
    mt.define_at(&stores.other, still);
    let route = ports.conduction(mt);
    let still = mt.site_kernel(|mut k| {
        let zero = k.zero::<Rate<Energy>>();
        k.finish(zero, mt.every_mode(), class)
    });
    mt.transfer_at(&route, still);
}

fn bind(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let class = class();
    let (mut rc, ports) = body.receptor()?.parts();
    let dilator = ports.binding::<Junction, Acetylcholine>(&mut rc);
    let still = rc.site_kernel(|mut k| {
        let zero = k.zero::<Rate<Acetylcholine>>();
        k.finish(zero, rc.every_mode(), class)
    });
    rc.transfer_at(&dilator, still);
    let constrictor = ports.binding::<Junction, Norepinephrine>(&mut rc);
    let still = rc.site_kernel(|mut k| {
        let zero = k.zero::<Rate<Norepinephrine>>();
        k.finish(zero, rc.every_mode(), class)
    });
    rc.transfer_at(&constrictor, still);
    let share =
        rc.site_parameter::<Junction, Dimensionless>(ParameterKey::new("rc.share"), Admissible::finite());
    for out in [
        ports.effect::<Junction, Acetylcholine>(&mut rc),
        ports.effect::<Junction, Norepinephrine>(&mut rc),
    ] {
        let value = rc.site_kernel(|mut k| {
            let value = k.site_parameter(&share);
            k.finish(value, rc.every_mode(), class)
        });
        rc.define_at(&out, value);
    }
    body.add(rc.finish());
    Ok(())
}

fn compiled(ratio: Ratio) -> Result<(CompiledBody, Sites), AdmissionError> {
    let (anatomy, sites) = map(ratio)?;
    let mut schema = Schema::new(anatomy)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    schema.leave_out(Space::Interstitium, "a bench of blood and heat");
    schema.leave_out(Space::Cells, "a bench of blood and heat");
    for store in [sites.root, sites.skin] {
        schema.pool::<Energy>(store, Space::Thermal)?;
    }
    schema.carry::<Acetylcholine, Junction>(Space::Cleft);
    schema.carry::<Acetylcholine, Junction>(Space::Bound);
    schema.carry::<Norepinephrine, Junction>(Space::Cleft);
    schema.carry::<Norepinephrine, Junction>(Space::Bound);
    let class = class();
    let key = ParameterKey::new;
    let body = assemble(schema, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        publish(&mut mt, &ports);
        body.add(mt.finish());
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let (out, pressure) = ports.blood_pressure(&mut cv);
        let held = cv.site_parameter::<Blood, Pressure>(key("cv.pressure"), Admissible::finite());
        let value = cv.site_kernel(|mut k| {
            let value = k.site_parameter(&held);
            k.finish(value, cv.every_mode(), class)
        });
        cv.define_at(&out, value);
        let ratio = (ratio != Ratio::Unread).then(|| {
            let temperatures: PerfusionTemperatures = cv.receive();
            let dilator = ports.effect::<Acetylcholine>();
            let reference =
                cv.parameter::<Reference<Temperature>>(key("ns.skin-reference"), Admissible::finite());
            let width = cv.parameter::<Deviation<Temperature>>(key("cv.skin-width"), Admissible::positive());
            let (out, read) = cv.site_field::<Beds<Skin>, Dimensionless>(RATIO, Admissible::finite());
            let value = cv.site_kernel(|mut k| {
                let felt = k.at_junction(&dilator);
                let (skin, reference) = (temperatures.tissue(&mut k), k.parameter(&reference));
                let (warmth, width) = (k.deviation(skin, reference), k.parameter(&width));
                let quotient = k.divide(warmth, width);
                let local = k.as_dimension(quotient);
                let value = k.add(felt, local);
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&out, value);
            read
        });
        let alpha = ports.effect::<Norepinephrine>();
        let resistance =
            cv.edge_parameter::<VascularEdge, Resistance>(key("cv.resistance"), Admissible::positive());
        let route = ports.circulation(&mut cv);
        let flow = cv.edge_kernel(|mut k| {
            let tone = k.at_junction_if_supplied(&alpha);
            let base = k.edge_parameter(&resistance);
            let held = k.scale(base, tone);
            let resistance = match &ratio {
                Some(ratio) => {
                    let log = k.downstream_if_in(ratio);
                    let factor = k.exp(log);
                    k.scale(held, factor)
                }
                None => held,
            };
            let (up, down) = (k.upstream(&pressure), k.downstream(&pressure));
            let head = k.subtract(up, down);
            let quotient = k.divide(head, resistance);
            let value = k.as_dimension(quotient);
            k.finish(value, cv.every_mode(), class)
        });
        cv.transfer_along(&route, flow);
        body.add(cv.finish());
        let (mut hp, ports) = body.hepatic(None)?.parts();
        let sinusoids = ports.sinusoids(&mut hp);
        let still = hp.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, hp.every_mode(), class)
        });
        hp.transfer_along(&sinusoids, still);
        body.add(hp.finish());
        bind(&mut body)?;
        body.compile()
    })?;
    Ok((body, sites))
}

struct Bench {
    body: CompiledBody,
    sites: Sites,
    parameters: Vec<f64>,
    modes: Vec<usize>,
    unknowns: Vec<f64>,
}

fn bench(ratio: Ratio) -> Bench {
    let (body, sites) = compiled(ratio).unwrap_or_else(|why| panic!("{why:?}"));
    let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
    let note = "synthetic";
    let key = ParameterKey::new;
    let vascular = Binding::Network(Network::Vascular);
    let resistance = measure::<Resistance>(RESISTANCE);
    (packet.synthetic(key("cv.resistance"), vascular, resistance, note)).expect("once");
    let share = Binding::Site(sites.junction);
    (packet.synthetic(key("rc.share"), share, measure::<Dimensionless>(1.0), note)).expect("once");
    for site in [sites.artery, sites.vein, sites.bed, sites.liver] {
        let pressure = measure::<Pressure>(1.0);
        (packet.synthetic(key("cv.pressure"), Binding::Site(site), pressure, note)).expect("once");
    }
    if ratio != Ratio::Unread {
        let reference = measure::<Reference<Temperature>>(REFERENCE);
        (packet.synthetic(key("ns.skin-reference"), Binding::Body, reference, note)).expect("once");
        let width = measure::<Deviation<Temperature>>(WIDTH);
        (packet.synthetic(key("cv.skin-width"), Binding::Body, width, note)).expect("once");
    }
    packet.start_rest(&body, |_| 1.0, note).expect("once");
    let bound = body.bind(packet).expect("bound");
    let unknowns = (body.rows().iter())
        .map(|row| bound.starts[row.id().index()])
        .collect();
    let mut bench = Bench {
        parameters: bound.values,
        modes: bound.modes,
        unknowns,
        body,
        sites,
    };
    let junction = Context::Site(bench.sites.junction);
    let acetylcholine = bench.algebraic(Receptor::CARD, Species::Acetylcholine.names()[4], junction);
    let norepinephrine = bench.algebraic(Receptor::CARD, Species::Norepinephrine.names()[4], junction);
    bench.set(acetylcholine, DILATOR);
    bench.set(norepinephrine, CONSTRICTOR);
    let skin_store = Context::Site(bench.sites.skin);
    let temperature = bench.algebraic("MT", crate::scope::TISSUE_TEMPERATURE, skin_store);
    bench.set(temperature, SKIN_TEMPERATURE);
    for (site, value) in [
        (bench.sites.artery, UP),
        (bench.sites.vein, DOWN),
        (bench.sites.bed, DOWN),
        (bench.sites.liver, DOWN),
    ] {
        let row = bench.algebraic("CV", crate::scope::BLOOD_PRESSURE, Context::Site(site));
        bench.set(row, value);
    }
    for edge in bench.plain() {
        let row = bench.algebraic("CV", "circulation", edge);
        bench.set(row, 0.0);
    }
    if ratio == Ratio::Read {
        let row = bench.ratio();
        bench.set(row, log_ratio());
    }
    bench
}

impl Bench {
    fn row(&self, kind: RowKind) -> usize {
        let found = self.body.rows().iter().position(|row| row.kind() == kind);
        found.unwrap_or_else(|| panic!("no row {kind:?}"))
    }
    fn algebraic(&self, owner: &'static str, local: &'static str, at: Context) -> usize {
        self.row(RowKind::Algebraic(Slot {
            law: LawName { owner, local },
            at,
        }))
    }
    fn set(&mut self, row: usize, value: f64) {
        self.unknowns[row] = value;
    }
    fn ratio(&self) -> usize {
        self.algebraic("CV", RATIO, Context::Site(self.sites.bed))
    }
    fn plain(&self) -> [Context; 3] {
        let edge = |from, to| Context::Edge { from, to };
        [
            edge(self.sites.artery, self.sites.bed),
            edge(self.sites.bed, self.sites.vein),
            edge(self.sites.vein, self.sites.artery),
        ]
    }
    fn inputs<'a>(&'a self, frame: &'a DriveFrame) -> Inputs<'a> {
        Inputs {
            parameters: &self.parameters,
            frame,
            unknowns: &self.unknowns,
            modes: &self.modes,
            draws: &[],
            factors: &[],
        }
    }
    fn residuals(&self, law: LawName) -> Vec<(Context, u64)> {
        let frame = DriveFrame::default();
        let inputs = self.inputs(&frame);
        let values = self.body.instance_values(&inputs).expect("finite");
        let programs = self.body.programs();
        let mut found: Vec<(Context, u64)> = (self.body.instances().iter().zip(values))
            .filter(|(instance, _)| programs[instance.program().index()].law == law)
            .filter(|(instance, _)| matches!(instance.target(), crate::body::Target::Row(_)))
            .map(|(instance, value)| (instance.context, value.expect("a value").to_bits()))
            .collect();
        found.sort_by_key(|(context, _)| *context);
        found
    }
    fn slopes(&self, column: usize) -> Vec<(Context, u64)> {
        let frame = DriveFrame::default();
        let inputs = self.inputs(&frame);
        let jacobian = self.body.jacobian(&inputs, 0.0).expect("finite");
        let (sparsity, rows) = (self.body.sparsity(), self.body.rows());
        let mut slopes = Vec::new();
        for (at, row) in rows.iter().enumerate() {
            let RowKind::Algebraic(slot) = row.kind() else {
                continue;
            };
            if slot.law != cv("circulation") {
                continue;
            }
            let span = sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1];
            let entries = (sparsity.unknowns()[span.clone()].iter()).zip(&jacobian[span]);
            for (unknown, slope) in entries {
                if unknown.index() == rows[column].unknown().index() {
                    slopes.push((slot.at, slope.to_bits()));
                }
            }
        }
        slopes.sort_by_key(|(context, _)| *context);
        slopes
    }
}

fn log_ratio() -> f64 {
    DILATOR + (SKIN_TEMPERATURE - REFERENCE) / WIDTH
}

#[test]
fn the_skin_field_reads_the_dilator_at_the_junction_and_the_skins_warmth_above_the_fixed_reference() {
    let mut bench = bench(Ratio::Read);
    let at_the_bed = Context::Site(bench.sites.bed);
    for row in [0.0, log_ratio()] {
        let at = bench.ratio();
        bench.set(at, row);
        let found = bench.residuals(cv(RATIO));
        let residual = row - log_ratio();
        assert_eq!(found, [(at_the_bed, residual.to_bits())], "one skin bed");
    }
}

#[test]
fn the_skin_edge_scales_its_resistance_by_the_exponential_of_the_field_and_every_other_edge_is_unmoved() {
    let mut read = bench(Ratio::Read);
    let unread = bench(Ratio::Unread);
    let held = RESISTANCE * CONSTRICTOR;
    let into_the_skin = Context::Edge {
        from: read.sites.artery,
        to: read.sites.bed,
    };
    let away = |flows: &[(Context, u64)]| {
        (flows.iter().copied())
            .filter(|&(at, _)| at != into_the_skin)
            .collect::<Vec<(Context, u64)>>()
    };
    let [feeder, draining, back] = read.plain();
    for log in [log_ratio(), 0.0] {
        let row = read.ratio();
        read.set(row, log);
        let found = read.residuals(cv("circulation"));
        let flows = [
            (feeder, (UP - DOWN) / (held * crate::pinned::exp(log))),
            (draining, (DOWN - DOWN) / held),
            (back, (DOWN - UP) / held),
        ];
        let mut twin: Vec<(Context, u64)> = (flows.iter())
            .map(|&(at, flow)| (at, (0.0 - flow).to_bits()))
            .collect();
        twin.sort_by_key(|(context, _)| *context);
        assert_eq!(found, twin, "three plain edges, the skin's scaled by exp({log})");
        assert_eq!(
            away(&found),
            away(&unread.residuals(cv("circulation"))),
            "every other edge answers pressure to the bit"
        );
        let pressures = |bench: &Bench| {
            let sites = [
                bench.sites.artery,
                bench.sites.vein,
                bench.sites.bed,
                bench.sites.liver,
            ];
            sites.map(|site| bench.algebraic("CV", crate::scope::BLOOD_PRESSURE, Context::Site(site)))
        };
        let mut compared = 0;
        for (row, twin) in pressures(&read).into_iter().zip(pressures(&unread)) {
            let found = away(&read.slopes(row));
            assert_eq!(
                found,
                away(&unread.slopes(twin)),
                "every other edge's slope in pressure, to the bit"
            );
            compared += found.len();
        }
        assert_eq!(
            compared, 4,
            "the two other edges, each with a slope at both its ends"
        );
    }
}

#[test]
fn only_the_skin_edges_flow_has_a_slope_in_the_skin_log_ratio() {
    let bench = bench(Ratio::Read);
    let [feeder, _, _] = bench.plain();
    let found = bench.slopes(bench.ratio());
    let [(edge, slope)] = found[..] else {
        panic!("one edge reads the skin, not {found:?}");
    };
    assert_eq!(edge, feeder, "the skin's feeding edge alone");
    // The residual is the row less the flow, and the flow is the pressure head over a resistance the exponential of
    // the field scales, so the residual's slope in the field is the flow itself, up to the last bit of the two ways
    // of reaching it.
    let flow = (UP - DOWN) / (RESISTANCE * CONSTRICTOR * crate::pinned::exp(log_ratio()));
    let slope = f64::from_bits(slope);
    assert!(
        (slope - flow).abs() <= f64::EPSILON * flow.abs(),
        "{slope} against the flow {flow}"
    );
    assert!(slope > 0.0, "the flow falls as the field rises");
}

#[test]
fn the_skin_read_covers_the_plain_edges_and_leaves_out_the_hepatic_cards_sinusoids() {
    use crate::law::{Input, Op, Where};
    // Every program of one law, since a transfer's row and its column each carry one.
    let counted = |ratio: Ratio, law: LawName| {
        let (body, _) = compiled(ratio).unwrap_or_else(|why| panic!("{why:?}"));
        let programs: Vec<&crate::body::Program> = (body.programs().iter())
            .filter(|program| program.law == law)
            .collect();
        assert!(!programs.is_empty(), "no {law:?}");
        let ops = || programs.iter().flat_map(|program| program.code.ops.iter());
        let read = (ops())
            .filter(|op| matches!(op, Op::Input(Input::Read(_, Where::DownIn(_)))))
            .count();
        let exponential = (ops()).filter(|op| matches!(op, Op::Exp(_))).count();
        (ops().count(), read, exponential)
    };
    let sinusoids = LawName {
        owner: "HP",
        local: "sinusoids",
    };
    let (plain, read, exponential) = counted(Ratio::Read, cv("circulation"));
    assert_eq!((read, exponential), (1, 1), "the plain edges read the skin once");
    let (before, read, exponential) = counted(Ratio::Unread, cv("circulation"));
    assert_eq!((read, exponential), (0, 0));
    let kinds = |ratio: Ratio| {
        let (body, _) = compiled(ratio).expect("a body");
        let ops: Vec<&Vec<Op>> = (body.programs().iter())
            .filter(|program| program.law == cv("circulation"))
            .map(|program| &program.code.ops)
            .collect();
        format!("{ops:?}")
    };
    assert_eq!(
        plain - before,
        SKIN_FACTOR_OPS,
        "{} against {}",
        kinds(Ratio::Read),
        kinds(Ratio::Unread)
    );
    assert_eq!(
        counted(Ratio::Read, sinusoids),
        counted(Ratio::Unread, sinusoids),
        "the hepatic card's law is the same with the skin's factor and without"
    );
    assert_eq!(counted(Ratio::Read, sinusoids).1, 0, "no sinusoid reads the skin");
    let (body, sites) = compiled(Ratio::Read).expect("a body");
    let programs = body.programs();
    let standing: Vec<Context> = (body.instances().iter())
        .filter(|instance| programs[instance.program().index()].law == sinusoids)
        .filter(|instance| matches!(instance.target(), crate::body::Target::Row(_)))
        .map(|instance| instance.context)
        .collect();
    let liver = [
        Context::Edge {
            from: sites.artery,
            to: sites.liver,
        },
        Context::Edge {
            from: sites.liver,
            to: sites.vein,
        },
    ];
    assert_eq!(standing.len(), liver.len());
    for edge in liver {
        assert!(standing.contains(&edge), "the liver's own edges");
    }
}

#[test]
fn an_edge_that_can_never_end_in_the_skins_beds_is_refused_by_name() {
    assert_eq!(
        compiled(Ratio::NoBed).map(|_| ()).err(),
        Some(AdmissionError::Compile(CompileDefect::EndOffSet {
            law: cv("circulation")
        }))
    );
}
