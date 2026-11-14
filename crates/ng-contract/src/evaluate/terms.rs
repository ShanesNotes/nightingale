use super::{Inputs, Undefined};
use crate::body::Target;
use crate::boundary::DriveFrame;
use crate::law::ReadPort;
use crate::*;

fn measure<U: Dimension>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

#[derive(Clone, Copy, PartialEq)]
enum Host {
    Plain,
    Logarithm,
    LogarithmAfter,
    ByHand,
    Main,
    MainIn,
    Discarded,
}

#[derive(Clone, Copy, PartialEq)]
enum Term {
    Collapse,
    LogLevel,
    Level,
    Clock,
    NestedInVeins,
    NestedInBlood,
}

#[derive(Clone, Copy, PartialEq)]
enum Read {
    Clean,
    Absent,
    Draw,
}

#[derive(Clone, Copy)]
struct Case {
    marks: &'static [usize],
    host: Host,
    term: Term,
    name: &'static str,
    read: Read,
    split: bool,
}

const PLAIN: Case = Case {
    marks: &[0],
    host: Host::Plain,
    term: Term::Collapse,
    name: "collapse",
    read: Read::Clean,
    split: false,
};

const MAIN: Case = Case {
    marks: &[],
    host: Host::Main,
    ..PLAIN
};

fn compiled(case: Case) -> Result<(CompiledBody, [SiteId; 4]), AdmissionError> {
    let Case {
        marks,
        host,
        term: shape,
        name,
        read,
        split,
    } = case;
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at());
    let artery = map.site::<VesselSegment>("artery", at());
    let bed = map.site::<CapillaryBed>("bed", at());
    let upper = map.site::<VesselSegment>("upper vein", at());
    let lower = map.site::<VesselSegment>("lower vein", at());
    let sites = [artery.id(), bed.id(), upper.id(), lower.id()];
    for (at, &site) in sites.iter().enumerate() {
        map.contain(root.id(), site);
        map.connect(Network::Vascular, site, sites[(at + 1) % sites.len()]);
    }
    map.unsupplied(&sites);
    let veins = [&upper, &lower];
    for &mark in marks {
        map.vein(veins[mark]);
    }
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
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
        let dice = (read == Read::Draw).then(|| cv.stream("dice"));
        if let Some(dice) = &dice {
            let integral = cv.state::<Dimensionless>("integral", Admissible::nonnegative());
            let [a, b] = cv.modes("rhythm", ["a", "b"]);
            cv.hazard(HazardGuard::new(&integral, dice, a.to(&b)?));
            let rate = cv.kernel(|mut k| {
                let rate = k.zero::<Rate<Dimensionless>>();
                k.finish(rate, cv.every_mode(), class)
            });
            cv.memory_rate(&integral, rate);
        }
        let share =
            cv.site_parameter::<Blood, Dimensionless>(ParameterKey::new("cv.share"), Admissible::finite());
        let (level, x) = cv.site_field::<Blood, Dimensionless>("level", Admissible::finite());
        let modes = split.then(|| cv.modes("valve", ["off", "on"]));
        let holding = match &modes {
            Some([off, on]) => vec![off.only(), on.only()],
            None => vec![cv.every_mode()],
        };
        let held: Vec<_> = (holding.into_iter())
            .map(|holds| {
                cv.site_kernel(|mut k| {
                    let value = k.site_parameter(&share);
                    k.finish(value, holds, class)
                })
            })
            .collect();
        for kernel in held {
            cv.define_at(&level, kernel);
        }
        let t = cv.site_parameter::<Veins, Dimensionless>(ParameterKey::new("cv.t"), Admissible::finite());
        let p = cv.parameter::<Dimensionless>(ParameterKey::new("cv.p"), Admissible::finite());
        let absent: ReadPort<'_, Cardiovascular, PerSite<Blood, Dimensionless>> = ReadPort::field(LawName {
            owner: "CV",
            local: "absent",
        });
        let declared = !matches!(host, Host::ByHand | Host::Main | Host::MainIn);
        let inner = (declared && matches!(shape, Term::NestedInVeins | Term::NestedInBlood)).then(|| {
            cv.site_term::<Veins, Dimensionless, _>("inner", |mut k| {
                let knob = k.site_parameter(&t);
                let log = k.ln(knob);
                k.finish_term(log)
            })
        });
        let in_blood = (declared && shape == Term::NestedInBlood).then(|| {
            cv.site_term::<Blood, Dimensionless, _>(name, |mut k| {
                let level = k.at_site_in(&x);
                let log = k.ln(level);
                let value = k.add_from(log, inner.as_ref().expect("declared"));
                k.finish_term(value)
            })
        });
        let in_veins = (declared && shape != Term::NestedInBlood).then(|| {
            cv.site_term::<Veins, Dimensionless, _>(name, |mut k| {
                let level = k.at_site_in(&x);
                let mut value = match shape {
                    Term::Collapse => {
                        let knob = k.site_parameter(&t);
                        let log = k.ln(knob);
                        let fitted = k.parameter(&p);
                        let product = k.scale(level, fitted);
                        k.scale(log, product)
                    }
                    Term::LogLevel => k.ln(level),
                    Term::Level => level,
                    Term::Clock => {
                        let (cosine, _) = k.clock();
                        k.scale(level, cosine)
                    }
                    Term::NestedInVeins | Term::NestedInBlood => {
                        let log = k.ln(level);
                        k.add_from(log, inner.as_ref().expect("declared"))
                    }
                };
                if read == Read::Absent {
                    let wrong = k.at_site_in(&absent);
                    value = k.scale(value, wrong);
                }
                if let Some(dice) = &dice {
                    let drawn = k.draw(dice);
                    value = k.scale(value, drawn);
                }
                k.finish_term(value)
            })
        });
        for local in ["probe", "echo"] {
            if host == Host::ByHand {
                let (probe, _) = cv.site_field::<Veins, Dimensionless>(local, Admissible::finite());
                let written = cv.site_kernel(|mut k| {
                    let base = k.at_site_in(&x);
                    let knob = k.site_parameter(&t);
                    let log = k.ln(knob);
                    let fitted = k.parameter(&p);
                    let product = k.scale(base, fitted);
                    let mut value = k.scale(log, product);
                    if let Some(dice) = &dice {
                        let drawn = k.draw(dice);
                        value = k.scale(value, drawn);
                    }
                    let sum = k.add(base, value);
                    k.finish(sum, cv.every_mode(), class)
                });
                cv.define_at(&probe, written);
                continue;
            }
            let (probe, _) = cv.site_field::<Blood, Dimensionless>(local, Admissible::finite());
            let hosted = cv.site_kernel(|mut k| {
                let level = match host {
                    Host::MainIn => k.at_site_in(&x),
                    _ => k.at_site(&x),
                };
                let base = match host {
                    Host::Logarithm => k.ln(level),
                    _ => level,
                };
                let sum = match (&in_veins, &in_blood) {
                    (Some(term), _) => k.add_from(base, term),
                    (None, Some(term)) => k.add_from(base, term),
                    (None, None) => base,
                };
                let value = match host {
                    Host::Discarded => base,
                    Host::LogarithmAfter => {
                        let log = k.ln(level);
                        k.scale(sum, log)
                    }
                    _ => sum,
                };
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&probe, hosted);
        }
        body.add(cv.finish());
        body.compile()
    })?;
    Ok((body, sites))
}

fn bench(case: Case) -> (CompiledBody, [SiteId; 4]) {
    compiled(case).expect("compiled")
}

fn packet(body: &CompiledBody, level: f64) -> ParameterPacket<SyntheticFixture> {
    let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
    let vascular = Binding::Network(Network::Vascular);
    let synthetic = "synthetic";
    let wanted =
        |key: &'static str| (body.parameters().iter()).any(|slot| slot.key() == ParameterKey::new(key));
    let share = measure::<Dimensionless>(level);
    (packet.synthetic(ParameterKey::new("cv.share"), vascular, share, synthetic)).expect("once");
    if wanted("cv.t") {
        let t = measure::<Dimensionless>(1.0);
        (packet.synthetic(ParameterKey::new("cv.t"), vascular, t, synthetic)).expect("once");
    }
    if wanted("cv.p") {
        let p = measure::<Dimensionless>(0.75);
        (packet.synthetic(ParameterKey::new("cv.p"), Binding::Body, p, synthetic)).expect("once");
    }
    let start = |row: RowKind| match row {
        RowKind::Algebraic(slot) if slot.law.local == "level" => level,
        _ => 1.0,
    };
    packet.start_rest(body, start, synthetic).expect("once");
    packet
}

fn bound(body: &CompiledBody, t: f64, level: f64) -> BoundParameters {
    let mut bound = body.bind(packet(body, level)).expect("bound");
    for (slot, parameter) in body.parameters().iter().enumerate() {
        if parameter.key() == ParameterKey::new("cv.t") {
            bound.values[slot] = t;
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

fn cv(local: &'static str) -> LawName {
    LawName { owner: "CV", local }
}

fn refusal(body: &CompiledBody, t: f64, level: f64) -> Option<Undefined> {
    let bound = bound(body, t, level);
    let frame = DriveFrame::default();
    let unknowns = starts(body, &bound);
    body.instance_values(&inputs(&bound, &frame, &unknowns)).err()
}

fn refusal_at(body: &CompiledBody, site: SiteId, t: f64, level: f64) -> Option<Undefined> {
    let bound = bound(body, t, level);
    let frame = DriveFrame::default();
    let unknowns = starts(body, &bound);
    let inputs = inputs(&bound, &frame, &unknowns);
    let mut values = vec![0.0; body.tape.longest];
    (body.layout.instances.iter().enumerate())
        .filter(|(_, instance)| instance.context == Context::Site(site))
        .find_map(|(at, instance)| {
            let (arithmetic, reads) = (body.tape.program(instance.program), body.tape.instance(at));
            let refused =
                super::forward(arithmetic, reads, &inputs, body.partial_volume(), &mut values).err()?;
            Some(body.undefined(at, &inputs, refused))
        })
}

fn named(refused: Option<Undefined>) -> Option<(&'static str, String)> {
    refused.map(|refused| (refused.law.local, format!("{:?}", refused.domain)))
}

fn count(body: &CompiledBody, source: RefusalSource) -> usize {
    body.refusal_sources()
        .iter()
        .filter(|known| **known == source)
        .count()
}

fn log() -> String {
    "LogarithmArgument".into()
}

/// Interrogation finding 1 and ruling 3: a refusal in the term names the term's law, and a refusal in the host's own
/// arithmetic names the host, wherever the two share a program.
#[test]
fn a_refusal_in_a_term_names_the_terms_law_and_one_in_the_host_names_the_host() {
    let (body, _) = bench(Case {
        host: Host::Logarithm,
        ..PLAIN
    });
    assert_eq!(named(refusal(&body, 0.0, 0.5)), Some(("collapse", log())));
    assert_eq!(named(refusal(&body, 2.0, -0.5)), Some(("echo", log())));
    assert_eq!(
        count(&body, RefusalSource::LogarithmArgument { law: cv("collapse") }),
        1
    );
    assert_eq!(count(&body, RefusalSource::NonFinite { law: cv("collapse") }), 1);
    assert_eq!(
        count(&body, RefusalSource::LogarithmArgument { law: cv("probe") }),
        1
    );
    let (plain, _) = bench(Case {
        marks: &[0, 1],
        ..PLAIN
    });
    assert_eq!(
        count(&plain, RefusalSource::LogarithmArgument { law: cv("probe") }),
        0
    );
    assert_eq!(
        count(&plain, RefusalSource::LogarithmArgument { law: cv("collapse") }),
        1
    );
}

/// Ruling 4: where an op of the host and an op of the term both refuse, the first in program order names its law,
/// whether it computes a value or reads one that is not finite.
#[test]
fn where_host_and_term_both_refuse_the_first_in_program_order_names_its_law() {
    let (host_first, [_, _, vein, _]) = bench(Case {
        host: Host::Logarithm,
        ..PLAIN
    });
    let (term_first, _) = bench(Case {
        host: Host::LogarithmAfter,
        ..PLAIN
    });
    let at_vein = |body, t| named(refusal_at(body, vein, t, -0.5));
    assert_eq!(at_vein(&host_first, 0.0), Some(("echo", log())));
    assert_eq!(at_vein(&term_first, 0.0), Some(("collapse", log())));
    assert_eq!(at_vein(&host_first, f64::NAN), Some(("echo", log())));
    assert_eq!(
        at_vein(&term_first, f64::NAN),
        Some(("collapse", "NonFinite".into()))
    );
}

#[test]
fn a_logarithm_host_and_term_both_take_is_the_hosts() {
    let (body, _) = bench(Case {
        host: Host::LogarithmAfter,
        term: Term::LogLevel,
        ..PLAIN
    });
    assert_eq!(named(refusal(&body, 1.0, -0.5)), Some(("echo", log())));
    assert_eq!(
        count(&body, RefusalSource::LogarithmArgument { law: cv("collapse") }),
        0
    );
}

fn programs_at(body: &CompiledBody, local: &'static str) -> Vec<(SiteId, ProgramId)> {
    let rows = body.rows();
    let mut found: Vec<(SiteId, ProgramId)> = (body.instances().iter())
        .filter_map(|instance| match (instance.target(), instance.context()) {
            (Target::Row(row), Context::Site(site)) => match rows[row.index()].kind() {
                RowKind::Algebraic(slot) if slot.law == cv(local) => Some((site, instance.program())),
                _ => None,
            },
            _ => None,
        })
        .collect();
    found.sort_unstable();
    found
}

fn sealed(program: &crate::body::Program) -> Digest {
    let mut out = crate::identity::Canonical::new("program");
    crate::identity::Seal::seal(program, &mut out);
    out.digest()
}

fn sealed_all(body: &CompiledBody) -> Vec<Digest> {
    body.programs().iter().map(sealed).collect()
}

/// Ruling 2 and interrogation finding 2: off the term's set each host runs main's program, with main's id and bytes,
/// however many hosts or marks there are and in whatever order the marks are made.
#[test]
fn off_its_set_each_host_runs_mains_program_with_mains_id() {
    let (main, _) = bench(MAIN);
    let main_programs = sealed_all(&main);
    for marks in [&[0][..], &[1], &[0, 1], &[1, 0]] {
        let (body, [_, _, upper, lower]) = bench(Case { marks, ..PLAIN });
        let programs = sealed_all(&body);
        assert_eq!(programs[..main_programs.len()], main_programs[..]);
        for local in ["probe", "echo"] {
            let marked: Vec<SiteId> = marks.iter().map(|&mark| [upper, lower][mark]).collect();
            for ((site, program), (_, on_main)) in programs_at(&body, local)
                .into_iter()
                .zip(programs_at(&main, local))
            {
                if marked.contains(&site) {
                    assert!(program.index() >= main_programs.len());
                } else {
                    assert_eq!(program, on_main);
                }
            }
        }
    }
    let (forward, _) = bench(Case {
        marks: &[0, 1],
        ..PLAIN
    });
    let (reverse, _) = bench(Case {
        marks: &[1, 0],
        ..PLAIN
    });
    assert_eq!(forward.model_hash(), reverse.model_hash());
}

/// Ruling 5 and interrogation finding 7: a declared and added term with no site in its set leaves every byte of main,
/// whatever it is named.
#[test]
fn with_no_vein_marked_every_byte_is_mains() {
    let (main, _) = bench(MAIN);
    for name in ["collapse", "renamed"] {
        for host in [Host::Plain, Host::Discarded] {
            let (body, _) = bench(Case {
                marks: &[],
                host,
                name,
                ..PLAIN
            });
            assert_eq!(body.model_hash(), main.model_hash());
            assert_eq!(body.refusal_sources(), main.refusal_sources());
            assert_eq!(body.parameters(), main.parameters());
            assert_eq!(sealed_all(&body), sealed_all(&main));
        }
    }
}

#[test]
fn a_marked_terms_name_is_in_the_model_hash() {
    let (collapse, _) = bench(PLAIN);
    let (renamed, _) = bench(Case {
        name: "renamed",
        ..PLAIN
    });
    assert_ne!(collapse.model_hash(), renamed.model_hash());
}

#[test]
fn with_no_vein_marked_the_refusal_sources_are_mains_and_list_each_law_once() {
    let (main, _) = bench(Case { split: true, ..MAIN });
    let laws: Vec<LawName> = main.programs().iter().map(|program| program.law).collect();
    assert!(
        laws.windows(2).any(|pair| pair[0] == pair[1]),
        "a law with adjacent programs"
    );
    let (body, _) = bench(Case {
        marks: &[],
        split: true,
        ..PLAIN
    });
    let sources = body.refusal_sources();
    assert_eq!(sources, main.refusal_sources());
    for source in sources {
        assert_eq!(
            sources.iter().filter(|known| *known == source).count(),
            1,
            "{source:?}"
        );
    }
}

/// Ruling 4 and interrogation finding 4: a term's invalid read is refused at a site of its set even where the host
/// throws the sum away, as main refuses a kernel's unused input.
#[test]
fn a_terms_unused_invalid_read_is_still_refused() {
    let refused = |marks: &'static [usize], host| {
        compiled(Case {
            marks,
            host,
            read: Read::Absent,
            ..PLAIN
        })
        .err()
    };
    let absent = AdmissionError::Compile(CompileDefect::UnemittedField {
        law: cv("echo"),
        field: cv("absent"),
    });
    assert_eq!(refused(&[0], Host::Plain), Some(absent));
    assert_eq!(refused(&[0], Host::Discarded), Some(absent));
    assert!(refused(&[], Host::Discarded).is_none());
}

/// Ruling 10: a term cannot hide what its host may not read. A draw an algebraic law reads is refused at admission
/// whether the law reads it by hand or through a term at a marked vein, and a day's input read with no parameter is
/// refused through a term as by hand.
#[test]
fn a_read_main_refuses_by_hand_is_refused_through_a_term() {
    let modes = |case: Case| {
        let (body, _) = bench(Case {
            read: Read::Draw,
            ..case
        });
        let mut packet = packet(&body, 0.5);
        packet.start_mode(cv("rhythm"), "a", "synthetic").expect("once");
        body.bind(packet).err()
    };
    let refused = Some(AdmissionError::ModesNotYetRun {
        variable: cv("rhythm").into(),
    });
    let by_hand = Case {
        host: Host::ByHand,
        ..PLAIN
    };
    assert_eq!(modes(by_hand), refused);
    assert_eq!(modes(PLAIN), refused);
    assert_eq!(modes(Case { marks: &[], ..PLAIN }), None);
    for marks in [&[][..], &[0]] {
        let unscaled = compiled(Case {
            marks,
            term: Term::Clock,
            ..PLAIN
        });
        assert!(matches!(
            unscaled.err(),
            Some(AdmissionError::Compile(CompileDefect::UnscaledInput { .. }))
        ));
    }
}

/// Ruling 5: `at_site_in` on a kernel's own set writes the op `at_site` writes.
#[test]
fn a_read_in_its_own_set_writes_the_op_a_plain_read_writes() {
    let (main, _) = bench(MAIN);
    let (spelled, _) = bench(Case {
        host: Host::MainIn,
        ..MAIN
    });
    assert_eq!(sealed_all(&spelled), sealed_all(&main));
    assert_eq!(spelled.model_hash(), main.model_hash());
}

/// Ruling 6: a term that adds a term. No set lies strictly inside the veins, so an outer term on the veins shows a
/// site in neither set and in both, and an outer term on blood a site in the outer only and in both. Each refusal
/// names its own law.
#[test]
fn a_term_that_adds_a_term_names_each_refusal_by_its_own_law() {
    let nested = |marks: &'static [usize], term: Term| bench(Case { marks, term, ..PLAIN }).0;
    let (main, _) = bench(MAIN);
    let neither = nested(&[], Term::NestedInVeins);
    assert_eq!(neither.model_hash(), main.model_hash());
    assert_eq!(sealed_all(&neither), sealed_all(&main));
    let outer_only = nested(&[], Term::NestedInBlood);
    assert_eq!(named(refusal(&outer_only, 2.0, -0.5)), Some(("collapse", log())));
    assert_eq!(named(refusal(&outer_only, 0.0, 0.5)), None);
    assert_eq!(
        count(&outer_only, RefusalSource::LogarithmArgument { law: cv("inner") }),
        0
    );
    for term in [Term::NestedInVeins, Term::NestedInBlood] {
        let both = nested(&[0], term);
        assert_eq!(named(refusal(&both, 0.0, 0.5)), Some(("inner", log())));
        assert_eq!(named(refusal(&both, 2.0, -0.5)), Some(("collapse", log())));
        assert_eq!(named(refusal(&both, 0.0, -0.5)), Some(("collapse", log())));
        assert_eq!(
            count(&both, RefusalSource::LogarithmArgument { law: cv("inner") }),
            1
        );
        assert_eq!(
            count(&both, RefusalSource::LogarithmArgument { law: cv("collapse") }),
            1
        );
    }
}

/// Ruling 7: a term whose every op merges into its host's leaves no op of its own, so its host's variant lists the
/// host's law again in the refusal sources, as main lists a law whose programs are not adjacent.
#[test]
fn a_term_whose_ops_all_merge_into_its_hosts_repeats_the_hosts_law() {
    let merged = |marks: &'static [usize]| {
        bench(Case {
            marks,
            term: Term::Level,
            ..PLAIN
        })
        .0
    };
    let finite = RefusalSource::NonFinite { law: cv("probe") };
    assert_eq!(count(&merged(&[]), finite), 1);
    assert_eq!(count(&merged(&[0]), finite), 2);
    assert_eq!(
        count(&merged(&[0]), RefusalSource::NonFinite { law: cv("collapse") }),
        0
    );
}

/// Find-out 4: the Jacobian and the parameter slopes of a host that adds a term reading an algebraic unknown and a
/// parameter equal those of the same expression written by hand, bit for bit, and the program is the same.
#[test]
fn a_placed_terms_derivatives_are_the_hand_written_expressions() {
    let (placed, [_, _, upper, lower]) = bench(Case {
        marks: &[0, 1],
        ..PLAIN
    });
    let (by_hand, _) = bench(Case {
        marks: &[0, 1],
        host: Host::ByHand,
        ..PLAIN
    });
    let ops = |body: &CompiledBody, local, site| {
        let (_, program) = (programs_at(body, local).into_iter())
            .find(|found| found.0 == site)
            .expect("an instance");
        body.programs()[program.index()].code.ops.clone()
    };
    for site in [upper, lower] {
        assert_eq!(ops(&placed, "probe", site), ops(&by_hand, "probe", site));
    }
    let row_slopes = |body: &CompiledBody| {
        let bound = bound(body, 2.5, 0.375);
        let frame = DriveFrame::default();
        let unknowns = starts(body, &bound);
        let inputs = inputs(&bound, &frame, &unknowns);
        let jacobian = body.jacobian(&inputs, 0.0).expect("finite");
        let sparsity = body.sparsity();
        let rows = body.rows();
        let mut out = Vec::new();
        for (at, row) in rows.iter().enumerate() {
            let RowKind::Algebraic(slot) = row.kind() else {
                continue;
            };
            if !matches!(slot.at, Context::Site(site) if site == upper || site == lower)
                || slot.law.local == "level"
            {
                continue;
            }
            for entry in sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1] {
                let column = rows[sparsity.unknowns()[entry].index()].kind();
                out.push(format!(
                    "{row:?} d/{column:?} {:016x}",
                    jacobian[entry].to_bits(),
                    row = row.kind()
                ));
            }
            for slot in 0..bound.values.len() {
                let key = body.parameters()[slot].key();
                let slopes = body
                    .parameter_slopes(&inputs, &[crate::body::ParameterId::at(slot)])
                    .expect("finite");
                out.push(format!(
                    "{:?} d/{key:?} {:016x}",
                    row.kind(),
                    slopes[at].to_bits()
                ));
            }
        }
        out.sort();
        out
    };
    let placed_slopes = row_slopes(&placed);
    assert_eq!(placed_slopes, row_slopes(&by_hand));
    assert!(
        placed_slopes
            .iter()
            .any(|line| line.contains("cv.p") && !line.ends_with("0000000000000000"))
    );
}

#[test]
fn a_term_named_as_a_slot_is_refused() {
    let named = compiled(Case {
        name: "level",
        ..PLAIN
    })
    .err();
    let twice = AdmissionError::Compile(CompileDefect::NamedTwice { law: cv("level") });
    assert_eq!(named, Some(twice));
}
