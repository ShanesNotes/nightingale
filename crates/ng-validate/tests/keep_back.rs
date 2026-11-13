extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use expect_test::expect;
use ng_contract::*;
use ng_validate::*;

fn in_blood_alone<R: Substance>() -> Result<CompiledBody, AdmissionError> {
    let mut schema = arena_schema()?;
    schema.carry::<R, Blood>(Space::Plasma);
    schema.carry::<R, Blood>(Space::Lumen);
    assemble(schema, |mut body| {
        hold_outside_slice(&mut body)?;
        arena_organs(&mut body)?;
        body.compile()
    })
}

fn stranding<R: Substance>() -> (Species, Option<LawName>) {
    let law = match in_blood_alone::<R>() {
        Ok(_) => None,
        Err(AdmissionError::Compile(CompileDefect::CargoStranded { law, species }))
            if species == R::SPECIES =>
        {
            Some(law)
        }
        Err(other) => panic!("{:?} in blood alone: {other:?}", R::SPECIES),
    };
    (R::SPECIES, law)
}

#[test]
fn a_hormone_in_blood_alone_compiles_on_the_slices_own_fluid_routes() {
    assert_eq!(stranding::<Cortisol>(), (Species::Cortisol, None));
}

fn at_an_end(route: FluidRoute, (site, space, _): (SiteId, Space, Species)) -> bool {
    let at = |there: Space, key: &str| space == there && site.key() == key;
    match route {
        FluidRoute::Starling => matches!(space, Space::Plasma | Space::Interstitium),
        FluidRoute::Lymph => space == Space::Interstitium || at(Space::Lumen, "central veins"),
        FluidRoute::Filtration => at(Space::Plasma, "glomerular bed") || at(Space::Lumen, "renal tubule"),
    }
}

#[test]
fn each_keep_back_line_stops_its_species_on_its_route_and_every_other_species_in_blood_crosses()
-> Result<(), AdmissionError> {
    let body = whole_body()?;
    let pools: Vec<(SiteId, Space, Species)> = (body.rows().iter())
        .filter_map(|row| match row.kind() {
            RowKind::Amount { site, space, species } => Some((site, space, species)),
            _ => None,
        })
        .collect();
    let species_of = |row: RowId| match body.rows().iter().find(|known| known.id() == row)?.kind() {
        RowKind::Amount { species, .. } => Some(species),
        _ => None,
    };
    let in_blood: BTreeSet<Species> = (pools.iter())
        .filter(|pool| pool.1 == Space::Plasma)
        .map(|pool| pool.2)
        .collect();
    let mut tally = BTreeMap::new();
    for route in [FluidRoute::Starling, FluidRoute::Lymph, FluidRoute::Filtration] {
        let moved: BTreeSet<Species> = (body.columns().iter())
            .filter(|column| column.origin() == route.law())
            .flat_map(|column| column.terms().iter().filter_map(|&(row, _)| species_of(row)))
            .collect();
        let lines: Vec<&KeepBack> = KEEP_BACKS.iter().filter(|line| line.route == route).collect();
        let kept: BTreeSet<Species> = lines.iter().map(|line| line.species).collect();
        assert_eq!(kept.len(), lines.len(), "{route:?} keeps a species back twice");
        let carried: BTreeSet<Species> = in_blood.difference(&kept).copied().collect();
        assert_eq!(
            moved, carried,
            "{route:?} carries every species in blood it does not keep back"
        );
        for line in &lines {
            let reached = pools
                .iter()
                .any(|&pool| pool.2 == line.species && at_an_end(route, pool));
            assert_eq!(reached, !line.inert, "{line:?}");
        }
        let inert: Vec<Species> = lines
            .iter()
            .filter(|line| line.inert)
            .map(|line| line.species)
            .collect();
        tally.insert(route, (lines.len(), inert, moved));
    }
    let mut released = BTreeMap::new();
    for line in KEEP_BACKS {
        *released.entry(line.release).or_insert(0) += 1;
    }
    let pin = format!("{tally:?}\n{released:?}\n");
    expect![[r#"
        {Starling: (28, [Ammonium], {Water, Sodium, Potassium, Chloride, Albumin}), Lymph: (28, [Ammonium], {Water, Sodium, Potassium, Chloride, Albumin}), Filtration: (29, [], {Water, Sodium, Potassium, Chloride})}
        {Ticket(8, "FL"): 2, Ticket(8, "MT"): 2, Ticket(9, "FL"): 6, Ticket(13, "FL"): 3, Ticket(14, "FL"): 4, Ticket(14, "RN"): 1, Ticket(15, "FL"): 4, Ticket(15, "RN"): 1, Ticket(16, "FL"): 4, Ticket(16, "RN"): 1, Ticket(17, "FL"): 6, Ticket(18, "FL"): 4, Ticket(18, "RN"): 2, Ticket(19, "FL"): 8, Ticket(19, "RN"): 3, Ticket(20, "FL"): 2, Ticket(21, "FL"): 2, Kept: 30}
    "#]].assert_eq(&pin);
    Ok(())
}

#[test]
fn each_keep_back_line_names_the_ticket_that_releases_it() {
    let lines: String = (KEEP_BACKS.iter())
        .map(|line| {
            let inert = if line.inert { " inert" } else { "" };
            format!("{:?} {:?} {:?}{inert}\n", line.route, line.species, line.release)
        })
        .collect();
    expect![[""]].assert_eq(&lines);
}
