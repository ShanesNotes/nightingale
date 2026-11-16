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
    staged_on(body, body_packet(body)?, each)
}

fn staged_on(
    body: &CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    each: &[Vec<(usize, f64)>],
) -> Result<Vec<Vec<f64>>, InitializationFailure> {
    let lens = body_lens(body)?;
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

/// The packet's row at an exponent of -1, so `(1 - h)^C - 1` is `h / (1 - h)`: 1.5 at the reference hematocrit 0.6,
/// so a row that read `H_ref` for `1 - H_ref` would take 2/3 there. Plasma 2, red cells' own hemoglobin 5, 4 at the
/// reference and a wall factor of 1.25.
fn closed_form(hematocrit: f64) -> f64 {
    let odds = |share: f64| share / (1.0 - share);
    let fraction = odds(hematocrit) / odds(0.6);
    2.0 * 1.25 * (1.0 + (4.0 - 1.0) * fraction * 1.25)
}

/// At an exponent of 1 the row is linear in the hematocrit: `(1 - h)^1 - 1` is `-h`, so the fraction is `h / 0.6`.
#[test]
fn viscosity_at_a_positive_exponent_follows_the_row() -> Result<(), InitializationFailure> {
    let body = whole_body()?;
    let (hemoglobin, water, field) = (
        index(&body, veins(Species::Hemoglobin)),
        index(&body, veins(Species::Water)),
        index(&body, viscosity()),
    );
    let packet = body_packet_at_viscosity_exponent(&body, Measure::new(1.0)?)?;
    let cases = [(0.4, 8.75), (0.2, 5.625)];
    let each: Vec<_> = (cases.iter())
        .map(|(hematocrit, _)| {
            vec![
                (water, WATER),
                (hemoglobin, 5.0 * hematocrit * WATER),
                (field, 1.0),
            ]
        })
        .collect();
    for ((hematocrit, closed), residuals) in cases.iter().zip(staged_on(&body, packet, &each)?) {
        let law = 1.0 - residuals[field];
        assert!(
            (law - closed).abs() <= 1e-12 * closed,
            "at a hematocrit of {hematocrit}: {law} against {closed}"
        );
    }
    Ok(())
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
        .map(|hematocrit| {
            vec![
                (water, WATER),
                (hemoglobin, 5.0 * hematocrit * WATER),
                (field, 1.0),
            ]
        })
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
    let laws: Vec<f64> = staged(&body, &each)?
        .iter()
        .map(|residuals| -residuals[flow])
        .collect();
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

/// Viscosity scales each of the body's 21 circulation edges by its ratio, and no other law reads it: the five valves,
/// the liver's three sinusoids and the lymph and kidney edges keep theirs.
#[test]
fn viscosity_scales_every_circulation_edge_and_no_other_law() -> Result<(), InitializationFailure> {
    let body = whole_body()?;
    let field = index(&body, viscosity());
    let cv = |row: &&Row, local: &dyn Fn(&str) -> bool| matches!(row.kind(), RowKind::Algebraic(slot) if slot.law.owner == "CV" && local(slot.law.local));
    let rows: Vec<&Row> = body.rows().iter().collect();
    let of = |local: &dyn Fn(&str) -> bool| -> Vec<usize> {
        (0..rows.len()).filter(|&at| cv(&rows[at], local)).collect()
    };
    let circulation = of(&|local| local == "circulation");
    let pressures = of(&|local| local.contains("pressure"));
    let mut start: Vec<(usize, f64)> = (pressures.iter().enumerate())
        .map(|(n, &at)| (at, 10.0 + 7.0 * n as f64))
        .collect();
    start.extend(circulation.iter().map(|&at| (at, 0.0)));
    let each: Vec<_> = [1.0, 2.0]
        .iter()
        .map(|mu| start.iter().copied().chain([(field, *mu)]).collect::<Vec<_>>())
        .collect();
    let staged = staged(&body, &each)?;
    let moved: Vec<usize> = (0..rows.len())
        .filter(|&at| staged[0][at] != staged[1][at])
        .collect();
    let mut read = circulation.clone();
    read.push(field);
    read.sort_unstable();
    assert_eq!(circulation.len(), 21);
    assert_eq!(moved, read);
    for at in circulation {
        let (once, twice) = (-staged[0][at], -staged[1][at]);
        assert!(
            once != 0.0 && (twice - once / 2.0).abs() <= 1e-12 * once.abs(),
            "{:?}: {once} at viscosity 1 and {twice} at 2",
            rows[at].kind()
        );
    }
    Ok(())
}
