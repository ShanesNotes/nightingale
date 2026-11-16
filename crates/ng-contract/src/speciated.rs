use crate::boundary::DriveFrame;
use crate::catalog::{SpeciationReader, WiredRead};
use crate::evaluate::Inputs;
use crate::law::ReadPort;
use crate::*;
use core::f64::consts::LN_10;

fn measure<U: Dimension>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

const PRESSURE: [f64; 4] = [12.5, 5.25, 4.75, 6.0];
const ACID: [f64; 4] = [7.375, 7.25, 7.0, 6.875];
const PK: f64 = 7.125;

const SENSORS: [SpeciationReader; 1] = [SpeciationReader {
    reader: "NS",
    sites: Sites::Speciated,
    label: "a synthetic sensor",
    wired: &[
        WiredRead {
            quantity: OxygenPartialPressure::NAME,
            body_ticket: 11,
        },
        WiredRead {
            quantity: CarbonDioxidePartialPressure::NAME,
            body_ticket: 11,
        },
    ],
    held: &[],
}];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Table {
    Sensors,
    Today,
}

#[derive(Clone, Copy, Debug)]
struct Case {
    table: Table,
    landmark: usize,
    oxygen: bool,
    carbon_dioxide: bool,
    metabolism: bool,
    acidity: bool,
    respiratory: bool,
    guard: bool,
    asleep: Option<Reader>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Reader {
    Written,
    ByHand,
}

const SENSED: Case = Case {
    table: Table::Sensors,
    landmark: 0,
    oxygen: true,
    carbon_dioxide: true,
    metabolism: false,
    acidity: false,
    respiratory: false,
    guard: false,
    asleep: None,
};

fn compiled(case: Case) -> Result<(CompiledBody, [SiteId; 4]), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at());
    let artery = map.site::<VesselSegment>("artery", at());
    let first = map.site::<CapillaryBed>("first bed", at());
    let vein = map.site::<VesselSegment>("vein", at());
    let second = map.site::<CapillaryBed>("second bed", at());
    let sites = [artery.id(), first.id(), vein.id(), second.id()];
    for (at, &site) in sites.iter().enumerate() {
        map.contain(root.id(), site);
        map.connect(Network::Vascular, site, sites[(at + 1) % sites.len()]);
    }
    map.unsupplied(&sites);
    map.sample(&artery);
    let mark = Landmark::PeripheralChemoreceptor;
    map.landmark(mark, sites[case.landmark], sites[case.landmark]);
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Interstitium, "a bench with no tissue");
    schema.leave_out(Space::Cells, "a bench with no tissue");
    let body = assemble(schema, |mut body| {
        let class = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        body.add(cv.finish());

        let (mut bc, ports) = body.blood_chemistry(None)?.parts();
        let stated = ports.speciation(&mut bc);
        let key = ParameterKey::new;
        let pressure = bc.site_parameter::<Speciated, Pressure>(key("bc.pressure"), Admissible::positive());
        let share =
            bc.site_parameter::<Speciated, Dimensionless>(key("bc.share"), Admissible::unit_interval());
        let acid = bc.site_parameter::<Speciated, Dimensionless>(key("bc.acid"), Admissible::finite());
        let base = bc.site_parameter::<Speciated, Concentration<CarbonDioxide>>(
            key("bc.base"),
            Admissible::positive(),
        );
        let pk = bc.parameter::<Dimensionless>(key("bc.pk"), Admissible::finite());
        let ln_ten = bc.parameter::<Dimensionless>(key("bc.ln-ten"), Admissible::positive());
        let value = bc.site_kernel(|mut k| {
            let value = k.site_parameter(&pressure);
            k.finish(value, bc.every_mode(), class)
        });
        bc.define_at(&stated.oxygen_partial_pressure.0, value);
        for (output, parameter) in [(&stated.oxygen_saturation.0, &share), (&stated.acidity.0, &acid)] {
            let value = bc.site_kernel(|mut k| {
                let value = k.site_parameter(parameter);
                k.finish(value, bc.every_mode(), class)
            });
            bc.define_at(output, value);
        }
        let value = bc.site_kernel(|mut k| {
            let value = k.site_parameter(&base);
            k.finish(value, bc.every_mode(), class)
        });
        bc.define_at(&stated.bicarbonate.0, value);
        bc.express_at(&stated.carbon_dioxide_partial_pressure.0, |mut k| {
            let (oxygen, acid) = (
                k.at_site(&stated.oxygen_partial_pressure.1),
                k.at_site(&stated.acidity.1),
            );
            let pk = k.parameter(&pk);
            let above = k.subtract(acid, pk);
            let ln_ten = k.parameter(&ln_ten);
            let power = k.scale(above, ln_ten);
            let ratio = k.exp(power);
            let value = k.scale(oxygen, ratio);
            k.finish_term(value)
        });
        let water = ports.blood::<Water>();
        let (volume_out, _) = bc.site_field::<Speciated, Water>("volume", Admissible::finite());
        let volume = bc.site_kernel(|mut k| {
            let value = k.at_site(&water);
            k.finish(value, bc.every_mode(), class)
        });
        bc.define_at(&volume_out, volume);
        if let Some(reader) = case.asleep {
            let (probe, _) = bc.site_field::<Speciated, Pressure>("probe", Admissible::finite());
            let value = bc.site_kernel(|mut k| {
                let pressure = match reader {
                    Reader::Written => k.at_site(&stated.carbon_dioxide_partial_pressure.1),
                    Reader::ByHand => {
                        let (oxygen, acid) = (
                            k.at_site(&stated.oxygen_partial_pressure.1),
                            k.at_site(&stated.acidity.1),
                        );
                        let pk = k.parameter(&pk);
                        let above = k.subtract(acid, pk);
                        let ln_ten = k.parameter(&ln_ten);
                        let power = k.scale(above, ln_ten);
                        let ratio = k.exp(power);
                        k.scale(oxygen, ratio)
                    }
                };
                let sleep = k.asleep();
                let value = k.scale(pressure, sleep);
                k.finish(value, bc.every_mode(), class)
            });
            bc.define_at(&probe, value);
        }
        body.add(bc.finish());

        let (mut ns, _) = body.nervous()?.parts();
        let sensors = [
            (case.oxygen, "sensed oxygen", OxygenPartialPressure::NAME),
            (
                case.carbon_dioxide,
                "sensed carbon dioxide",
                CarbonDioxidePartialPressure::NAME,
            ),
        ];
        for (sensed, name, quantity) in sensors {
            if !sensed {
                continue;
            }
            let read: ReadPort<'_, Nervous, PerSite<Speciated, Pressure>> = ReadPort::field(LawName {
                owner: "BC",
                local: quantity,
            });
            if case.guard {
                let [low, high] = ns.modes(name, ["low", "high"]);
                let zero = ns.kernel(|mut k| {
                    let value = k.at_landmark(mark, &read);
                    k.finish(value, low.only(), class)
                });
                ns.event(Guard::new(zero, Crossing::Rising, low.to(&high)?));
                continue;
            }
            let slot = ns.algebraic::<Pressure>(name, Admissible::finite());
            let residual = ns.kernel(|mut k| {
                let value = k.at_landmark(mark, &read);
                let own = k.algebraic(&slot);
                let residual = k.subtract(own, value);
                k.finish(residual, ns.every_mode(), class)
            });
            ns.residual(&slot, residual);
        }
        body.add(ns.finish());

        if case.metabolism {
            let (mut mt, _) = body.metabolism(None)?.parts();
            let read: ReadPort<'_, Metabolism, PerSite<Speciated, Pressure>> = ReadPort::field(LawName {
                owner: "BC",
                local: OxygenPartialPressure::NAME,
            });
            let (uptake_out, _) = mt.site_field::<Speciated, Pressure>("uptake", Admissible::finite());
            let uptake = mt.site_kernel(|mut k| {
                let value = k.at_site(&read);
                k.finish(value, mt.every_mode(), class)
            });
            mt.define_at(&uptake_out, uptake);
            body.add(mt.finish());
        }
        if case.acidity {
            let (mut mt, _) = body.metabolism(None)?.parts();
            let read: ReadPort<'_, Metabolism, PerSite<CapillaryBed, Dimensionless>> =
                ReadPort::field(LawName {
                    owner: "BC",
                    local: Acidity::NAME,
                });
            let (acid_out, _) = mt.site_field::<CapillaryBed, Dimensionless>("acid", Admissible::finite());
            let acid = mt.site_kernel(|mut k| {
                let value = k.at_site(&read);
                k.finish(value, mt.every_mode(), class)
            });
            mt.define_at(&acid_out, acid);
            body.add(mt.finish());
        }
        if case.respiratory {
            let (mut rs, _) = body.respiratory(None)?.parts();
            let read: ReadPort<'_, Respiratory, PerSite<CapillaryBed, Pressure>> = ReadPort::field(LawName {
                owner: "BC",
                local: OxygenPartialPressure::NAME,
            });
            let (alveolar_out, _) = rs.site_field::<CapillaryBed, Pressure>("alveolar", Admissible::finite());
            let alveolar = rs.site_kernel(|mut k| {
                let value = k.at_site(&read);
                k.finish(value, rs.every_mode(), class)
            });
            rs.define_at(&alveolar_out, alveolar);
            body.add(rs.finish());
        }
        match case.table {
            Table::Sensors => body.compile_reading(&SENSORS),
            Table::Today => body.compile(),
        }
    })?;
    Ok((body, sites))
}

fn refused(case: Case) -> CompileDefect {
    match compiled(case) {
        Err(AdmissionError::Compile(defect)) => defect,
        other => panic!("{case:?}: {:?}", other.map(|_| ())),
    }
}

fn values(body: &CompiledBody, sites: [SiteId; 4]) -> Vec<(LawName, Context, f64)> {
    let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
    let note = "synthetic";
    for (index, site) in [(0, sites[0]), (1, sites[1]), (3, sites[3])] {
        let binding = Binding::Site(site);
        let pressure = measure::<Pressure>(PRESSURE[index]);
        packet
            .synthetic(ParameterKey::new("bc.pressure"), binding, pressure, note)
            .expect("once");
        let acid = measure::<Dimensionless>(ACID[index]);
        packet
            .synthetic(ParameterKey::new("bc.acid"), binding, acid, note)
            .expect("once");
        let share = measure::<Dimensionless>(0.5);
        packet
            .synthetic(ParameterKey::new("bc.share"), binding, share, note)
            .expect("once");
        let base = measure::<Concentration<CarbonDioxide>>(24.0);
        packet
            .synthetic(ParameterKey::new("bc.base"), binding, base, note)
            .expect("once");
    }
    for (key, value) in [("bc.pk", PK), ("bc.ln-ten", LN_10)] {
        let value = measure::<Dimensionless>(value);
        packet
            .synthetic(ParameterKey::new(key), Binding::Body, value, note)
            .expect("once");
    }
    let index = |site: SiteId| sites.iter().position(|known| *known == site).expect("a site");
    let start = |row: RowKind| match row {
        RowKind::Amount { site, .. } => 10.0 + 7.0 * index(site) as f64,
        RowKind::Algebraic(Slot {
            law,
            at: Context::Site(site),
        }) => match law.local {
            "oxygen partial pressure" => PRESSURE[index(site)],
            "acidity" => ACID[index(site)],
            "bicarbonate" => 24.0,
            "oxygen saturation" => 0.5,
            _ => 0.0,
        },
        _ => 0.0,
    };
    packet.start_rest(body, start, note).expect("once");
    let bound = body.bind(packet).expect("bound");
    let unknowns: Vec<f64> = (body.rows().iter())
        .map(|row| bound.starts[row.id().index()])
        .collect();
    let frame = DriveFrame::default();
    let inputs = Inputs {
        parameters: &bound.values,
        frame: &frame,
        unknowns: &unknowns,
        modes: &bound.modes,
        draws: &[],
        factors: &[],
    };
    let values = body.instance_values(&inputs).expect("finite");
    (body.layout.instances.iter().zip(values))
        .filter_map(|(instance, value)| {
            let law = body.layout.programs[instance.program.index()].law;
            Some((law, instance.context, value?))
        })
        .collect()
}

fn rows_at(body: &CompiledBody, site: SiteId) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = (body.rows().iter())
        .filter_map(|row| match row.kind() {
            RowKind::Algebraic(slot) if slot.law.owner == "BC" && slot.at == Context::Site(site) => {
                Some(slot.law.local)
            }
            _ => None,
        })
        .collect();
    found.sort_unstable();
    found
}

#[test]
fn a_sampled_segment_is_speciated_and_its_sensor_reads_it_at_the_landmark() {
    let (body, sites) = compiled(SENSED).unwrap_or_else(|refused| panic!("{refused:?}"));
    let stated = [
        "acidity",
        "bicarbonate",
        "oxygen partial pressure",
        "oxygen saturation",
        "volume",
    ];
    for site in [sites[0], sites[1], sites[3]] {
        assert_eq!(rows_at(&body, site), stated, "{site:?}");
    }
    assert!(
        rows_at(&body, sites[2]).is_empty(),
        "an unsampled segment is no speciated site"
    );
    let found = values(&body, sites);
    let value = |local: &str, at: Context| {
        let found = found.iter().find(|found| found.0.local == local && found.1 == at);
        found.expect("an instance").2
    };
    for (index, &site) in sites.iter().enumerate().filter(|&(index, _)| index != 2) {
        let pool = 10.0 + 7.0 * index as f64;
        assert_eq!(
            value("volume", Context::Site(site)),
            -pool,
            "the blood's water at {site:?}"
        );
    }
    let carbon_dioxide = PRESSURE[0] * crate::pinned::exp((ACID[0] - PK) * LN_10);
    assert_ne!(carbon_dioxide, PRESSURE[0]);
    assert_eq!(value("sensed oxygen", Context::Body), -PRESSURE[0]);
    assert_eq!(
        value("sensed carbon dioxide", Context::Body).to_bits(),
        (-carbon_dioxide).to_bits()
    );
    let sensor = (body.programs().iter()).find(|program| program.law.local == "sensed carbon dioxide");
    let origins = &sensor.expect("a program").code.origins;
    assert!(!origins.is_empty());
    let field = LawName {
        owner: "BC",
        local: CarbonDioxidePartialPressure::NAME,
    };
    assert!(origins.iter().all(|&(_, law)| law == field));
}

#[test]
fn a_read_todays_table_does_not_wire_is_refused() {
    let law = LawName {
        owner: "NS",
        local: "sensed oxygen",
    };
    let field = LawName {
        owner: "BC",
        local: OxygenPartialPressure::NAME,
    };
    let refused = refused(Case {
        table: Table::Today,
        carbon_dioxide: false,
        ..SENSED
    });
    assert_eq!(refused, CompileDefect::UnwiredRead { law, field });
}

#[test]
fn a_wired_reader_off_its_rows_set_is_refused() {
    let law = LawName {
        owner: "MT",
        local: "uptake",
    };
    let field = LawName {
        owner: "BC",
        local: OxygenPartialPressure::NAME,
    };
    let refused = refused(Case {
        table: Table::Today,
        oxygen: false,
        carbon_dioxide: false,
        metabolism: true,
        ..SENSED
    });
    assert_eq!(refused, CompileDefect::UnwiredRead { law, field });
}

#[test]
fn a_sampled_segment_no_other_card_reads_is_refused() {
    let (_, sites) = compiled(SENSED).unwrap_or_else(|refused| panic!("{refused:?}"));
    let refused = refused(Case {
        oxygen: false,
        carbon_dioxide: false,
        ..SENSED
    });
    assert_eq!(refused, CompileDefect::UnreadSample { site: sites[0] });
}

#[test]
fn a_sensor_at_a_segment_the_map_does_not_sample_is_refused() {
    let law = |local| LawName { owner: "NS", local };
    let row = refused(Case {
        landmark: 2,
        carbon_dioxide: false,
        ..SENSED
    });
    assert_eq!(
        row,
        CompileDefect::NoSuchRead {
            law: law("sensed oxygen")
        }
    );
    let field = LawName {
        owner: "BC",
        local: CarbonDioxidePartialPressure::NAME,
    };
    let written = refused(Case {
        landmark: 2,
        oxygen: false,
        ..SENSED
    });
    assert_eq!(
        written,
        CompileDefect::UnemittedField {
            law: law("sensed carbon dioxide"),
            field
        }
    );
}

fn guard_reads(body: &CompiledBody, law: LawName, site: SiteId) -> bool {
    let row = RowKind::Algebraic(Slot {
        law,
        at: Context::Site(site),
    });
    let inputs = body.layout.guards.iter().flat_map(|guard| &guard.inputs);
    inputs.into_iter().any(|input| {
        matches!(input, crate::body::Bound::Unknown(unknown) if body.rows()[unknown.index()].kind() == row)
    })
}

#[test]
fn a_sampled_segment_read_only_by_a_wired_guard_is_not_unread() {
    let case = Case {
        guard: true,
        carbon_dioxide: false,
        ..SENSED
    };
    let (body, sites) = compiled(case).unwrap_or_else(|refused| panic!("{refused:?}"));
    let law = LawName {
        owner: "BC",
        local: OxygenPartialPressure::NAME,
    };
    assert!(
        guard_reads(&body, law, sites[0]),
        "the guard reads the artery's oxygen row"
    );
}

#[test]
fn a_guard_reads_an_explicit_field_at_a_landmark() {
    let case = Case {
        guard: true,
        oxygen: false,
        ..SENSED
    };
    let (body, sites) = compiled(case).unwrap_or_else(|refused| panic!("{refused:?}"));
    let field = LawName {
        owner: "BC",
        local: CarbonDioxidePartialPressure::NAME,
    };
    let [guard] = &body.layout.guards[..] else {
        panic!("one guard")
    };
    assert!(!guard.code.origins.is_empty());
    assert!(guard.code.origins.iter().all(|&(_, law)| law == field));
    let row = |local| LawName { owner: "BC", local };
    for local in [OxygenPartialPressure::NAME, Acidity::NAME] {
        assert!(
            guard_reads(&body, row(local), sites[0]),
            "the field's own reads land at the artery"
        );
    }
}

#[test]
fn a_day_input_scaled_inside_an_explicit_field_is_judged_as_by_hand() {
    for reader in [Reader::ByHand, Reader::Written] {
        let case = Case {
            asleep: Some(reader),
            ..SENSED
        };
        let (body, _) = compiled(case).unwrap_or_else(|refused| panic!("{reader:?}: {refused:?}"));
        let probe = (body.programs().iter()).find(|program| program.law.local == "probe");
        assert!(probe.is_some(), "{reader:?}");
    }
}

#[test]
fn a_sample_read_only_at_another_site_is_unread() {
    let (_, sites) = compiled(SENSED).unwrap_or_else(|refused| panic!("{refused:?}"));
    let refused = refused(Case {
        landmark: 1,
        ..SENSED
    });
    assert_eq!(refused, CompileDefect::UnreadSample { site: sites[0] });
}

#[test]
fn a_wired_reader_of_a_quantity_its_row_holds_is_refused() {
    let law = LawName {
        owner: "MT",
        local: "acid",
    };
    let field = LawName {
        owner: "BC",
        local: Acidity::NAME,
    };
    let refused = refused(Case {
        table: Table::Today,
        oxygen: false,
        carbon_dioxide: false,
        acidity: true,
        ..SENSED
    });
    assert_eq!(refused, CompileDefect::UnwiredRead { law, field });
}

#[test]
fn a_card_with_no_wired_row_is_refused_where_another_card_is_wired() {
    let law = LawName {
        owner: "RS",
        local: "alveolar",
    };
    let field = LawName {
        owner: "BC",
        local: OxygenPartialPressure::NAME,
    };
    let refused = refused(Case {
        table: Table::Today,
        oxygen: false,
        carbon_dioxide: false,
        respiratory: true,
        ..SENSED
    });
    assert_eq!(refused, CompileDefect::UnwiredRead { law, field });
}

#[test]
fn a_guards_read_is_held_to_the_table() {
    let law = LawName {
        owner: "NS",
        local: "sensed oxygen",
    };
    let field = LawName {
        owner: "BC",
        local: OxygenPartialPressure::NAME,
    };
    let refused = refused(Case {
        table: Table::Today,
        guard: true,
        carbon_dioxide: false,
        ..SENSED
    });
    assert_eq!(refused, CompileDefect::UnwiredRead { law, field });
}
