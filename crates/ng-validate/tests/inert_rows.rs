//! The slice with memory rows that decay to zero and that nothing reads. Each sits before the heart's rows, so it moves
//! the index of every row the guards read, and compile orders guards by a digest that holds those indices: each count
//! of rows holds the slice's twelve guards in another order, the clock before or after the guard that ends systole.
//! Admission must not see it.

use ng_contract::*;
use ng_validate::*;

const MILLISECOND: u64 = 1_000_000;
const INERT: [&str; 4] = ["a0", "a1", "a2", "a3"];

/// The slice's exits, with `inert` rows in the same fragment. With none it is the slice's own.
fn exits(body: &mut Assembly<'_>, inert: &[&'static str]) -> Result<(), AdmissionError> {
    let (mut laws, ports) = body.boundary()?.parts();
    let urine = ports.route::<Water>("urine")?.route;
    let rest = laws.parameter::<Water>(ParameterKey::new("bd.bladder-rest"), Admissible::nonnegative());
    let voiding =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.bladder-speed"), Admissible::positive());
    let width = laws.parameter::<Rate<Water>>(ParameterKey::new("bd.bladder-width"), Admissible::positive());
    let heat = ports.route::<Energy>("heat")?.route;
    let loss =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("bd.heat-speed"), Admissible::positive());
    let class = RateClass::new(RateBand::Hours, 3_600_000_000_000)?;
    let rate = laws.kernel(|mut k| {
        let (held, floor, pace) = (
            k.read(&urine.amount::<Water>()),
            k.parameter(&rest),
            k.parameter(&voiding),
        );
        let draining = k.relax(held, floor, pace);
        let (zero, soft) = (k.zero(), k.parameter(&width));
        let rate = k.smooth_max(draining, zero, soft);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&urine, rate));
    let rate = laws.kernel(|mut k| {
        let (held, zero, pace) = (k.read(&heat.amount::<Energy>()), k.zero(), k.parameter(&loss));
        let rate = k.relax(held, zero, pace);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&heat, rate));
    for &local in inert {
        let still = laws.state::<Dimensionless>(local, Admissible::finite());
        let rate = laws.kernel(|mut k| {
            let (zero, current, pace) = (k.zero(), k.state(&still), k.parameter(&loss));
            let rate = k.relax(zero, current, pace);
            k.finish(rate, laws.every_mode(), class)
        });
        laws.memory_rate(&still, rate);
    }
    body.add(laws.finish());
    Ok(())
}

fn slice(inert: &[&'static str]) -> CompiledBody {
    assemble(arena_schema().expect("a schema"), |mut body| {
        hold_outside_slice(&mut body)?;
        exits(&mut body, inert)?;
        let fragment = ng_cardiovascular::declare(body.cardiovascular()?)?;
        body.add(fragment);
        let fragment = ng_fluids::declare(body.fluids()?)?;
        body.add(fragment);
        let fragment = ng_renal::declare(body.renal()?)?;
        body.add(fragment);
        let fragment = ng_nervous::declare(body.nervous()?)?;
        body.add(fragment);
        let fragment = ng_receptor::declare(body.receptor()?)?;
        body.add(fragment);
        body.compile()
    })
    .expect("compiled")
}

/// Each row's admitted value, by kind, or why admission refused.
fn admitted(body: &CompiledBody) -> Result<Vec<(RowKind, u64)>, String> {
    let store = MemoryStore::default();
    let core = ng_solver::Core::new(10 * MILLISECOND);
    let packet = arena_packet(body).map_err(|refused| format!("{refused:?}"))?;
    let lens = arena_lens(body).map_err(|refused| format!("{refused:?}"))?;
    let port = BoundaryPort::new(Vec::new()).map_err(|refused| format!("{refused:?}"))?;
    let admitted = admit_at(body, packet, port, &core, &lens, anchor_moment(), &store)
        .map_err(|refused| format!("{refused:?}"))?;
    let view = admitted.session.record().accepted();
    Ok((body.rows().iter())
        .map(|row| (row.kind(), view.value(row.id()).bits()[0]))
        .collect())
}

#[test]
fn the_slice_admits_to_one_start_whatever_order_inert_rows_hold_its_guards_in() {
    let plain = slice(&[]);
    assert_eq!(
        plain.model_hash().digest(),
        arena_body().expect("compiled").model_hash().digest(),
        "with no inert row this is the slice"
    );
    let start = admitted(&plain).expect("the slice admits");
    for count in 1..=INERT.len() {
        let read = admitted(&slice(&INERT[..count]))
            .unwrap_or_else(|refused| panic!("with {count} inert rows: {refused}"));
        let shared: Vec<(RowKind, u64)> = (read.into_iter())
            .filter(|(kind, _)| start.iter().any(|(known, _)| known == kind))
            .collect();
        assert!(
            shared == start,
            "with {count} inert rows the slice admits to another start"
        );
    }
}
