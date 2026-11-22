//! Publication: MT defines each tissue heat store's temperature from that store's own heat and capacity, and CV at
//! the carriage's tissue end, NS at the skin thermoreceptor and MT at its skin exit read the one skin row.
use crate::boundary::DriveFrame;
use crate::catalog::{PerfusionTemperatures, SkinThermoreceptor};
use crate::evaluate::Inputs;
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

/// MT's temperature law: `T_0 + H / C`, with one body capacity for the core store and one site capacity for the rest.
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

fn read_at_the_carriage<'b>(cv: &mut FragmentBuilder<'b, Cardiovascular>, reads: Reads, class: RateClass) {
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
        Reads::Granted => {}
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
    }
}

fn sense<'b>(ns: &mut FragmentBuilder<'b, Nervous>, class: RateClass) {
    let sensor: SkinThermoreceptor<'b> = ns.receive();
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
        let (mut cv, _) = body.cardiovascular()?.parts();
        read_at_the_carriage(&mut cv, reads, class);
        body.add(cv.finish());
        let (mut ns, _) = body.nervous()?.parts();
        sense(&mut ns, class);
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

/// The granted assembly bound with synthetic values: the core, skin and airway stores' heats, and the airway's
/// capacity apart from the skin's.
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
    /// The store's temperature: its defining law's value, with every algebraic unknown at zero.
    fn temperature(&self, store: SiteId) -> f64 {
        let frame = DriveFrame::default();
        let values = self.body.instance_values(&self.inputs(&frame)).expect("finite");
        let row = RowId::at(self.temperature_row(store));
        let defining = (self.body.layout.instances.iter().zip(values))
            .find(|(instance, _)| instance.target == Target::Row(row))
            .and_then(|(_, value)| value);
        -defining.expect("one law defines the row")
    }
    /// The slope of a temperature row's residual in a store's heat, zero where the row does not read it.
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

/// The map with no heat pools, for laws that move none.
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

#[derive(Clone, Copy, PartialEq)]
enum Lookup {
    BodyFieldOfTheName,
    AlgebraicOfTheName,
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
        }
        let name = named("MT", TISSUE_TEMPERATURE);
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
