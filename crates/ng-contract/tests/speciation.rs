use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

fn speciated(twice: bool) -> Result<(CompiledBody, [SiteId; 4]), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let artery = map.site::<VesselSegment>("artery", at()?);
    let first = map.site::<CapillaryBed>("first bed", at()?);
    let vein = map.site::<VesselSegment>("vein", at()?);
    let second = map.site::<CapillaryBed>("second bed", at()?);
    let sites = [artery.id(), first.id(), vein.id(), second.id()];
    for (at, &site) in sites.iter().enumerate() {
        map.contain(root.id(), site);
        map.connect(Network::Vascular, site, sites[(at + 1) % sites.len()]);
    }
    map.unsupplied(&sites);
    let mut schema = Schema::new(map.finish()?)?;
    schema.leave_out(Space::Interstitium, "a bench with no tissue");
    schema.leave_out(Space::Cells, "a bench with no tissue");
    let body = assemble(schema, |mut body| {
        let class = RateClass::new(RateBand::Wave, 3_600_000_000_000)?;
        let (mut cv, ports) = body.cardiovascular()?.parts();
        let route = ports.circulation(&mut cv);
        let still = cv.edge_kernel(|mut k| {
            let zero = k.zero::<Rate<Water>>();
            k.finish(zero, cv.every_mode(), class)
        });
        cv.transfer_along(&route, still);
        body.add(cv.finish());
        let (mut bc, ports) = body.blood_chemistry(None)?.parts();
        let stated = ports.speciation(&mut bc);
        if twice {
            ports.speciation(&mut bc);
        }
        let key = ParameterKey::new;
        let pressure = bc.site_parameter::<Speciated, Pressure>(key("bc.pressure"), Admissible::positive());
        let share =
            bc.site_parameter::<Speciated, Dimensionless>(key("bc.share"), Admissible::unit_interval());
        let base = bc.site_parameter::<Speciated, Concentration<CarbonDioxide>>(
            key("bc.base"),
            Admissible::positive(),
        );
        let pressure_rows = [&stated.oxygen_partial_pressure.0];
        for output in pressure_rows {
            let value = bc.site_kernel(|mut k| {
                let value = k.site_parameter(&pressure);
                k.finish(value, bc.every_mode(), class)
            });
            bc.define_at(output, value);
        }
        for output in [&stated.oxygen_saturation.0, &stated.acidity.0] {
            let value = bc.site_kernel(|mut k| {
                let value = k.site_parameter(&share);
                k.finish(value, bc.every_mode(), class)
            });
            bc.define_at(output, value);
        }
        let value = bc.site_kernel(|mut k| {
            let value = k.site_parameter(&base);
            k.finish(value, bc.every_mode(), class)
        });
        bc.define_at(&stated.bicarbonate.0, value);
        let value = bc.site_kernel(|mut k| {
            let value = k.at_site(&stated.oxygen_partial_pressure.1);
            k.finish(value, bc.every_mode(), class)
        });
        bc.express_at(&stated.carbon_dioxide_partial_pressure.0, value);
        body.add(bc.finish());
        body.compile()
    })?;
    Ok((body, sites))
}

#[test]
fn speciation_states_four_rows_at_each_bed_and_none_for_its_row_free_field() -> Result<(), AdmissionError> {
    let (body, sites) = speciated(false)?;
    let rows_at = |site: SiteId| -> Vec<&'static str> {
        (body.rows().iter())
            .filter_map(|row| match row.kind() {
                RowKind::Algebraic(slot) if slot.law.owner == "BC" && slot.at == Context::Site(site) => {
                    Some(slot.law.local)
                }
                _ => None,
            })
            .collect()
    };
    let mut names = [
        OxygenPartialPressure::NAME,
        OxygenSaturation::NAME,
        Bicarbonate::NAME,
        Acidity::NAME,
    ];
    names.sort_unstable();
    for bed in [sites[1], sites[3]] {
        let mut found = rows_at(bed);
        found.sort_unstable();
        assert_eq!(found, names, "{bed:?}");
    }
    for segment in [sites[0], sites[2]] {
        assert!(
            rows_at(segment).is_empty(),
            "an unsampled segment is no speciated site"
        );
    }
    let named = |local| {
        (body.rows().iter())
            .any(|row| matches!(row.kind(), RowKind::Algebraic(slot) if slot.law.local == local))
    };
    assert!(!named(CarbonDioxidePartialPressure::NAME));
    Ok(())
}

#[test]
fn speciation_issued_twice_names_each_quantity_twice() {
    let law = LawName {
        owner: "BC",
        local: Acidity::NAME,
    };
    let refused = speciated(true).err();
    assert_eq!(
        refused,
        Some(AdmissionError::Compile(CompileDefect::NamedTwice { law }))
    );
}
