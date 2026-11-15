use ng_contract::*;
use ng_validate::*;

const WATER: f64 = 100.0;
const PRESSURES: (f64, f64) = (100.0, 10.0);

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = || Measure::new(0.0).expect("finite");
    AnatomyBuilder::new()
        .site::<C>(key, Position::new(origin(), origin(), origin()))
        .id()
}

fn algebraic(owner: &'static str, local: &'static str, at: Context) -> RowKind {
    RowKind::Algebraic(Slot {
        law: LawName { owner, local },
        at,
    })
}

fn viscosity() -> RowKind {
    algebraic("HC", "viscosity", Context::Body)
}

fn veins(species: Species) -> RowKind {
    RowKind::Amount {
        site: site::<VesselSegment>("central veins"),
        species,
        space: Space::Lumen,
    }
}

fn index(body: &CompiledBody, kind: RowKind) -> usize {
    (body.rows().iter().position(|row| row.kind() == kind)).unwrap_or_else(|| panic!("no row {kind:?}"))
}

/// Each row's residual at every stage, with each named row's unknown set at every stage first. A stage's row is its
/// value less its law.
fn staged(body: &CompiledBody, each: &[Vec<(usize, f64)>]) -> Result<Vec<Vec<f64>>, InitializationFailure> {
    let (packet, lens) = (body_packet(body)?, body_lens(body)?);
    let parameters = body.bind(packet)?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(anchor_moment()).reading(&lens);
    let system = body.initialization(parameters, port, request)?;
    let mut workspace = system.workspace();
    let stages = workspace.fitted_slots().start;
    let rows = body.rows().len();
    let mut residuals = Vec::with_capacity(each.len());
    for values in each {
        for &(row, value) in values {
            for slot in (0..stages).filter(|slot| slot % rows == row) {
                workspace.unknowns_mut()[slot] = value;
            }
        }
        residuals.push(
            system
                .stage_residuals(&workspace, workspace.legs_of(0)[0])?
                .to_vec(),
        );
    }
    Ok(residuals)
}

/// The packet's row at an exponent of -1, so `(1 - Hct)^C - 1` is `Hct / (1 - Hct)`: plasma 2, red cells' own
/// hemoglobin 5, the reference hematocrit 0.5, 3 at the reference and a wall factor of 1.25.
fn closed_form(hematocrit: f64) -> f64 {
    let fraction = (hematocrit / (1.0 - hematocrit)) / (0.5 / (1.0 - 0.5));
    2.0 * 1.25 * (1.0 + (3.0 - 1.0) * fraction * 1.25)
}

#[test]
fn viscosity_at_two_hematocrits_follows_the_row() -> Result<(), InitializationFailure> {
    let body = whole_body()?;
    let (hemoglobin, water, field) = (
        index(&body, veins(Species::Hemoglobin)),
        index(&body, veins(Species::Water)),
        index(&body, viscosity()),
    );
    let hematocrits = [0.4, 0.2];
    let each: Vec<_> = (hematocrits.iter())
        .map(|hematocrit| vec![(water, WATER), (hemoglobin, 5.0 * hematocrit * WATER), (field, 1.0)])
        .collect();
    for (hematocrit, residuals) in hematocrits.iter().zip(staged(&body, &each)?) {
        let (law, closed) = (1.0 - residuals[field], closed_form(*hematocrit));
        assert!(
            (law - closed).abs() <= 1e-12 * closed,
            "at a hematocrit of {hematocrit}: {law} against {closed}"
        );
    }
    Ok(())
}

/// The coronary bed's edges have no tone, no feedback share and no fall, so the drive is the pressure difference and
/// the resistance only the packet's and viscosity's ratio.
#[test]
fn a_plain_edges_flow_falls_as_viscosity_over_its_reference_rises() -> Result<(), InitializationFailure> {
    let body = whole_body()?;
    let (arteries, coronary) = (
        site::<VesselSegment>("systemic arteries"),
        site::<CapillaryBed>("coronary bed"),
    );
    let pressure = |at| index(&body, algebraic("CV", "blood pressure", Context::Site(at)));
    let flow = index(
        &body,
        algebraic(
            "CV",
            "circulation",
            Context::Edge {
                from: arteries,
                to: coronary,
            },
        ),
    );
    let field = index(&body, viscosity());
    let viscosities = [1.0, 2.0, 0.5];
    let each: Vec<_> = (viscosities.iter())
        .map(|mu| {
            vec![
                (pressure(arteries), PRESSURES.0),
                (pressure(coronary), PRESSURES.1),
                (flow, 0.0),
                (field, *mu),
            ]
        })
        .collect();
    let laws: Vec<f64> = staged(&body, &each)?.iter().map(|residuals| -residuals[flow]).collect();
    assert!(laws[0] > 0.0, "{laws:?}");
    for (mu, law) in viscosities.iter().zip(&laws) {
        let closed = laws[0] / mu;
        assert!(
            (law - closed).abs() <= 1e-12 * closed,
            "at viscosity {mu}: {law} against {closed}"
        );
    }
    Ok(())
}

#[test]
fn only_the_whole_body_reads_viscosity() -> Result<(), AdmissionError> {
    let reads = |body: &CompiledBody| {
        let key = ParameterKey::new("cv.reference-viscosity-reciprocal");
        let declared = body.parameters().iter().any(|slot| slot.key() == key);
        (declared, body.rows().iter().any(|row| row.kind() == viscosity()))
    };
    assert_eq!(reads(&whole_body()?), (true, true));
    assert_eq!(reads(&arena_body()?), (false, false));
    Ok(())
}
