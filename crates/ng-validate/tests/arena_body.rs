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
