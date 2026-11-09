use core::fmt::Write;
use expect_test::expect;
use ng_contract::*;
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = || Measure::new(0.0).expect("finite");
    AnatomyBuilder::new()
        .site::<C>(key, Position::new(origin(), origin(), origin()))
        .id()
}

fn field(owner: &'static str, local: &'static str, at: Context) -> RowKind {
    let law = LawName { owner, local };
    RowKind::Algebraic(Slot { law, at })
}

fn hex(digest: Digest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

struct FirstSecond {
    named: String,
    trace: String,
    sealed: String,
}

fn first_second(core: &dyn CoupledSolver) -> FirstSecond {
    let store = MemoryStore::default();
    let body = arena_body().expect("compiled");
    let (packet, lens) = (
        arena_packet(&body).expect("a packet"),
        arena_lens(&body).expect("a lens"),
    );
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let admitted = admit_at(&body, packet, port, core, &lens, anchor_moment(), &store).expect("admitted");
    let named = hex(admitted.session.record().identity().packet().digest());
    let at = |key| Context::Site(site::<VesselSegment>(key));
    let (arteries, atrium) = (at("systemic arteries"), at("right atrium"));
    let (ventricle, tubule) = (
        site::<VesselSegment>("left ventricle"),
        site::<VesselSegment>("renal tubule"),
    );
    let bed = site::<CapillaryBed>("glomerular bed");
    let aortic = Context::Edge {
        from: ventricle,
        to: site::<VesselSegment>("systemic arteries"),
    };
    let kinds = [
        ("arterial pressure", field("CV", "blood pressure", arteries)),
        ("right atrial pressure", field("CV", "blood pressure", atrium)),
        ("heart rate", field("CV", "rate", Context::Body)),
        (
            "sympathetic traffic",
            field("NS", "sympathetic traffic", Context::Body),
        ),
        (
            "filtration",
            field(
                "RN",
                "filtration",
                Context::Edge {
                    from: bed,
                    to: tubule,
                },
            ),
        ),
        ("urine", field("Boundary", "urine", Context::Body)),
        ("wall power", field("CV", "wall power", Context::Body)),
        (
            "aortic flow",
            RowKind::Memory(Slot {
                law: LawName {
                    owner: "CV",
                    local: "valves",
                },
                at: aortic,
            }),
        ),
    ];
    let mut session = admitted.session;
    let mut trace = String::new();
    let start = anchor_moment().nanoseconds();
    for tenth in 1..=10 {
        let end = SimTime::from_ns(start + tenth * 100 * MILLISECOND);
        session = match run_until(session, end) {
            Ok(reached) => reached,
            Err(stopped) => panic!("{:?}", stopped.fault()),
        };
        let view = session.record().accepted();
        let held = (view.time().nanoseconds() - start) as f64 / 1e6;
        write!(trace, "{held:>11.6} ms").expect("a string takes it");
        for (name, kind) in kinds {
            let row = body.rows().iter().find(|row| row.kind() == kind).expect("a row");
            let value = f64::from_bits(view.value(row.id()).bits()[0]);
            write!(trace, "  {name} {value:.6}").expect("a string takes it");
        }
        trace.push('\n');
    }
    let sealed = hex(session.record().seal().digest());
    FirstSecond { named, trace, sealed }
}

#[test]
fn the_first_second_of_the_slice_stays_where_it_was() {
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let core = ReferenceIntegrator::new(MILLISECOND)
        .with_scales(ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive"));
    let FirstSecond { named, trace, sealed } = first_second(&core);
    expect!["5d6ab2db1d7674c39b75207a82be1b30f0c5b7b83e67b98ff5bbaa22b9049898"].assert_eq(&named);
    expect!["20e920b6e1f607733f72a6068c3ce7415b739a8589619032820474c7ded4a8ca"].assert_eq(&sealed);
    expect![[r#"
         100.000000 ms  arterial pressure 124.298734  right atrial pressure 1.716272  heart rate 1.057101  sympathetic traffic 0.630364  filtration 1.755444  urine 0.020000  wall power 13118.223094  aortic flow 0.000000
         200.000000 ms  arterial pressure 120.311921  right atrial pressure 2.143001  heart rate 1.069538  sympathetic traffic 0.108170  filtration 2.110528  urine 0.020000  wall power 10144.442151  aortic flow 42.814404
         300.000000 ms  arterial pressure 114.327131  right atrial pressure 2.501846  heart rate 1.041650  sympathetic traffic 0.032395  filtration 2.303265  urine 0.020000  wall power 29443.399884  aortic flow 0.000000
         400.000000 ms  arterial pressure 107.530077  right atrial pressure 2.112622  heart rate 1.004675  sympathetic traffic 0.025551  filtration 2.357694  urine 0.020000  wall power 10342.160509  aortic flow 0.000000
         500.000000 ms  arterial pressure 101.236563  right atrial pressure 0.659367  heart rate 0.974177  sympathetic traffic 0.039492  filtration 2.308485  urine 0.020000  wall power 8802.136952  aortic flow 0.000000
         600.000000 ms  arterial pressure 95.370257  right atrial pressure 0.286384  heart rate 0.954042  sympathetic traffic 0.089523  filtration 2.185104  urine 0.020000  wall power 6526.617944  aortic flow 0.000000
         700.000000 ms  arterial pressure 89.869675  right atrial pressure 0.346673  heart rate 0.946465  sympathetic traffic 0.231207  filtration 2.020817  urine 0.020000  wall power 5082.584367  aortic flow 0.000000
         800.000000 ms  arterial pressure 84.698383  right atrial pressure 0.579205  heart rate 0.954055  sympathetic traffic 0.512277  filtration 1.841368  urine 0.020000  wall power 4284.040822  aortic flow 0.000000
         900.000000 ms  arterial pressure 79.846345  right atrial pressure 0.819889  heart rate 0.977665  sympathetic traffic 0.795496  filtration 1.660161  urine 0.020000  wall power 3691.621171  aortic flow 0.000000
        1000.000000 ms  arterial pressure 75.310329  right atrial pressure 1.084829  heart rate 1.014277  sympathetic traffic 0.935656  filtration 1.484114  urine 0.020000  wall power 3321.425480  aortic flow 0.000000
    "#]].assert_eq(&trace);
}

#[test]
fn the_first_second_of_the_slice_on_ng_solver_stays_where_it_was() {
    let FirstSecond { named, trace, sealed } = first_second(&ng_solver::Core::new(10 * MILLISECOND));
    expect!["e070345013569ec0aea04d066e5d29f724a3270ed7dff8dc8c143d23de0cafef"].assert_eq(&named);
    expect!["b21635cb29a05455fa4546b9ef82f0d813d0288dd5f762482a94c023b682ad6b"].assert_eq(&sealed);
    expect![[r#"
         100.592184 ms  arterial pressure 124.700810  right atrial pressure 1.709504  heart rate 1.057133  sympathetic traffic 0.617464  filtration 1.762463  urine 0.020000  wall power 13223.984373  aortic flow 0.000000
         200.155919 ms  arterial pressure 120.757418  right atrial pressure 2.135053  heart rate 1.068203  sympathetic traffic 0.102869  filtration 2.117650  urine 0.020000  wall power 10217.008010  aortic flow 34.391577
         303.861539 ms  arterial pressure 114.407882  right atrial pressure 2.503457  heart rate 1.038176  sympathetic traffic 0.029792  filtration 2.314721  urine 0.020000  wall power 28315.601472  aortic flow 0.000000
         406.141099 ms  arterial pressure 107.465656  right atrial pressure 1.985836  heart rate 1.000538  sympathetic traffic 0.024271  filtration 2.363877  urine 0.020000  wall power 10211.068642  aortic flow 0.000000
         503.538088 ms  arterial pressure 101.342437  right atrial pressure 0.664917  heart rate 0.971369  sympathetic traffic 0.037916  filtration 2.311132  urine 0.020000  wall power 8784.081074  aortic flow 0.000000
         608.059522 ms  arterial pressure 95.220920  right atrial pressure 0.291757  heart rate 0.951066  sympathetic traffic 0.090646  filtration 2.178708  urine 0.020000  wall power 6499.173648  aortic flow 0.000000
         708.059522 ms  arterial pressure 89.734985  right atrial pressure 0.357818  heart rate 0.944455  sympathetic traffic 0.235797  filtration 2.012979  urine 0.020000  wall power 5080.042582  aortic flow 0.000000
         808.059522 ms  arterial pressure 84.576789  right atrial pressure 0.585918  heart rate 0.952975  sympathetic traffic 0.519962  filtration 1.833707  urine 0.020000  wall power 4279.615616  aortic flow 0.000000
         908.059522 ms  arterial pressure 79.737983  right atrial pressure 0.824484  heart rate 0.977300  sympathetic traffic 0.800639  filtration 1.653206  urine 0.020000  wall power 3686.246468  aortic flow 0.000000
        1001.258770 ms  arterial pressure 75.511901  right atrial pressure 1.074171  heart rate 1.011451  sympathetic traffic 0.932055  filtration 1.489720  urine 0.020000  wall power 3310.284221  aortic flow 0.000000
    "#]].assert_eq(&trace);
}
