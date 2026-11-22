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
fn the_slices_model_hash_and_conserved_basis_stay_where_they_were() {
    let body = arena_body().expect("compiled");
    expect!["07ff0687d1f7f3e8b0583d7fc6aacde48f92a34e4e01cb4b779cf9412718a2e8"]
        .assert_eq(&hex(body.model_hash().digest()));
    expect!["1e5235d9018ec8b93dcb9045ec3bde7f6e024fc3de2f77e7bf7b94256e189b25"]
        .assert_eq(&hex(body.conserved_basis().digest()));
}

#[test]
fn the_first_second_of_the_slice_stays_where_it_was() {
    let scale = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let core = ReferenceIntegrator::new(MILLISECOND)
        .with_scales(ErrorScales::uniform(scale(1.0), scale(1e-3)).expect("positive"));
    let FirstSecond { named, trace, sealed } = first_second(&core);
    expect!["e5b9091d48babe6daa8d21fc122fa104e8442fcb53789d7a2dae163b325f2f58"].assert_eq(&named);
    expect!["0159da3a8fcc27e6dc5fccddef5cfd88412e4d5994ac5875d470bb912030c9e8"].assert_eq(&sealed);
    expect![[r#"
         100.000000 ms  arterial pressure 125.031526  right atrial pressure 1.614839  heart rate 1.053678  sympathetic traffic 0.709790  filtration 1.709556  urine 0.020000  wall power 14114.777657  aortic flow 0.000000
         200.000000 ms  arterial pressure 120.536165  right atrial pressure 2.050066  heart rate 1.070955  sympathetic traffic 0.130025  filtration 2.082239  urine 0.020000  wall power 10372.768283  aortic flow 39.898583
         300.000000 ms  arterial pressure 114.951075  right atrial pressure 2.417766  heart rate 1.045808  sympathetic traffic 0.035426  filtration 2.289425  urine 0.020000  wall power 34166.771882  aortic flow 0.000000
         400.000000 ms  arterial pressure 108.094858  right atrial pressure 2.243974  heart rate 1.008756  sympathetic traffic 0.025700  filtration 2.355691  urine 0.020000  wall power 10963.849791  aortic flow 0.000000
         500.000000 ms  arterial pressure 101.751143  right atrial pressure 0.692859  heart rate 0.977379  sympathetic traffic 0.037747  filtration 2.315063  urine 0.020000  wall power 8836.401861  aortic flow 0.000000
         600.000000 ms  arterial pressure 95.842549  right atrial pressure 0.264476  heart rate 0.956186  sympathetic traffic 0.083349  filtration 2.197330  urine 0.020000  wall power 6640.237335  aortic flow 0.000000
         700.000000 ms  arterial pressure 90.305500  right atrial pressure 0.298147  heart rate 0.947386  sympathetic traffic 0.214674  filtration 2.035672  urine 0.020000  wall power 5157.053918  aortic flow 0.000000
         800.000000 ms  arterial pressure 85.100901  right atrial pressure 0.523339  heart rate 0.953605  sympathetic traffic 0.486334  filtration 1.856880  urine 0.020000  wall power 4340.219915  aortic flow 0.000000
         900.000000 ms  arterial pressure 80.216803  right atrial pressure 0.766230  heart rate 0.975957  sympathetic traffic 0.777658  filtration 1.675338  urine 0.020000  wall power 3741.817642  aortic flow 0.000000
        1000.000000 ms  arterial pressure 75.650457  right atrial pressure 0.977602  heart rate 1.011708  sympathetic traffic 0.929035  filtration 1.498308  urine 0.020000  wall power 3235.984498  aortic flow 0.000000
    "#]].assert_eq(&trace);
}

#[test]
fn the_first_second_of_the_slice_on_ng_solver_stays_where_it_was() {
    let FirstSecond { named, trace, sealed } = first_second(&ng_solver::Core::new(10 * MILLISECOND));
    expect!["08c2bfc5eb7366774be203617cdd8f4c26846071154916b3c706c89ba091c7c7"].assert_eq(&named);
    expect!["9600d0aa9c838d310b2b5da961076b08b6015758c6f3b77dc10380c6c121a057"].assert_eq(&sealed);
    expect![[r#"
         101.025511 ms  arterial pressure 124.881347  right atrial pressure 1.618933  heart rate 1.053998  sympathetic traffic 0.701724  filtration 1.714622  urine 0.020000  wall power 14073.352137  aortic flow 0.000000
         204.196326 ms  arterial pressure 120.437818  right atrial pressure 2.066060  heart rate 1.070075  sympathetic traffic 0.122474  filtration 2.092131  urine 0.020000  wall power 10290.113799  aortic flow 38.733977
         301.819179 ms  arterial pressure 114.726834  right atrial pressure 2.421059  heart rate 1.044703  sympathetic traffic 0.035896  filtration 2.288072  urine 0.020000  wall power 33397.850534  aortic flow 0.000000
         404.750633 ms  arterial pressure 107.699129  right atrial pressure 2.143814  heart rate 1.007057  sympathetic traffic 0.026741  filtration 2.351021  urine 0.020000  wall power 10792.643731  aortic flow 0.000000
         500.220928 ms  arterial pressure 101.663114  right atrial pressure 0.724932  heart rate 0.977652  sympathetic traffic 0.039177  filtration 2.309621  urine 0.020000  wall power 8820.997183  aortic flow 0.000000
         608.059522 ms  arterial pressure 95.320008  right atrial pressure 0.271199  heart rate 0.955824  sympathetic traffic 0.092840  filtration 2.180685  urine 0.020000  wall power 6564.828653  aortic flow 0.000000
         708.059522 ms  arterial pressure 89.819790  right atrial pressure 0.312766  heart rate 0.948724  sympathetic traffic 0.237384  filtration 2.018221  urine 0.020000  wall power 5114.084400  aortic flow 0.000000
         808.059522 ms  arterial pressure 84.650386  right atrial pressure 0.534494  heart rate 0.956889  sympathetic traffic 0.519500  filtration 1.840168  urine 0.020000  wall power 4298.193436  aortic flow 0.000000
         908.059522 ms  arterial pressure 79.802371  right atrial pressure 0.775696  heart rate 0.980977  sympathetic traffic 0.799346  filtration 1.659674  urine 0.020000  wall power 3701.223616  aortic flow 0.000000
        1000.225153 ms  arterial pressure 75.615069  right atrial pressure 0.972701  heart rate 1.014594  sympathetic traffic 0.930462  filtration 1.497448  urine 0.020000  wall power 3168.578689  aortic flow 0.000000
    "#]].assert_eq(&trace);
}

#[test]
fn the_first_second_of_the_slice_on_the_doubled_core_stays_where_it_was() {
    let FirstSecond { named, trace, sealed } = first_second(&ng_solver::Core::doubled(10 * MILLISECOND));
    expect!["08c2bfc5eb7366774be203617cdd8f4c26846071154916b3c706c89ba091c7c7"].assert_eq(&named);
    expect!["2dbb404fb261024534229c1742023e609ef32d2a8861065695346df69bb57823"].assert_eq(&sealed);
    expect![[r#"
         100.995240 ms  arterial pressure 124.888330  right atrial pressure 1.618767  heart rate 1.053981  sympathetic traffic 0.701687  filtration 1.714644  urine 0.020000  wall power 14077.903399  aortic flow 0.000000
         203.737601 ms  arterial pressure 120.449016  right atrial pressure 2.064166  heart rate 1.070086  sympathetic traffic 0.123396  filtration 2.090924  urine 0.020000  wall power 10295.098909  aortic flow 37.677959
         301.935596 ms  arterial pressure 114.713576  right atrial pressure 2.421279  heart rate 1.044606  sympathetic traffic 0.035906  filtration 2.288044  urine 0.020000  wall power 33346.827598  aortic flow 0.000000
         405.821771 ms  arterial pressure 107.624420  right atrial pressure 2.122305  heart rate 1.006655  sympathetic traffic 0.026815  filtration 2.350810  urine 0.020000  wall power 10726.208547  aortic flow 0.000000
         502.685028 ms  arterial pressure 101.508806  right atrial pressure 0.705360  heart rate 0.977002  sympathetic traffic 0.039860  filtration 2.307240  urine 0.020000  wall power 8777.506270  aortic flow 0.000000
         608.059522 ms  arterial pressure 95.315999  right atrial pressure 0.270982  heart rate 0.955829  sympathetic traffic 0.092957  filtration 2.180470  urine 0.020000  wall power 6565.742880  aortic flow 0.000000
         708.059522 ms  arterial pressure 89.815947  right atrial pressure 0.312328  heart rate 0.948743  sympathetic traffic 0.237620  filtration 2.018037  urine 0.020000  wall power 5114.214529  aortic flow 0.000000
         808.059522 ms  arterial pressure 84.646721  right atrial pressure 0.534007  heart rate 0.956923  sympathetic traffic 0.519806  filtration 1.840007  urine 0.020000  wall power 4297.961761  aortic flow 0.000000
         908.059522 ms  arterial pressure 79.798903  right atrial pressure 0.775238  heart rate 0.981024  sympathetic traffic 0.799532  filtration 1.659529  urine 0.020000  wall power 3700.917801  aortic flow 0.000000
        1000.229729 ms  arterial pressure 75.611585  right atrial pressure 0.972281  heart rate 1.014652  sympathetic traffic 0.930536  filtration 1.497307  urine 0.020000  wall power 3168.258998  aortic flow 0.000000
    "#]].assert_eq(&trace);
}
