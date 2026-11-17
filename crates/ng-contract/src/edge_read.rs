use crate::boundary::DriveFrame;
use crate::evaluate::Inputs;
use crate::*;

const DEBT: [f64; 2] = [0.375, 1.625];
const G: f64 = 2.5;

fn measure<U: Dimension>(value: f64) -> Measure<U> {
    Measure::new(value).expect("finite")
}

fn at() -> Position {
    Position::new(measure(0.0), measure(0.0), measure(0.0))
}

fn cv(local: &'static str) -> LawName {
    LawName { owner: "CV", local }
}

fn compiled(with_beds: bool) -> Result<(CompiledBody, Vec<SiteId>), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at());
    let artery = map.site::<VesselSegment>("artery", at());
    let vein = map.site::<VesselSegment>("vein", at());
    let sites = if with_beds {
        let first = map.site::<CapillaryBed>("first bed", at());
        let second = map.site::<CapillaryBed>("second bed", at());
        vec![artery.id(), first.id(), vein.id(), second.id()]
    } else {
        vec![artery.id(), vein.id()]
    };
    for (at, &site) in sites.iter().enumerate() {
        map.contain(root.id(), site);
        map.connect(Network::Vascular, site, sites[(at + 1) % sites.len()]);
    }
    map.unsupplied(&sites);
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    schema.leave_out(Space::Interstitium, "a bench with no tissue");
    schema.leave_out(Space::Cells, "a bench with no tissue");
    let body = assemble(schema, |mut body| {
        let class = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let key = ParameterKey::new;
        let g = cv.edge_parameter::<VascularEdge, Rate<Water>>(key("cv.g"), Admissible::finite());
        let per_bed = cv.site_parameter::<CapillaryBed, Dimensionless>(key("cv.debt"), Admissible::finite());
        let (out, debt) = cv.site_field::<CapillaryBed, Dimensionless>("debt", Admissible::finite());
        let value = cv.site_kernel(|mut k| {
            let value = k.site_parameter(&per_bed);
            k.finish(value, cv.every_mode(), class)
        });
        cv.define_at(&out, value);
        let flow = cv.edge_kernel(|mut k| {
            let g = k.edge_parameter(&g);
            let d = k.downstream_if_in(&debt);
            let scaled = k.scale(g, d);
            let value = k.add(scaled, g);
            k.finish(value, cv.every_mode(), class)
        });
        cv.transfer_along(&route, flow);
        body.add(cv.finish());
        body.compile()
    })?;
    Ok((body, sites))
}

fn bound(body: &CompiledBody, beds: [SiteId; 2]) -> BoundParameters {
    let mut packet: ParameterPacket<SyntheticFixture> = ParameterPacket::new();
    let note = "synthetic";
    for (bed, site) in beds.into_iter().enumerate() {
        let value = measure::<Dimensionless>(DEBT[bed]);
        packet
            .synthetic(ParameterKey::new("cv.debt"), Binding::Site(site), value, note)
            .expect("once");
    }
    let vascular = Binding::Network(Network::Vascular);
    packet
        .synthetic(
            ParameterKey::new("cv.g"),
            vascular,
            measure::<Rate<Water>>(G),
            note,
        )
        .expect("once");
    let start = |row: RowKind| match row {
        RowKind::Algebraic(slot) => (beds.iter())
            .position(|site| slot.at == Context::Site(*site))
            .map_or(0.0, |bed| DEBT[bed]),
        RowKind::Amount { .. } => 10.0,
        _ => 0.0,
    };
    packet.start_rest(body, start, note).expect("once");
    body.bind(packet).expect("bound")
}

#[test]
fn each_beds_inflow_edge_reads_its_beds_value_and_every_other_edge_reads_zero() {
    let (body, sites) = compiled(true).unwrap_or_else(|refused| panic!("{refused:?}"));
    let beds = [sites[1], sites[3]];
    let bound = bound(&body, beds);
    let frame = DriveFrame::default();
    let unknowns: Vec<f64> = (body.rows().iter())
        .map(|row| bound.starts[row.id().index()])
        .collect();
    let inputs = Inputs {
        parameters: &bound.values,
        frame: &frame,
        unknowns: &unknowns,
        modes: &bound.modes,
        draws: &[],
        factors: &[],
    };
    let edge = |from: usize, to: usize| Context::Edge {
        from: sites[from],
        to: sites[to],
    };
    let fed = [
        (edge(0, 1), Some(0)),
        (edge(1, 2), None),
        (edge(2, 3), Some(1)),
        (edge(3, 0), None),
    ];
    let values = body.instance_values(&inputs).expect("finite");
    let instances = body.layout.instances.iter().zip(&values);
    let mut found: Vec<(Context, u64)> = instances
        .filter(|(instance, _)| body.layout.programs[instance.program.index()].law == cv("circulation"))
        .map(|(instance, value)| (instance.context, value.expect("a value").to_bits()))
        .collect();
    found.sort_by_key(|(context, _)| fed.iter().position(|(edge, _)| edge == context));
    let twin: Vec<(Context, u64)> = (fed.iter())
        .map(|&(edge, bed)| (edge, (G * bed.map_or(0.0, |bed| DEBT[bed]) + G).to_bits()))
        .collect();
    assert_ne!(DEBT[0].to_bits(), DEBT[1].to_bits());
    assert_eq!(found, twin);

    let jacobian = body.jacobian(&inputs, 0.0).expect("finite");
    let (sparsity, rows) = (body.sparsity(), body.rows());
    let mut slopes = Vec::new();
    for (at, row) in rows.iter().enumerate() {
        let RowKind::Amount { site, .. } = row.kind() else {
            continue;
        };
        for entry in sparsity.row_offsets()[at]..sparsity.row_offsets()[at + 1] {
            let column = rows[sparsity.unknowns()[entry].index()].kind();
            if let RowKind::Algebraic(Slot {
                law,
                at: Context::Site(bed),
            }) = column
                && law == cv("debt")
            {
                slopes.push((site, bed, jacobian[entry].to_bits()));
            }
        }
    }
    slopes.sort_unstable();
    let mut by_hand = Vec::new();
    for (bed, (from, to)) in [(0, (sites[0], sites[1])), (1, (sites[2], sites[3]))] {
        by_hand.push((from, beds[bed], G.to_bits()));
        by_hand.push((to, beds[bed], (-G).to_bits()));
    }
    by_hand.sort_unstable();
    assert_eq!(slopes, by_hand);
}

#[test]
fn a_downstream_reads_set_is_in_its_program_seal() {
    use crate::body::{Code, Program};
    use crate::identity::{Canonical, Seal};
    use crate::law::{Input, Op, Where};

    let (body, _) = compiled(true).unwrap_or_else(|refused| panic!("{refused:?}"));
    let program = body
        .programs()
        .iter()
        .find(|program| program.law == cv("circulation"))
        .expect("the edge law");
    let code = &program.code;
    let mut ops = code.ops.clone();
    let mut changed = 0;
    for op in &mut ops {
        if let Op::Input(Input::Read(_, Where::DownIn(set))) = op {
            assert_eq!(*set, CapillaryBed::SET);
            *set = Speciated::SET;
            changed += 1;
        }
    }
    assert_eq!(changed, 1);
    let changed = Program {
        id: program.id,
        law: program.law,
        code: Code {
            ops,
            dimension: code.dimension.clone(),
            predicate: code.predicate,
            rate: code.rate,
            validity: code.validity.clone(),
            defines: code.defines,
            origins: code.origins.clone(),
            judged: code.judged.clone(),
        },
    };
    let seal = |program: &Program| {
        let mut out = Canonical::new("program");
        program.seal(&mut out);
        out.digest()
    };
    assert_ne!(program, &changed);
    assert_ne!(seal(program), seal(&changed));
}

#[test]
fn a_law_none_of_whose_edges_ends_in_the_set_is_refused_by_name() {
    let refused = compiled(false).map(|_| ()).err();
    let law = cv("circulation");
    assert_eq!(
        refused,
        Some(AdmissionError::Compile(CompileDefect::EndOffSet { law }))
    );
}

#[test]
fn an_edge_guard_none_of_whose_edges_ends_in_the_set_is_refused_by_name() {
    fn compiled(with_beds: bool) -> Result<(CompiledBody, Vec<SiteId>), AdmissionError> {
        let mut map = AnatomyBuilder::new();
        let root = map.site::<TissueSite>("body", at());
        let artery = map.site::<VesselSegment>("artery", at());
        let vein = map.site::<VesselSegment>("vein", at());
        let sites = if with_beds {
            let first = map.site::<CapillaryBed>("first bed", at());
            let second = map.site::<CapillaryBed>("second bed", at());
            vec![artery.id(), first.id(), vein.id(), second.id()]
        } else {
            vec![artery.id(), vein.id()]
        };
        for (at, &site) in sites.iter().enumerate() {
            map.contain(root.id(), site);
            map.connect(Network::Vascular, site, sites[(at + 1) % sites.len()]);
        }
        map.unsupplied(&sites);
        let mut schema = Schema::new(map.finish()?)?;
        schema.partial_volume(Measure::new(1.0)?)?;
        schema.leave_out(Space::Interstitium, "a bench with no tissue");
        schema.leave_out(Space::Cells, "a bench with no tissue");
        let body = assemble(schema, |mut body| {
            let class = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
            let (mut cv, ports) = body.cardiovascular()?.parts();
            let route = ports.circulation(&mut cv);
            let key = ParameterKey::new;
            let g = cv.edge_parameter::<VascularEdge, Rate<Water>>(key("cv.g"), Admissible::finite());
            let per_bed =
                cv.site_parameter::<CapillaryBed, Dimensionless>(key("cv.debt"), Admissible::finite());
            let (out, debt) = cv.site_field::<CapillaryBed, Dimensionless>("debt", Admissible::finite());
            let value = cv.site_kernel(|mut k| {
                let value = k.site_parameter(&per_bed);
                k.finish(value, cv.every_mode(), class)
            });
            cv.define_at(&out, value);
            let flow = cv.edge_kernel(|mut k| {
                let g = k.edge_parameter(&g);
                k.finish(g, cv.every_mode(), class)
            });
            cv.transfer_along(&route, flow);
            let [on, off] = cv.modes_along(&route, "switch", ["on", "off"]);
            let gap = cv.edge_kernel::<VascularEdge, _, _>(|mut k| {
                let d = k.downstream_if_in(&debt);
                k.finish(d, on.only(), class)
            });
            cv.event(Guard::new(gap, Crossing::Rising, on.to(&off)?));
            body.add(cv.finish());
            body.compile()
        })?;
        Ok((body, sites))
    }

    compiled(true).unwrap_or_else(|refused| panic!("{refused:?}"));
    let refused = compiled(false).map(|_| ()).err();
    let law = cv("switch");
    assert_eq!(
        refused,
        Some(AdmissionError::Compile(CompileDefect::EndOffSet { law }))
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Reads {
    DebtThenTone,
    ToneThenDebt,
    Plain,
    ValveRate,
    Discarded,
    BodyGuard,
}

fn stated<'b, D: SiteSet>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    name: &'static str,
    class: RateClass,
) -> ReadPort<'b, Cardiovascular, PerSite<D, Dimensionless>>
where
    Blood: Includes<D>,
{
    let held = cv.site_parameter::<Blood, Dimensionless>(ParameterKey::new("cv.held"), Admissible::finite());
    let (out, read) = cv.site_field::<D, Dimensionless>(name, Admissible::finite());
    let value = cv.site_kernel::<D, _, _>(|mut k| {
        let value = k.site_parameter_in(&held);
        k.finish(value, cv.every_mode(), class)
    });
    cv.define_at(&out, value);
    read
}

fn ring_without_veins(valves: &[usize], reads: Reads) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at());
    let sites = [
        map.site::<VesselSegment>("artery", at()).id(),
        map.site::<CapillaryBed>("first bed", at()).id(),
        map.site::<VesselSegment>("vein", at()).id(),
        map.site::<CapillaryBed>("second bed", at()).id(),
    ];
    for (edge, &site) in sites.iter().enumerate() {
        map.contain(root.id(), site);
        let to = sites[(edge + 1) % sites.len()];
        if valves.contains(&edge) {
            map.valve(site, to);
        } else {
            map.connect(Network::Vascular, site, to);
        }
    }
    map.unsupplied(&sites);
    let mut schema = Schema::new(map.finish()?)?;
    schema.partial_volume(Measure::new(1.0)?)?;
    schema.leave_out(Space::Interstitium, "a bench with no tissue");
    schema.leave_out(Space::Cells, "a bench with no tissue");
    assemble(schema, |mut body| {
        let class = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let key = ParameterKey::new;
        let g = cv.edge_parameter::<VascularEdge, Rate<Water>>(key("cv.g"), Admissible::finite());
        let debt = stated::<CapillaryBed>(&mut cv, "debt", class);
        let tone = (reads == Reads::DebtThenTone || reads == Reads::ToneThenDebt)
            .then(|| stated::<Veins>(&mut cv, "tone", class));
        let flow = cv.edge_kernel::<VascularEdge, _, _>(|mut k| {
            let g = k.edge_parameter(&g);
            let value = match (reads, &tone) {
                (Reads::DebtThenTone | Reads::ToneThenDebt, Some(tone)) => {
                    let (d, t) = (k.downstream_if_in(&debt), k.downstream_if_in(tone));
                    let (by_debt, by_tone) = (k.scale(g, d), k.scale(g, t));
                    match reads {
                        Reads::DebtThenTone => k.add(by_debt, by_tone),
                        _ => k.add(by_tone, by_debt),
                    }
                }
                (Reads::Plain, _) => {
                    let d = k.downstream_if_in(&debt);
                    k.scale(g, d)
                }
                (Reads::Discarded, _) => {
                    let _ = k.downstream_if_in(&debt);
                    g
                }
                _ => g,
            };
            k.finish(value, cv.every_mode(), class)
        });
        cv.transfer_along(&route, flow);
        let gates = ports.valves(&mut cv);
        let memory = gates.flow_memory();
        let r = cv.edge_parameter::<ValveEdge, Rate<Rate<Water>>>(key("cv.r"), Admissible::finite());
        let rate = cv.edge_kernel::<ValveEdge, _, _>(|mut k| {
            let r = k.edge_parameter(&r);
            let value = if reads == Reads::ValveRate {
                let d = k.downstream_if_in(&debt);
                k.scale(r, d)
            } else {
                r
            };
            k.finish(value, cv.every_mode(), class)
        });
        cv.memory_rate(&memory, rate);
        if reads == Reads::BodyGuard {
            let [on, off] = cv.modes("switch", ["on", "off"]);
            let gap = cv.edge_kernel::<VascularEdge, _, _>(|mut k| {
                let d = k.downstream_if_in(&debt);
                k.finish(d, on.only(), class)
            });
            cv.event(Guard::new(gap, Crossing::Rising, on.to(&off)?));
        }
        body.add(cv.finish());
        body.compile()
    })
}

fn end_off_set(local: &'static str) -> Option<AdmissionError> {
    let law = cv(local);
    Some(AdmissionError::Compile(CompileDefect::EndOffSet { law }))
}

#[test]
fn a_kernel_that_reads_two_sets_is_refused_when_either_misses_in_either_order() {
    for reads in [Reads::DebtThenTone, Reads::ToneThenDebt] {
        assert_eq!(ring_without_veins(&[], reads).err(), end_off_set("circulation"));
    }
}

#[test]
fn a_law_whose_edges_only_leave_the_set_is_refused() {
    assert_eq!(
        ring_without_veins(&[0, 2], Reads::ValveRate).map(|_| ()).err(),
        None
    );
    assert_eq!(
        ring_without_veins(&[0, 2], Reads::Plain).err(),
        end_off_set("circulation")
    );
}

#[test]
fn a_law_with_no_edge_at_all_is_refused_when_it_states_the_read() {
    assert_eq!(ring_without_veins(&[], Reads::Plain).map(|_| ()).err(), None);
    assert_eq!(
        ring_without_veins(&[], Reads::ValveRate).err(),
        end_off_set("valves")
    );
}

#[test]
fn a_read_the_law_discards_is_still_checked() {
    assert_eq!(ring_without_veins(&[], Reads::Discarded).map(|_| ()).err(), None);
    assert_eq!(
        ring_without_veins(&[0, 2], Reads::Discarded).err(),
        end_off_set("circulation")
    );
}

#[test]
fn a_guard_on_a_body_wide_mode_that_states_the_read_is_refused() {
    assert_eq!(
        ring_without_veins(&[], Reads::BodyGuard).err(),
        end_off_set("switch")
    );
}
