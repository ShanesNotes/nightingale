//! The whole body's places and routes on a synthetic map: which sites each set holds, and where each route family
//! a port requests stands.

use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = at().expect("finite");
    AnatomyBuilder::new().site::<C>(key, origin).id()
}

fn wave() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Wave, 1_000_000_000)
}

const BEDS: [&str; 5] = ["bare bed", "gut bed", "liver bed", "lung bed", "skin bed"];

/// The skin bed sits in the skin heat site, every other site in the body. The bare bed serves no tissue.
fn whole_body() -> Result<Anatomy, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at()?);
    let airway = map.site::<TissueSite>("conducting airway", at()?);
    let skin = map.site::<TissueSite>("skin heat site", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    let stomach = map.site::<VesselSegment>("stomach", at()?);
    let intestine = map.site::<VesselSegment>("small intestine", at()?);
    let lung = map.site::<CapillaryBed>("lung bed", at()?);
    let liver = map.site::<CapillaryBed>("liver bed", at()?);
    let gut = map.site::<CapillaryBed>("gut bed", at()?);
    let skin_bed = map.site::<CapillaryBed>("skin bed", at()?);
    let bare = map.site::<CapillaryBed>("bare bed", at()?);
    let beds = [lung.id(), liver.id(), gut.id(), skin_bed.id(), bare.id()];
    for site in [airway.id(), skin.id(), vessel.id(), stomach.id(), intestine.id()] {
        map.contain(body.id(), site);
    }
    for bed in beds {
        let holder = if bed == skin_bed.id() {
            skin.id()
        } else {
            body.id()
        };
        map.contain(holder, bed);
        map.connect(Network::Vascular, vessel.id(), bed);
        map.connect(Network::Vascular, bed, vessel.id());
    }
    for (bed, tissue) in [
        (&lung, Tissue::Lung),
        (&liver, Tissue::Liver),
        (&gut, Tissue::Gut),
        (&skin_bed, Tissue::Skin),
    ] {
        map.serves(bed, tissue);
    }
    map.connect(Network::Airway, airway.id(), lung.id());
    map.connect(Network::Gut, stomach.id(), intestine.id());
    map.connect(Network::Gut, intestine.id(), gut.id());
    map.finish()
}

fn holders(schema: &Schema, species: Species) -> Vec<(&'static str, Space)> {
    (schema.pools())
        .filter(|pool| pool.2 == species)
        .map(|(site, space, _)| (site.key(), space))
        .collect()
}

#[test]
fn each_site_set_holds_the_sites_the_map_places_on_it() -> Result<(), AdmissionError> {
    let mut schema = Schema::new(whole_body()?)?;
    schema.carry::<Glucose, Alimentary>(Space::Lumen);
    schema.carry::<NitrogenGas, Airway>(Space::Gas);
    schema.carry::<Glycogen, Beds<Liver>>(Space::Cells);
    schema.carry::<BodyProtein, Besides<Liver>>(Space::Cells);
    assert_eq!(
        holders(&schema, Species::Glucose),
        [("small intestine", Space::Lumen), ("stomach", Space::Lumen)]
    );
    assert_eq!(
        holders(&schema, Species::NitrogenGas),
        [("conducting airway", Space::Gas), ("lung bed", Space::Gas)]
    );
    assert_eq!(holders(&schema, Species::Glycogen), [("liver bed", Space::Cells)]);
    assert_eq!(
        holders(&schema, Species::BodyProtein),
        [
            ("bare bed", Space::Cells),
            ("gut bed", Space::Cells),
            ("lung bed", Space::Cells),
            ("skin bed", Space::Cells),
        ]
    );
    Ok(())
}

fn watery_spaces_left_out(schema: &mut Schema) {
    for space in [Space::Plasma, Space::Interstitium, Space::Cells, Space::Lumen] {
        schema.leave_out(space, "a bench of energy alone");
    }
}

type Placed = (Context, Option<&'static str>, Vec<(&'static str, Space, i64)>);

/// Each column a law places under `local`: where it stands, its receipt, and the pools it takes from and gives to.
fn columns(compiled: &CompiledBody, local: &str) -> Vec<Placed> {
    (compiled.columns().iter())
        .filter(|column| column.origin().local == local)
        .map(|column| {
            let mut ends: Vec<(&'static str, Space, i64)> = (column.terms().iter())
                .map(|&(row, coefficient)| match compiled.rows()[row.index()].kind() {
                    RowKind::Amount { site, space, .. } => (site.key(), space, coefficient),
                    other => panic!("an amount row, not {other:?}"),
                })
                .collect();
            ends.sort_by_key(|end| end.2);
            let receipt = column.receipt().map(|receipt| receipt.boundary());
            (column.context(), receipt, ends)
        })
        .collect()
}

#[test]
fn the_cell_membrane_moves_a_species_from_each_beds_interstitium_to_its_cells() -> Result<(), AdmissionError>
{
    let mut schema = Schema::new(whole_body()?)?;
    for space in [Space::Plasma, Space::Lumen] {
        schema.leave_out(space, "the cell membrane alone");
    }
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.cell_membrane::<CapillaryBed, Water>(&mut fl);
        let still = fl.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, fl.every_mode(), class)
        });
        fl.transfer_at(&route, still);
        body.add(fl.finish());
        body.compile()
    })?;
    let crossed = BEDS.map(|bed| {
        let ends = vec![(bed, Space::Interstitium, -1), (bed, Space::Cells, 1)];
        (Context::Site(site::<CapillaryBed>(bed)), None, ends)
    });
    assert_eq!(columns(&compiled, "cell membrane water"), crossed);
    Ok(())
}

#[test]
fn the_pumps_work_turns_chemical_energy_into_each_beds_heat() -> Result<(), AdmissionError> {
    let mut schema = Schema::new(whole_body()?)?;
    watery_spaces_left_out(&mut schema);
    schema.carry::<Energy, CapillaryBed>(Space::Thermal);
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        let route = ports.pump_work(&mut fl);
        let still = fl.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, fl.every_mode(), class)
        });
        fl.transfer_at(&route, still);
        body.add(fl.finish());
        body.compile()
    })?;
    let paid = BEDS.map(|bed| {
        let ends = vec![(bed, Space::Thermal, 1)];
        (
            Context::Site(site::<CapillaryBed>(bed)),
            Some("chemical energy"),
            ends,
        )
    });
    assert_eq!(columns(&compiled, "pump work"), paid);
    Ok(())
}

/// Only the skin bed sits in a store other than the core.
#[test]
fn blood_carries_heat_from_the_core_store_to_the_store_a_bed_sits_in_at_that_bed()
-> Result<(), AdmissionError> {
    let mut schema = Schema::new(whole_body()?)?;
    watery_spaces_left_out(&mut schema);
    for heat_site in ["body", "skin heat site"] {
        schema.pool::<Energy>(site::<TissueSite>(heat_site), Space::Thermal)?;
    }
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.heat_carried(&mut cv);
        let still = cv.site_kernel(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_at(&route, still);
        body.add(cv.finish());
        body.compile()
    })?;
    let carried = (
        Context::Site(site::<CapillaryBed>("skin bed")),
        None,
        vec![
            ("body", Space::Thermal, -1),
            ("skin heat site", Space::Thermal, 1),
        ],
    );
    assert_eq!(columns(&compiled, "heat carried by blood"), [carried]);
    Ok(())
}

/// The tubule also carries the bed's blood, so the map holds two sites and every pool moves.
#[test]
fn the_tubules_cells_secrete_into_the_lumen_against_each_reclaim_edge() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let tubule = map.site::<VesselSegment>("tubule", at()?).id();
    let bed = map.site::<CapillaryBed>("peritubular bed", at()?).id();
    for (from, to) in [(root.id(), tubule), (root.id(), bed)] {
        map.contain(from, to);
    }
    map.connect(Network::Vascular, tubule, bed);
    map.connect(Network::Vascular, bed, tubule);
    map.connect(Network::Reclaim, tubule, bed);
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Plasma, Space::Interstitium] {
        schema.leave_out(space, "secretion alone");
    }
    let class = wave()?;
    let compiled = assemble(schema, |mut body| {
        let (mut rn, ports) = body.renal()?.parts();
        let route = ports.secretion::<Water>(&mut rn);
        let still = rn.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, rn.every_mode(), class)
        });
        rn.transfer_along(&route, still);
        body.add(rn.finish());
        body.compile()
    })?;
    let secreted = (
        Context::Edge {
            from: tubule,
            to: bed,
        },
        None,
        vec![("peritubular bed", Space::Cells, -1), ("tubule", Space::Lumen, 1)],
    );
    assert_eq!(columns(&compiled, "secrete water"), [secreted]);
    Ok(())
}

/// A crossing's name ignores its set, so one owner crosses one wall once for each species.
#[test]
fn one_owner_crossing_one_wall_for_one_species_at_two_sets_is_named_twice() -> Result<(), AdmissionError> {
    let compiled = assemble(Schema::new(whole_body()?)?, |mut body| {
        let (mut fl, ports) = body.fluids()?.parts();
        ports.cell_membrane::<Beds<Liver>, Water>(&mut fl);
        ports.cell_membrane::<Besides<Liver>, Water>(&mut fl);
        body.add(fl.finish());
        body.compile()
    });
    let law = LawName {
        owner: <Fluids as Owner>::CARD,
        local: "cell membrane water",
    };
    assert_eq!(
        compiled.err(),
        Some(AdmissionError::Compile(CompileDefect::NamedTwice { law }))
    );
    Ok(())
}
