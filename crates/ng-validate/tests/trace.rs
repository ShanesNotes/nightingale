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
         100 ms  arterial pressure 128.151085  right atrial pressure 2.468340  heart rate 1.011649  sympathetic traffic 0.574114  filtration 1.734505  urine 0.020000  heat 16144.602433  aortic flow 0.000000
         200 ms  arterial pressure 122.955330  right atrial pressure 2.872428  heart rate 1.017817  sympathetic traffic 0.506614  filtration 2.212603  urine 0.020000  heat 11928.782110  aortic flow 49.528669
         300 ms  arterial pressure 116.454963  right atrial pressure 3.258583  heart rate 1.015404  sympathetic traffic 0.459067  filtration 2.450996  urine 0.020000  heat 39299.680503  aortic flow 0.000000
         400 ms  arterial pressure 108.820732  right atrial pressure 3.137660  heart rate 1.007785  sympathetic traffic 0.433563  filtration 2.494251  urine 0.020000  heat 12340.238971  aortic flow 0.000000
         500 ms  arterial pressure 101.877960  right atrial pressure 1.425512  heart rate 0.998476  sympathetic traffic 0.427617  filtration 2.403183  urine 0.020000  heat 10477.688584  aortic flow 0.000000
         600 ms  arterial pressure 95.498083  right atrial pressure 0.913915  heart rate 0.990116  sympathetic traffic 0.437750  filtration 2.224998  urine 0.020000  heat 7595.112154  aortic flow 0.000000
         700 ms  arterial pressure 89.580991  right atrial pressure 0.906730  heart rate 0.984467  sympathetic traffic 0.461436  filtration 2.004260  urine 0.020000  heat 5638.449065  aortic flow 0.000000
         800 ms  arterial pressure 84.055823  right atrial pressure 1.127657  heart rate 0.982692  sympathetic traffic 0.496603  filtration 1.776034  urine 0.020000  heat 4603.760304  aortic flow 0.000000
         900 ms  arterial pressure 78.875293  right atrial pressure 1.387644  heart rate 0.985553  sympathetic traffic 0.541115  filtration 1.561282  urine 0.020000  heat 3926.726190  aortic flow 0.000000
        1000 ms  arterial pressure 74.008201  right atrial pressure 1.675609  heart rate 0.993509  sympathetic traffic 0.592431  filtration 1.369660  urine 0.020000  heat 3566.027869  aortic flow 0.000000
    "#]].assert_eq(&trace);
}
