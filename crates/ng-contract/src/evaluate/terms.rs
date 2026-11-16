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
/// whether the law reads it by hand or through a term at a marked vein. A day's input a term reads with no parameter
/// is refused where a vein runs the variant, and with no vein the body is the one with no term.
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
    let unscaled = |marks: &'static [usize]| {
        compiled(Case {
            marks,
            term: Term::Clock,
            ..PLAIN
        })
        .err()
    };
    assert!(matches!(
        unscaled(&[0]),
        Some(AdmissionError::Compile(CompileDefect::UnscaledInput { .. }))
    ));
    assert_eq!(unscaled(&[]), None);
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
/// parameter equal, bit for bit, those of the same expression written by hand with the read they share bound once,
/// and the program is the same.
#[test]
fn a_placed_terms_derivatives_are_the_hand_written_expressions() {
    let placed = compiled(Case {
        marks: &[0, 1],
        ..PLAIN
    });
    assert!(
        placed.is_ok(),
        "a variant closes its row as main closes it: {:?}",
        placed.err()
    );
    let (placed, [_, _, upper, lower]) = placed.expect("compiled");
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

/// A term added and its sum thrown away leaves the variant equal to main's program, so a marked vein runs main's
/// program with main's id.
#[test]
fn a_term_whose_sum_is_thrown_away_runs_mains_program_at_a_marked_vein() {
    let (main, _) = bench(Case { marks: &[0], ..MAIN });
    let (discarded, _) = bench(Case {
        host: Host::Discarded,
        ..PLAIN
    });
    assert_eq!(sealed_all(&discarded), sealed_all(&main));
    assert_eq!(discarded.model_hash(), main.model_hash());
}

/// Which op a term computes is sealed, not only how many it computes and whose they are.
#[test]
fn an_origins_position_is_sealed() {
    let (body, _) = bench(PLAIN);
    let program = (body.programs().iter())
        .find(|program| !program.code.origins.is_empty())
        .expect("a variant");
    let code = &program.code;
    let free = (0..code.ops.len())
        .find(|at| code.origin(*at).is_none())
        .expect("a host op");
    let mut origins = code.origins.clone();
    origins[0].0 = free;
    origins.sort_unstable_by_key(|origin| origin.0);
    let moved = crate::body::Code {
        ops: code.ops.clone(),
        dimension: code.dimension.clone(),
        predicate: code.predicate,
        rate: code.rate,
        validity: code.validity.clone(),
        defines: code.defines,
        origins,
        judged: Vec::new(),
    };
    let seal = |code: &crate::body::Code| {
        let mut out = crate::identity::Canonical::new("code");
        crate::identity::Seal::seal(code, &mut out);
        out.digest()
    };
    assert_ne!(seal(&moved), seal(code));
}

mod forms {
    use super::*;

    type Cv = Cardiovascular;
    type K<'b, 'k, S> = Kernel<'b, 'k, Cv, S>;
    type X<'b, 'k> = Expr<'b, 'k, Cv, Dimensionless>;

    struct Decl<'b> {
        share: SiteParameter<'b, Cv, Blood, Dimensionless>,
        t: SiteParameter<'b, Cv, Veins, Dimensionless>,
        b: SiteParameter<'b, Cv, CapillaryBed, Dimensionless>,
        p: Parameter<'b, Cv, Dimensionless>,
        x: ReadPort<'b, Cv, PerSite<Blood, Dimensionless>>,
        y: ReadPort<'b, Cv, PerSite<Blood, Dimensionless>>,
        water: ReadPort<'b, Cv, PerSite<Blood, Water>>,
        effect: ReadPort<'b, Cv, PerSite<Junction, Dimensionless>>,
        wobble: Option<Forcing<'b, Cv>>,
        inner: Vec<SiteTerm<'b, Cv, Veins, Dimensionless>>,
        veins: Vec<SiteTerm<'b, Cv, Veins, Dimensionless>>,
        beds: Vec<SiteTerm<'b, Cv, CapillaryBed, Dimensionless>>,
    }

    type BloodFn = for<'b, 'k> fn(&mut K<'b, 'k, Blood>, &Decl<'b>) -> X<'b, 'k>;
    type VeinFn = for<'b, 'k> fn(&mut K<'b, 'k, Veins>, &Decl<'b>) -> X<'b, 'k>;
    type BedFn = for<'b, 'k> fn(&mut K<'b, 'k, CapillaryBed>, &Decl<'b>) -> X<'b, 'k>;

    #[derive(Clone, Default)]
    struct Fix {
        marks: Vec<usize>,
        no_partial: bool,
        forcing: bool,
        spare: Option<VeinFn>,
        inner: Vec<(&'static str, VeinFn)>,
        veins: Vec<(&'static str, VeinFn)>,
        beds: Vec<(&'static str, BedFn)>,
        hosts: Vec<(&'static str, BloodFn, bool)>,
        hands: Vec<(&'static str, VeinFn)>,
        guard: Option<BloodFn>,
    }

    fn build(fix: &Fix) -> Result<(CompiledBody, [SiteId; 4]), AdmissionError> {
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
        for &mark in &fix.marks {
            map.vein(veins[mark]);
        }
        let mut schema = Schema::new(map.finish()?)?;
        if !fix.no_partial {
            schema.partial_volume(Measure::new(1.0)?)?;
        }
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
            let share = cv
                .site_parameter::<Blood, Dimensionless>(ParameterKey::new("cv.share"), Admissible::finite());
            let t =
                cv.site_parameter::<Veins, Dimensionless>(ParameterKey::new("cv.t"), Admissible::finite());
            let b = cv.site_parameter::<CapillaryBed, Dimensionless>(
                ParameterKey::new("cv.b"),
                Admissible::finite(),
            );
            let p = cv.parameter::<Dimensionless>(ParameterKey::new("cv.p"), Admissible::finite());
            let (level, x) = cv.site_field::<Blood, Dimensionless>("level", Admissible::finite());
            let (depth, y) = cv.site_field::<Blood, Dimensionless>("depth", Admissible::finite());
            for out in [&level, &depth] {
                let held = cv.site_kernel(|mut k| {
                    let value = k.site_parameter(&share);
                    k.finish(value, cv.every_mode(), class)
                });
                cv.define_at(out, held);
            }
            let wobble = fix.forcing.then(|| {
                cv.forcing(
                    "wobble",
                    ParameterKey::new("cv.period"),
                    ParameterKey::new("cv.amplitude"),
                )
            });
            let mut decl = Decl {
                share,
                t,
                b,
                p,
                x,
                y,
                water: ports.amount::<Water>(),
                effect: ports.effect::<Norepinephrine>(),
                wobble,
                inner: Vec::new(),
                veins: Vec::new(),
                beds: Vec::new(),
            };
            for &(name, shape) in &fix.beds {
                let term = cv.site_term::<CapillaryBed, Dimensionless, _>(name, |mut k| {
                    let value = shape(&mut k, &decl);
                    k.finish_term(value)
                });
                decl.beds.push(term);
            }
            for (terms, declared) in [(&fix.inner, false), (&fix.veins, true)] {
                for &(name, shape) in terms {
                    let term = cv.site_term::<Veins, Dimensionless, _>(name, |mut k| {
                        let value = shape(&mut k, &decl);
                        k.finish_term(value)
                    });
                    if declared {
                        decl.veins.push(term);
                    } else {
                        decl.inner.push(term);
                    }
                }
            }
            if let Some(shape) = fix.spare {
                let _ = cv.site_term::<Veins, Dimensionless, _>("spare", |mut k| {
                    let value = shape(&mut k, &decl);
                    k.finish_term(value)
                });
            }
            for &(name, shape, summed) in &fix.hosts {
                let hosted = cv.site_kernel::<Blood, Dimensionless, _>(|mut k| {
                    let value = shape(&mut k, &decl);
                    k.finish(value, cv.every_mode(), class)
                });
                if summed {
                    let total = cv.algebraic::<Dimensionless>(name, Admissible::finite());
                    let own = cv.kernel(|mut k| {
                        let value = k.algebraic(&total);
                        k.finish(value, cv.every_mode(), class)
                    });
                    cv.sum_at(&total, own, hosted);
                } else {
                    let (probe, _) = cv.site_field::<Blood, Dimensionless>(name, Admissible::finite());
                    cv.define_at(&probe, hosted);
                }
            }
            for &(name, shape) in &fix.hands {
                let written = cv.site_kernel::<Veins, Dimensionless, _>(|mut k| {
                    let value = shape(&mut k, &decl);
                    k.finish(value, cv.every_mode(), class)
                });
                let (probe, _) = cv.site_field::<Veins, Dimensionless>(name, Admissible::finite());
                cv.define_at(&probe, written);
            }
            if let Some(shape) = fix.guard {
                let [on, off] = cv.modes("valve", ["on", "off"]);
                let zero = cv.site_kernel::<Blood, Dimensionless, _>(|mut k| {
                    let value = shape(&mut k, &decl);
                    k.finish(value, on.only(), class)
                });
                cv.event(Guard::new(zero, Crossing::Rising, on.to(&off)?));
            }
            body.add(cv.finish());
            body.compile()
        })?;
        Ok((body, sites))
    }

    fn plus<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>, mut value: X<'b, 'k>) -> X<'b, 'k> {
        for term in &d.veins {
            value = k.add_from(value, term);
        }
        for term in &d.beds {
            value = k.add_from(value, term);
        }
        value
    }
    fn h_param<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let base = k.site_parameter(&d.share);
        plus(k, d, base)
    }
    fn h_level<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let base = k.at_site(&d.x);
        plus(k, d, base)
    }
    fn h_clock<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let (cosine, _) = k.clock();
        let level = k.at_site(&d.x);
        let base = k.scale(level, cosine);
        plus(k, d, base)
    }
    fn h_clock_discards<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let (cosine, _) = k.clock();
        let level = k.at_site(&d.x);
        let base = k.scale(level, cosine);
        let _ = plus(k, d, base);
        base
    }
    fn h_big<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let level = k.at_site(&d.x);
        let sum = plus(k, d, level);
        let big = k.site_parameter(&d.share);
        k.scale(sum, big)
    }
    fn h_guard<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let bar = k.parameter(&d.p);
        plus(k, d, bar)
    }
    fn h_nerve<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let felt = k.at_junction_if_supplied(&d.effect);
        let level = k.at_site(&d.x);
        let base = k.scale(level, felt);
        plus(k, d, base)
    }
    fn h_nerve_plain<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let felt = k.at_junction(&d.effect);
        let level = k.at_site(&d.x);
        let base = k.scale(level, felt);
        plus(k, d, base)
    }

    fn t_param<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        k.site_parameter(&d.t)
    }
    fn t_ln<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let knob = k.site_parameter(&d.t);
        k.ln(knob)
    }
    fn t_ln_level<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let level = k.at_site_in(&d.x);
        k.ln(level)
    }
    fn t_y_times_t<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let depth = k.at_site_in(&d.y);
        let knob = k.site_parameter(&d.t);
        k.scale(depth, knob)
    }
    fn t_collapse_y<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let depth = k.at_site_in(&d.y);
        let knob = k.site_parameter(&d.t);
        let log = k.ln(knob);
        let fitted = k.parameter(&d.p);
        let product = k.scale(depth, fitted);
        k.scale(log, product)
    }
    fn t_forcing<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let varied = k.forcing(d.wobble.as_ref().expect("forcing"));
        let knob = k.site_parameter(&d.t);
        k.scale(knob, varied)
    }
    fn t_ln_level_plus_inner<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let level = k.at_site_in(&d.x);
        let log = k.ln(level);
        k.add_from(log, &d.inner[0])
    }
    fn t_volume<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let water = k.at_site_in(&d.water);
        let volume = k.volume(water);
        let knob = k.site_parameter(&d.t);
        let per = k.divide(knob, volume);
        let back = k.multiply(per, volume);
        k.as_dimension(back)
    }
    fn t_volume_unused<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let water = k.at_site_in(&d.water);
        let _ = k.volume(water);
        k.site_parameter(&d.t)
    }
    fn t_nerve<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let felt = k.at_junction_if_supplied(&d.effect);
        let knob = k.site_parameter(&d.t);
        k.scale(knob, felt)
    }
    fn t_nerve_plain<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let felt = k.at_junction(&d.effect);
        let knob = k.site_parameter(&d.t);
        k.scale(knob, felt)
    }
    fn hand_param_collapse_y<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let base = k.site_parameter_in(&d.share);
        let value = t_collapse_y(k, d);
        k.add(base, value)
    }

    fn two(shape: BloodFn) -> Vec<(&'static str, BloodFn, bool)> {
        vec![("probe", shape, false), ("echo", shape, false)]
    }
    fn term(name: &'static str, shape: VeinFn) -> Vec<(&'static str, VeinFn)> {
        vec![(name, shape)]
    }
    fn refused(fix: &Fix) -> Option<CompileDefect> {
        match build(fix) {
            Err(AdmissionError::Compile(defect)) => Some(defect),
            Err(other) => panic!("{other:?}"),
            Ok(_) => None,
        }
    }

    fn bind(body: &CompiledBody) -> BoundParameters {
        let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
        let mut seen: Vec<(ParameterKey, Binding)> = Vec::new();
        for slot in body.parameters() {
            let entry = (slot.key, slot.default.unwrap_or(slot.binding));
            if !seen.contains(&entry) {
                seen.push(entry);
                (packet.synthetic(entry.0, entry.1, measure::<Dimensionless>(1.0), "synthetic"))
                    .expect("once");
            }
        }
        packet.start_rest(body, |_| 1.0, "synthetic").expect("once");
        body.bind(packet).expect("bound")
    }

    struct Point<'a> {
        body: &'a CompiledBody,
        bound: BoundParameters,
        unknowns: Vec<f64>,
        frame: DriveFrame,
    }
    impl<'a> Point<'a> {
        fn new(body: &'a CompiledBody) -> Self {
            let bound = bind(body);
            let unknowns = starts(body, &bound);
            Self {
                body,
                bound,
                unknowns,
                frame: DriveFrame::default(),
            }
        }
        fn inputs(&self) -> Inputs<'_> {
            inputs(&self.bound, &self.frame, &self.unknowns)
        }
        fn set(&mut self, key: &'static str, value: f64) {
            for (slot, parameter) in self.body.parameters().iter().enumerate() {
                if parameter.key() == ParameterKey::new(key) {
                    self.bound.values[slot] = value;
                }
            }
        }
        fn know(&mut self, local: &str, at: Option<SiteId>, value: f64) {
            for (row, known) in self.body.rows().iter().enumerate() {
                let (RowKind::Algebraic(slot) | RowKind::Memory(slot)) = known.kind() else {
                    continue;
                };
                let here = at.is_none_or(|site| slot.at == Context::Site(site));
                if slot.law.local == local && here {
                    self.unknowns[row] = value;
                }
            }
        }
        fn values_at(&self, site: SiteId) -> Option<Undefined> {
            let mut values = vec![0.0; self.body.tape.longest];
            let inputs = self.inputs();
            (self.body.layout.instances.iter().enumerate())
                .filter(|(_, instance)| instance.context == Context::Site(site))
                .find_map(|(at, instance)| {
                    let (arithmetic, reads) = (
                        self.body.tape.program(instance.program),
                        self.body.tape.instance(at),
                    );
                    let partial_volume = self.body.partial_volume();
                    let refused =
                        super::super::forward(arithmetic, reads, &inputs, partial_volume, &mut values)
                            .err()?;
                    Some(self.body.undefined(at, &inputs, refused))
                })
        }
    }

    fn row_of(body: &CompiledBody, instance: &crate::body::Instance) -> Option<LawName> {
        match instance.target() {
            Target::Row(row) => match body.rows()[row.index()].kind() {
                RowKind::Algebraic(slot) | RowKind::Memory(slot) => Some(slot.law),
                RowKind::Amount { .. } => None,
            },
            Target::Column(_) => None,
        }
    }
    fn ops_of(body: &CompiledBody, local: &str, site: SiteId) -> Vec<Op> {
        let instance = (body.instances().iter())
            .find(|instance| {
                instance.context() == Context::Site(site)
                    && row_of(body, instance).is_some_and(|law| law.local == local)
            })
            .expect("an instance");
        body.programs()[instance.program().index()].code.ops.clone()
    }

    /// Ruling 1, defect 1 of the check: the host is judged as main judges it, so a term that reads a parameter does
    /// not lend it one, with or without a vein marked, the sum used or thrown away.
    #[test]
    fn a_host_main_refuses_for_an_unscaled_input_is_refused_when_it_adds_a_term() {
        let main = Fix {
            hosts: two(h_clock),
            ..Fix::default()
        };
        let added = Fix {
            veins: term("collapse", t_param),
            ..main.clone()
        };
        let discarded = Fix {
            hosts: two(h_clock_discards),
            ..added.clone()
        };
        for fix in [main, added, discarded] {
            for marks in [vec![], vec![0]] {
                let fix = Fix { marks, ..fix.clone() };
                assert!(matches!(refused(&fix), Some(CompileDefect::UnscaledInput { .. })));
            }
        }
    }

    /// Ruling 1: a forcing only a term reads counts as read where a site of the term's set runs it, through a term the
    /// term adds too, and with no such site it is unread, as with no term.
    #[test]
    fn a_forcing_a_term_reads_counts_only_where_a_site_runs_the_term() {
        let unread = Fix {
            forcing: true,
            hosts: two(h_level),
            ..Fix::default()
        };
        let direct = Fix {
            veins: term("collapse", t_forcing),
            ..unread.clone()
        };
        let nested = Fix {
            inner: term("inner", t_forcing),
            veins: term("collapse", t_ln_level_plus_inner),
            ..unread.clone()
        };
        let spare = Fix {
            spare: Some(t_forcing),
            ..unread.clone()
        };
        let unread_forcing = Some(CompileDefect::UnreadForcing {
            forcing: cv("wobble"),
        });
        for fix in [&unread, &direct, &nested, &spare] {
            assert_eq!(refused(fix), unread_forcing);
        }
        for fix in [direct, nested] {
            assert_eq!(
                refused(&Fix {
                    marks: vec![0],
                    ..fix
                }),
                None
            );
        }
        assert_eq!(
            refused(&Fix {
                marks: vec![0],
                ..spare
            }),
            unread_forcing
        );
    }

    /// Ruling 1: a volume a term uses needs a partial volume where a site of its set runs the term. One it never uses
    /// counts no more than in main's program, and with no such site the body compiles as with no term.
    #[test]
    fn a_volume_a_term_reads_needs_a_partial_volume_only_where_a_site_runs_the_term() {
        let main = Fix {
            no_partial: true,
            hosts: two(h_level),
            ..Fix::default()
        };
        assert_eq!(refused(&main), None);
        for (shape, used) in [(t_volume as VeinFn, true), (t_volume_unused, false)] {
            let added = Fix {
                veins: term("collapse", shape),
                ..main.clone()
            };
            assert_eq!(refused(&added), None);
            let marked = refused(&Fix {
                marks: vec![0],
                ..added
            });
            assert_eq!(
                matches!(marked, Some(CompileDefect::MissingPartialVolume { .. })),
                used
            );
        }
    }

    /// Ruling 2: a guard stands at the body or an edge, never at a site, so a guard whose zero adds a term is refused
    /// by its mode variable's name.
    #[test]
    fn a_guard_whose_zero_adds_a_term_is_refused_by_name() {
        let plain = Fix {
            marks: vec![0, 1],
            guard: Some(h_guard),
            ..Fix::default()
        };
        assert_eq!(refused(&plain), None);
        for marks in [vec![], vec![0, 1]] {
            let added = Fix {
                marks,
                veins: term("collapse", t_param),
                ..plain.clone()
            };
            assert_eq!(
                refused(&added),
                Some(CompileDefect::TermOffSite { law: cv("valve") })
            );
        }
    }

    /// Ruling 3: a term's read that takes no nerve keeps that at a site the map says no nerve reaches, and a plain read
    /// in a term is refused there beside a host read that takes no nerve.
    #[test]
    fn a_terms_read_that_takes_no_nerve_keeps_that_through_placing_and_sharing() {
        let fix = |marks: Vec<usize>, host: BloodFn, shape: VeinFn| Fix {
            marks,
            veins: term("collapse", shape),
            hosts: two(host),
            ..Fix::default()
        };
        let unsupplied = |fix: &Fix| matches!(refused(fix), Some(CompileDefect::UnsuppliedRead { .. }));
        assert_eq!(refused(&fix(vec![0], h_level, t_nerve)), None);
        assert_eq!(refused(&fix(vec![0], h_nerve, t_nerve)), None);
        assert!(unsupplied(&fix(vec![0], h_nerve, t_nerve_plain)));
        assert!(unsupplied(&fix(vec![0], h_nerve_plain, t_nerve)));
        assert_eq!(refused(&fix(vec![], h_nerve, t_nerve_plain)), None);
    }

    /// Ruling 3: a summed law that adds a term is closed at a vein as main closes a summed law, and its value there is
    /// the hand-written law's, bit for bit, with the read they share bound once.
    #[test]
    fn a_summed_law_that_adds_a_term_closes_as_main_closes_it() {
        let summed = |with_term: bool| Fix {
            marks: vec![0],
            veins: if with_term {
                term("collapse", t_collapse_y)
            } else {
                Vec::new()
            },
            hosts: vec![("total", h_param as BloodFn, true)],
            ..Fix::default()
        };
        let (placed, [_, _, upper, lower]) = build(&summed(true)).expect("compiled");
        let (plain, _) = build(&summed(false)).expect("compiled");
        let by_hand = Fix {
            marks: vec![0],
            hands: vec![("probe", hand_param_collapse_y as VeinFn)],
            ..Fix::default()
        };
        let (written, _) = build(&by_hand).expect("compiled");
        let (on, off) = (ops_of(&placed, "total", upper), ops_of(&placed, "total", lower));
        let hand = ops_of(&written, "probe", upper);
        assert_eq!(off, ops_of(&plain, "total", lower));
        assert!(matches!(on[on.len() - 2..], [Op::Zero(_), Op::Subtract(_)]));
        assert_eq!(on[..on.len() - 2], hand[..hand.len() - 2]);
        let value = |body: &CompiledBody, local: &'static str| {
            let mut point = Point::new(body);
            point.set("cv.t", 2.5);
            point.set("cv.p", 0.75);
            point.know("depth", None, 0.4375);
            point.know(local, None, 0.0);
            let values = body.instance_values(&point.inputs()).expect("defined");
            (body.instances().iter().zip(values))
                .find(|(instance, _)| {
                    instance.context() == Context::Site(upper) && row_of(body, instance) == Some(cv(local))
                })
                .and_then(|(_, value)| value)
                .expect("a value")
                .to_bits()
        };
        assert_eq!(value(&placed, "total"), value(&written, "probe"));
    }

    fn stroke(marks: &[usize], with_term: bool) -> Result<CompiledBody, AdmissionError> {
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
        let store = root.id();
        let mut schema = Schema::new(map.finish()?)?;
        schema.partial_volume(Measure::new(1.0)?)?;
        schema.leave_out(Space::Interstitium, "a bench with no tissue");
        schema.leave_out(Space::Cells, "a bench with no tissue");
        schema.pool::<Energy>(store, Space::Elastic)?;
        assemble(schema, |mut body| {
            let class = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
            let (mut cv, ports) = body.cardiovascular()?.parts();
            let route = ports.circulation(&mut cv);
            let still = cv.edge_kernel(|mut k| {
                let zero = k.zero::<Rate<Water>>();
                k.finish(zero, cv.every_mode(), class)
            });
            cv.transfer_along(&route, still);
            let share = cv
                .site_parameter::<Blood, Dimensionless>(ParameterKey::new("cv.share"), Admissible::finite());
            let (level, x) = cv.site_field::<Blood, Dimensionless>("level", Admissible::finite());
            let held = cv.site_kernel(|mut k| {
                let value = k.site_parameter(&share);
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&level, held);
            let work =
                cv.site_parameter::<Blood, Rate<Energy>>(ParameterKey::new("cv.work"), Admissible::finite());
            let leak =
                cv.site_parameter::<Veins, Rate<Energy>>(ParameterKey::new("cv.leak"), Admissible::finite());
            let collapse = with_term.then(|| {
                cv.site_term::<Veins, Rate<Energy>, _>("collapse", |mut k| {
                    let rate = k.site_parameter(&leak);
                    let level = k.at_site_in(&x);
                    let value = k.scale(rate, level);
                    k.finish_term(value)
                })
            });
            let paid_into = ports.stroke_work(&mut cv);
            let paid = cv.site_kernel::<Blood, Rate<Energy>, _>(|mut k| {
                let base = k.site_parameter(&work);
                let value = match &collapse {
                    Some(term) => k.add_from(base, term),
                    None => base,
                };
                k.finish(value, cv.every_mode(), class)
            });
            cv.transfer_at(&paid_into, paid);
            body.add(cv.finish());
            body.compile()
        })
    }

    /// Ruling 3: a column into a store that adds a term runs a variant at a vein, which reads an unknown and so makes
    /// the energy book regulated, and with no vein the body is the one with no term.
    #[test]
    fn a_column_that_adds_a_term_runs_it_at_a_vein() {
        let energy = |body: &CompiledBody| {
            (body.conserved_basis().combinations().iter())
                .find(|combination| format!("{:?}", combination.book()).contains("Energy"))
                .map(|combination| combination.regulated())
        };
        let (marked, plain) = (
            stroke(&[0], true).expect("compiled"),
            stroke(&[0], false).expect("compiled"),
        );
        let (none, bare) = (
            stroke(&[], true).expect("compiled"),
            stroke(&[], false).expect("compiled"),
        );
        assert_eq!(none.model_hash(), bare.model_hash());
        assert_eq!(sealed_all(&none), sealed_all(&bare));
        let main_programs = sealed_all(&plain);
        assert_eq!(sealed_all(&marked)[..main_programs.len()], main_programs[..]);
        assert_eq!(
            (energy(&plain), energy(&none), energy(&marked)),
            (Some(false), Some(false), Some(true))
        );
    }

    /// Two laws whose variants have one code keep a program each, so a refusal in either names its own law.
    #[test]
    fn variants_of_two_laws_with_one_code_stay_apart() {
        let fix = Fix {
            marks: vec![0],
            veins: term("collapse", t_param),
            hosts: vec![("total", h_param as BloodFn, true), ("whole", h_param, true)],
            ..Fix::default()
        };
        let (body, _) = build(&fix).expect("compiled");
        for instance in body.instances() {
            if let Some(law) = row_of(&body, instance) {
                assert_eq!(body.programs()[instance.program().index()].law, law);
            }
        }
    }

    /// Ruling 4: of two reads that are not finite, the first in program order names its law, here the host's.
    #[test]
    fn of_two_reads_that_are_not_finite_the_first_names_its_law() {
        let fix = Fix {
            marks: vec![0],
            veins: term("collapse", t_ln),
            hosts: two(h_level),
            ..Fix::default()
        };
        let (body, [_, _, upper, _]) = build(&fix).expect("compiled");
        let mut point = Point::new(&body);
        point.know("level", Some(upper), f64::NAN);
        point.set("cv.t", f64::NAN);
        let refused = point.values_at(upper).expect("refused");
        assert_ne!(refused.law, cv("collapse"));
        assert_eq!(format!("{:?}", refused.domain), "NonFinite");
    }

    /// Ruling 5 of the fix round: a slope that is not finite names the law of the read it belongs to, so the host for a
    /// read the host shares and the term for a read only the term makes.
    #[test]
    fn a_slope_that_is_not_finite_names_the_law_of_its_read() {
        let jacobian = |body: &CompiledBody, point: &Point<'_>| {
            body.jacobian(&point.inputs(), 0.0).expect_err("refused")
        };
        let shared = Fix {
            marks: vec![0],
            veins: term("collapse", t_ln_level),
            hosts: two(h_level),
            ..Fix::default()
        };
        let (body, [_, _, upper, _]) = build(&shared).expect("compiled");
        let mut point = Point::new(&body);
        point.know("level", Some(upper), 1.0e-320);
        let refused = jacobian(&body, &point);
        assert_ne!(refused.law, cv("collapse"));
        assert_eq!(format!("{:?}", refused.domain), "NonFinite");
        let own = Fix {
            marks: vec![0],
            veins: term("collapse", t_y_times_t),
            hosts: two(h_big),
            ..Fix::default()
        };
        let (body, _) = build(&own).expect("compiled");
        let mut point = Point::new(&body);
        point.set("cv.share", 1.0e200);
        point.set("cv.t", 1.0e200);
        point.know("depth", None, 0.0);
        assert_eq!(jacobian(&body, &point).law, cv("collapse"));
    }

    fn h_shared_ln_by_hand<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let level = k.at_site(&d.x);
        let log = k.ln(level);
        let sum = k.add(level, log);
        k.scale(sum, log)
    }
    fn h_term_then_ln<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let level = k.at_site(&d.x);
        let sum = plus(k, d, level);
        let log = k.ln(level);
        k.scale(sum, log)
    }

    /// Fix round 2: an op the host and the term both compute is one op, so the program and its slopes are those of
    /// the law written with that op bound once, bit for bit. A law that writes the op twice adds its slopes in another
    /// order and may differ near overflow.
    #[test]
    fn a_shared_op_has_the_slopes_of_the_law_with_it_bound_once() {
        let fix = |veins: Vec<(&'static str, VeinFn)>, host: BloodFn| Fix {
            marks: vec![0],
            veins,
            hosts: vec![("probe", host, false)],
            ..Fix::default()
        };
        let (placed, [_, _, upper, _]) =
            build(&fix(term("collapse", t_ln_level), h_term_then_ln)).expect("compiled");
        let (by_hand, _) = build(&fix(Vec::new(), h_shared_ln_by_hand)).expect("compiled");
        assert_eq!(ops_of(&placed, "probe", upper), ops_of(&by_hand, "probe", upper));
        let row_slopes = |body: &CompiledBody| {
            let mut point = Point::new(body);
            point.know("level", None, 0.625);
            let entries = body.jacobian(&point.inputs(), 0.0).expect("finite");
            let rows = body.rows();
            let sparsity = body.sparsity();
            let at = (rows.iter())
                .position(|row| matches!(row.kind(), RowKind::Algebraic(slot) if slot.law == cv("probe") && slot.at == Context::Site(upper)))
                .expect("a row");
            (sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1])
                .map(|entry| {
                    (
                        rows[sparsity.unknowns()[entry].index()].kind(),
                        entries[entry].to_bits(),
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(row_slopes(&placed), row_slopes(&by_hand));
    }

    fn t_clock_bare<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let (cosine, _) = k.clock();
        let level = k.at_site_in(&d.x);
        k.scale(level, cosine)
    }
    fn t_clock_dead_param<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let _ = k.site_parameter(&d.t);
        t_clock_bare(k, d)
    }
    fn t_forcing_dead<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let _ = k.forcing(d.wobble.as_ref().expect("forcing"));
        k.site_parameter(&d.t)
    }
    fn h_level_dead_share<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let _ = k.site_parameter(&d.share);
        h_level(k, d)
    }
    fn h_level_dead_forcing<'b, 'k>(k: &mut K<'b, 'k, Blood>, d: &Decl<'b>) -> X<'b, 'k> {
        let _ = k.forcing(d.wobble.as_ref().expect("forcing"));
        h_level(k, d)
    }
    fn hand_clock_dead_param<'b, 'k>(k: &mut K<'b, 'k, Veins>, d: &Decl<'b>) -> X<'b, 'k> {
        let base = k.at_site_in(&d.x);
        let (cosine, _) = k.clock();
        let _ = k.site_parameter(&d.t);
        let wave = k.scale(base, cosine);
        k.add(base, wave)
    }
    fn bt_param<'b, 'k>(k: &mut K<'b, 'k, CapillaryBed>, d: &Decl<'b>) -> X<'b, 'k> {
        k.site_parameter(&d.b)
    }
    fn bt_clock<'b, 'k>(k: &mut K<'b, 'k, CapillaryBed>, d: &Decl<'b>) -> X<'b, 'k> {
        let (cosine, _) = k.clock();
        let level = k.at_site_in(&d.x);
        k.scale(level, cosine)
    }

    /// Ruling 5 of fix round 2: a variant is judged as main judges a program, pruned, so a read nothing uses neither
    /// passes a check nor fails one. A parameter or a forcing a term or its host reads and never uses is refused at a
    /// marked vein as main refuses the law written by hand.
    #[test]
    fn a_read_nothing_uses_neither_passes_nor_fails_a_check_at_a_marked_vein() {
        let unscaled = |fix: &Fix| matches!(refused(fix), Some(CompileDefect::UnscaledInput { .. }));
        let by_hand = Fix {
            marks: vec![0],
            hands: vec![("probe", hand_clock_dead_param as VeinFn)],
            ..Fix::default()
        };
        assert!(unscaled(&by_hand));
        let marked = |veins, hosts| Fix {
            marks: vec![0],
            veins,
            hosts,
            ..Fix::default()
        };
        assert!(unscaled(&marked(
            term("collapse", t_clock_dead_param),
            two(h_level)
        )));
        assert!(unscaled(&marked(
            term("collapse", t_clock_bare),
            two(h_level_dead_share)
        )));
        let unread = Some(CompileDefect::UnreadForcing {
            forcing: cv("wobble"),
        });
        for (veins, hosts) in [
            (term("collapse", t_forcing_dead), two(h_level)),
            (term("collapse", t_param), two(h_level_dead_forcing)),
        ] {
            assert_eq!(
                refused(&Fix {
                    forcing: true,
                    ..marked(veins, hosts)
                }),
                unread
            );
        }
    }

    /// Ruling 8 of fix round 2: two terms on different sets are each judged only where their own set holds the site.
    #[test]
    fn two_terms_on_different_sets_are_each_judged_at_their_own_sites() {
        let fix = |marks: Vec<usize>, vein: VeinFn, bed: BedFn| Fix {
            marks,
            veins: term("vein part", vein),
            beds: vec![("bed part", bed)],
            hosts: two(h_level),
            ..Fix::default()
        };
        let unscaled = |fix: &Fix| matches!(refused(fix), Some(CompileDefect::UnscaledInput { .. }));
        assert!(unscaled(&fix(vec![], t_param, bt_clock)));
        assert!(unscaled(&fix(vec![0], t_param, bt_clock)));
        assert!(unscaled(&fix(vec![0], t_clock_bare, bt_param)));
        assert_eq!(refused(&fix(vec![], t_clock_bare, bt_param)), None);
    }
}
