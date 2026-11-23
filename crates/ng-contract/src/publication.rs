use crate::boundary::DriveFrame;
use crate::catalog::{PerfusionTemperatures, SkinThermoreceptor};
use crate::evaluate::Inputs;
use crate::law::{StoreHeat, StoreTemperature};
use crate::scope::{CORE_TEMPERATURE, TISSUE_TEMPERATURE};
use crate::*;

type Capacity = Quotient<Energy, Temperature>;
type Conductance = Quotient<Rate<Energy>, Temperature>;

const ZERO: f64 = 0.5;
const CORE_CAPACITY: f64 = 2.0;
const SKIN_CAPACITY: f64 = 4.0;
const AIRWAY_CAPACITY: f64 = 8.0;

fn measure<U: Dimension>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn named(owner: &'static str, local: &'static str) -> LawName {
    LawName { owner, local }
}

#[derive(Clone, Copy, PartialEq)]
enum Mark {
    SampledAtSkin,
    SampledAtAirway,
    Unplaced,
}

#[derive(Clone, Copy, PartialEq)]
enum Reads {
    Granted,
    OffSkin,
    Discarded,
    Guard,
    Kept,
    Forged,
    ForgedHeat,
    ForgedOwnBed,
    GuardOnNoValve,
    SensedInAGuard,
    SensedAtASite,
    PlainCoreInExplicit,
    UnreadExplicitCore,
    TermAtSkin,
    TermAtMuscle,
    TermUnrealized,
    NoSite,
}

struct Map {
    root: SiteId,
    skin: SiteId,
    airway: SiteId,
}

fn map(mark: Mark) -> Result<(Anatomy, Map), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at());
    let skin = map.site::<TissueSite>("skin heat site", at());
    let airway = map.site::<TissueSite>("conducting airway", at());
    let artery = map.site::<VesselSegment>("artery", at());
    let vein = map.site::<VesselSegment>("vein", at());
    let skin_bed = map.site::<CapillaryBed>("skin bed", at());
    let muscle_bed = map.site::<CapillaryBed>("muscle bed", at());
    for site in [skin.id(), airway.id(), artery.id(), vein.id(), muscle_bed.id()] {
        map.contain(root.id(), site);
    }
    map.contain(skin.id(), skin_bed.id());
    map.serves(&skin_bed, Tissue::Skin);
    map.serves(&muscle_bed, Tissue::Muscle);
    for bed in [skin_bed.id(), muscle_bed.id()] {
        map.connect(Network::Vascular, artery.id(), bed);
        map.connect(Network::Vascular, bed, vein.id());
    }
    map.connect(Network::Vascular, vein.id(), artery.id());
    map.unsupplied(&[artery.id(), vein.id(), skin_bed.id(), muscle_bed.id()]);
    match mark {
        Mark::SampledAtSkin => map.landmark(Landmark::SkinThermoreceptor, skin.id(), airway.id()),
        Mark::SampledAtAirway => map.landmark(Landmark::SkinThermoreceptor, airway.id(), skin.id()),
        Mark::Unplaced => {}
    }
    let sites = Map {
        root: root.id(),
        skin: skin.id(),
        airway: airway.id(),
    };
    Ok((map.finish()?, sites))
}

fn publish<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, ports: &MetabolismPorts<'b>, class: RateClass) {
    let stores = ports.temperatures(mt);
    let key = ParameterKey::new;
    let zero = mt.parameter::<Temperature>(key("mt.thermal-zero"), Admissible::finite());
    let core = mt.parameter::<Capacity>(key("mt.core-capacity"), Admissible::positive());
    let other = mt.site_parameter::<TissueSite, Capacity>(key("mt.tissue-capacity"), Admissible::positive());
    let value = mt.kernel(|mut k| {
        let (heat, capacity, zero) = (k.read(&stores.core_heat), k.parameter(&core), k.parameter(&zero));
        let ratio = k.divide(heat, capacity);
        let rise = k.as_dimension(ratio);
        let value = k.add(zero, rise);
        k.finish(value, mt.every_mode(), class)
    });
    mt.define(&stores.core, value);
    let value = mt.site_kernel(|mut k| {
        let (heat, capacity, zero) = (
            k.at_site(&stores.tissue_heat),
            k.site_parameter(&other),
            k.parameter(&zero),
        );
        let ratio = k.divide(heat, capacity);
        let rise = k.as_dimension(ratio);
        let value = k.add(zero, rise);
        k.finish(value, mt.every_mode(), class)
    });
    mt.define_at(&stores.other, value);
    let route = ports.conduction(mt);
    let conductance = mt.parameter::<Conductance>(key("mt.conductance"), Admissible::positive());
    let published = stores.published();
    let rate = mt.site_kernel(|mut k| {
        let (core, own) = (k.read(&published.core), k.at_site(&published.other));
        let (across, conductance) = (k.subtract(core, own), k.parameter(&conductance));
        let product = k.multiply(conductance, across);
        let value = k.as_dimension(product);
        k.finish(value, mt.every_mode(), class)
    });
    mt.transfer_at(&route, rate);
    let exit = ports
        .boundary::<SkinEvaporativeHeat>()
        .expect("the schema places the exit");
    let still = mt.kernel(|mut k| {
        let zero = k.zero::<Rate<Energy>>();
        k.finish(zero, mt.every_mode(), class)
    });
    mt.transfer(Transfer::new(&exit.route, still));
    let at_exit = published.at_exit(&exit);
    let sensed = mt.algebraic::<Temperature>("exit temperature", Admissible::finite());
    let residual = mt.kernel(|mut k| {
        let (own, read) = (k.algebraic(&sensed), k.read(&at_exit));
        let value = k.subtract(own, read);
        k.finish(value, mt.every_mode(), class)
    });
    mt.residual(&sensed, residual);
    let at_bed = published.at_bed();
    let (out, _) = mt.site_field::<CapillaryBed, Temperature>("bed temperature", Admissible::finite());
    let value = mt.site_kernel(|mut k| {
        let value = k.at_site(&at_bed);
        k.finish(value, mt.every_mode(), class)
    });
    mt.define_at(&out, value);
}

fn read_at_the_carriage<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    ports: &CardiovascularPorts<'b>,
    reads: Reads,
    class: RateClass,
) {
    let temperatures: PerfusionTemperatures<'b> = cv.receive();
    let (out, _) = cv.site_field::<Beds<Skin>, Temperature>("tissue end", Admissible::finite());
    let value = cv.site_kernel(|mut k| {
        let value = temperatures.tissue(&mut k);
        k.finish(value, cv.every_mode(), class)
    });
    cv.define_at(&out, value);
    let (out, _) = cv.site_field::<Beds<Skin>, Temperature>("core end", Admissible::finite());
    let value = cv.site_kernel(|mut k| {
        let value = temperatures.core(&mut k);
        k.finish(value, cv.every_mode(), class)
    });
    cv.define_at(&out, value);
    let forged = ReadPort::<Cardiovascular, Temperature>::field(named("MT", TISSUE_TEMPERATURE));
    match reads {
        Reads::Granted | Reads::SensedInAGuard | Reads::SensedAtASite => {}
        Reads::OffSkin => {
            let (out, _) = cv.site_field::<CapillaryBed, Temperature>("every bed", Admissible::finite());
            let value = cv.site_kernel(|mut k| {
                let value = temperatures.tissue(&mut k);
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&out, value);
        }
        Reads::Discarded => {
            let (out, _) = cv.site_field::<Beds<Skin>, Temperature>("discarded", Admissible::finite());
            let value = cv.site_kernel(|mut k| {
                let _ = k.read(&forged);
                let value = temperatures.tissue(&mut k);
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&out, value);
        }
        Reads::Guard => {
            let [on, off] = cv.modes("switch", ["on", "off"]);
            let gap = cv.kernel(|mut k| {
                let value = k.read(&forged);
                k.finish(value, on.only(), class)
            });
            cv.event(Guard::new(
                gap,
                Crossing::Rising,
                on.to(&off).expect("two values"),
            ));
        }
        Reads::Kept => {
            let (out, _) = cv.site_field::<Beds<Skin>, Temperature>("kept", Admissible::finite());
            let value = cv.site_kernel(|mut k| {
                let value = k.read(&forged);
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&out, value);
        }
        Reads::Forged => {
            let port =
                ReadPort::<Cardiovascular, Temperature>::temperature(StoreTemperature::SkinThermoreceptor);
            let sensed = cv.algebraic::<Temperature>("forged", Admissible::finite());
            let residual = cv.kernel(|mut k| {
                let (own, read) = (k.algebraic(&sensed), k.read(&port));
                let value = k.subtract(own, read);
                k.finish(value, cv.every_mode(), class)
            });
            cv.residual(&sensed, residual);
        }
        Reads::ForgedHeat => {
            let port = ReadPort::<Cardiovascular, Energy>::heat(StoreHeat::Core);
            let sensed = cv.algebraic::<Energy>("forged heat", Admissible::finite());
            let residual = cv.kernel(|mut k| {
                let (own, read) = (k.algebraic(&sensed), k.read(&port));
                let value = k.subtract(own, read);
                k.finish(value, cv.every_mode(), class)
            });
            cv.residual(&sensed, residual);
        }
        Reads::ForgedOwnBed => {
            let port = ReadPort::<Cardiovascular, Temperature>::temperature(StoreTemperature::OwnBed);
            let (out, _) = cv.site_field::<Beds<Skin>, Temperature>("forged own bed", Admissible::finite());
            let value = cv.site_kernel(|mut k| {
                let value = k.read(&port);
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&out, value);
        }
        Reads::GuardOnNoValve => {
            let valves = ports.valves(cv);
            let still = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
                let zero = k.zero::<Rate<Rate<Water>>>();
                k.finish(zero, cv.every_mode(), class)
            });
            cv.memory_rate(&valves.flow_memory(), still);
            let [open, shut] = cv.modes_along(&valves, "valve", ["open", "shut"]);
            let gap = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
                let value = k.read(&forged);
                k.finish(value, open.only(), class)
            });
            cv.event(Guard::new(
                gap,
                Crossing::Rising,
                open.to(&shut).expect("two values"),
            ));
        }
        Reads::PlainCoreInExplicit | Reads::UnreadExplicitCore => {
            let core = ReadPort::<Cardiovascular, Temperature>::field(named("MT", CORE_TEMPERATURE));
            let (explicit, read) = cv.explicit_field::<Beds<Muscle>, Temperature>("explicit core");
            cv.express_at(&explicit, |mut k| {
                let value = k.read(&core);
                k.finish_term(value)
            });
            if reads == Reads::UnreadExplicitCore {
                return;
            }
            let (out, _) =
                cv.site_field::<Beds<Muscle>, Temperature>("muscle temperature", Admissible::finite());
            let value = cv.site_kernel(|mut k| {
                let value = k.at_site(&read);
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&out, value);
        }
        Reads::TermAtSkin => {
            let term = cv.site_term::<Beds<Skin>, Temperature, _>("skin term", |mut k| {
                let value = temperatures.tissue(&mut k);
                k.finish_term(value)
            });
            host(cv, "skin host", &term, class);
        }
        Reads::TermAtMuscle => {
            let term = cv.site_term::<Beds<Muscle>, Temperature, _>("muscle term", |mut k| {
                let value = temperatures.tissue(&mut k);
                k.finish_term(value)
            });
            host(cv, "muscle host", &term, class);
        }
        Reads::TermUnrealized => {
            let term = cv.site_term::<Beds<Fat>, Temperature, _>("fat term", |mut k| {
                let value = k.read(&forged);
                k.finish_term(value)
            });
            host(cv, "fat host", &term, class);
        }
        Reads::NoSite => {
            let (out, _) = cv.site_field::<Beds<Fat>, Temperature>("no site", Admissible::finite());
            let value = cv.site_kernel(|mut k| {
                let value = k.read(&forged);
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&out, value);
        }
    }
}

fn host<'b, D: SiteSet>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    name: &'static str,
    term: &SiteTerm<'b, Cardiovascular, D, Temperature>,
    class: RateClass,
) where
    CapillaryBed: Includes<D>,
{
    let (out, _) = cv.site_field::<CapillaryBed, Temperature>(name, Admissible::finite());
    let value = cv.site_kernel(|mut k| {
        let zero = k.zero::<Temperature>();
        let value = k.add_from(zero, term);
        k.finish(value, cv.every_mode(), class)
    });
    cv.define_at(&out, value);
}

fn sense<'b>(ns: &mut FragmentBuilder<'b, Nervous>, reads: Reads, class: RateClass) {
    let sensor: SkinThermoreceptor<'b> = ns.receive();
    if reads == Reads::SensedInAGuard {
        let [on, off] = ns.modes("shiver", ["on", "off"]);
        let gap = ns.kernel(|mut k| {
            let value = sensor.temperature(&mut k);
            k.finish(value, on.only(), class)
        });
        ns.event(Guard::new(
            gap,
            Crossing::Rising,
            on.to(&off).expect("two values"),
        ));
        return;
    }
    if reads == Reads::SensedAtASite {
        let port = ReadPort::<Nervous, Temperature>::temperature(StoreTemperature::SkinThermoreceptor);
        let (out, _) = ns.site_field::<TissueSite, Temperature>("sensed at a site", Admissible::finite());
        let value = ns.site_kernel(|mut k| {
            let value = k.read(&port);
            k.finish(value, ns.every_mode(), class)
        });
        ns.define_at(&out, value);
    }
    let sensed = ns.algebraic::<Temperature>("sensed skin", Admissible::finite());
    let residual = ns.kernel(|mut k| {
        let (own, read) = (k.algebraic(&sensed), sensor.temperature(&mut k));
        let value = k.subtract(own, read);
        k.finish(value, ns.every_mode(), class)
    });
    ns.residual(&sensed, residual);
}

fn compiled(mark: Mark, reads: Reads) -> Result<(CompiledBody, Map), AdmissionError> {
    let (anatomy, sites) = map(mark)?;
    let mut schema = Schema::new(anatomy)?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of heat alone");
    }
    for store in [sites.root, sites.skin, sites.airway] {
        schema.pool::<Energy>(store, Space::Thermal)?;
    }
    schema.process::<SkinEvaporativeHeat>(sites.skin, Space::Thermal)?;
    let body = assemble(schema, |mut body| {
        let class = RateClass::new(RateBand::Wave, 1_000_000_000)?;
        let (mut mt, ports) = body.metabolism(None)?.parts();
        publish(&mut mt, &ports, class);
        body.add(mt.finish());
        let (mut cv, ports) = body.cardiovascular()?.parts();
        read_at_the_carriage(&mut cv, &ports, reads, class);
        body.add(cv.finish());
        let (mut ns, _) = body.nervous()?.parts();
        sense(&mut ns, reads, class);
        body.add(ns.finish());
        body.compile()
    })?;
    Ok((body, sites))
}

fn refused(defect: CompileDefect) -> Option<AdmissionError> {
    Some(AdmissionError::Compile(defect))
}

struct Bench {
    body: CompiledBody,
    sites: Map,
    parameters: Vec<f64>,
    unknowns: Vec<f64>,
    modes: Vec<usize>,
}

fn bound(heats: [f64; 3], airway_capacity: f64) -> Bench {
    let (body, sites) =
        compiled(Mark::SampledAtSkin, Reads::Granted).unwrap_or_else(|refused| panic!("{refused:?}"));
    let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
    let (key, note) = (ParameterKey::new, "synthetic");
    packet
        .synthetic(
            key("mt.thermal-zero"),
            Binding::Body,
            measure::<Temperature>(ZERO),
            note,
        )
        .expect("once");
    packet
        .synthetic(
            key("mt.core-capacity"),
            Binding::Body,
            measure::<Capacity>(CORE_CAPACITY),
            note,
        )
        .expect("once");
    packet
        .synthetic(
            key("mt.conductance"),
            Binding::Body,
            measure::<Conductance>(1.0),
            note,
        )
        .expect("once");
    for (store, capacity) in [(sites.skin, SKIN_CAPACITY), (sites.airway, airway_capacity)] {
        let capacity = measure::<Capacity>(capacity);
        packet
            .synthetic(key("mt.tissue-capacity"), Binding::Site(store), capacity, note)
            .expect("once");
    }
    let stores = [sites.root, sites.skin, sites.airway];
    let start = |row: RowKind| match row {
        RowKind::Amount { site, .. } => {
            heats[stores.iter().position(|&store| store == site).expect("a store")]
        }
        _ => 0.0,
    };
    packet.start_rest(&body, start, note).expect("once");
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
    fn row(&self, kind: RowKind) -> usize {
        let found = self.body.rows().iter().position(|row| row.kind() == kind);
        found.unwrap_or_else(|| panic!("no row {kind:?}"))
    }
    fn temperature_row(&self, store: SiteId) -> usize {
        match store == self.sites.root {
            true => self.row(RowKind::Algebraic(Slot::from(named("MT", CORE_TEMPERATURE)))),
            false => self.row(RowKind::Algebraic(Slot {
                law: named("MT", TISSUE_TEMPERATURE),
                at: Context::Site(store),
            })),
        }
    }
    fn heat_row(&self, store: SiteId) -> usize {
        self.row(RowKind::Amount {
            site: store,
            space: Space::Thermal,
            species: Species::Energy,
        })
    }
    fn temperature(&self, store: SiteId) -> f64 {
        let frame = DriveFrame::default();
        let values = self.body.instance_values(&self.inputs(&frame)).expect("finite");
        let row = RowId::at(self.temperature_row(store));
        let defining = (self.body.layout.instances.iter().zip(values))
            .find(|(instance, _)| instance.target == Target::Row(row))
            .and_then(|(_, value)| value);
        -defining.expect("one law defines the row")
    }
    fn slope(&self, temperature: SiteId, heat: SiteId) -> f64 {
        let frame = DriveFrame::default();
        let jacobian = self.body.jacobian(&self.inputs(&frame), 0.0).expect("finite");
        let (row, column) = (self.temperature_row(temperature), self.heat_row(heat));
        let sparsity = self.body.sparsity();
        (sparsity.offsets[row]..sparsity.offsets[row + 1])
            .find(|&at| sparsity.unknowns[at] == UnknownId::at(column))
            .map_or(0.0, |at| jacobian[at])
    }
}

#[test]
fn each_store_has_its_own_temperature_with_slope_one_over_its_own_capacity() {
    let first = bound([10.0, 6.0, 20.0], AIRWAY_CAPACITY);
    let (core, skin, airway) = (first.sites.root, first.sites.skin, first.sites.airway);
    let temperatures = |bench: &Bench| [core, skin, airway].map(|store| bench.temperature(store).to_bits());
    let bits = |values: [f64; 3]| values.map(f64::to_bits);
    assert_eq!(temperatures(&first), bits([5.5, 2.0, 3.0]));
    let airway_heat_moves = bound([10.0, 6.0, 28.0], AIRWAY_CAPACITY);
    assert_eq!(temperatures(&airway_heat_moves), bits([5.5, 2.0, 4.0]));
    let skin_heat_moves = bound([10.0, 14.0, 20.0], AIRWAY_CAPACITY);
    assert_eq!(temperatures(&skin_heat_moves), bits([5.5, 4.0, 3.0]));
    let airway_capacity_moves = bound([10.0, 6.0, 20.0], 16.0);
    assert_eq!(temperatures(&airway_capacity_moves), bits([5.5, 2.0, 1.75]));
    let slopes = |bench: &Bench| {
        [
            (skin, skin),
            (skin, airway),
            (airway, airway),
            (airway, skin),
            (core, core),
            (skin, core),
        ]
        .map(|(row, heat)| bench.slope(row, heat).to_bits())
    };
    let own = |capacity: f64| -1.0 / capacity;
    let expected = [
        own(SKIN_CAPACITY),
        0.0,
        own(AIRWAY_CAPACITY),
        0.0,
        own(CORE_CAPACITY),
        0.0,
    ];
    assert_eq!(slopes(&first), expected.map(f64::to_bits));
    let moved = [own(SKIN_CAPACITY), 0.0, own(16.0), 0.0, own(CORE_CAPACITY), 0.0];
    assert_eq!(slopes(&airway_capacity_moves), moved.map(f64::to_bits));
}

#[test]
fn the_carriages_tissue_end_the_skin_thermoreceptor_and_the_skin_exit_read_the_one_skin_row() {
    let bench = bound([10.0, 6.0, 20.0], AIRWAY_CAPACITY);
    let stores = [bench.sites.root, bench.sites.skin, bench.sites.airway];
    let rows = stores.map(|store| UnknownId::at(bench.temperature_row(store)));
    let read = |law: LawName| -> Vec<(Context, Vec<usize>)> {
        let instances = bench.body.layout.instances.iter();
        let placed =
            instances.filter(|instance| bench.body.layout.programs[instance.program.index()].law == law);
        placed
            .map(|instance| {
                let unknowns = instance.inputs.iter().filter_map(crate::body::Bound::unknown);
                let stores = unknowns.filter_map(|unknown| rows.iter().position(|&row| row == unknown));
                (instance.context, stores.collect())
            })
            .collect()
    };
    let skin_bed = Context::Site(
        bench
            .body
            .layout
            .instances
            .iter()
            .find_map(|instance| match instance.context {
                Context::Site(site) if site.key() == "skin bed" => Some(site),
                _ => None,
            })
            .expect("the skin bed's instance"),
    );
    let (core, skin) = (0, 1);
    assert_eq!(read(named("CV", "tissue end")), [(skin_bed, vec![skin])]);
    assert_eq!(read(named("CV", "core end")), [(skin_bed, vec![core])]);
    assert_eq!(read(named("NS", "sensed skin")), [(Context::Body, vec![skin])]);
    assert_eq!(
        read(named("MT", "exit temperature")),
        [(Context::Body, vec![skin])]
    );
    let mut beds = read(named("MT", "bed temperature"));
    beds.sort_by_key(|(context, _)| *context);
    let muscle_bed = beds
        .iter()
        .map(|(context, _)| *context)
        .find(|&context| context != skin_bed);
    let mut expected = vec![
        (skin_bed, vec![skin]),
        (muscle_bed.expect("the muscle bed"), vec![core]),
    ];
    expected.sort_by_key(|(context, _)| *context);
    assert_eq!(beds, expected);
    let at = |store| Context::Site(store);
    let mut conduction = read(named("MT", "conduction"));
    conduction.sort_by_key(|(context, _)| *context);
    let mut expected = vec![
        (at(bench.sites.skin), vec![core, skin]),
        (at(bench.sites.airway), vec![core, 2]),
    ];
    expected.sort_by_key(|(context, _)| *context);
    assert_eq!(conduction, expected);
}

#[test]
fn the_thermoreceptor_reads_its_sampled_store_and_refuses_one_no_skin_bed_sits_in() {
    let skin_temperature = named("MT", "skin_temperature");
    let law = named("NS", "sensed skin");
    let moved = compiled(Mark::SampledAtAirway, Reads::Granted).err();
    assert_eq!(
        moved,
        refused(CompileDefect::UnwiredRead {
            law,
            field: skin_temperature
        })
    );
    let unplaced = compiled(Mark::Unplaced, Reads::Granted).err();
    assert_eq!(unplaced, refused(CompileDefect::NoSuchRead { law }));
    let guarded = compiled(Mark::SampledAtAirway, Reads::SensedInAGuard).err();
    assert_eq!(
        guarded,
        refused(CompileDefect::UnwiredRead {
            law: named("NS", "shiver"),
            field: skin_temperature
        })
    );
}

#[test]
fn an_off_skin_place_and_an_ungranted_field_refuse_even_in_a_discarded_read_or_a_guard() {
    let carried = named("MT", "temperature");
    let law = named("CV", "every bed");
    let off_skin = compiled(Mark::SampledAtSkin, Reads::OffSkin).err();
    assert_eq!(
        off_skin,
        refused(CompileDefect::UnwiredRead { law, field: carried })
    );
    let field = named("MT", TISSUE_TEMPERATURE);
    for (reads, local) in [(Reads::Discarded, "discarded"), (Reads::Guard, "switch")] {
        let law = named("CV", local);
        let found = compiled(Mark::SampledAtSkin, reads).err();
        assert_eq!(
            found,
            refused(CompileDefect::UnwiredRead { law, field }),
            "{local}"
        );
    }
}

#[test]
fn the_catalog_checks_a_kept_or_forged_read_each_terms_variant_and_a_read_no_site_realizes() {
    let tissue = named("MT", TISSUE_TEMPERATURE);
    let (sensed, heat) = (named("MT", "skin_temperature"), named("MT", "core_heat"));
    let carried = named("MT", "temperature");
    let cases = [
        (Reads::Kept, named("CV", "kept"), tissue),
        (Reads::Forged, named("CV", "forged"), sensed),
        (Reads::ForgedHeat, named("CV", "forged heat"), heat),
        (Reads::ForgedOwnBed, named("CV", "forged own bed"), tissue),
        (Reads::SensedAtASite, named("NS", "sensed at a site"), sensed),
        (Reads::TermAtMuscle, named("CV", "muscle host"), carried),
        (Reads::TermUnrealized, named("CV", "fat host"), tissue),
        (Reads::NoSite, named("CV", "no site"), tissue),
        (Reads::GuardOnNoValve, named("CV", "valve"), tissue),
    ];
    for (reads, law, field) in cases {
        let found = compiled(Mark::SampledAtSkin, reads).err();
        assert_eq!(
            found,
            refused(CompileDefect::UnwiredRead { law, field }),
            "{}",
            law.local
        );
    }
    compiled(Mark::SampledAtSkin, Reads::TermAtSkin).unwrap_or_else(|refused| panic!("{refused:?}"));
}

/// Ticket 102 (0011, 31): "In the contract's own tests it refuses two reads by a reader and place the
/// catalog does not authorize: a forged projected read, and a plain `Source::Field` read of a protected name."
#[test]
fn a_plain_core_temperature_read_inlined_at_a_muscle_bed_is_refused() {
    let found = compiled(Mark::SampledAtSkin, Reads::PlainCoreInExplicit).err();
    let field = named("MT", CORE_TEMPERATURE);
    let laws = [named("CV", "explicit core"), named("CV", "muscle temperature")];
    assert!(
        laws.into_iter()
            .any(|law| found == refused(CompileDefect::UnwiredRead { law, field })),
        "expected UnwiredRead for {field:?} in the CV muscle read, got {found:?}"
    );
}

/// Section 7: "A store temperature read by a card the catalog grants no such read, or at a place its arrow
/// does not allow, kept, discarded, in a guard, in a site term no site realizes or in an explicit field".
/// "`Assembly::compile` refuses it before placing it (`UnwiredRead`) and names the law and the field."
#[test]
fn an_unread_explicit_field_must_have_the_right_to_read_the_core_temperature() {
    let build = |reads| compiled(Mark::SampledAtSkin, reads).map(|_| ());
    assert_eq!(build(Reads::Granted), Ok(()));
    let law = named("CV", "explicit core");
    let field = named("MT", CORE_TEMPERATURE);
    assert_eq!(
        build(Reads::UnreadExplicitCore),
        Err(AdmissionError::Compile(CompileDefect::UnwiredRead { law, field }))
    );
}

fn bare() -> Result<Schema, AdmissionError> {
    let mut schema = Schema::new(map(Mark::SampledAtSkin)?.0)?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench with no water");
    }
    Ok(schema)
}

fn class() -> RateClass {
    RateClass::new(RateBand::Wave, 1_000_000_000).expect("a class")
}

fn row_kinds(body: &CompiledBody) -> Vec<RowKind> {
    body.rows().iter().map(|row| row.kind()).collect()
}

#[test]
fn the_temperatures_stand_at_every_store_whatever_transfers_mt_asks_for() -> Result<(), AdmissionError> {
    let (anatomy, sites) = map(Mark::SampledAtSkin)?;
    let mut schema = Schema::new(anatomy)?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of heat alone");
    }
    for store in [sites.root, sites.skin, sites.airway] {
        schema.pool::<Energy>(store, Space::Thermal)?;
    }
    schema.process::<AirwayHeat>(sites.airway, Space::Thermal)?;
    let class = class();
    let body = assemble(schema, |mut body| {
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
        body.add(mt.finish());
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.heat_carried(&mut cv);
        let still = cv.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, cv.every_mode(), class)
        });
        route.post(&mut cv, still);
        body.add(cv.finish());
        let (mut rs, ports) = body.respiratory(None)?.parts();
        let exit = ports.boundary::<AirwayHeat>()?;
        let still = rs.kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, rs.every_mode(), class)
        });
        rs.transfer(Transfer::new(&exit.route, still));
        body.add(rs.finish());
        body.compile()
    })?;
    let temperature = |store: SiteId| {
        RowKind::Algebraic(Slot {
            law: named("MT", TISSUE_TEMPERATURE),
            at: Context::Site(store),
        })
    };
    let core = RowKind::Algebraic(Slot::from(named("MT", CORE_TEMPERATURE)));
    let mut found: Vec<RowKind> = (row_kinds(&body).into_iter())
        .filter(|kind| matches!(kind, RowKind::Algebraic(slot) if slot.law.owner == "MT"))
        .collect();
    let mut expected = vec![core, temperature(sites.skin), temperature(sites.airway)];
    expected.sort_by_key(|kind| format!("{kind:?}"));
    found.sort_by_key(|kind| format!("{kind:?}"));
    assert_eq!(found, expected);
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum Emits {
    CoreAlone,
    DebtAlone,
    CoreBesideTheTemperatures,
}

fn emitted(emits: Emits) -> Result<CompiledBody, AdmissionError> {
    let class = class();
    assemble(bare()?, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let value = mt.kernel(|mut k| {
            let value = k.zero::<Temperature>();
            k.finish(value, mt.every_mode(), class)
        });
        match emits {
            Emits::CoreAlone => {
                let core: catalog::CoreTemperature<catalog::Defines<'_, Metabolism>> = mt.emit();
                mt.define(&core.core_temperature, value);
            }
            Emits::DebtAlone => {
                let debt: catalog::OxygenDebtPerBed<catalog::Defines<'_, Metabolism>> = mt.emit();
                let value = mt.site_kernel(|mut k| {
                    let value = k.zero::<Oxygen>();
                    k.finish(value, mt.every_mode(), class)
                });
                mt.define_at(&debt.oxygen_debt_per_bed, value);
            }
            Emits::CoreBesideTheTemperatures => {
                let _stores = ports.temperatures(&mut mt);
                let core: catalog::CoreTemperature<catalog::Defines<'_, Metabolism>> = mt.emit();
                mt.define(&core.core_temperature, value);
            }
        }
        body.add(mt.finish());
        body.compile()
    })
}

#[test]
fn the_core_scalar_and_the_debt_each_emit_alone_and_the_core_twice_is_named_twice() {
    let core = emitted(Emits::CoreAlone).unwrap_or_else(|refused| panic!("{refused:?}"));
    let core_row = RowKind::Algebraic(Slot::from(named("MT", CORE_TEMPERATURE)));
    assert_eq!(row_kinds(&core), [core_row]);
    let debt = emitted(Emits::DebtAlone).unwrap_or_else(|refused| panic!("{refused:?}"));
    let owed =
        |kind: &RowKind| matches!(kind, RowKind::Algebraic(slot) if slot.law.local != "oxygen_debt_per_bed");
    assert!(!row_kinds(&debt).is_empty() && !row_kinds(&debt).iter().any(owed));
    let twice = emitted(Emits::CoreBesideTheTemperatures).err();
    let law = named("MT", CORE_TEMPERATURE);
    assert_eq!(twice, refused(CompileDefect::NamedTwice { law }));
}

/// Item 4: "An explicit field's own reads, kept and discarded, are checked for reader, field and place
/// under the field's own law, at the site the field is written into, before compile writes them into
/// the law or the guard that reads the field."
/// Section 7: "`Assembly::compile` refuses it before placing it (`UnwiredRead`) and names the law and the field."
#[test]
fn a_receptor_fields_core_temperature_read_is_checked_when_only_a_guard_uses_it() {
    let law = named(Receptor::CARD, "guard temperature");
    let field = named("MT", CORE_TEMPERATURE);
    let build = |read_core: bool| {
        assemble(bare()?, |mut body| {
            let (mut mt, _) = body.metabolism(None)?.parts();
            let core: catalog::CoreTemperature<catalog::Defines<'_, Metabolism>> = mt.emit();
            let value = mt.kernel(|mut k| {
                let value = k.zero::<Temperature>();
                k.finish(value, mt.every_mode(), class())
            });
            mt.define(&core.core_temperature, value);
            body.add(mt.finish());

            let (mut rc, _) = body.receptor()?.parts();
            let (explicit, _) = rc.explicit_field::<TissueSite, Temperature>(law.local);
            let core = ReadPort::<Receptor, Temperature>::field(field);
            rc.express_at(&explicit, |mut k| {
                let value = if read_core { k.read(&core) } else { k.zero() };
                k.finish_term(value)
            });
            body.add(rc.finish());

            let (mut ns, _) = body.nervous()?.parts();
            let sensed = ReadPort::<Nervous, PerSite<TissueSite, Temperature>>::field(law);
            let [on, off] = ns.modes("shiver", ["on", "off"]);
            let gap = ns.kernel(|mut k| {
                let value = k.at_landmark(Landmark::SkinThermoreceptor, &sensed);
                k.finish(value, on.only(), class())
            });
            ns.event(Guard::new(gap, Crossing::Rising, on.to(&off)?));
            body.add(ns.finish());
            body.compile()
        })
        .map(|_| ())
    };
    assert_eq!(build(false), Ok(()));
    assert_eq!(
        build(true),
        Err(AdmissionError::Compile(CompileDefect::UnwiredRead { law, field }))
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Lookup {
    BodyFieldOfTheName,
    AlgebraicOfTheName,
    CoreAtASite,
}

fn looked_up(lookup: Lookup) -> Option<AdmissionError> {
    let class = class();
    let body = assemble(bare().expect("a map"), |mut body| {
        let (mut mt, _) = body.metabolism(None)?.parts();
        let value = mt.kernel(|mut k| {
            let value = k.zero::<Temperature>();
            k.finish(value, mt.every_mode(), class)
        });
        match lookup {
            Lookup::BodyFieldOfTheName => {
                mt.owe_defined::<Temperature>(TISSUE_TEMPERATURE, law::Over::Body);
                mt.define(&Output::named(TISSUE_TEMPERATURE), value);
            }
            Lookup::AlgebraicOfTheName => {
                let row = mt.algebraic::<Temperature>(TISSUE_TEMPERATURE, Admissible::finite());
                mt.residual(&row, value);
            }
            Lookup::CoreAtASite => {
                mt.owe_defined::<Temperature>(CORE_TEMPERATURE, law::Over::Every(TissueSite::SET));
                let core: Output<'_, Metabolism, PerSite<TissueSite, Temperature>> =
                    Output::named(CORE_TEMPERATURE);
                let value = mt.site_kernel(|mut k| {
                    let value = k.zero::<Temperature>();
                    k.finish(value, mt.every_mode(), class)
                });
                mt.define_at(&core, value);
            }
        }
        let name = match lookup {
            Lookup::CoreAtASite => named("MT", CORE_TEMPERATURE),
            _ => named("MT", TISSUE_TEMPERATURE),
        };
        let port = ReadPort::<Metabolism, PerSite<TissueSite, Temperature>>::field(name);
        let (out, _) = mt.site_field::<TissueSite, Temperature>("reads", Admissible::finite());
        let value = mt.site_kernel(|mut k| {
            let value = k.at_site(&port);
            k.finish(value, mt.every_mode(), class)
        });
        mt.define_at(&out, value);
        body.add(mt.finish());
        body.compile()
    });
    body.err()
}

#[test]
fn a_site_temperature_read_finds_its_own_row_and_never_the_bodys_of_that_name() {
    let unemitted = refused(CompileDefect::UnemittedField {
        law: named("MT", "reads"),
        field: named("MT", TISSUE_TEMPERATURE),
    });
    assert_eq!(looked_up(Lookup::BodyFieldOfTheName), unemitted);
    assert_eq!(looked_up(Lookup::AlgebraicOfTheName), unemitted);
}

#[test]
fn the_core_read_finds_only_the_bodys_row_and_a_store_read_only_its_stores_row() {
    let core = refused(CompileDefect::UnemittedField {
        law: named("MT", "reads"),
        field: named("MT", CORE_TEMPERATURE),
    });
    assert_eq!(looked_up(Lookup::CoreAtASite), core);
    let tissue = refused(CompileDefect::UnemittedField {
        law: named("CV", "tissue end"),
        field: named("MT", TISSUE_TEMPERATURE),
    });
    assert_eq!(sent(Sender::BodyOnly, Reader::Carriage), tissue);
}

#[derive(Clone, Copy, PartialEq)]
enum Missing {
    CoreStore,
    HeatAtAnUnheatedSite,
    ExitAtABedsHeat,
    ExitAtTheElasticStore,
}

fn missing(missing: Missing) -> Option<AdmissionError> {
    let class = class();
    let (anatomy, sites) = map(Mark::SampledAtSkin).expect("a map");
    let muscle_bed = anatomy.in_set(Sites::Tissue(Tissue::Muscle))[0];
    let mut schema = Schema::new(anatomy).expect("a schema");
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of heat alone");
    }
    let stores = match missing {
        Missing::CoreStore => vec![sites.skin],
        _ => vec![sites.root, sites.skin],
    };
    for store in stores {
        schema.pool::<Energy>(store, Space::Thermal).expect("a store");
    }
    match missing {
        Missing::ExitAtABedsHeat => {
            schema
                .pool::<Energy>(muscle_bed, Space::Thermal)
                .expect("a bed's heat");
            schema
                .process::<DryHeatLoss>(muscle_bed, Space::Thermal)
                .expect("an exit");
        }
        Missing::ExitAtTheElasticStore => {
            schema
                .pool::<Energy>(sites.root, Space::Elastic)
                .expect("the elastic store");
            schema
                .process::<DryHeatLoss>(sites.root, Space::Elastic)
                .expect("an exit");
        }
        Missing::CoreStore | Missing::HeatAtAnUnheatedSite => {}
    }
    let body = assemble(schema, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let stores = ports.temperatures(&mut mt);
        let key = ParameterKey::new;
        let capacity = mt.parameter::<Capacity>(key("mt.core-capacity"), Admissible::positive());
        let conductance = mt.parameter::<Conductance>(key("mt.conductance"), Admissible::positive());
        let value = mt.kernel(|mut k| {
            let (heat, capacity) = (k.read(&stores.core_heat), k.parameter(&capacity));
            let ratio = k.divide(heat, capacity);
            let value = k.as_dimension(ratio);
            k.finish(value, mt.every_mode(), class)
        });
        mt.define(&stores.core, value);
        let value = mt.site_kernel(|mut k| {
            let value = k.zero::<Temperature>();
            k.finish(value, mt.every_mode(), class)
        });
        mt.define_at(&stores.other, value);
        match missing {
            Missing::HeatAtAnUnheatedSite => {
                let (out, _) = mt.site_field::<TissueSite, Energy>("every site's heat", Admissible::finite());
                let value = mt.site_kernel(|mut k| {
                    let value = k.at_site(&stores.tissue_heat);
                    k.finish(value, mt.every_mode(), class)
                });
                mt.define_at(&out, value);
            }
            Missing::ExitAtABedsHeat | Missing::ExitAtTheElasticStore => {
                let exit = ports.boundary::<DryHeatLoss>()?;
                let at_exit = stores.published().at_exit(&exit);
                let rate = mt.kernel(|mut k| {
                    let (temperature, conductance) = (k.read(&at_exit), k.parameter(&conductance));
                    let product = k.multiply(conductance, temperature);
                    let value = k.as_dimension(product);
                    k.finish(value, mt.every_mode(), class)
                });
                mt.transfer(Transfer::new(&exit.route, rate));
            }
            Missing::CoreStore => {}
        }
        body.add(mt.finish());
        body.compile()
    });
    body.err()
}

#[test]
fn a_store_read_whose_store_is_missing_or_no_tissue_heat_store_is_refused() {
    let no_read = |local| {
        refused(CompileDefect::NoSuchRead {
            law: named("MT", local),
        })
    };
    assert_eq!(missing(Missing::CoreStore), no_read(CORE_TEMPERATURE));
    assert_eq!(
        missing(Missing::HeatAtAnUnheatedSite),
        no_read("every site's heat")
    );
    assert_eq!(missing(Missing::ExitAtABedsHeat), no_read("dry heat loss"));
    assert_eq!(missing(Missing::ExitAtTheElasticStore), no_read("dry heat loss"));
}

fn heated() -> (Schema, Map) {
    let (anatomy, sites) = map(Mark::SampledAtSkin).expect("a map");
    let mut schema = Schema::new(anatomy).expect("a schema");
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of heat alone");
    }
    for store in [sites.root, sites.skin, sites.airway] {
        schema.pool::<Energy>(store, Space::Thermal).expect("a store");
    }
    (schema, sites)
}

fn zeroed<'b>(mt: &mut FragmentBuilder<'b, Metabolism>, stores: &TissueTemperatures<'b>, class: RateClass) {
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
}

#[derive(Clone, Copy, PartialEq)]
enum Sender {
    Absent,
    Undefined,
    OtherDimension,
    BodyOnly,
}

#[derive(Clone, Copy, PartialEq)]
enum Reader {
    Carriage,
    DiscardedCarriage,
    Thermoreceptor,
}

fn sent(sender: Sender, reader: Reader) -> Option<AdmissionError> {
    let class = class();
    let body = assemble(heated().0, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        match sender {
            Sender::Absent => {}
            Sender::Undefined => {
                let stores = ports.temperatures(&mut mt);
                let value = mt.kernel(|mut k| {
                    let value = k.zero::<Temperature>();
                    k.finish(value, mt.every_mode(), class)
                });
                mt.define(&stores.core, value);
            }
            Sender::OtherDimension => {
                mt.owe_defined::<Energy>(TISSUE_TEMPERATURE, law::Over::OtherTissueHeatStores);
                let other: Output<'_, Metabolism, PerSite<TissueSite, Energy>> =
                    Output::named(TISSUE_TEMPERATURE);
                let value = mt.site_kernel(|mut k| {
                    let value = k.zero::<Energy>();
                    k.finish(value, mt.every_mode(), class)
                });
                mt.define_at(&other, value);
            }
            Sender::BodyOnly => {
                mt.owe_defined::<Temperature>(TISSUE_TEMPERATURE, law::Over::Body);
                let value = mt.kernel(|mut k| {
                    let value = k.zero::<Temperature>();
                    k.finish(value, mt.every_mode(), class)
                });
                mt.define(&Output::named(TISSUE_TEMPERATURE), value);
            }
        }
        body.add(mt.finish());
        match reader {
            Reader::Carriage | Reader::DiscardedCarriage => {
                let (mut cv, _) = body.cardiovascular()?.parts();
                let temperatures: PerfusionTemperatures<'_> = cv.receive();
                let (out, _) = cv.site_field::<Beds<Skin>, Temperature>("tissue end", Admissible::finite());
                let value = cv.site_kernel(|mut k| {
                    let read = temperatures.tissue(&mut k);
                    let value = match reader {
                        Reader::Carriage => read,
                        _ => k.zero::<Temperature>(),
                    };
                    k.finish(value, cv.every_mode(), class)
                });
                cv.define_at(&out, value);
                body.add(cv.finish());
            }
            Reader::Thermoreceptor => {
                let (mut ns, _) = body.nervous()?.parts();
                sense(&mut ns, Reads::Granted, class);
                body.add(ns.finish());
            }
        }
        body.compile()
    });
    body.err()
}

#[test]
fn each_projected_grant_refuses_a_missing_an_undefined_and_a_wrongly_dimensioned_sender() {
    let field = named("MT", TISSUE_TEMPERATURE);
    let (carriage, sensor) = (named("CV", "tissue end"), named("NS", "sensed skin"));
    for (reader, law) in [(Reader::Carriage, carriage), (Reader::Thermoreceptor, sensor)] {
        let absent = refused(CompileDefect::UnemittedField { law, field });
        assert_eq!(sent(Sender::Absent, reader), absent);
        let omitted = refused(CompileDefect::OmittedLaw { law: field });
        assert_eq!(sent(Sender::Undefined, reader), omitted);
    }
    for (reader, law) in [
        (Reader::Carriage, carriage),
        (Reader::DiscardedCarriage, carriage),
        (Reader::Thermoreceptor, sensor),
    ] {
        let dimension = refused(CompileDefect::FieldDimension { law, field });
        assert_eq!(sent(Sender::OtherDimension, reader), dimension);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Expressed {
    Inflow,
    StoreHeat,
    StoreTemperature,
}

fn expressed(read: Expressed) -> Option<AdmissionError> {
    let class = class();
    let body = assemble(heated().0, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let stores = ports.temperatures(&mut mt);
        zeroed(&mut mt, &stores, class);
        match read {
            Expressed::StoreHeat => {
                let (explicit, _) = mt.explicit_field::<TissueSite, Energy>("explicit heat");
                mt.express_at(&explicit, |mut k| {
                    let value = k.at_site(&stores.tissue_heat);
                    k.finish_term(value)
                });
            }
            Expressed::StoreTemperature => {
                let at_bed = stores.published().at_bed();
                let (explicit, _) = mt.explicit_field::<CapillaryBed, Temperature>("explicit temperature");
                mt.express_at(&explicit, |mut k| {
                    let value = k.at_site(&at_bed);
                    k.finish_term(value)
                });
            }
            Expressed::Inflow => {}
        }
        body.add(mt.finish());
        if read == Expressed::Inflow {
            let (mut cv, _) = body.cardiovascular()?.parts();
            let (explicit, _) = cv.explicit_field::<CapillaryBed, Rate<Water>>("explicit inflow");
            cv.express_at(&explicit, |mut k| {
                let value = k.bed_inflow();
                k.finish_term(value)
            });
            body.add(cv.finish());
        }
        body.compile()
    });
    body.err()
}

#[test]
fn an_explicit_field_reading_the_inflow_a_stores_heat_or_a_stores_temperature_is_refused() {
    let cases = [
        (Expressed::Inflow, named("CV", "explicit inflow")),
        (Expressed::StoreHeat, named("MT", "explicit heat")),
        (Expressed::StoreTemperature, named("MT", "explicit temperature")),
    ];
    for (read, law) in cases {
        assert_eq!(
            expressed(read),
            refused(CompileDefect::NoSuchRead { law }),
            "{}",
            law.local
        );
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Choice {
    Base,
    CoreEnd,
    DryExit,
    CoreHeat,
}

fn hashed(choice: Choice) -> ModelHash {
    let class = class();
    let (mut schema, sites) = heated();
    schema
        .process::<SkinEvaporativeHeat>(sites.skin, Space::Thermal)
        .expect("an exit");
    schema
        .process::<DryHeatLoss>(sites.skin, Space::Thermal)
        .expect("an exit");
    let body = assemble(schema, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let stores = ports.temperatures(&mut mt);
        zeroed(&mut mt, &stores, class);
        let (evaporative, dry) = (
            ports.boundary::<SkinEvaporativeHeat>()?,
            ports.boundary::<DryHeatLoss>()?,
        );
        for exit in [&evaporative, &dry] {
            let still = mt.kernel(|mut k| {
                let zero = k.zero::<Rate<Energy>>();
                k.finish(zero, mt.every_mode(), class)
            });
            mt.transfer(Transfer::new(&exit.route, still));
        }
        let route = ports.conduction(&mut mt);
        let still = mt.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, mt.every_mode(), class)
        });
        mt.transfer_at(&route, still);
        let published = stores.published();
        let at_exit = match choice {
            Choice::DryExit => published.at_exit(&dry),
            _ => published.at_exit(&evaporative),
        };
        let sensed = mt.algebraic::<Temperature>("exit temperature", Admissible::finite());
        let residual = mt.kernel(|mut k| {
            let (own, read) = (k.algebraic(&sensed), k.read(&at_exit));
            let value = k.subtract(own, read);
            k.finish(value, mt.every_mode(), class)
        });
        mt.residual(&sensed, residual);
        let (out, _) = mt.site_field::<TissueSite, Energy>("store heat", Admissible::finite());
        let value = mt.site_kernel(|mut k| {
            let value = match choice {
                Choice::CoreHeat => k.read(&stores.core_heat),
                _ => k.at_site(&stores.tissue_heat),
            };
            k.finish(value, mt.every_mode(), class)
        });
        mt.define_at(&out, value);
        body.add(mt.finish());
        let (mut cv, _) = body.cardiovascular()?.parts();
        let temperatures: PerfusionTemperatures<'_> = cv.receive();
        let (out, _) = cv.site_field::<Beds<Skin>, Temperature>("end", Admissible::finite());
        let value = cv.site_kernel(|mut k| {
            let value = match choice {
                Choice::CoreEnd => temperatures.core(&mut k),
                _ => temperatures.tissue(&mut k),
            };
            k.finish(value, cv.every_mode(), class)
        });
        cv.define_at(&out, value);
        body.add(cv.finish());
        body.compile()
    });
    body.unwrap_or_else(|refused| panic!("{refused:?}")).model_hash()
}

/// The two exits sit on one store, so their reads bind one row and only the exit's identity in the program tells
/// them apart.
#[test]
fn changing_a_carriage_end_an_exit_or_the_store_a_heat_read_finds_changes_the_model_hash() {
    let base = hashed(Choice::Base);
    assert_eq!(hashed(Choice::Base), base);
    for choice in [Choice::CoreEnd, Choice::DryExit, Choice::CoreHeat] {
        assert_ne!(hashed(choice), base, "{choice:?}");
    }
}
