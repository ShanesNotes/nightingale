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
    let body = arena_body().expect("compiled");
    let (packet, lens) = (
        arena_packet(&body).expect("a packet"),
        arena_lens(&body).expect("a lens"),
    );
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let admitted = admit_at(&body, packet, port, core, &lens, anchor_moment()).expect("admitted");
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
    expect!["76102706f42e42535b0dbe928b8b37f327f73211741352bd572ad36ad56ca998"].assert_eq(&named);
    expect!["37c32c2faa03b9be4e6f8c7b21d3d598679f8bf35441a1072a0141f050abc658"].assert_eq(&sealed);
    expect![[r#"
         100.000000 ms  arterial pressure 125.258703  right atrial pressure 1.978090  heart rate 1.060493  sympathetic traffic 0.627398  filtration 1.756965  urine 0.020000  wall power 13155.465232  aortic flow 0.000000
         200.000000 ms  arterial pressure 121.183896  right atrial pressure 2.408025  heart rate 1.072033  sympathetic traffic 0.099483  filtration 2.122865  urine 0.020000  wall power 10204.160071  aortic flow 43.787660
         300.000000 ms  arterial pressure 115.204711  right atrial pressure 2.776252  heart rate 1.042311  sympathetic traffic 0.028103  filtration 2.323099  urine 0.020000  wall power 29955.997841  aortic flow 0.000000
         400.000000 ms  arterial pressure 108.377463  right atrial pressure 2.356706  heart rate 1.003952  sympathetic traffic 0.021436  filtration 2.381799  urine 0.020000  wall power 10520.792824  aortic flow 0.000000
         500.000000 ms  arterial pressure 102.058794  right atrial pressure 0.857878  heart rate 0.972383  sympathetic traffic 0.032581  filtration 2.334157  urine 0.020000  wall power 9219.044074  aortic flow 0.000000
         600.000000 ms  arterial pressure 96.168856  right atrial pressure 0.447726  heart rate 0.951027  sympathetic traffic 0.073782  filtration 2.210139  urine 0.020000  wall power 6765.692241  aortic flow 0.000000
         700.000000 ms  arterial pressure 90.643009  right atrial pressure 0.484389  heart rate 0.941721  sympathetic traffic 0.195118  filtration 2.043952  urine 0.020000  wall power 5206.823830  aortic flow 0.000000
         800.000000 ms  arterial pressure 85.442489  right atrial pressure 0.710306  heart rate 0.947047  sympathetic traffic 0.458271  filtration 1.862544  urine 0.020000  wall power 4367.925766  aortic flow 0.000000
         900.000000 ms  arterial pressure 80.555990  right atrial pressure 0.950116  heart rate 0.968385  sympathetic traffic 0.758488  filtration 1.680027  urine 0.020000  wall power 3762.427087  aortic flow 0.000000
        1000.000000 ms  arterial pressure 75.983019  right atrial pressure 1.196612  heart rate 1.003336  sympathetic traffic 0.921874  filtration 1.502906  urine 0.020000  wall power 3160.158064  aortic flow 0.000000
    "#]].assert_eq(&trace);
}

#[test]
fn the_first_second_of_the_slice_on_ng_solver_stays_where_it_was() {
    let FirstSecond { named, trace, sealed } = first_second(&ng_solver::Core::new(10 * MILLISECOND));
    expect!["0ec011d81366c367549a2fcdfe72146eb4c00ac911c7610dd2572dcf6b732887"].assert_eq(&named);
    expect!["5ec15539788f8c56202e46224ebc84e72e58c5b357a5b3704610065a4d3e1711"].assert_eq(&sealed);
    expect![[r#"
         101.886157 ms  arterial pressure 125.610022  right atrial pressure 1.972744  heart rate 1.060968  sympathetic traffic 0.603080  filtration 1.770138  urine 0.020000  wall power 13182.602061  aortic flow 0.000000
         201.324380 ms  arterial pressure 121.641644  right atrial pressure 2.401322  heart rate 1.070452  sympathetic traffic 0.092120  filtration 2.133878  urine 0.020000  wall power 10260.466255  aortic flow 34.425720
         302.444320 ms  arterial pressure 115.415385  right atrial pressure 2.770258  heart rate 1.039232  sympathetic traffic 0.025867  filtration 2.333827  urine 0.020000  wall power 29125.846457  aortic flow 0.000000
         401.018353 ms  arterial pressure 108.675638  right atrial pressure 2.312602  heart rate 1.001486  sympathetic traffic 0.020012  filtration 2.389089  urine 0.020000  wall power 10651.260919  aortic flow 0.000000
         500.022833 ms  arterial pressure 102.405047  right atrial pressure 0.879740  heart rate 0.970403  sympathetic traffic 0.030303  filtration 2.340577  urine 0.020000  wall power 9265.225213  aortic flow 0.000000
         600.022833 ms  arterial pressure 96.500819  right atrial pressure 0.458667  heart rate 0.949109  sympathetic traffic 0.068751  filtration 2.216060  urine 0.020000  wall power 6893.045195  aortic flow 0.000000
         700.022833 ms  arterial pressure 90.960455  right atrial pressure 0.481360  heart rate 0.939649  sympathetic traffic 0.183144  filtration 2.050313  urine 0.020000  wall power 5292.002305  aortic flow 0.000000
         800.022833 ms  arterial pressure 85.745604  right atrial pressure 0.697026  heart rate 0.944582  sympathetic traffic 0.438538  filtration 1.869605  urine 0.020000  wall power 4420.229118  aortic flow 0.000000
         900.022833 ms  arterial pressure 80.845038  right atrial pressure 0.935826  heart rate 0.965374  sympathetic traffic 0.743399  filtration 1.687693  urine 0.020000  wall power 3803.691574  aortic flow 0.000000
        1001.309992 ms  arterial pressure 76.200704  right atrial pressure 1.182158  heart rate 1.000291  sympathetic traffic 0.917158  filtration 1.508729  urine 0.020000  wall power 3179.073336  aortic flow 0.000000
    "#]].assert_eq(&trace);
}
