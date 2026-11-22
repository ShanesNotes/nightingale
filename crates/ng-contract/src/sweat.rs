use crate::boundary::DriveFrame;
use crate::catalog::{PerfusionTemperatures, SkinThermoreceptor};
use crate::evaluate::Inputs;
use crate::scope::TISSUE_TEMPERATURE;
use crate::*;

const MOST: f64 = 3.0;
const WIDTH: f64 = 2.0;
const REFERENCE: f64 = 2.0;
const LATENT: f64 = 5.0;
const SKIN: &str = "ns.skin-reference";
const SWEAT: LawName = LawName {
    owner: "MT",
    local: "sweat rate",
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
enum Nerve {
    Supplied,
    NoSupply,
    Bare,
    NoReceptor,
}

#[derive(Clone, Copy, PartialEq)]
enum Reads {
    Granted,
    Lenient,
    OtherLigand,
    AtTheSite,
    OffSkin,
}

struct Sites {
    root: SiteId,
    skin: SiteId,
    bed: SiteId,
    junction: SiteId,
}

fn map(nerve: Nerve) -> Result<(Anatomy, Sites), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at());
    let skin = map.site::<TissueSite>("skin heat site", at());
    let artery = map.site::<VesselSegment>("artery", at());
    let vein = map.site::<VesselSegment>("vein", at());
    let junction = map.site::<Junction>("skin junction", at());
    let bed = map.site::<CapillaryBed>("skin bed", at());
    let muscle = map.site::<CapillaryBed>("muscle bed", at());
    for site in [skin.id(), artery.id(), vein.id(), junction.id(), muscle.id()] {
        map.contain(root.id(), site);
    }
    map.contain(skin.id(), bed.id());
    map.serves(&bed, Tissue::Skin);
    map.serves(&muscle, Tissue::Muscle);
    for bed in [bed.id(), muscle.id()] {
        map.connect(Network::Vascular, artery.id(), bed);
        map.connect(Network::Vascular, bed, vein.id());
    }
    map.connect(Network::Vascular, vein.id(), artery.id());
    map.landmark(Landmark::SkinThermoreceptor, skin.id(), root.id());
    match nerve {
        Nerve::Supplied | Nerve::NoReceptor => map.supply(&junction, &[bed.id(), muscle.id()]),
        Nerve::NoSupply => map.supply(&junction, &[muscle.id()]),
        Nerve::Bare => {
            map.supply(&junction, &[muscle.id()]);
            map.unsupplied(&[bed.id()]);
        }
    }
    map.unsupplied(&[artery.id(), vein.id(), junction.id()]);
    let sites = Sites {
        root: root.id(),
        skin: skin.id(),
        bed: bed.id(),
        junction: junction.id(),
    };
    Ok((map.finish()?, sites))
}

fn publish<'b>(
    mt: &mut FragmentBuilder<'b, Metabolism>,
    ports: &MetabolismPorts<'b>,
) -> StoreTemperatures<'b> {
    let stores = ports.temperatures(mt);
    let still = mt.kernel(|mut k| {
        let value = k.zero::<Temperature>();
        k.finish(value, mt.every_mode(), class())
    });
    mt.define(&stores.core, still);
    let still = mt.site_kernel(|mut k| {
        let value = k.zero::<Temperature>();
        k.finish(value, mt.every_mode(), class())
    });
    mt.define_at(&stores.other, still);
    let route = ports.conduction(mt);
    let still = mt.site_kernel(|mut k| {
        let zero = k.zero::<Rate<Energy>>();
        k.finish(zero, mt.every_mode(), class())
    });
    mt.transfer_at(&route, still);
    stores.published()
}

fn secrete<'b>(
    mt: &mut FragmentBuilder<'b, Metabolism>,
    effect: &SkinSweatEffect<'b>,
    at_bed: &ReadPort<'b, Metabolism, PerSite<CapillaryBed, Temperature>>,
    exit: &BoundaryRoute<'b, Energy, Metabolism>,
    reads: Reads,
) {
    let class = class();
    let most = mt.parameter::<Rate<Water>>(ParameterKey::new("mt.sweat-most"), Admissible::positive());
    let width =
        mt.parameter::<Deviation<Temperature>>(ParameterKey::new("mt.sweat-width"), Admissible::positive());
    let reference = mt.parameter::<Reference<Temperature>>(ParameterKey::new(SKIN), Admissible::finite());
    let forged = |local: &'static str| {
        ReadPort::<Metabolism, PerSite<Junction, Dimensionless>>::field(LawName {
            owner: Receptor::CARD,
            local,
        })
    };
    let acetylcholine = Species::Acetylcholine.names()[4];
    let sweat = mt.algebraic::<Rate<Water>>("sweat rate", Admissible::finite());
    let row = mt.kernel(|mut k| {
        let own = k.algebraic(&sweat);
        k.finish(own, mt.every_mode(), class)
    });
    if reads == Reads::OffSkin {
        let (out, _) =
            mt.site_field::<CapillaryBed, Dimensionless>("felt at every bed", Admissible::finite());
        let value = mt.site_kernel(|mut k| {
            let felt = k.at_junction(&forged(acetylcholine));
            k.finish(felt, mt.every_mode(), class)
        });
        mt.define_at(&out, value);
    }
    let secreted = mt.site_kernel::<Beds<Skin>, Rate<Water>, _>(|mut k| {
        let felt = match reads {
            Reads::Granted | Reads::OffSkin => effect.at_skin(&mut k),
            Reads::Lenient => k.lenient_at_junction(&forged(acetylcholine)),
            Reads::OtherLigand => k.at_junction(&forged(Species::Norepinephrine.names()[4])),
            Reads::AtTheSite => {
                let here = ReadPort::<Metabolism, PerSite<Beds<Skin>, Dimensionless>>::field(LawName {
                    owner: Receptor::CARD,
                    local: acetylcholine,
                });
                k.at_site(&here)
            }
        };
        let (skin, reference) = (k.at_site_in(at_bed), k.parameter(&reference));
        let (warmth, width) = (k.deviation(skin, reference), k.parameter(&width));
        let ratio = k.divide(warmth, width);
        let drive = k.as_dimension(ratio);
        let most = k.parameter(&most);
        let value = k.scale(most, drive);
        let value = k.scale(value, felt);
        k.finish(value, mt.every_mode(), class)
    });
    mt.only_at(&sweat, row, secreted);
    let latent =
        mt.parameter::<Quotient<Energy, Water>>(ParameterKey::new("mt.latent-heat"), Admissible::positive());
    let evaporated = mt.kernel(|mut k| {
        let (secreted, latent) = (k.algebraic(&sweat), k.parameter(&latent));
        let product = k.multiply(secreted, latent);
        let value = k.as_dimension(product);
        k.finish(value, mt.every_mode(), class)
    });
    mt.transfer(Transfer::new(&exit.route, evaporated));
}

fn lose<'b>(
    mt: &mut FragmentBuilder<'b, Metabolism>,
    at_exit: &ReadPort<'b, Metabolism, Temperature>,
    exit: &BoundaryRoute<'b, Energy, Metabolism>,
) {
    let class = class();
    let conductance = mt.parameter::<Quotient<Rate<Energy>, Temperature>>(
        ParameterKey::new("mt.loss"),
        Admissible::positive(),
    );
    let rate = mt.kernel(|mut k| {
        let (temperature, conductance) = (k.read(at_exit), k.parameter(&conductance));
        let product = k.multiply(conductance, temperature);
        let value = k.as_dimension(product);
        k.finish(value, mt.every_mode(), class)
    });
    mt.transfer(Transfer::new(&exit.route, rate));
}

fn bind(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let class = class();
    let (mut rc, ports) = body.receptor()?.parts();
    let route = ports.binding::<Junction, Acetylcholine>(&mut rc);
    let still = rc.site_kernel(|mut k| {
        let zero = k.zero::<Rate<Acetylcholine>>();
        k.finish(zero, rc.every_mode(), class)
    });
    rc.transfer_at(&route, still);
    let out = ports.effect::<Junction, Acetylcholine>(&mut rc);
    let share =
        rc.site_parameter::<Junction, Dimensionless>(ParameterKey::new("rc.share"), Admissible::finite());
    let effect = rc.site_kernel(|mut k| {
        let value = k.site_parameter(&share);
        k.finish(value, rc.every_mode(), class)
    });
    rc.define_at(&out, effect);
    body.add(rc.finish());
    Ok(())
}

fn compiled(nerve: Nerve, reads: Reads) -> Result<(CompiledBody, Sites), AdmissionError> {
    let (anatomy, sites) = map(nerve)?;
    let mut schema = Schema::new(anatomy)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of sweat and heat");
    }
    for store in [sites.root, sites.skin] {
        schema.pool::<Energy>(store, Space::Thermal)?;
    }
    schema.process::<SkinEvaporativeHeat>(sites.skin, Space::Thermal)?;
    schema.process::<DryHeatLoss>(sites.skin, Space::Thermal)?;
    if nerve != Nerve::NoReceptor {
        schema.carry::<Acetylcholine, Junction>(Space::Cleft);
        schema.carry::<Acetylcholine, Junction>(Space::Bound);
    }
    let body = assemble(schema, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let published = publish(&mut mt, &ports);
        let effect = ports.sweat_effect();
        let evaporative = ports.boundary::<SkinEvaporativeHeat>()?;
        secrete(&mut mt, &effect, &published.at_bed(), &evaporative, reads);
        let dry = ports.boundary::<DryHeatLoss>()?;
        lose(&mut mt, &published.at_exit(&dry), &dry);
        body.add(mt.finish());
        if nerve != Nerve::NoReceptor {
            bind(&mut body)?;
        }
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

fn bench() -> Bench {
    let (body, sites) = compiled(Nerve::Supplied, Reads::Granted).unwrap_or_else(|why| panic!("{why:?}"));
    let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
    let note = "synthetic";
    let key = ParameterKey::new;
    (packet.synthetic(
        key("mt.sweat-most"),
        Binding::Body,
        measure::<Rate<Water>>(MOST),
        note,
    ))
    .expect("once");
    let width = measure::<Deviation<Temperature>>(WIDTH);
    (packet.synthetic(key("mt.sweat-width"), Binding::Body, width, note)).expect("once");
    let reference = measure::<Reference<Temperature>>(REFERENCE);
    (packet.synthetic(key(SKIN), Binding::Body, reference, note)).expect("once");
    (packet.synthetic(
        key("mt.loss"),
        Binding::Body,
        measure::<Quotient<Rate<Energy>, Temperature>>(1.0),
        note,
    ))
    .expect("once");
    let latent = measure::<Quotient<Energy, Water>>(LATENT);
    (packet.synthetic(key("mt.latent-heat"), Binding::Body, latent, note)).expect("once");
    let share = Binding::Site(sites.junction);
    (packet.synthetic(key("rc.share"), share, measure::<Dimensionless>(1.0), note)).expect("once");
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
    fn algebraic(&self, owner: &'static str, local: &'static str, at: Context) -> usize {
        self.row(RowKind::Algebraic(Slot {
            law: LawName { owner, local },
            at,
        }))
    }
    fn set(&mut self, row: usize, value: f64) {
        self.unknowns[row] = value;
    }
    fn values(&self, law: LawName) -> Vec<(Context, f64)> {
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
        (self.body.instances().iter().zip(values))
            .filter(|(instance, _)| programs[instance.program().index()].law == law)
            .map(|(instance, value)| (instance.context, value.expect("a value")))
            .collect()
    }
}

fn refused(defect: CompileDefect) -> Option<AdmissionError> {
    Some(AdmissionError::Compile(defect))
}

#[test]
fn the_sweat_rate_reads_the_effect_at_the_junction_and_the_temperature_of_the_store_the_bed_sits_in() {
    let mut bench = bench();
    let skin = bench.algebraic("MT", TISSUE_TEMPERATURE, Context::Site(bench.sites.skin));
    let junction = Context::Site(bench.sites.junction);
    let effect = bench.algebraic(Receptor::CARD, Species::Acetylcholine.names()[4], junction);
    let sweat = bench.algebraic("MT", "sweat rate", Context::Body);
    for (temperature, felt) in [(5.0, 1.0), (5.0, 0.5), (2.0, 1.0)] {
        bench.set(skin, temperature);
        bench.set(effect, felt);
        let secreted = MOST * ((temperature - REFERENCE) / WIDTH) * felt;
        for row in [0.0, secreted] {
            bench.set(sweat, row);
            let mut found = bench.values(SWEAT);
            found.sort_by_key(|(context, _)| *context);
            // `only_at` holds the body row to the one site's value, so the site's kernel enters the row's
            // residual negated and the two instances sum to zero when the row stands at what the bed secretes.
            let mut twin = vec![(Context::Body, row), (Context::Site(bench.sites.bed), -secreted)];
            twin.sort_by_key(|(context, _)| *context);
            assert_eq!(found, twin, "the bed secretes {secreted}");
            assert_eq!(found.iter().map(|(_, value)| value).sum::<f64>(), row - secreted);
        }
    }
}

#[test]
fn the_sweat_row_is_the_one_skin_beds_and_the_boundary_that_drains_nothing_reads_it() {
    let mut bench = bench();
    let sweat = bench.algebraic("MT", "sweat rate", Context::Body);
    for rate in [0.0, 4.0] {
        bench.set(sweat, rate);
        let lost = bench.values(LawName {
            owner: "MT",
            local: "skin evaporative heat",
        });
        assert_eq!(lost, [(Context::Body, rate * LATENT)]);
    }
    let at_the_bed = (bench.values(SWEAT).iter())
        .filter(|(context, _)| *context != Context::Body)
        .count();
    assert_eq!(at_the_bed, 1, "one skin bed secretes");
}

#[test]
fn the_sweat_grant_refuses_a_bed_no_nerve_supplies_and_a_junction_with_no_receptor() {
    let law = SWEAT;
    let (_, sites) = map(Nerve::Supplied).expect("a map");
    assert_eq!(
        compiled(Nerve::NoSupply, Reads::Granted).map(|_| ()).err(),
        refused(CompileDefect::NoSupply { law, site: sites.bed })
    );
    assert_eq!(
        compiled(Nerve::Bare, Reads::Granted).map(|_| ()).err(),
        refused(CompileDefect::UnsuppliedRead { law, site: sites.bed })
    );
    assert_eq!(
        compiled(Nerve::NoReceptor, Reads::Granted).map(|_| ()).err(),
        refused(CompileDefect::NoReceptor {
            law,
            junction: sites.junction,
        })
    );
}

#[test]
fn the_catalog_refuses_the_lenient_read_another_ligand_a_read_off_the_junction_and_one_off_the_skin() {
    let unwired = |law, local| {
        refused(CompileDefect::UnwiredRead {
            law,
            field: LawName {
                owner: Receptor::CARD,
                local,
            },
        })
    };
    let acetylcholine = Species::Acetylcholine.names()[4];
    assert_eq!(
        compiled(Nerve::Supplied, Reads::Lenient).map(|_| ()).err(),
        unwired(SWEAT, acetylcholine)
    );
    assert_eq!(
        compiled(Nerve::Supplied, Reads::OtherLigand).map(|_| ()).err(),
        unwired(SWEAT, Species::Norepinephrine.names()[4])
    );
    assert_eq!(
        compiled(Nerve::Supplied, Reads::AtTheSite).map(|_| ()).err(),
        unwired(SWEAT, acetylcholine)
    );
    let every_bed = LawName {
        owner: "MT",
        local: "felt at every bed",
    };
    assert_eq!(
        compiled(Nerve::Supplied, Reads::OffSkin).map(|_| ()).err(),
        unwired(every_bed, acetylcholine)
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Declared {
    Alike,
    Conflicting,
}

fn compared(declared: Declared) -> Result<CompiledBody, AdmissionError> {
    let (anatomy, sites) = map(Nerve::Supplied)?;
    let mut schema = Schema::new(anatomy)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of heat alone");
    }
    for store in [sites.root, sites.skin] {
        schema.pool::<Energy>(store, Space::Thermal)?;
    }
    let class = class();
    let key = ParameterKey::new;
    let warm = |declared| match declared {
        Declared::Alike => Admissible::<Reference<Temperature>>::finite(),
        Declared::Conflicting => Admissible::<Reference<Temperature>>::positive(),
    };
    assemble(schema, |mut body| {
        let (mut mt, ports) = body.metabolism(None)?.parts();
        let published = publish(&mut mt, &ports);
        let at_bed = published.at_bed();
        let reference = mt.parameter::<Reference<Temperature>>(key(SKIN), Admissible::finite());
        let (out, _) =
            mt.site_field::<Beds<Skin>, Deviation<Temperature>>("sweat warmth", Admissible::finite());
        let value = mt.site_kernel(|mut k| {
            let (skin, reference) = (k.at_site_in(&at_bed), k.parameter(&reference));
            let value = k.deviation(skin, reference);
            k.finish(value, mt.every_mode(), class)
        });
        mt.define_at(&out, value);
        body.add(mt.finish());
        let (mut cv, _) = body.cardiovascular()?.parts();
        let temperatures: PerfusionTemperatures = cv.receive();
        let reference = cv.parameter::<Reference<Temperature>>(key(SKIN), Admissible::finite());
        let (out, _) =
            cv.site_field::<Beds<Skin>, Deviation<Temperature>>("arteriole warmth", Admissible::finite());
        let value = cv.site_kernel(|mut k| {
            let (skin, reference) = (temperatures.tissue(&mut k), k.parameter(&reference));
            let value = k.deviation(skin, reference);
            k.finish(value, cv.every_mode(), class)
        });
        cv.define_at(&out, value);
        body.add(cv.finish());
        let (mut ns, _) = body.nervous()?.parts();
        let sensor: SkinThermoreceptor = ns.receive();
        let reference = ns.parameter::<Reference<Temperature>>(key(SKIN), warm(declared));
        let command = ns.algebraic::<Deviation<Temperature>>("sudomotor command", Admissible::finite());
        let residual = ns.kernel(|mut k| {
            let own = k.algebraic(&command);
            let (skin, reference) = (sensor.temperature(&mut k), k.parameter(&reference));
            let warmth = k.deviation(skin, reference);
            let value = k.subtract(own, warmth);
            k.finish(value, ns.every_mode(), class)
        });
        ns.residual(&command, residual);
        body.add(ns.finish());
        body.compile()
    })
}

#[test]
fn the_three_skin_reads_bind_one_reference_slot_and_a_card_that_declares_it_otherwise_is_refused() {
    let body = compared(Declared::Alike).unwrap_or_else(|why| panic!("{why:?}"));
    let skin = ParameterKey::new(SKIN);
    let slots: Vec<usize> = (body.parameters().iter().enumerate())
        .filter(|(_, slot)| slot.key() == skin)
        .map(|(at, _)| at)
        .collect();
    assert_eq!(slots.len(), 1, "one slot under one key");
    let slot = &body.parameters()[slots[0]];
    assert_eq!(slot.binding(), Binding::Body);
    assert_eq!(slot.default_binding(), None);
    assert_eq!(slot.admissible(), AdmissibleSet::Finite);
    assert_eq!(
        slot.dimension,
        crate::quantity::dimension::<Reference<Temperature>>()
    );
    let programs = body.programs();
    for (owner, local) in [
        ("MT", "sweat warmth"),
        ("CV", "arteriole warmth"),
        ("NS", "sudomotor command"),
    ] {
        let law = LawName { owner, local };
        let mut bound = (body.instances().iter())
            .filter(|instance| programs[instance.program().index()].law == law)
            .map(|instance| {
                (instance.parameters().iter())
                    .map(|at| at.index())
                    .collect::<Vec<usize>>()
            });
        let first = bound.next().unwrap_or_else(|| panic!("the {local} law"));
        assert!(first.contains(&slots[0]), "{local} binds the one reference");
    }
    assert_eq!(
        compared(Declared::Conflicting).map(|_| ()).err(),
        refused(CompileDefect::ParameterConflict { key: skin })
    );
}
