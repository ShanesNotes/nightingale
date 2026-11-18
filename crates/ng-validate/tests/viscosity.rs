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
    let first = |workspace: &AdmissionWorkspace| vec![workspace.legs_of(0)[0]];
    let staged = staged_in(body, packet, each, first)?;
    Ok(staged.into_iter().flatten().collect())
}

fn staged_in(
    body: &CompiledBody,
    packet: ParameterPacket<SyntheticFixture>,
    each: &[Vec<(usize, f64)>],
    legs: impl Fn(&AdmissionWorkspace) -> Vec<LegHandle>,
) -> Result<Vec<Vec<Vec<f64>>>, InitializationFailure> {
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
        let at_each = (legs(&workspace).into_iter())
            .map(|leg| system.stage_residuals(&workspace, leg))
            .collect::<Result<Vec<_>, _>>()?;
        residuals.push(at_each);
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
    assert_eq!(circulation.len(), 22);
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

fn debt(bed: SiteId) -> RowKind {
    algebraic("MT", "oxygen_debt_per_bed", Context::Site(bed))
}

fn circulation(from: SiteId, to: SiteId) -> RowKind {
    algebraic("CV", "circulation", Context::Edge { from, to })
}

#[test]
fn a_beds_inflow_resistance_falls_with_its_debt_by_its_form() -> Result<(), InitializationFailure> {
    let body = whole_body()?;
    let (arteries, coronary, veins) = (
        site::<VesselSegment>("systemic arteries"),
        site::<CapillaryBed>("coronary bed"),
        site::<VesselSegment>("central veins"),
    );
    let pressure = |at| index(&body, algebraic("CV", "blood pressure", Context::Site(at)));
    let (inflow, outflow) = (
        index(&body, circulation(arteries, coronary)),
        index(&body, circulation(coronary, veins)),
    );
    let (owed, field) = (index(&body, debt(coronary)), index(&body, viscosity()));
    let (share, half) = (0.5, 8e-3);
    let debts = [0.0, -1e-29, half, 3.0 * half];
    let each: Vec<_> = (debts.iter())
        .map(|d| {
            vec![
                (pressure(arteries), PRESSURES.0),
                (pressure(coronary), 50.0),
                (pressure(veins), PRESSURES.1),
                (inflow, 0.0),
                (outflow, 0.0),
                (field, 1.0),
                (owed, *d),
            ]
        })
        .collect();
    let staged = staged(&body, &each)?;
    let (into, out): (Vec<f64>, Vec<f64>) = staged.iter().map(|r| (-r[inflow], -r[outflow])).unzip();
    assert!(into[0] > 0.0 && out[0] > 0.0, "{into:?} {out:?}");
    assert_eq!(into[1].to_bits(), into[0].to_bits(), "a debt a hair under zero");
    for ((d, law), away) in debts.iter().zip(&into).zip(&out) {
        let closed = into[0] * (d + half) / (d + half - share * d);
        assert!(
            (law - closed).abs() <= 1e-12 * closed,
            "at a debt of {d}: {law} against {closed}"
        );
        assert_eq!(
            away.to_bits(),
            out[0].to_bits(),
            "the edge out of the bed at a debt of {d}"
        );
    }
    Ok(())
}

#[test]
fn the_shunt_keeps_its_fraction_when_the_pulmonary_bed_owes_oxygen() -> Result<(), InitializationFailure> {
    let body = whole_body()?;
    let artery = site::<VesselSegment>("pulmonary artery");
    let lung = site::<CapillaryBed>("pulmonary bed");
    let vein = site::<VesselSegment>("pulmonary vein");
    let pressure = |at| index(&body, algebraic("CV", "blood pressure", Context::Site(at)));
    let inflow = index(&body, circulation(artery, lung));
    let outflow = index(&body, circulation(lung, vein));
    let shunt = index(&body, circulation(artery, vein));
    let trial = |owed, bed_pressure, through, bypass| {
        vec![
            (pressure(artery), PRESSURES.0),
            (pressure(lung), bed_pressure),
            (pressure(vein), PRESSURES.1),
            (inflow, through),
            (outflow, through),
            (shunt, bypass),
            (index(&body, viscosity()), 1.0),
            (index(&body, debt(lung)), owed),
        ]
    };
    let midpoint = (PRESSURES.0 + PRESSURES.1) / 2.0;
    for owed in [0.0, 1e-3] {
        let first = &staged(&body, &[trial(owed, midpoint, 0.0, 0.0)])?[0];
        let (into, out) = (-first[inflow], -first[outflow]);
        assert!(into > 0.0 && out > 0.0);
        let bed_pressure = PRESSURES.1 + (PRESSURES.0 - PRESSURES.1) * into / (into + out);
        let through = 2.0 * into * out / (into + out);
        let expected_bypass = through * 0.02 / 0.98;
        let rows = &staged(&body, &[trial(owed, bed_pressure, through, expected_bypass)])?[0];
        let (into, out, bypass) = (
            through - rows[inflow],
            through - rows[outflow],
            expected_bypass - rows[shunt],
        );
        assert!(into > 0.0 && bypass > 0.0 && (into - out).abs() <= 1e-12 * into);
        let fraction = bypass / (bypass + into);
        assert!(
            (fraction - 0.02).abs() < 1e-12,
            "shunt fraction {fraction} at debt {owed}"
        );
    }
    Ok(())
}

#[test]
fn every_beds_inflow_but_the_liver_the_peritubular_and_the_pulmonary_bed_widens_with_its_debt()
-> Result<(), InitializationFailure> {
    let body = whole_body()?;
    let rows: Vec<RowKind> = body.rows().iter().map(Row::kind).collect();
    let owed: Vec<usize> = (0..rows.len())
        .filter(|&at| matches!(rows[at], RowKind::Algebraic(slot) if slot.law.local == "oxygen_debt_per_bed"))
        .collect();
    let field = index(&body, viscosity());
    let pressures: Vec<usize> = (0..rows.len())
        .filter(|&at| matches!(rows[at], RowKind::Algebraic(slot) if slot.law.owner == "CV" && slot.law.local.contains("pressure")))
        .collect();
    let mut start: Vec<(usize, f64)> = (pressures.iter().enumerate())
        .map(|(n, &at)| (at, 10.0 + 7.0 * n as f64))
        .collect();
    start.push((field, 1.0));
    let each: Vec<_> = [0.0, 1.0]
        .iter()
        .map(|d| {
            (start.iter().copied())
                .chain(owed.iter().map(|&at| (at, *d)))
                .collect::<Vec<_>>()
        })
        .collect();
    let staged = staged(&body, &each)?;
    let mut moved: Vec<RowKind> = (0..rows.len())
        .filter(|&at| staged[0][at] != staged[1][at] && !owed.contains(&at))
        .map(|at| rows[at])
        .collect();
    let arteries = site::<VesselSegment>("systemic arteries");
    let mut fed: Vec<RowKind> = [
        "gut bed",
        "muscle bed",
        "glomerular bed",
        "brain bed",
        "coronary bed",
        "fat bed",
        "skin bed",
        "bone bed",
    ]
    .into_iter()
    .map(|bed| circulation(arteries, site::<CapillaryBed>(bed)))
    .collect();
    moved.sort_unstable();
    fed.sort_unstable();
    assert_eq!(owed.len(), 11);
    assert_eq!(moved, fed);
    Ok(())
}

fn cells(bed: &'static str, species: Species) -> RowKind {
    RowKind::Amount {
        site: site::<CapillaryBed>(bed),
        species,
        space: Space::Cells,
    }
}

fn memory(local: &'static str) -> RowKind {
    RowKind::Memory(Slot {
        law: LawName { owner: "CV", local },
        at: Context::Body,
    })
}

fn refused_at_its_row(kind: RowKind) -> Result<(), InitializationFailure> {
    let body = whole_body()?;
    let held = &body.rows()[index(&body, kind)];
    let lens = body_lens(&body)?;
    let parameters = body.bind(body_packet(&body)?)?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(anchor_moment()).reading(&lens);
    let system = body.initialization(parameters, port, request)?;
    let mut trial = system.workspace();
    let slot = trial.endpoint(0).start + held.id().index();
    trial.unknowns_mut()[slot] = -1.0;
    let history = MethodHistory::new(Digest::of_canonical(b"synthetic"), anchor_moment(), Vec::new());
    assert_eq!(
        system.admit(trial, history).err(),
        Some(InitializationFailure::Inconsistent {
            row: AdmissionRow::Body(held.id()),
        })
    );
    Ok(())
}

#[test]
fn admission_refuses_negative_myocardial_work_at_its_memory_row() -> Result<(), InitializationFailure> {
    refused_at_its_row(memory("work memory"))
}

#[test]
fn admission_refuses_a_negative_contractile_store_at_its_row() -> Result<(), InitializationFailure> {
    refused_at_its_row(algebraic("CV", "contractile store", Context::Body))
}

const BEDS: [&str; 11] = [
    "pulmonary bed",
    "gut bed",
    "muscle bed",
    "glomerular bed",
    "peritubular bed",
    "brain bed",
    "coronary bed",
    "liver bed",
    "fat bed",
    "skin bed",
    "bone bed",
];

/// Each bed's cells hold oxygen at 10,000 mmHg, so every oxygen rate sits at its demand and the burns move with the
/// demand alone.
#[test]
fn only_the_coronary_beds_burn_follows_the_hearts_work_and_in_proportion() -> Result<(), InitializationFailure>
{
    let body = whole_body()?;
    let work = index(&body, algebraic("CV", "myocardial_work", Context::Body));
    let oxygen: Vec<usize> = (BEDS.iter())
        .map(|bed| index(&body, cells(bed, Species::Oxygen)))
        .collect();
    let mut plenty: Vec<(usize, f64)> = (BEDS.iter())
        .map(|bed| (index(&body, cells(bed, Species::Water)), 1_000.0))
        .collect();
    plenty.extend(oxygen.iter().map(|row| (*row, 10_000.0)));
    let each: Vec<_> = [0.0, 18_000.0, 36_000.0]
        .iter()
        .map(|w| [plenty.clone(), vec![(work, *w)]].concat())
        .collect();
    let staged = staged(&body, &each)?;
    for (bed, row) in BEDS.iter().zip(&oxygen) {
        let (once, twice) = (
            staged[1][*row] - staged[0][*row],
            staged[2][*row] - staged[0][*row],
        );
        if *bed == "coronary bed" {
            assert!(once != 0.0, "the coronary bed's burn ignores the work");
            assert!(
                (twice - 2.0 * once).abs() <= 1e-6 * once.abs(),
                "{once} then {twice}"
            );
        } else {
            assert_eq!(
                once.to_bits(),
                0f64.to_bits(),
                "the {bed}'s burn moves with the work"
            );
        }
    }
    Ok(())
}

/// Every leg a staged start holds is in systole, the start's mode, so diastole's fall shows only in a run, where the
/// work's whole-beat mean meets what the stroke work paid.
#[test]
fn the_work_follows_the_contractile_store_in_systole_and_the_field_is_the_memory()
-> Result<(), InitializationFailure> {
    let body = whole_body()?;
    let (held, store, activation, work) = (
        index(&body, memory("work memory")),
        index(&body, algebraic("CV", "contractile store", Context::Body)),
        index(&body, memory("activation")),
        index(&body, algebraic("CV", "myocardial_work", Context::Body)),
    );
    let at = |memory: f64, stored: f64, field: f64| {
        vec![(held, memory), (store, stored), (activation, 0.0), (work, field)]
    };
    let each = [
        at(0.0, 1e4, 0.0),
        at(0.0, 2e4, 0.0),
        at(0.0, 3e4, 0.0),
        at(1e4, 1e4, 1e4),
        at(2e4, 1e4, 0.0),
    ];
    let every = |workspace: &AdmissionWorkspace| -> Vec<LegHandle> {
        (0..)
            .map(|point| workspace.legs_of(point))
            .take_while(|legs| !legs.is_empty())
            .flatten()
            .collect()
    };
    let staged = staged_in(&body, body_packet(&body)?, &each, every)?;
    assert!(!staged[0].is_empty());
    for (leg, first) in staged[0].iter().enumerate() {
        let row = |state: usize, row: usize| staged[state][leg][row];
        let (once, twice) = (row(1, held) - first[held], row(2, held) - first[held]);
        let itself = row(3, held) - first[held];
        assert!(once != 0.0, "leg {leg}: the memory ignores the store");
        assert!(
            (twice - 2.0 * once).abs() <= 1e-9 * once.abs(),
            "leg {leg}: {once} then {twice}"
        );
        assert!(
            itself * once < 0.0,
            "leg {leg}: the memory moves with itself as with the store"
        );
        assert_eq!(row(3, work), 0.0, "leg {leg}: the field is not the memory");
        assert_eq!(row(4, work).abs(), 2e4, "leg {leg}: the field is not the memory");
    }
    Ok(())
}
