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
        write!(trace, "{:>4} ms", tenth * 100).expect("a string takes it");
        for (name, kind) in kinds {
            let row = body.rows().iter().find(|row| row.kind() == kind).expect("a row");
            let value = f64::from_bits(view.value(row.id()).bits()[0]);
            write!(trace, "  {name} {value:.6}").expect("a string takes it");
        }
        trace.push('\n');
    }
    expect![[r#"
         100 ms  arterial pressure 125.258703  right atrial pressure 1.978090  heart rate 1.060493  sympathetic traffic 0.627398  filtration 1.756965  urine 0.020000  wall power 13155.465232  aortic flow 0.000000
         200 ms  arterial pressure 121.183896  right atrial pressure 2.408025  heart rate 1.072033  sympathetic traffic 0.099483  filtration 2.122865  urine 0.020000  wall power 10204.160071  aortic flow 43.787660
         300 ms  arterial pressure 115.204711  right atrial pressure 2.776252  heart rate 1.042311  sympathetic traffic 0.028103  filtration 2.323099  urine 0.020000  wall power 29955.997841  aortic flow 0.000000
         400 ms  arterial pressure 108.377463  right atrial pressure 2.356706  heart rate 1.003952  sympathetic traffic 0.021436  filtration 2.381799  urine 0.020000  wall power 10520.792824  aortic flow 0.000000
         500 ms  arterial pressure 102.058794  right atrial pressure 0.857878  heart rate 0.972383  sympathetic traffic 0.032581  filtration 2.334157  urine 0.020000  wall power 9219.044074  aortic flow 0.000000
         600 ms  arterial pressure 96.168856  right atrial pressure 0.447726  heart rate 0.951027  sympathetic traffic 0.073782  filtration 2.210139  urine 0.020000  wall power 6765.692241  aortic flow 0.000000
         700 ms  arterial pressure 90.643009  right atrial pressure 0.484389  heart rate 0.941721  sympathetic traffic 0.195118  filtration 2.043952  urine 0.020000  wall power 5206.823830  aortic flow 0.000000
         800 ms  arterial pressure 85.442489  right atrial pressure 0.710306  heart rate 0.947047  sympathetic traffic 0.458271  filtration 1.862544  urine 0.020000  wall power 4367.925766  aortic flow 0.000000
         900 ms  arterial pressure 80.555990  right atrial pressure 0.950116  heart rate 0.968385  sympathetic traffic 0.758488  filtration 1.680027  urine 0.020000  wall power 3762.427087  aortic flow 0.000000
        1000 ms  arterial pressure 75.983019  right atrial pressure 1.196612  heart rate 1.003336  sympathetic traffic 0.921874  filtration 1.502906  urine 0.020000  wall power 3160.158064  aortic flow 0.000000
    "#]].assert_eq(&trace);
}
