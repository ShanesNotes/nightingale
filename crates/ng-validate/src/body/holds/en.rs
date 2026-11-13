use super::FIXED;
use ng_contract::*;

constant_routes! {Endocrine;
    Secretion<Insulin> => "en.insulin-secretion",
    Clearance<Insulin> => "en.insulin-clearance",
    Secretion<Glucagon> => "en.glucagon-secretion",
    Clearance<Glucagon> => "en.glucagon-clearance",
    Secretion<Renin> => "en.renin-secretion",
    Clearance<Renin> => "en.renin-clearance",
    AngiotensinIIMade => "en.angiotensin-ii-made",
    Clearance<AngiotensinII> => "en.angiotensin-ii-clearance",
    Secretion<Aldosterone> => "en.aldosterone-secretion",
    Clearance<Aldosterone> => "en.aldosterone-clearance",
    Secretion<NatriureticPeptide> => "en.natriuretic-peptide-secretion",
    Clearance<NatriureticPeptide> => "en.natriuretic-peptide-clearance",
    Secretion<ParathyroidHormone> => "en.parathyroid-hormone-secretion",
    Clearance<ParathyroidHormone> => "en.parathyroid-hormone-clearance",
    Secretion<Cortisol> => "en.cortisol-secretion",
    Clearance<Cortisol> => "en.cortisol-clearance",
}

const SECRETION: &str = <Secretion<Vasopressin> as BoundaryProcess>::NAME;
const CLEARANCE: &str = <Clearance<Vasopressin> as BoundaryProcess>::NAME;

/// The slice's vasopressin: secretion follows the central veins' sodium concentration on a logistic, clearance is
/// first order.
fn vasopressin<'b>(
    laws: &mut FragmentBuilder<'b, Endocrine>,
    ports: &HeldPorts<'b, Endocrine>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Minutes, 60_000_000_000)?;
    let secretion = ports.route::<Vasopressin>(SECRETION)?.route;
    let clearance = ports.route::<Vasopressin>(CLEARANCE)?.route;
    let most = laws.parameter::<Rate<Vasopressin>>(ParameterKey::new("en.avp-most"), Admissible::positive());
    let set =
        laws.parameter::<Concentration<Sodium>>(ParameterKey::new("en.avp-sodium"), Admissible::positive());
    let width =
        laws.parameter::<Concentration<Sodium>>(ParameterKey::new("en.avp-width"), Admissible::positive());
    let speed =
        laws.parameter::<Rate<Dimensionless>>(ParameterKey::new("en.avp-clearance"), Admissible::positive());
    let rate = laws.kernel(|mut k| {
        let (sodium, water) = (
            k.read(&secretion.amount::<Sodium>()),
            k.read(&secretion.amount::<Water>()),
        );
        let volume = k.volume(water);
        let ratio = k.divide(sodium, volume);
        let seen: Expr<'_, '_, _, Concentration<Sodium>> = k.as_dimension(ratio);
        let center = k.parameter(&set);
        let above = k.subtract(seen, center);
        let spread = k.parameter(&width);
        let ratio = k.divide(above, spread);
        let drive = k.as_dimension(ratio);
        let share = k.logistic(drive);
        let full = k.parameter(&most);
        let rate = k.scale(full, share);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&secretion, rate));
    let rate = laws.kernel(|mut k| {
        let held = k.read(&clearance.amount::<Vasopressin>());
        let (zero, pace) = (k.zero(), k.parameter(&speed));
        let leaving = k.relax(held, zero, pace);
        k.finish(leaving, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&clearance, rate));
    Ok(())
}

/// Each hormone's secretion and clearance, and each hormone in the interstitium, which the capillary wall keeps
/// back until its ticket.
pub(super) fn endocrine(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let supplies = [CONSTANT, &[SECRETION, CLEARANCE]].concat();
    let declaration = HeldDeclaration::<Endocrine>::new(supplies, Vec::new(), FIXED)
        .pool::<Insulin, CapillaryBed>(Space::Interstitium)
        .pool::<Glucagon, CapillaryBed>(Space::Interstitium)
        .pool::<AngiotensinII, CapillaryBed>(Space::Interstitium)
        .pool::<Aldosterone, CapillaryBed>(Space::Interstitium)
        .pool::<NatriureticPeptide, CapillaryBed>(Space::Interstitium)
        .pool::<ParathyroidHormone, CapillaryBed>(Space::Interstitium)
        .pool::<Cortisol, CapillaryBed>(Space::Interstitium);
    let (mut laws, ports) = body.hold(declaration)?.parts();
    constants(&mut laws, &ports)?;
    vasopressin(&mut laws, &ports)?;
    body.add(laws.finish());
    Ok(())
}
