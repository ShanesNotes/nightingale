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

/// The slice's first second, to show a change and never to judge it: a PR that moves it updates this file and says why.
/// Only checks drawn from anchors and requirements judge physiology.
#[test]
fn the_first_second_of_the_slice_stays_where_it_was() {
    let body = arena_body().expect("compiled");
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let core = ReferenceIntegrator::new(MILLISECOND)
        .with_scales(ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive"));
    let (packet, lens) = (
        arena_packet(&body).expect("a packet"),
        arena_lens(&body).expect("a lens"),
    );
    let port = BoundaryPort::new(Vec::new()).expect("no event");
    let admitted = admit_at(&body, packet, port, &core, &lens, anchor_moment()).expect("admitted");
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
        ("heat", field("CV", "dissipated power", Context::Body)),
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
        write!(trace, "{:>4} ms", tenth * 100).expect("a string takes it");
        for (name, kind) in kinds {
            let row = body.rows().iter().find(|row| row.kind() == kind).expect("a row");
            let value = f64::from_bits(view.value(row.id()).bits()[0]);
            write!(trace, "  {name} {value:.6}").expect("a string takes it");
        }
        trace.push('\n');
    }
    expect![[r#"
         100 ms  arterial pressure 125.258742  right atrial pressure 1.978089  heart rate 1.060493  sympathetic traffic 0.627386  filtration 1.756970  urine 0.020000  heat 13155.804122  aortic flow 0.000000
         200 ms  arterial pressure 121.183675  right atrial pressure 2.408024  heart rate 1.072032  sympathetic traffic 0.099481  filtration 2.123018  urine 0.020000  heat 10204.418149  aortic flow 43.788682
         300 ms  arterial pressure 115.204314  right atrial pressure 2.776255  heart rate 1.042310  sympathetic traffic 0.028103  filtration 2.323308  urine 0.020000  heat 29955.960258  aortic flow 0.000000
         400 ms  arterial pressure 108.377068  right atrial pressure 2.356713  heart rate 1.003951  sympathetic traffic 0.021437  filtration 2.381966  urine 0.020000  heat 10520.718265  aortic flow 0.000000
         500 ms  arterial pressure 102.058542  right atrial pressure 0.857887  heart rate 0.972383  sympathetic traffic 0.032583  filtration 2.334231  urine 0.020000  heat 9218.926793  aortic flow 0.000000
         600 ms  arterial pressure 96.168804  right atrial pressure 0.447735  heart rate 0.951028  sympathetic traffic 0.073786  filtration 2.210111  urine 0.020000  heat 6765.565549  aortic flow 0.000000
         700 ms  arterial pressure 90.643157  right atrial pressure 0.484396  heart rate 0.941722  sympathetic traffic 0.195123  filtration 2.043842  urine 0.020000  heat 5206.714346  aortic flow 0.000000
         800 ms  arterial pressure 85.442799  right atrial pressure 0.710311  heart rate 0.947048  sympathetic traffic 0.458271  filtration 1.862382  urine 0.020000  heat 4367.848716  aortic flow 0.000000
         900 ms  arterial pressure 80.556399  right atrial pressure 0.950120  heart rate 0.968386  sympathetic traffic 0.758482  filtration 1.679848  urine 0.020000  heat 3762.393444  aortic flow 0.000000
        1000 ms  arterial pressure 75.983437  right atrial pressure 1.196615  heart rate 1.003337  sympathetic traffic 0.921870  filtration 1.502752  urine 0.020000  heat 3160.176857  aortic flow 0.000000
    "#]].assert_eq(&trace);
}
