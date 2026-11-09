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
    expect!["b73b6912ec52bfab7cd378e16d9be3b630c611203cd82a1af14ed10d9b223255"].assert_eq(&named);
    expect!["9975252e08b9571e69b507c133ee30aea383f32564d5332967d6ca829056eeb4"].assert_eq(&sealed);
    expect![[r#"
         100.000000 ms  arterial pressure 125.034455  right atrial pressure 1.617394  heart rate 1.053684  sympathetic traffic 0.709794  filtration 1.709744  urine 0.020000  wall power 14119.392387  aortic flow 0.000000
         200.000000 ms  arterial pressure 120.538735  right atrial pressure 2.047878  heart rate 1.070960  sympathetic traffic 0.129996  filtration 2.082523  urine 0.020000  wall power 10381.269823  aortic flow 39.901422
         300.000000 ms  arterial pressure 114.952630  right atrial pressure 2.409895  heart rate 1.045808  sympathetic traffic 0.035414  filtration 2.289627  urine 0.020000  wall power 34172.503681  aortic flow 0.000000
         400.000000 ms  arterial pressure 108.094965  right atrial pressure 2.236688  heart rate 1.008752  sympathetic traffic 0.025692  filtration 2.355712  urine 0.020000  wall power 10962.345897  aortic flow 0.000000
         500.000000 ms  arterial pressure 101.750158  right atrial pressure 0.687175  heart rate 0.977374  sympathetic traffic 0.037741  filtration 2.314930  urine 0.020000  wall power 8834.665940  aortic flow 0.000000
         600.000000 ms  arterial pressure 95.840930  right atrial pressure 0.260726  heart rate 0.956181  sympathetic traffic 0.083351  filtration 2.197128  urine 0.020000  wall power 6639.067239  aortic flow 0.000000
         700.000000 ms  arterial pressure 90.303691  right atrial pressure 0.296159  heart rate 0.947383  sympathetic traffic 0.214704  filtration 2.035474  urine 0.020000  wall power 5155.672110  aortic flow 0.000000
         800.000000 ms  arterial pressure 85.099323  right atrial pressure 0.522489  heart rate 0.953604  sympathetic traffic 0.486405  filtration 1.856741  urine 0.020000  wall power 4337.566670  aortic flow 0.000000
         900.000000 ms  arterial pressure 80.215792  right atrial pressure 0.766121  heart rate 0.975960  sympathetic traffic 0.777711  filtration 1.675287  urine 0.020000  wall power 3738.096527  aortic flow 0.000000
        1000.000000 ms  arterial pressure 75.650158  right atrial pressure 0.978124  heart rate 1.011713  sympathetic traffic 0.929052  filtration 1.498346  urine 0.020000  wall power 3233.475834  aortic flow 0.000000
    "#]].assert_eq(&trace);
}

#[test]
fn the_first_second_of_the_slice_on_ng_solver_stays_where_it_was() {
    let FirstSecond { named, trace, sealed } = first_second(&ng_solver::Core::new(10 * MILLISECOND));
    expect!["eeb726c2b0974fe97cb809fc3a9e32b1aacae59b9f41beb121d61e43fa500129"].assert_eq(&named);
    expect!["1f9a4083420d676edfe35678d5625570d3b93896f00fa5fe1e50730c10a94276"].assert_eq(&sealed);
    expect![[r#"
         100.976282 ms  arterial pressure 124.892593  right atrial pressure 1.621228  heart rate 1.053980  sympathetic traffic 0.701829  filtration 1.714747  urine 0.020000  wall power 14084.714534  aortic flow 0.000000
         203.732841 ms  arterial pressure 120.451780  right atrial pressure 2.061712  heart rate 1.070090  sympathetic traffic 0.123378  filtration 2.091194  urine 0.020000  wall power 10303.511849  aortic flow 37.681462
         301.933952 ms  arterial pressure 114.715182  right atrial pressure 2.413377  heart rate 1.044606  sympathetic traffic 0.035894  filtration 2.288238  urine 0.020000  wall power 33352.656889  aortic flow 0.000000
         405.975048 ms  arterial pressure 107.614406  right atrial pressure 2.112523  heart rate 1.006598  sympathetic traffic 0.026813  filtration 2.350821  urine 0.020000  wall power 10714.649980  aortic flow 0.000000
         502.679847 ms  arterial pressure 101.508134  right atrial pressure 0.699883  heart rate 0.976998  sympathetic traffic 0.039853  filtration 2.307114  urine 0.020000  wall power 8775.852881  aortic flow 0.000000
         608.059522 ms  arterial pressure 95.314397  right atrial pressure 0.267472  heart rate 0.955824  sympathetic traffic 0.092960  filtration 2.180273  urine 0.020000  wall power 6564.546065  aortic flow 0.000000
         708.059522 ms  arterial pressure 89.814211  right atrial pressure 0.310531  heart rate 0.948740  sympathetic traffic 0.237653  filtration 2.017851  urine 0.020000  wall power 5112.736809  aortic flow 0.000000
         808.059522 ms  arterial pressure 84.645257  right atrial pressure 0.533318  heart rate 0.956922  sympathetic traffic 0.519874  filtration 1.839884  urine 0.020000  wall power 4295.204966  aortic flow 0.000000
         908.059522 ms  arterial pressure 79.798020  right atrial pressure 0.775253  heart rate 0.981027  sympathetic traffic 0.799579  filtration 1.659492  urine 0.020000  wall power 3697.335290  aortic flow 0.000000
        1000.229928 ms  arterial pressure 75.611325  right atrial pressure 0.972800  heart rate 1.014656  sympathetic traffic 0.930553  filtration 1.497348  urine 0.020000  wall power 3165.684347  aortic flow 0.000000
    "#]].assert_eq(&trace);
}
