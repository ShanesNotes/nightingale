use expect_test::expect;
use ng_contract::*;
use ng_validate::*;

fn hex(digest: Digest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn a_reaction_counting_its_reference_once_keeps_the_burning_beds_hash() {
    let burning = BurningBeds::burning().expect("compiled");
    expect!["97f56356df7c0ce48dc256fa58ac1bf29500c29a391bfc474629771cc55d6859"]
        .assert_eq(&hex(burning.body().model_hash().digest()));
}
