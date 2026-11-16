use crate::boundary::DriveFrame;
use crate::evaluate::Inputs;
use crate::law::{Op, ReadPort};
use crate::*;
use core::f64::consts::LN_10;

fn measure<U: Dimension>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Reader {
    Written,
    ByHand,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Host {
    Plain,
    Twice,
    Cancel,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Wrong {
    Nothing,
    ReadsAnother,
    ByMode,
    OwnSlot,
    Term,
    Junction,
    Drain,
    EdgeEnd,
    OffItsSet,
    SiteParameter,
    Ranged,
}

#[derive(Clone, Copy, Debug)]
struct Case {
    reader: Reader,
    host: Host,
    second: bool,
    landmark: Option<usize>,
    wrong: Wrong,
}

const PLAIN: Case = Case {
    reader: Reader::Written,
    host: Host::Plain,
    second: false,
    landmark: None,
    wrong: Wrong::Nothing,
};

const LEVEL: [f64; 2] = [1.5, 0.375];
const ACID: [f64; 2] = [7.25, 6.5];
const PK: f64 = 6.875;
const SOL: f64 = 0.03;
const D: f64 = 0.5;
const CANCEL: f64 = 18_014_398_509_481_984.0;
const FIELD: &str = "carbon dioxide partial pressure";

struct Henry<'b> {
    pk: Parameter<'b, BloodChemistry, Dimensionless>,
    ln_ten: Parameter<'b, BloodChemistry, Dimensionless>,
    sol: Parameter<'b, BloodChemistry, Dimensionless>,
}

fn pressure<'b, 'k, S>(
    k: &mut Kernel<'b, 'k, BloodChemistry, S>,
    henry: &Henry<'b>,
    level: Expr<'b, 'k, BloodChemistry, Dimensionless>,
    acid: Expr<'b, 'k, BloodChemistry, Dimensionless>,
) -> Expr<'b, 'k, BloodChemistry, Dimensionless> {
    let (pk, ln_ten) = (k.parameter(&henry.pk), k.parameter(&henry.ln_ten));
    let above = k.subtract(acid, pk);
    let power = k.scale(above, ln_ten);
    let ratio = k.exp(power);
    let quotient = k.divide(level, ratio);
    let quotient: Expr<'b, 'k, BloodChemistry, Dimensionless> = k.as_dimension(quotient);
    let sol = k.parameter(&henry.sol);
    let value = k.divide(quotient, sol);
    k.as_dimension(value)
}

fn bc(local: &'static str) -> LawName {
    LawName { owner: "BC", local }
}

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
    if let Some(site) = case.landmark {
        map.landmark(Landmark::PeripheralChemoreceptor, sites[site], sites[site]);
    }
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    schema.leave_out(Space::Interstitium, "a bench with no tissue");
    schema.leave_out(Space::Cells, "a bench with no tissue");
    let body = assemble(schema, |mut body| {
        let class = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let field = bc(FIELD);
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            if case.wrong == Wrong::EdgeEnd {
                k.upstream::<Dimensionless>(&ReadPort::field(field));
            }
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        body.add(cv.finish());

        let (mut bc, _) = body.blood_chemistry(None)?.parts();
        let key = ParameterKey::new;
        let level_at =
            bc.site_parameter::<CapillaryBed, Dimensionless>(key("bc.level"), Admissible::finite());
        let acid_at = bc.site_parameter::<CapillaryBed, Dimensionless>(key("bc.acid"), Admissible::finite());
        let henry = Henry {
            pk: bc.parameter(key("bc.pk"), Admissible::finite()),
            ln_ten: bc.parameter(key("bc.ln-ten"), Admissible::finite()),
            sol: bc.parameter(key("bc.sol"), Admissible::finite()),
        };
        let d = bc.parameter::<Dimensionless>(key("bc.d"), Admissible::finite());
        let cancel = bc.parameter::<Dimensionless>(key("bc.cancel"), Admissible::finite());
        let (level_out, level) = bc.site_field::<CapillaryBed, Dimensionless>("level", Admissible::finite());
        let (acid_out, acid) = bc.site_field::<CapillaryBed, Dimensionless>("acid", Admissible::finite());
        for (out, parameter) in [(&level_out, &level_at), (&acid_out, &acid_at)] {
            let kernel = bc.site_kernel(|mut k| {
                let value = k.site_parameter(parameter);
                k.finish(value, bc.every_mode(), class)
            });
            bc.define_at(out, kernel);
        }
        let gain = (case.wrong == Wrong::OwnSlot).then(|| {
            let gain = bc.algebraic::<Dimensionless>("gain", Admissible::finite());
            let residual = bc.kernel(|mut k| {
                let (slot, value) = (k.algebraic(&gain), k.parameter(&d));
                let residual = k.subtract(slot, value);
                k.finish(residual, bc.every_mode(), class)
            });
            bc.residual(&gain, residual);
            gain
        });
        let term = (case.wrong == Wrong::Term).then(|| {
            bc.site_term::<CapillaryBed, Dimensionless, _>("acid term", |mut k| {
                let value = k.at_site(&acid);
                k.finish_term(value)
            })
        });
        let modes = (case.wrong == Wrong::ByMode).then(|| bc.modes("valve", ["open", "shut"]));
        let (explicit, port) = bc.explicit_field::<CapillaryBed, Dimensionless>(FIELD);
        let expression = bc.site_kernel(|mut k| {
            let (level, acid) = (k.at_site(&level), k.at_site(&acid));
            let value = pressure(&mut k, &henry, level, acid);
            let value = match (&gain, &term) {
                (Some(gain), _) => {
                    let gain = k.algebraic(gain);
                    k.scale(value, gain)
                }
                (_, Some(term)) => k.add_from(value, term),
                _ if case.wrong == Wrong::SiteParameter => {
                    let acid = k.site_parameter(&acid_at);
                    k.add(value, acid)
                }
                _ => value,
            };
            match &modes {
                Some([open, _]) => k.finish(value, open.only(), class),
                None => k.finish(value, bc.every_mode(), class),
            }
        });
        if let Some([_, shut]) = &modes {
            let other = bc.site_kernel(|mut k| {
                let value = k.at_site(&level);
                k.finish(value, shut.only(), class)
            });
            bc.express_at(&explicit, other);
        }
        if case.wrong == Wrong::Ranged {
            let basis = Basis::Fixture("below either bed's value");
            let range = ValidityRange::new(Measure::new(-1.0)?, Measure::new(1.0)?, basis)?;
            bc.validity(&expression, range);
        }
        bc.express_at(&explicit, expression);
        if case.wrong == Wrong::ReadsAnother {
            let (twice, _) = bc.explicit_field::<CapillaryBed, Dimensionless>("twice");
            let doubled = bc.site_kernel(|mut k| {
                let value = k.at_site(&port);
                let value = k.add(value, value);
                k.finish(value, bc.every_mode(), class)
            });
            bc.express_at(&twice, doubled);
        }
        let (probe_out, _) = bc.site_field::<CapillaryBed, Dimensionless>("probe", Admissible::finite());
        let probe = bc.site_kernel(|mut k| {
            let read = match (case.reader, case.wrong) {
                (_, Wrong::Junction) => k.at_junction(&ReadPort::field(field)),
                (_, Wrong::Drain) => k.drained(&ReadPort::field(field)),
                (Reader::Written, _) => k.at_site(&port),
                (Reader::ByHand, _) => {
                    let (level, acid) = (k.at_site(&level), k.at_site(&acid));
                    pressure(&mut k, &henry, level, acid)
                }
            };
            let read = match case.host {
                Host::Plain => read,
                Host::Twice => {
                    let own = k.at_site(&acid);
                    let square = k.scale(own, own);
                    k.add(read, square)
                }
                Host::Cancel => {
                    let (first, second, third) = (k.at_site(&acid), k.at_site(&acid), k.at_site(&acid));
                    let gain = k.parameter(&cancel);
                    let zero = k.zero();
                    let negative = k.subtract(zero, gain);
                    let first = k.scale(first, gain);
                    let second = k.scale(second, negative);
                    let pair = k.add(first, second);
                    let base = k.add(pair, third);
                    k.add(base, read)
                }
            };
            let d = k.parameter(&d);
            let value = k.divide(read, d);
            let value = k.as_dimension(value);
            k.finish(value, bc.every_mode(), class)
        });
        bc.define_at(&probe_out, probe);
        if case.landmark.is_some() {
            let sensed = bc.algebraic::<Dimensionless>("sensed", Admissible::finite());
            let residual = bc.kernel(|mut k| {
                let mark = Landmark::PeripheralChemoreceptor;
                let read = match case.reader {
                    Reader::Written => k.at_landmark(mark, &port),
                    Reader::ByHand => {
                        let (level, acid) = (k.at_landmark(mark, &level), k.at_landmark(mark, &acid));
                        pressure(&mut k, &henry, level, acid)
                    }
                };
                let slot = k.algebraic(&sensed);
                let residual = k.subtract(slot, read);
                k.finish(residual, bc.every_mode(), class)
            });
            bc.residual(&sensed, residual);
        }
        if case.wrong == Wrong::OffItsSet {
            let (spread_out, _) = bc.site_field::<Blood, Dimensionless>("spread", Admissible::finite());
            let spread = bc.site_kernel(|mut k| {
                let value = k.at_site(&ReadPort::field(field));
                k.finish(value, bc.every_mode(), class)
            });
            bc.define_at(&spread_out, spread);
        }
        body.add(bc.finish());

        if case.second {
            let (mut mt, _) = body.metabolism(None)?.parts();
            let (uptake_out, _) =
                mt.site_field::<CapillaryBed, Dimensionless>("uptake", Admissible::finite());
            let uptake = mt.site_kernel(|mut k| {
                let value = k.at_site(&ReadPort::field(field));
                k.finish(value, mt.every_mode(), class)
            });
            mt.define_at(&uptake_out, uptake);
            body.add(mt.finish());
        }
        body.compile()
    })?;
    Ok((body, sites))
}

fn bench(case: Case) -> (CompiledBody, [SiteId; 4]) {
    compiled(case).unwrap_or_else(|refused| panic!("{case:?}: {refused:?}"))
}

fn refused(case: Case) -> CompileDefect {
    match compiled(case) {
        Err(AdmissionError::Compile(defect)) => defect,
        other => panic!("{case:?}: {:?}", other.map(|_| ())),
    }
}

fn bound(body: &CompiledBody, sites: [SiteId; 4], overrides: &[(&'static str, f64)]) -> BoundParameters {
    let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
    let note = "synthetic";
    let set = |packet: &mut ParameterPacket<SyntheticFixture>, key: &'static str, binding, value: f64| {
        let value = measure::<Dimensionless>(value);
        packet
            .synthetic(ParameterKey::new(key), binding, value, note)
            .expect("once");
    };
    for (bed, site) in [sites[1], sites[3]].into_iter().enumerate() {
        set(&mut packet, "bc.level", Binding::Site(site), LEVEL[bed]);
        set(&mut packet, "bc.acid", Binding::Site(site), ACID[bed]);
    }
    let body_values = [
        ("bc.pk", PK),
        ("bc.ln-ten", LN_10),
        ("bc.sol", SOL),
        ("bc.d", D),
        ("bc.cancel", CANCEL),
    ];
    let read =
        |key: &'static str| (body.parameters().iter()).any(|slot| slot.key() == ParameterKey::new(key));
    for (key, value) in body_values.into_iter().filter(|&(key, _)| read(key)) {
        set(&mut packet, key, Binding::Body, value);
    }
    let pool =
        |site: SiteId| 10.0 + 7.0 * sites.iter().position(|known| *known == site).expect("a site") as f64;
    let bed = |at: Context| {
        [sites[1], sites[3]]
            .iter()
            .position(|site| at == Context::Site(*site))
    };
    let start = |row: RowKind| match row {
        RowKind::Amount { site, .. } => pool(site),
        RowKind::Algebraic(slot) => match (slot.law.local, bed(slot.at)) {
            ("level", Some(bed)) => LEVEL[bed],
            ("acid", Some(bed)) => ACID[bed],
            _ => 0.0,
        },
        _ => 0.0,
    };
    packet.start_rest(body, start, note).expect("once");
    let mut bound = body.bind(packet).expect("bound");
    for (slot, parameter) in body.parameters().iter().enumerate() {
        for &(key, value) in overrides {
            if parameter.key() == ParameterKey::new(key) {
                bound.values[slot] = value;
            }
        }
    }
    bound
}

fn inputs<'a>(bound: &'a BoundParameters, frame: &'a DriveFrame, unknowns: &'a [f64]) -> Inputs<'a> {
    Inputs {
        parameters: &bound.values,
        frame,
        unknowns,
        modes: &bound.modes,
        draws: &[],
        factors: &[],
    }
}

fn starts(body: &CompiledBody, bound: &BoundParameters) -> Vec<f64> {
    (body.rows().iter())
        .map(|row| bound.starts[row.id().index()])
        .collect()
}

fn values(body: &CompiledBody, sites: [SiteId; 4]) -> Vec<(LawName, Context, u64)> {
    let bound = bound(body, sites, &[]);
    let frame = DriveFrame::default();
    let unknowns = starts(body, &bound);
    let values = body
        .instance_values(&inputs(&bound, &frame, &unknowns))
        .expect("finite");
    (body.layout.instances.iter().zip(values))
        .filter_map(|(instance, value)| {
            let law = body.layout.programs[instance.program.index()].law;
            Some((law, instance.context, value?.to_bits()))
        })
        .collect()
}

fn value(found: &[(LawName, Context, u64)], law: LawName, at: Context) -> f64 {
    let found = found.iter().find(|found| found.0 == law && found.1 == at);
    f64::from_bits(found.expect("an instance").2)
}

fn refusal(body: &CompiledBody, sites: [SiteId; 4], overrides: &[(&'static str, f64)]) -> Option<LawName> {
    let bound = bound(body, sites, overrides);
    let frame = DriveFrame::default();
    let unknowns = starts(body, &bound);
    let refused = body.instance_values(&inputs(&bound, &frame, &unknowns)).err()?;
    Some(refused.law)
}

fn slopes(body: &CompiledBody, sites: [SiteId; 4]) -> Vec<String> {
    let bound = bound(body, sites, &[]);
    let frame = DriveFrame::default();
    let unknowns = starts(body, &bound);
    let inputs = inputs(&bound, &frame, &unknowns);
    let jacobian = body.jacobian(&inputs, 0.0).expect("finite");
    let (sparsity, rows) = (body.sparsity(), body.rows());
    let mut out = Vec::new();
    for (at, row) in rows.iter().enumerate() {
        for entry in sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1] {
            let column = rows[sparsity.unknowns()[entry].index()].kind();
            out.push(format!(
                "{:?} d/{column:?} {:016x}",
                row.kind(),
                jacobian[entry].to_bits()
            ));
        }
        for slot in 0..bound.values.len() {
            let key = body.parameters()[slot].key();
            let slopes = body.parameter_slopes(&inputs, &[crate::body::ParameterId::at(slot)]);
            let slope = slopes.expect("finite")[at];
            out.push(format!("{:?} d/{key:?} {:016x}", row.kind(), slope.to_bits()));
        }
    }
    out
}

fn program_of(body: &CompiledBody, law: LawName) -> &crate::body::Program {
    (body.programs().iter())
        .find(|program| program.law == law)
        .expect("a program")
}

fn closed_form(bed: usize) -> f64 {
    let ratio = crate::pinned::exp((ACID[bed] - PK) * LN_10);
    LEVEL[bed] / ratio / SOL
}

#[test]
fn a_written_field_is_its_hand_written_twin_in_ops_values_and_slopes_on_three_hosts() {
    for host in [Host::Plain, Host::Twice, Host::Cancel] {
        let case = Case {
            host,
            landmark: Some(3),
            ..PLAIN
        };
        let (written, sites) = bench(case);
        let (by_hand, _) = bench(Case {
            reader: Reader::ByHand,
            ..case
        });
        for law in [bc("probe"), bc("sensed")] {
            let ops = |body| &program_of(body, law).code.ops;
            assert_eq!(ops(&written), ops(&by_hand), "{host:?} {law:?}");
        }
        assert_eq!(values(&written, sites), values(&by_hand, sites), "{host:?}");
        assert_eq!(slopes(&written, sites), slopes(&by_hand, sites), "{host:?}");
    }
}

#[test]
fn each_bed_reads_its_own_value_of_the_field() {
    let (body, sites) = bench(Case {
        landmark: Some(3),
        ..PLAIN
    });
    let found = values(&body, sites);
    assert_ne!(closed_form(0).to_bits(), closed_form(1).to_bits());
    for (bed, site) in [(0, sites[1]), (1, sites[3])] {
        let probe = value(&found, bc("probe"), Context::Site(site));
        assert_eq!(probe.to_bits(), (-(closed_form(bed) / D)).to_bits(), "bed {bed}");
    }
}

#[test]
fn a_read_at_a_landmark_reads_the_field_where_the_landmark_stands() {
    let (body, sites) = bench(Case {
        landmark: Some(3),
        ..PLAIN
    });
    let sensed = value(&values(&body, sites), bc("sensed"), Context::Body);
    assert_eq!(sensed.to_bits(), (-closed_form(1)).to_bits());
}

#[test]
fn a_second_reader_adds_no_row_and_reads_each_beds_own_value() {
    let (one, sites) = bench(PLAIN);
    let (two, _) = bench(Case {
        second: true,
        ..PLAIN
    });
    let named = |body: &CompiledBody, local| {
        (body.rows().iter())
            .filter(|row| matches!(row.kind(), RowKind::Algebraic(slot) if slot.law.local == local))
            .count()
    };
    assert_eq!(named(&one, FIELD) + named(&two, FIELD), 0, "the field has no row");
    assert_eq!(named(&two, "uptake"), 2);
    assert_eq!(two.rows().len(), one.rows().len() + 2);
    let found = values(&two, sites);
    for (bed, site) in [(0, sites[1]), (1, sites[3])] {
        let uptake = value(
            &found,
            LawName {
                owner: "MT",
                local: "uptake",
            },
            Context::Site(site),
        );
        assert_eq!(uptake.to_bits(), (-closed_form(bed)).to_bits(), "bed {bed}");
    }
}

#[test]
fn a_failure_in_the_field_names_the_field_and_one_in_the_reader_names_the_reader() {
    let case = Case {
        second: true,
        landmark: Some(3),
        ..PLAIN
    };
    let (body, sites) = bench(case);
    let (resumed, _) = bench(case);
    assert_eq!(body.model_hash(), resumed.model_hash());
    for (key, value, law) in [
        ("bc.sol", 0.0, bc(FIELD)),
        ("bc.ln-ten", -1.0e4, bc(FIELD)),
        ("bc.d", 0.0, bc("probe")),
    ] {
        let first = refusal(&body, sites, &[(key, value)]);
        assert_eq!(first, Some(law), "{key}");
        assert_eq!(refusal(&body, sites, &[(key, value)]), first, "{key}, retried");
        assert_eq!(
            refusal(&resumed, sites, &[(key, value)]),
            first,
            "{key}, in a fresh core"
        );
    }
    let sources = body.refusal_sources();
    assert!(sources.contains(&RefusalSource::Divisor { law: bc(FIELD) }));
    let uptake = program_of(
        &body,
        LawName {
            owner: "MT",
            local: "uptake",
        },
    );
    assert!(!uptake.code.origins.is_empty());
    assert!(uptake.code.origins.iter().all(|&(_, law)| law == bc(FIELD)));
}

#[test]
fn the_closing_row_read_stays_last() {
    let (body, _) = bench(PLAIN);
    let ops = &program_of(&body, bc("probe")).code.ops;
    let [.., row, close] = &ops[..] else {
        panic!("a short program")
    };
    assert_eq!(*row, Op::Input(crate::law::Input::Algebraic("probe")));
    assert_eq!(*close, Op::Subtract([ops.len() - 2, ops.len() - 3]));
    assert_eq!(
        ops.iter().filter(|op| *op == row).count(),
        1,
        "the row is read once, last"
    );
}

#[test]
fn a_program_that_reads_no_field_keeps_its_bytes() {
    let (body, _) = bench(PLAIN);
    for local in ["level", "acid"] {
        assert!(program_of(&body, bc(local)).code.origins.is_empty(), "{local}");
    }
}

#[test]
fn a_field_that_reads_another_row_free_field_is_refused() {
    let refused = refused(Case {
        wrong: Wrong::ReadsAnother,
        ..PLAIN
    });
    assert_eq!(refused, CompileDefect::NoSuchRead { law: bc("twice") });
}

#[test]
fn a_field_that_holds_by_a_mode_is_refused() {
    let refused = refused(Case {
        wrong: Wrong::ByMode,
        ..PLAIN
    });
    let mode = ModeValue {
        variable: "valve",
        value: "shut",
    };
    assert_eq!(refused, CompileDefect::UncoveredMode { law: bc(FIELD), mode });
}

#[test]
fn a_field_that_reads_its_owners_slot_by_its_local_name_is_refused() {
    let refused = refused(Case {
        wrong: Wrong::OwnSlot,
        second: true,
        ..PLAIN
    });
    assert_eq!(refused, CompileDefect::NoSuchRead { law: bc(FIELD) });
}

#[test]
fn a_field_that_adds_a_site_term_is_refused() {
    let refused = refused(Case {
        wrong: Wrong::Term,
        ..PLAIN
    });
    assert_eq!(refused, CompileDefect::NoSuchRead { law: bc(FIELD) });
}

#[test]
fn a_read_through_a_junction_a_drain_or_an_edges_end_is_refused() {
    for (wrong, law) in [
        (Wrong::Junction, bc("probe")),
        (Wrong::Drain, bc("probe")),
        (
            Wrong::EdgeEnd,
            LawName {
                owner: "CV",
                local: Family::Circulation.name(),
            },
        ),
    ] {
        let refused = refused(Case { wrong, ..PLAIN });
        assert_eq!(refused, CompileDefect::NoSuchRead { law }, "{wrong:?}");
    }
}

#[test]
fn a_read_off_the_fields_set_is_refused() {
    let field = bc(FIELD);
    let at_a_site = refused(Case {
        wrong: Wrong::OffItsSet,
        ..PLAIN
    });
    assert_eq!(
        at_a_site,
        CompileDefect::UnemittedField {
            law: bc("spread"),
            field
        }
    );
    let at_a_landmark = refused(Case {
        landmark: Some(0),
        ..PLAIN
    });
    assert_eq!(
        at_a_landmark,
        CompileDefect::UnemittedField {
            law: bc("sensed"),
            field
        }
    );
}

#[test]
fn a_field_that_reads_a_site_parameter_is_refused_at_a_site_and_at_a_landmark() {
    for landmark in [None, Some(3)] {
        let refused = refused(Case {
            wrong: Wrong::SiteParameter,
            landmark,
            ..PLAIN
        });
        assert_eq!(
            refused,
            CompileDefect::NoSuchRead { law: bc(FIELD) },
            "{landmark:?}"
        );
    }
}

#[test]
fn a_fields_range_is_judged_under_the_field_in_each_program_it_is_written_into() {
    let (body, sites) = bench(Case {
        wrong: Wrong::Ranged,
        second: true,
        ..PLAIN
    });
    let bound = bound(&body, sites, &[]);
    let frame = DriveFrame::default();
    let unknowns = starts(&body, &bound);
    let values = body
        .instance_values(&inputs(&bound, &frame, &unknowns))
        .expect("finite");
    let excursions = body.excursions(&values, &unknowns, SimTime::from_ns(0));
    let mut found: Vec<(LawName, Context)> = excursions
        .iter()
        .map(|excursion| (excursion.law(), excursion.context()))
        .collect();
    found.sort_unstable();
    let beds = [Context::Site(sites[1]), Context::Site(sites[3])];
    assert!(closed_form(0) > 1.0 && closed_form(1) > 1.0);
    let mut wanted: Vec<(LawName, Context)> = beds
        .iter()
        .flat_map(|&bed| [(bc(FIELD), bed), (bc(FIELD), bed)])
        .collect();
    wanted.sort_unstable();
    assert_eq!(
        found, wanted,
        "the probe's and the uptake's programs each judge the field at each bed"
    );
}
