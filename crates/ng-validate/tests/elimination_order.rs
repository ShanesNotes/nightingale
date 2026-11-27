use ng_contract::*;
use ng_validate::{arena_body, whole_body};

fn amount_row<'a>(body: &'a CompiledBody, site: &str, space: Space, species: Species) -> &'a Row {
    body.rows()
        .iter()
        .find(|row| {
            matches!(
                row.kind(),
                RowKind::Amount { site: at, space: s, species: sp }
                    if at.key() == site && s == space && sp == species
            )
        })
        .unwrap_or_else(|| panic!("missing amount row ({site}, {space:?}, {species:?})"))
}

#[test]
fn joining_rows_to_the_largest_block_stays_within_the_factoring_price() {
    let body = whole_body().expect("whole body compiles");
    let base = body.elimination_metrics();
    let max_scc = body.blocks().iter().map(|b| b.rows().len()).max().unwrap_or(0);
    assert_eq!(base.blocks, body.blocks().len());
    assert_eq!(base.largest_block, max_scc);
    assert!(
        base.nonzeros <= 16_000,
        "expected whole-body elimination nonzeros <= 16000, got {}",
        base.nonzeros
    );
    assert!(
        base.factor_macs <= 45_000,
        "expected whole-body factor MACs <= 45000, got {}",
        base.factor_macs
    );

    let interstitium_water = amount_row(&body, "pulmonary bed", Space::Interstitium, Species::Water);
    let alveolar_gas_water = amount_row(&body, "pulmonary bed", Space::Gas, Species::Water);
    let joined_gas = body
        .elimination_metrics_with_reads(&[(interstitium_water.id(), alveolar_gas_water.unknown())])
        .expect("in-bounds read");
    assert_eq!(
        joined_gas.largest_block,
        base.largest_block + 2,
        "expected the 2 gas-water rows to join the largest block"
    );
    assert!(
        joined_gas.factor_macs * 10 <= base.factor_macs * 11,
        "joining 2 gas-water rows grew factor MACs over +10%: {} -> {}",
        base.factor_macs,
        joined_gas.factor_macs
    );
    assert!(
        joined_gas.nonzeros * 10 <= base.nonzeros * 11,
        "joining 2 gas-water rows grew nonzeros over +10%: {} -> {}",
        base.nonzeros,
        joined_gas.nonzeros
    );

    let solute_block = body
        .blocks()
        .iter()
        .find(|block| block.rows().len() == 21)
        .expect("whole body has a 21-row blood-solute block");
    let joined_solute = body
        .elimination_metrics_with_reads(&[(interstitium_water.id(), solute_block.unknowns()[0])])
        .expect("in-bounds read");
    assert_eq!(
        joined_solute.largest_block,
        base.largest_block + 21,
        "expected the 21-row blood-solute block to join the largest block"
    );
    assert!(
        joined_solute.factor_macs * 10 <= base.factor_macs * 11,
        "joining a 21-row blood-solute block grew factor MACs over +10%: {} -> {}",
        base.factor_macs,
        joined_solute.factor_macs
    );
    assert!(
        joined_solute.nonzeros * 10 <= base.nonzeros * 11,
        "joining a 21-row blood-solute block grew nonzeros over +10%: {} -> {}",
        base.nonzeros,
        joined_solute.nonzeros
    );

    let slice = arena_body().expect("slice compiles");
    let foreign = body.rows().last().expect("whole body has rows");
    assert_eq!(
        slice.elimination_metrics_with_reads(&[(foreign.id(), foreign.unknown())]),
        None
    );
}
