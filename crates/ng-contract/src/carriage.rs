use crate::boundary::DriveFrame;
use crate::catalog::PerfusionTemperatures;
use crate::evaluate::Inputs;
use crate::scope::{CORE_TEMPERATURE, TISSUE_TEMPERATURE};
use crate::*;

type BloodHeat = Quotient<Quotient<Rate<Energy>, Temperature>, Rate<Volume>>;

const PARTIAL_VOLUME: f64 = 2.0;
const BLOOD_HEAT: f64 = 3.0;
const OUTGOING: f64 = 71.0;
const CARRIED: LawName = LawName {
    owner: "CV",
    local: "heat carried by blood",
};

fn measure<U: Dimension>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn class() -> RateClass {
    RateClass::new(RateBand::Wave, 1_000_000_000).expect("a class")
}

#[derive(Clone, Copy, PartialEq)]
enum Feed {
    Circulation,
    Valve,
    TwoFeeders,
    Sinusoid,
    NoFlowRow,
    Dropped,
}

struct Sites {
    root: SiteId,
    skin: SiteId,
    bed: SiteId,
    artery: SiteId,
    vein: SiteId,
}

fn sites(feed: Feed) -> (AnatomyBuilder, Sites) {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at());
    let skin = map.site::<TissueSite>("skin heat site", at());
    let group = map.site::<TissueSite>("unheated group", at());
    let artery = map.site::<VesselSegment>("artery", at());
    let vein = map.site::<VesselSegment>("vein", at());
    let bed = map.site::<CapillaryBed>("skin bed", at());
    for site in [skin.id(), artery.id(), vein.id()] {
        map.contain(root.id(), site);
    }
    map.contain(skin.id(), group.id());
    map.contain(group.id(), bed.id());
    let tissue = if feed == Feed::Sinusoid {
        Tissue::Liver
    } else {
        Tissue::Skin
    };
    map.serves(&bed, tissue);
    map.unsupplied(&[artery.id(), vein.id(), bed.id()]);
    let sites = Sites {
        root: root.id(),
        skin: skin.id(),
        bed: bed.id(),
        artery: artery.id(),
        vein: vein.id(),
    };
    (map, sites)
}

fn compiled(feed: Feed) -> Result<(CompiledBody, Sites), AdmissionError> {
    compiled_with(feed, true)
}

fn compiled_with(feed: Feed, carry_heat: bool) -> Result<(CompiledBody, Sites), AdmissionError> {
    let (mut map, sites) = sites(feed);
    match feed {
        Feed::Valve => map.valve(sites.artery, sites.bed),
        _ => map.connect(Network::Vascular, sites.artery, sites.bed),
    }
    map.connect(Network::Vascular, sites.bed, sites.vein);
    map.connect(Network::Vascular, sites.vein, sites.artery);
    if feed == Feed::TwoFeeders {
        map.connect(Network::Vascular, sites.vein, sites.bed);
    }
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(PARTIAL_VOLUME).expect("finite"))?;
    for space in [Space::Cells, Space::Interstitium] {
        schema.leave_out(space, "a bench of blood and heat");
    }
    if feed != Feed::NoFlowRow {
        schema.carry::<Sodium, Blood>(Space::Plasma);
        schema.carry::<Sodium, Blood>(Space::Lumen);
    }
    for store in [sites.root, sites.skin] {
        schema.pool::<Energy>(store, Space::Thermal)?;
    }
    let body = assemble(schema, |mut body| {
        let class = class();
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let stores = ports.temperatures(&mut mt);
        let value = mt.kernel(|mut k| {
            let value = k.zero::<Temperature>();
            k.finish(value, mt.every_mode(), class)
        });
        mt.define(&stores.core, value);
        let value = mt.site_kernel(|mut k| {
            let value = k.zero::<Temperature>();
            k.finish(value, mt.every_mode(), class)
        });
        mt.define_at(&stores.other, value);
        if !carry_heat {
            let route = ports.conduction(&mut mt);
            let still = mt.site_kernel(|mut k| {
                let zero = k.zero::<Rate<Energy>>();
                k.finish(zero, mt.every_mode(), class)
            });
            mt.transfer_at(&route, still);
        }
        body.add(mt.finish());
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let circulation = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&circulation, still);
        if feed == Feed::Valve {
            let flow = ports.valves(&mut cv).flow_memory();
            let still = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
                let zero = k.zero::<Rate<Rate<Water>>>();
                k.finish(zero, cv.every_mode(), class)
            });
            cv.memory_rate(&flow, still);
        }
        if carry_heat {
            match feed {
                Feed::Dropped => drop(ports.heat_carried(&mut cv)),
                _ => carry(&mut cv, &ports, class),
            }
        }
        body.add(cv.finish());
        if feed == Feed::Sinusoid {
            let (mut hp, ports) = body.hepatic(None)?.parts();
            let route = ports.sinusoids(&mut hp);
            let still = hp.edge_kernel(|mut k| {
                let zero = k.zero::<Rate<Water>>();
                k.finish(zero, hp.every_mode(), class)
            });
            hp.transfer_along(&route, still);
            body.add(hp.finish());
        }
        body.compile()
    })?;
    Ok((body, sites))
}

fn carry<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    class: RateClass,
) {
    let carriage = ports.heat_carried(cv);
    let temperatures: PerfusionTemperatures<'b> = cv.receive();
    let blood = cv.parameter::<BloodHeat>(ParameterKey::new("cv.blood-heat"), Admissible::positive());
    let rate = cv.site_kernel(|mut k| {
        let water = k.bed_inflow();
        let flow = k.volume_flow(water);
        let (core, tissue) = (temperatures.core(&mut k), temperatures.tissue(&mut k));
        let across = k.subtract(core, tissue);
        let blood = k.parameter(&blood);
        let product = k.multiply(blood, flow);
        let per_kelvin = k.as_dimension(product);
        let product = k.multiply(per_kelvin, across);
        let value = k.as_dimension(product);
        k.finish(value, cv.every_mode(), class)
    });
    carriage.post(cv, rate);
}

struct Bench {
    body: CompiledBody,
    sites: Sites,
    parameters: Vec<f64>,
    modes: Vec<usize>,
    unknowns: Vec<f64>,
}

fn bench(feed: Feed) -> Bench {
    let (body, sites) = compiled(feed).unwrap_or_else(|refused| panic!("{refused:?}"));
    let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
    let note = "synthetic";
    let blood = measure::<BloodHeat>(BLOOD_HEAT);
    (packet.synthetic(ParameterKey::new("cv.blood-heat"), Binding::Body, blood, note)).expect("once");
    packet.start_rest(&body, |_| 1.0, note).expect("once");
    let bound = body.bind(packet).expect("bound");
    let unknowns = (body.rows().iter())
        .map(|row| bound.starts[row.id().index()])
        .collect();
    Bench {
        parameters: bound.values,
        modes: bound.modes,
        unknowns,
        body,
        sites,
    }
}

impl Bench {
    fn row(&self, kind: RowKind) -> usize {
        let found = self.body.rows().iter().position(|row| row.kind() == kind);
        found.unwrap_or_else(|| panic!("no row {kind:?}"))
    }
    fn flow(&self, from: SiteId, to: SiteId, family: &'static str) -> usize {
        let slot = Slot {
            law: LawName {
                owner: "CV",
                local: family,
            },
            at: Context::Edge { from, to },
        };
        match family {
            "valves" => self.row(RowKind::Memory(slot)),
            _ => self.row(RowKind::Algebraic(slot)),
        }
    }
    fn heat(&self, store: SiteId) -> RowId {
        let kind = RowKind::Amount {
            site: store,
            space: Space::Thermal,
            species: Species::Energy,
        };
        self.body.rows()[self.row(kind)].id()
    }
    fn set(&mut self, row: usize, value: f64) {
        self.unknowns[row] = value;
    }
    fn temperatures(&mut self, core: f64, tissue: f64) {
        let core_row = self.row(RowKind::Algebraic(Slot::from(LawName {
            owner: "MT",
            local: CORE_TEMPERATURE,
        })));
        let tissue_row = self.row(RowKind::Algebraic(Slot {
            law: LawName {
                owner: "MT",
                local: TISSUE_TEMPERATURE,
            },
            at: Context::Site(self.sites.skin),
        }));
        self.set(core_row, core);
        self.set(tissue_row, tissue);
    }
    fn carried(&self) -> (&Column, f64) {
        let frame = DriveFrame::default();
        let inputs = Inputs {
            parameters: &self.parameters,
            frame: &frame,
            unknowns: &self.unknowns,
            modes: &self.modes,
            draws: &[],
            factors: &[],
        };
        let values = self.body.instance_values(&inputs).expect("finite");
        let programs = self.body.programs();
        let mut posted = (self.body.instances().iter().zip(values))
            .filter(|(instance, _)| programs[instance.program().index()].law == CARRIED);
        let (instance, value) = posted.next().expect("one carriage law");
        assert!(posted.next().is_none(), "one bed carries");
        assert_eq!(instance.context, Context::Site(self.sites.bed));
        let mut columns = self
            .body
            .columns()
            .iter()
            .filter(|column| column.origin() == CARRIED);
        let column = columns.next().expect("one carriage column");
        assert!(columns.next().is_none(), "one bed carries");
        (column, value.expect("a rate"))
    }
}

#[test]
fn the_bed_inflow_binds_the_one_feeders_flow_row_or_the_valves_flow_memory() {
    for (feed, family) in [(Feed::Circulation, "circulation"), (Feed::Valve, "valves")] {
        let mut bench = bench(feed);
        bench.temperatures(5.0, 4.0);
        let (artery, bed, vein) = (bench.sites.artery, bench.sites.bed, bench.sites.vein);
        let (incoming, outgoing) = (
            bench.flow(artery, bed, family),
            bench.flow(bed, vein, "circulation"),
        );
        bench.set(outgoing, OUTGOING);
        for inflow in [1.0, 2.0] {
            bench.set(incoming, inflow);
            assert_eq!(
                bench.carried().1,
                BLOOD_HEAT * PARTIAL_VOLUME * inflow,
                "{family}"
            );
        }
    }
}

#[test]
fn two_feeders_a_sinusoid_and_an_edge_with_no_flow_row_refuse_the_bed_inflow() {
    for feed in [Feed::TwoFeeders, Feed::Sinusoid, Feed::NoFlowRow] {
        let refused = compiled(feed).err();
        let expected = AdmissionError::Compile(CompileDefect::NoSuchRead { law: CARRIED });
        assert_eq!(refused, Some(expected));
    }
    let (no_flow, sites) = compiled_with(Feed::NoFlowRow, false).expect("no-flow control compiles");
    let anatomy = &no_flow.layout.schema.anatomy;
    let feeders: Vec<(SiteId, SiteId)> = anatomy.vascular().filter(|edge| edge.1 == sites.bed).collect();
    assert_eq!(feeders, [(sites.artery, sites.bed)], "one vascular feeder");
    assert!(!anatomy.is_valve((sites.artery, sites.bed)), "plain vessel");
    assert_eq!(anatomy.tissue(sites.bed), Some(Tissue::Skin), "not a sinusoid");
    let flow_row = |from, to| {
        RowKind::Algebraic(Slot {
            law: LawName {
                owner: "CV",
                local: "circulation",
            },
            at: Context::Edge { from, to },
        })
    };
    assert!(
        !no_flow
            .rows()
            .iter()
            .any(|row| row.kind() == flow_row(sites.artery, sites.bed)),
        "Feed::NoFlowRow carries no circulation flow row, so BedInflow fails in Rows::inflow_row"
    );
    let (with_flow, with_sites) =
        compiled_with(Feed::Circulation, false).expect("circulation control compiles");
    assert!(
        with_flow
            .rows()
            .iter()
            .any(|row| row.kind() == flow_row(with_sites.artery, with_sites.bed)),
        "Feed::Circulation emits that flow row"
    );
    assert!(compiled(Feed::Circulation).is_ok());
}

#[test]
fn a_bed_no_vessel_feeds_is_refused_by_the_map_before_any_read() {
    let (mut map, sites) = sites(Feed::Circulation);
    map.connect(Network::Vascular, sites.bed, sites.vein);
    map.connect(Network::Vascular, sites.vein, sites.artery);
    let refused = map.finish().err();
    let expected = AdmissionError::Anatomy(AnatomyDefect::UnreachableBed { bed: "skin bed" });
    assert_eq!(refused, Some(expected));
}

#[test]
fn the_inflow_is_signed_and_one_column_debits_the_core_store_and_credits_the_beds_store() {
    let mut bench = bench(Feed::Circulation);
    bench.temperatures(5.0, 4.0);
    let (artery, bed, vein) = (bench.sites.artery, bench.sites.bed, bench.sites.vein);
    let (incoming, outgoing) = (
        bench.flow(artery, bed, "circulation"),
        bench.flow(bed, vein, "circulation"),
    );
    bench.set(outgoing, OUTGOING);
    for (inflow, posted) in [(-4.0, -24.0), (0.0, 0.0), (7.0, 42.0)] {
        bench.set(incoming, inflow);
        let (column, rate) = bench.carried();
        assert_eq!(rate, posted);
        let ends = [
            (bench.heat(bench.sites.root), -1),
            (bench.heat(bench.sites.skin), 1),
        ];
        assert_eq!(column.terms(), &ends);
    }
}

#[test]
fn the_carriage_posts_blood_heat_times_volume_flow_times_the_core_to_tissue_difference() {
    let mut bench = bench(Feed::Circulation);
    let incoming = bench.flow(bench.sites.artery, bench.sites.bed, "circulation");
    bench.set(incoming, 7.0);
    for (core, tissue) in [(5.0, 2.0), (2.0, 5.0), (3.0, 3.0)] {
        bench.temperatures(core, tissue);
        let posted = BLOOD_HEAT * (PARTIAL_VOLUME * 7.0) * (core - tissue);
        assert_eq!(bench.carried().1, posted);
    }
}

#[test]
fn a_carriage_dropped_unposted_still_owes_its_law() {
    let refused = compiled(Feed::Dropped).err();
    assert_eq!(
        refused,
        Some(AdmissionError::Compile(CompileDefect::OmittedLaw {
            law: CARRIED
        }))
    );
}
