//! A law reads one tissue's bed where the map has exactly one. Where it has none or two, compile refuses the law by
//! name: never a sum, and never the first match.

use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

const PARTIAL_PRESSURE: &str = "pulmonary partial pressure";

fn lung_pressure(beds: &[(&'static str, Tissue)]) -> Result<(CompiledBody, Vec<SiteId>), AdmissionError> {
    let mut map = AnatomyBuilder::new();
    let root = map.site::<TissueSite>("body", at()?);
    let vessel = map.site::<VesselSegment>("vessel", at()?);
    map.contain(root.id(), vessel.id());
    let mut sites = Vec::new();
    for &(key, tissue) in beds {
        let bed = map.site::<CapillaryBed>(key, at()?);
        map.contain(root.id(), bed.id());
        map.connect(Network::Vascular, vessel.id(), bed.id());
        map.connect(Network::Vascular, bed.id(), vessel.id());
        map.serves(&bed, tissue);
        sites.push(bed.id());
    }
    let mut schema = Schema::new(map.finish()?)?;
    for space in [Space::Plasma, Space::Lumen, Space::Interstitium, Space::Cells] {
        schema.leave_out(space, "a bench that moves no water");
    }
    let body = assemble(schema, |mut body| {
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let (mut mt, _) = body.metabolism(None)?.parts();
        let pressure = mt.site_parameter::<CapillaryBed, Pressure>(
            ParameterKey::new("mt.bed-partial-pressure"),
            Admissible::finite(),
        );
        let row = mt.algebraic::<Pressure>(PARTIAL_PRESSURE, Admissible::finite());
        let own = mt.kernel(|mut k| {
            let value = k.algebraic(&row);
            k.finish(value, mt.every_mode(), class)
        });
        let lung = mt.site_kernel::<Beds<Lung>, _, _>(|mut k| {
            let value = k.site_parameter_in(&pressure);
            k.finish(value, mt.every_mode(), class)
        });
        mt.only_at(&row, own, lung);
        body.add(mt.finish());
        body.compile()
    })?;
    Ok((body, sites))
}

fn not_one_site() -> Option<AdmissionError> {
    let law = LawName {
        owner: "MT",
        local: PARTIAL_PRESSURE,
    };
    let set = Sites::Tissue(Tissue::Lung);
    Some(AdmissionError::Compile(CompileDefect::NotOneSite { law, set }))
}

#[test]
fn the_row_is_the_one_lung_beds_value() -> Result<(), InitializationFailure> {
    let (body, sites) = lung_pressure(&[("lung bed", Tissue::Lung), ("gut bed", Tissue::Gut)])?;
    let mut packet = ParameterPacket::new();
    let value = Measure::<Pressure>::new(0.25)?;
    let key = ParameterKey::new("mt.bed-partial-pressure");
    packet.synthetic(key, Binding::Site(sites[0]), value, "synthetic")?;
    packet.start_rest(&body, |_| 0.0, "synthetic")?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(SimTime::from_ns(0));
    let system = body.initialization(body.bind(packet)?, port, request)?;
    let residuals = system.evaluate(&system.workspace())?;
    let row = RowKind::Algebraic(Slot {
        law: LawName {
            owner: "MT",
            local: PARTIAL_PRESSURE,
        },
        at: Context::Body,
    });
    let found = (body.rows().iter()).find(|known| known.kind() == row);
    let residual = residuals[found.expect("one body row").id().index()];
    assert_eq!(residual.to_bits(), (-0.25_f64).to_bits());
    Ok(())
}

#[test]
fn a_map_with_no_lung_bed_is_refused_by_the_law_and_the_set() {
    assert_eq!(lung_pressure(&[("gut bed", Tissue::Gut)]).err(), not_one_site());
}

#[test]
fn a_map_with_two_lung_beds_is_refused_by_the_law_and_the_set() {
    let beds = [
        ("left lung bed", Tissue::Lung),
        ("right lung bed", Tissue::Lung),
        ("gut bed", Tissue::Gut),
    ];
    assert_eq!(lung_pressure(&beds).err(), not_one_site());
}
