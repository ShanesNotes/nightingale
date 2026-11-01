use ng_contract::*;
use ng_validate::*;

/// The slice compiles: every law placed, every row matched, no pool an orphan. No kernel read takes a site, so this
/// passing is check 1 of ticket 10. It then binds a fixture and builds the admission ring, so what stands between this
/// and check 2 is the solve.
#[test]
fn the_slice_compiles_binds_and_lays_out_its_ring() -> Result<(), AdmissionError> {
    let body = arena_body()?;
    let rows = body.rows();
    let count = |wanted: fn(&RowKind) -> bool| rows.iter().filter(|row| wanted(&row.kind())).count();
    assert_eq!(count(|kind| matches!(kind, RowKind::Amount { .. })), 155);
    assert_eq!(count(|kind| matches!(kind, RowKind::Memory(_))), 8);
    assert_eq!(count(|kind| matches!(kind, RowKind::Algebraic(_))), 77);
    assert_eq!(body.columns().len(), 219);
    assert_eq!(body.instances().len(), 339);
    assert_eq!(body.sparsity().unknowns().len(), 1509);
    assert_eq!(body.longest_step_ns(), 10_000_000);

    let (packet, lens) = (arena_packet(&body)?, arena_lens(&body)?);
    let parameters = body.bind(packet)?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(anchor_moment()).reading(&lens);
    let system = body.initialization(parameters, port, request)?;
    assert_eq!(system.points(), 100);
    Ok(())
}

#[test]
fn debug_admit() {
    let body = arena_body().expect("compiled");
    let (packet, lens) = (
        arena_packet(&body).expect("packet"),
        arena_lens(&body).expect("lens"),
    );
    let port = BoundaryPort::new(Vec::new()).expect("port");
    let core = ReferenceIntegrator::new(1_000_000);
    let result = admit_at(&body, packet, port, &core, &lens, anchor_moment());
    match result {
        Ok(_) => println!("ADMITTED"),
        Err(failure) => {
            if let InitializationFailure::Inconsistent {
                row: AdmissionRow::Body(row),
            } = failure
            {
                println!("ROW {:?}", body.rows()[row.index()].kind());
            }
            panic!("{failure:?}")
        }
    }
}

fn row_of(body: &CompiledBody, name: &str, place: Option<&str>) -> RowId {
    let found = body.rows().iter().find(|row| match row.kind() {
        RowKind::Algebraic(slot) => {
            slot.law.local == name
                && place.is_none_or(|place| format!("{:?}", slot.at).contains(place))
        }
        RowKind::Memory(slot) => slot.law.local == name,
        _ => false,
    });
    found.expect("a row").id()
}

#[test]
fn debug_run() {
    let body = arena_body().expect("compiled");
    let (packet, lens) = (arena_packet(&body).expect("packet"), arena_lens(&body).expect("lens"));
    let port = BoundaryPort::new(Vec::new()).expect("port");
    let unit = |value| Measure::<Dimensionless>::new(value).expect("finite");
    let core = ReferenceIntegrator::new(1_000_000)
        .with_scales(ErrorScales::uniform(unit(1.0), unit(1e-3)).expect("scales"));
    let admitted = admit_at(&body, packet, port, &core, &lens, anchor_moment()).expect("admitted");
    let mut session = admitted.session;
    for (at, row) in body.rows().iter().enumerate() {
        if at == 157 || at == 158 || at == 159 || at == 160 || at == 163 {
            println!("SLOT {at} {:?}", row.kind());
        }
    }
    let art = row_of(&body, "blood pressure", Some("systemic arteries"));
    let rate = row_of(&body, "rate", None);
    let urine = row_of(&body, "urine", None);
    let mut steps = 0_u64;
    let origin = anchor_moment().nanoseconds();
    let mut mark = origin;
    let (mut sum, mut count, mut high, mut low) = (0.0, 0.0, f64::MIN, f64::MAX);
    let seconds: u64 = std::env::var("SECONDS_RUN").map_or(3, |v| v.parse().expect("number"));
    loop {
        session = match session.step() {
            ng_record::Step::Accepted(next, _) => next,
            ng_record::Step::Refused(mut refused) => {
                loop {
                    match refused.retry() {
                        ng_record::Step::Accepted(next, _) => break next,
                        ng_record::Step::Refused(again) => refused = again,
                        ng_record::Step::Faulted(stopped) => panic!("{:?}", stopped.fault()),
                    }
                }
            }
            ng_record::Step::Faulted(stopped) => {
                let fault = stopped.fault();
                if let Some(NumericalRefusal::Accuracy { row } | NumericalRefusal::StageResidual { row }) = fault.last_refusal() {
                    println!("ROW {:?}", body.rows()[row.index()].kind());
                }
                panic!("fault {fault:?}")
            }
        };
        steps += 1;
        let view = session.record().accepted();
        let value = |row: RowId| f64::from_bits(view.value(row).bits()[0]);
        let p = value(art);
        sum += p;
        count += 1.0;
        high = high.max(p);
        low = low.min(p);
        let now = view.time().nanoseconds();
        if now >= mark + 1_000_000_000 {
            println!(
                "t {:.3} steps {steps} map {:.2} high {:.2} low {:.2} rate {:.4} urine {:.4}",
                (now - origin) as f64 * 1e-9,
                sum / count,
                high,
                low,
                value(rate),
                value(urine)
            );
            (sum, count, high, low) = (0.0, 0.0, f64::MIN, f64::MAX);
            mark = now;
        }
        if now >= origin + seconds * 1_000_000_000 {
            break;
        }
    }
}
