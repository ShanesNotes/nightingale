use ng_contract::*;

fn at(axial: f64) -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(axial)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

struct Manufactured {
    map: AnatomyBuilder,
    body: Site<TissueSite>,
    organ: Site<TissueSite>,
    vessel: Site<VesselSegment>,
    first: Site<CapillaryBed>,
    second: Site<CapillaryBed>,
    junction: Site<Junction>,
    space: Site<EnclosedSpace>,
    segment: Site<NerveSegment>,
    region: RegionId,
}

fn manufactured() -> Result<Manufactured, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at(0.0)?);
    let organ = map.site::<TissueSite>("organ", at(1.0)?);
    let vessel = map.site::<VesselSegment>("vessel", at(2.0)?);
    let first = map.site::<CapillaryBed>("bed 1", at(1.0)?);
    let second = map.site::<CapillaryBed>("bed 2", at(1.0)?);
    let junction = map.site::<Junction>("junction", at(1.0)?);
    let space = map.site::<EnclosedSpace>("space", at(1.0)?);
    let segment = map.site::<NerveSegment>("segment", at(3.0)?);
    for site in [organ.id(), vessel.id(), space.id(), segment.id()] {
        map.contain(body.id(), site);
    }
    for site in [first.id(), second.id(), junction.id()] {
        map.contain(organ.id(), site);
    }
    for (from, to) in [
        (vessel.id(), first.id()),
        (first.id(), second.id()),
        (second.id(), vessel.id()),
    ] {
        map.connect(Network::Vascular, from, to);
    }
    map.enclose(&space, &[second.id()], Compliance::Rigid);
    map.served_by(first.id(), &segment, Afferent::Stretch);
    let region = map.region("organ", organ.id());
    Ok(Manufactured {
        map,
        body,
        organ,
        vessel,
        first,
        second,
        junction,
        space,
        segment,
        region,
    })
}

fn refused(defect: AnatomyDefect) -> Option<AdmissionError> {
    Some(AdmissionError::Anatomy(defect))
}

#[test]
fn the_manufactured_anatomy_builds_and_hashes() -> Result<(), AdmissionError> {
    let once = manufactured()?.map.finish()?.digest();
    assert_eq!(manufactured()?.map.finish()?.digest(), once);
    Ok(())
}

#[test]
fn declaration_order_and_repeated_facts_leave_the_digest_unchanged() -> Result<(), AdmissionError> {
    let before = manufactured()?.map.finish()?.digest();
    let mut map = AnatomyBuilder::new();
    let segment = map.site::<NerveSegment>("segment", at(3.0)?);
    let space = map.site::<EnclosedSpace>("space", at(1.0)?);
    let junction = map.site::<Junction>("junction", at(1.0)?);
    let second = map.site::<CapillaryBed>("bed 2", at(1.0)?);
    let first = map.site::<CapillaryBed>("bed 1", at(1.0)?);
    let vessel = map.site::<VesselSegment>("vessel", at(2.0)?);
    let organ = map.site::<TissueSite>("organ", at(1.0)?);
    let body = map.site::<TissueSite>("body", at(0.0)?);
    for _ in 0..2 {
        map.region("organ", organ.id());
        map.served_by(first.id(), &segment, Afferent::Stretch);
        map.enclose(&space, &[second.id(), second.id()], Compliance::Rigid);
        for (from, to) in [
            (second.id(), vessel.id()),
            (first.id(), second.id()),
            (vessel.id(), first.id()),
        ] {
            map.connect(Network::Vascular, from, to);
        }
        for site in [junction.id(), second.id(), first.id()] {
            map.contain(organ.id(), site);
        }
        for site in [segment.id(), space.id(), vessel.id(), organ.id()] {
            map.contain(body.id(), site);
        }
    }
    assert_eq!(map.finish()?.digest(), before);
    Ok(())
}

#[test]
fn adding_a_site_or_a_relation_changes_the_digest() -> Result<(), AdmissionError> {
    let before = manufactured()?.map.finish()?.digest();
    let mut grown = manufactured()?;
    let extra = grown.map.site::<TissueSite>("extra", at(1.0)?);
    grown.map.contain(grown.body.id(), extra.id());
    assert_ne!(grown.map.finish()?.digest(), before);
    let mut related = manufactured()?;
    related
        .map
        .served_by(related.second.id(), &related.segment, Afferent::Nociceptive);
    assert_ne!(related.map.finish()?.digest(), before);
    Ok(())
}

#[test]
fn a_position_enters_the_digest_by_value() -> Result<(), AdmissionError> {
    let digest = |lateral: f64| -> Result<Digest, AdmissionError> {
        let mut map = AnatomyBuilder::new();
        let position = Position::new(Measure::new(0.0)?, Measure::new(lateral)?, Measure::new(0.0)?);
        map.site::<TissueSite>("body", position);
        Ok(map.finish()?.digest())
    };
    assert_eq!(digest(-0.0)?, digest(0.0)?);
    assert_ne!(digest(0.5)?, digest(0.0)?);
    Ok(())
}

#[test]
fn a_regions_members_are_its_containment_subtree() -> Result<(), AdmissionError> {
    let Manufactured {
        mut map,
        body,
        organ,
        vessel,
        first,
        second,
        junction,
        region,
        ..
    } = manufactured()?;
    let whole = map.region("whole", body.id());
    let anatomy = map.finish()?;
    let organ_members = [organ.id(), first.id(), second.id(), junction.id()];
    assert_eq!(anatomy.members(region), Some(&organ_members[..]));
    assert_eq!(anatomy.members(whole).map(<[SiteId]>::len), Some(8));

    let Manufactured { mut map, .. } = manufactured()?;
    let third = map.site::<CapillaryBed>("bed 3", at(1.0)?);
    map.contain(organ.id(), third.id());
    map.connect(Network::Vascular, vessel.id(), third.id());
    map.connect(Network::Vascular, third.id(), vessel.id());
    let grown = [organ.id(), first.id(), second.id(), third.id(), junction.id()];
    assert_eq!(map.finish()?.members(region), Some(&grown[..]));
    Ok(())
}

#[test]
fn a_region_this_anatomy_does_not_name_has_no_members() -> Result<(), AdmissionError> {
    let mut elsewhere = AnatomyBuilder::new();
    let root = elsewhere.site::<TissueSite>("root", at(0.0)?);
    let foreign = elsewhere.region("elsewhere", root.id());
    assert_eq!(manufactured()?.map.finish()?.members(foreign), None);
    Ok(())
}

#[test]
fn the_registry_holds_a_water_pool_for_each_space() -> Result<(), AdmissionError> {
    let Manufactured {
        map,
        vessel,
        first,
        second,
        space,
        ..
    } = manufactured()?;
    let schema = Schema::new(map.finish()?)?;
    let pools: Vec<(SiteId, Space)> = schema.pools().collect();
    let expected = [
        (first.id(), Space::Plasma),
        (first.id(), Space::Interstitium),
        (first.id(), Space::Cells),
        (second.id(), Space::Plasma),
        (second.id(), Space::Interstitium),
        (second.id(), Space::Cells),
        (space.id(), Space::Fluid),
        (vessel.id(), Space::Lumen),
    ];
    assert_eq!(pools, expected);
    Ok(())
}

#[test]
fn a_measure_refuses_a_value_that_is_not_finite() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(matches!(
            Measure::<Length>::new(value),
            Err(AdmissionError::NonFinite)
        ));
    }
}

#[test]
fn an_unreachable_bed_is_refused() -> Result<(), AdmissionError> {
    let Manufactured {
        mut map,
        organ,
        vessel,
        ..
    } = manufactured()?;
    let third = map.site::<CapillaryBed>("bed 3", at(1.0)?);
    map.contain(organ.id(), third.id());
    map.connect(Network::Vascular, third.id(), vessel.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::UnreachableBed { bed: "bed 3" })
    );
    Ok(())
}

#[test]
fn an_undrained_bed_is_refused() -> Result<(), AdmissionError> {
    let Manufactured {
        mut map,
        organ,
        vessel,
        ..
    } = manufactured()?;
    let third = map.site::<CapillaryBed>("bed 3", at(1.0)?);
    map.contain(organ.id(), third.id());
    map.connect(Network::Vascular, vessel.id(), third.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::UndrainedBed { bed: "bed 3" })
    );
    Ok(())
}

#[test]
fn a_bed_on_a_second_loop_is_refused() -> Result<(), AdmissionError> {
    let Manufactured { mut map, organ, .. } = manufactured()?;
    let loose = map.site::<VesselSegment>("loose vessel", at(1.0)?);
    let third = map.site::<CapillaryBed>("bed 3", at(1.0)?);
    for site in [loose.id(), third.id()] {
        map.contain(organ.id(), site);
    }
    map.connect(Network::Vascular, loose.id(), third.id());
    map.connect(Network::Vascular, third.id(), loose.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::UnreachableBed { bed: "bed 3" })
    );
    Ok(())
}

#[test]
fn a_loop_of_beds_alone_is_no_circulation() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at(0.0)?);
    let first = map.site::<CapillaryBed>("bed a", at(1.0)?);
    let second = map.site::<CapillaryBed>("bed b", at(1.0)?);
    for site in [first.id(), second.id()] {
        map.contain(body.id(), site);
    }
    map.connect(Network::Vascular, first.id(), second.id());
    map.connect(Network::Vascular, second.id(), first.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::UnreachableBed { bed: "bed a" })
    );
    Ok(())
}

#[test]
fn a_bed_fed_with_no_circulation_to_return_to_is_undrained() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at(0.0)?);
    let inlet = map.site::<VesselSegment>("inlet", at(1.0)?);
    let bed = map.site::<CapillaryBed>("bed", at(1.0)?);
    let outlet = map.site::<VesselSegment>("outlet", at(1.0)?);
    for site in [inlet.id(), bed.id(), outlet.id()] {
        map.contain(body.id(), site);
    }
    map.connect(Network::Vascular, inlet.id(), bed.id());
    map.connect(Network::Vascular, bed.id(), outlet.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::UndrainedBed { bed: "bed" })
    );
    Ok(())
}

#[test]
fn an_anatomy_without_beds_needs_no_circulation() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let body = map.site::<TissueSite>("body", at(0.0)?);
    let reservoir = map.site::<VesselSegment>("reservoir", at(1.0)?);
    map.contain(body.id(), reservoir.id());
    assert!(map.finish().is_ok());
    assert!(AnatomyBuilder::new().finish().is_ok());
    Ok(())
}

#[test]
fn an_enclosure_cannot_name_a_tissue_site() -> Result<(), AdmissionError> {
    let Manufactured { mut map, body, .. } = manufactured()?;
    let other = map.site::<EnclosedSpace>("space 2", at(1.0)?);
    map.contain(body.id(), other.id());
    map.enclose(&other, &[body.id()], Compliance::Rigid);
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::EnclosedTissue { site: "body" })
    );
    Ok(())
}

#[test]
fn a_site_enclosed_twice_is_refused() -> Result<(), AdmissionError> {
    let Manufactured {
        mut map,
        body,
        second,
        ..
    } = manufactured()?;
    let other = map.site::<EnclosedSpace>("space 2", at(1.0)?);
    map.contain(body.id(), other.id());
    map.enclose(&other, &[second.id()], Compliance::Rigid);
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::EnclosedTwice { site: "bed 2" })
    );
    Ok(())
}

#[test]
fn a_site_this_anatomy_does_not_declare_is_refused() -> Result<(), AdmissionError> {
    let mut elsewhere = AnatomyBuilder::new();
    let unplaced = elsewhere.site::<CapillaryBed>("bed 9", at(1.0)?);
    let Manufactured { mut map, organ, .. } = manufactured()?;
    map.contain(organ.id(), unplaced.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::Undeclared { site: "bed 9" })
    );

    let other_class = elsewhere.site::<CapillaryBed>("junction", at(1.0)?);
    let Manufactured { mut map, organ, .. } = manufactured()?;
    map.contain(organ.id(), other_class.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::Undeclared { site: "junction" })
    );
    Ok(())
}

#[test]
fn a_containment_cycle_is_refused() -> Result<(), AdmissionError> {
    let Manufactured { mut map, .. } = manufactured()?;
    let a = map.site::<TissueSite>("loop a", at(1.0)?);
    let b = map.site::<TissueSite>("loop b", at(1.0)?);
    map.contain(a.id(), b.id());
    map.contain(b.id(), a.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::ContainmentCycle { site: "loop a" })
    );

    let mut map = AnatomyBuilder::new();
    let mut tissue =
        |key| -> Result<SiteId, AdmissionError> { Ok(map.site::<TissueSite>(key, at(0.0)?).id()) };
    let (a, b, c, x, y) = (
        tissue("a")?,
        tissue("b")?,
        tissue("c")?,
        tissue("x")?,
        tissue("y")?,
    );
    for (parent, child) in [(x, a), (y, x), (x, y), (c, b), (b, c)] {
        map.contain(parent, child);
    }
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::ContainmentCycle { site: "b" })
    );
    Ok(())
}

#[test]
fn an_orphan_site_is_refused() -> Result<(), AdmissionError> {
    let Manufactured { mut map, .. } = manufactured()?;
    map.site::<TissueSite>("stray", at(1.0)?);
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::OrphanSite { site: "stray" })
    );
    Ok(())
}

#[test]
fn the_orphan_named_does_not_depend_on_declaration_order() -> Result<(), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("a stray", at(1.0)?);
    let body = map.site::<TissueSite>("body", at(0.0)?);
    let organ = map.site::<TissueSite>("organ", at(1.0)?);
    map.contain(body.id(), organ.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::OrphanSite { site: "a stray" })
    );
    Ok(())
}

#[test]
fn a_site_declared_twice_is_refused() -> Result<(), AdmissionError> {
    let Manufactured { mut map, .. } = manufactured()?;
    map.site::<TissueSite>("organ", at(1.0)?);
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::DeclaredTwice { site: "organ" })
    );
    Ok(())
}

#[test]
fn a_region_key_on_two_roots_is_refused() -> Result<(), AdmissionError> {
    let Manufactured { mut map, body, .. } = manufactured()?;
    map.region("organ", body.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::RegionTwice { region: "organ" })
    );
    Ok(())
}

#[test]
fn a_vascular_edge_at_a_site_that_carries_no_blood_is_refused() -> Result<(), AdmissionError> {
    let Manufactured {
        mut map,
        vessel,
        junction,
        ..
    } = manufactured()?;
    map.connect(Network::Vascular, vessel.id(), junction.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::WrongRelation { site: "junction" })
    );
    Ok(())
}

#[test]
fn a_site_contained_twice_is_refused() -> Result<(), AdmissionError> {
    let Manufactured {
        mut map, body, first, ..
    } = manufactured()?;
    map.contain(body.id(), first.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::ContainedTwice { site: "bed 1" })
    );
    Ok(())
}

#[test]
fn a_space_given_two_compliances_is_refused() -> Result<(), AdmissionError> {
    let Manufactured { mut map, space, .. } = manufactured()?;
    map.enclose(&space, &[], Compliance::Held { card: "held card" });
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::ComplianceTwice { space: "space" })
    );
    Ok(())
}

#[test]
fn a_space_with_no_compliance_is_refused() -> Result<(), AdmissionError> {
    let Manufactured { mut map, body, .. } = manufactured()?;
    let other = map.site::<EnclosedSpace>("space 2", at(1.0)?);
    map.contain(body.id(), other.id());
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::NoCompliance { space: "space 2" })
    );
    Ok(())
}

#[test]
fn spaces_that_enclose_each_other_are_refused() -> Result<(), AdmissionError> {
    let Manufactured {
        mut map, body, space, ..
    } = manufactured()?;
    let other = map.site::<EnclosedSpace>("space 2", at(1.0)?);
    map.contain(body.id(), other.id());
    map.enclose(&other, &[space.id()], Compliance::Rigid);
    map.enclose(&space, &[other.id()], Compliance::Rigid);
    assert_eq!(
        map.finish().err(),
        refused(AnatomyDefect::EnclosureCycle { space: "space" })
    );
    Ok(())
}
