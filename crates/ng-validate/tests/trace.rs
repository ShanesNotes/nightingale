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
    expect!["c246e0c692ab3d450f8eb58496d3f541065eb652c0413a6f99f2f30febfd937a"].assert_eq(&named);
    expect!["817c99f7b9cc9f73a3f706ae71af30b6b0a271cb9a98666ec974fd4141df4c18"].assert_eq(&sealed);
    expect![[r#"
         100.000000 ms  arterial pressure 125.258703  right atrial pressure 1.978090  heart rate 1.060493  sympathetic traffic 0.627398  filtration 1.756965  urine 0.020000  wall power 13155.465232  aortic flow 0.000000
         200.000000 ms  arterial pressure 121.183896  right atrial pressure 2.408025  heart rate 1.072033  sympathetic traffic 0.099483  filtration 2.122865  urine 0.020000  wall power 10204.160071  aortic flow 43.787660
         300.000000 ms  arterial pressure 115.204711  right atrial pressure 2.776252  heart rate 1.042311  sympathetic traffic 0.028103  filtration 2.323099  urine 0.020000  wall power 29955.997842  aortic flow 0.000000
         400.000000 ms  arterial pressure 108.377463  right atrial pressure 2.356706  heart rate 1.003952  sympathetic traffic 0.021436  filtration 2.381799  urine 0.020000  wall power 10520.792825  aortic flow 0.000000
         500.000000 ms  arterial pressure 102.058794  right atrial pressure 0.857878  heart rate 0.972383  sympathetic traffic 0.032581  filtration 2.334157  urine 0.020000  wall power 9219.044075  aortic flow 0.000000
         600.000000 ms  arterial pressure 96.168856  right atrial pressure 0.447726  heart rate 0.951027  sympathetic traffic 0.073782  filtration 2.210139  urine 0.020000  wall power 6765.692243  aortic flow 0.000000
         700.000000 ms  arterial pressure 90.643009  right atrial pressure 0.484389  heart rate 0.941721  sympathetic traffic 0.195118  filtration 2.043952  urine 0.020000  wall power 5206.823832  aortic flow 0.000000
         800.000000 ms  arterial pressure 85.442489  right atrial pressure 0.710306  heart rate 0.947047  sympathetic traffic 0.458271  filtration 1.862544  urine 0.020000  wall power 4367.925767  aortic flow 0.000000
         900.000000 ms  arterial pressure 80.555990  right atrial pressure 0.950116  heart rate 0.968385  sympathetic traffic 0.758488  filtration 1.680027  urine 0.020000  wall power 3762.427089  aortic flow 0.000000
        1000.000000 ms  arterial pressure 75.983019  right atrial pressure 1.196612  heart rate 1.003336  sympathetic traffic 0.921874  filtration 1.502906  urine 0.020000  wall power 3160.158066  aortic flow 0.000000
    "#]].assert_eq(&trace);
}

#[test]
fn the_first_second_of_the_slice_on_ng_solver_stays_where_it_was() {
    let FirstSecond { named, trace, sealed } = first_second(&ng_solver::Core::new(10 * MILLISECOND));
    expect!["6d709479f0b103cadc1972febf6437a2dde172e19242eb76908b9dabde5bb752"].assert_eq(&named);
    expect!["58233f790617a4f1bf5122b47fc44efaeface2ebb7d4c3652567bb44ba176ecf"].assert_eq(&sealed);
    expect![[r#"
         101.893638 ms  arterial pressure 125.634639  right atrial pressure 1.972181  heart rate 1.060968  sympathetic traffic 0.602616  filtration 1.770379  urine 0.020000  wall power 13187.242370  aortic flow 0.000000
         201.334892 ms  arterial pressure 121.664534  right atrial pressure 2.400872  heart rate 1.070414  sympathetic traffic 0.091822  filtration 2.134323  urine 0.020000  wall power 10264.591599  aortic flow 34.427136
         302.451201 ms  arterial pressure 115.437044  right atrial pressure 2.769779  heart rate 1.039155  sympathetic traffic 0.025755  filtration 2.334368  urine 0.020000  wall power 29135.582237  aortic flow 0.000000
         401.042533 ms  arterial pressure 108.694805  right atrial pressure 2.311902  heart rate 1.001387  sympathetic traffic 0.019915  filtration 2.389656  urine 0.020000  wall power 10653.827978  aortic flow 0.000000
         500.056551 ms  arterial pressure 102.422504  right atrial pressure 0.879243  heart rate 0.970293  sympathetic traffic 0.030158  filtration 2.341098  urine 0.020000  wall power 9266.595085  aortic flow 0.000000
         608.059522 ms  arterial pressure 96.060955  right atrial pressure 0.447468  heart rate 0.947757  sympathetic traffic 0.073812  filtration 2.204358  urine 0.020000  wall power 6727.549988  aortic flow 0.000000
         708.059522 ms  arterial pressure 90.546757  right atrial pressure 0.494270  heart rate 0.939321  sympathetic traffic 0.197181  filtration 2.036546  urine 0.020000  wall power 5204.509270  aortic flow 0.000000
         808.059522 ms  arterial pressure 85.356217  right atrial pressure 0.716590  heart rate 0.945464  sympathetic traffic 0.462980  filtration 1.855342  urine 0.020000  wall power 4367.055242  aortic flow 0.000000
         908.059522 ms  arterial pressure 80.479861  right atrial pressure 0.953516  heart rate 0.967437  sympathetic traffic 0.762291  filtration 1.673628  urine 0.020000  wall power 3760.712248  aortic flow 0.000000
        1000.079707 ms  arterial pressure 76.269303  right atrial pressure 1.167881  heart rate 0.999515  sympathetic traffic 0.915560  filtration 1.511236  urine 0.020000  wall power 3178.162745  aortic flow 0.000000
    "#]].assert_eq(&trace);
}
