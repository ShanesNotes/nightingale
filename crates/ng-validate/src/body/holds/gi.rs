use super::{FIXED, enclosure_law, hours};
use ng_contract::*;

constant_routes! {Gastrointestinal;
    Feces<Water> => "gi.feces-water",
    Feces<Sodium> => "gi.feces-sodium",
    Feces<Potassium> => "gi.feces-potassium",
    Feces<Chloride> => "gi.feces-chloride",
    Feces<Glucose> => "gi.feces-glucose",
    Feces<Triglyceride> => "gi.feces-triglyceride",
    Feces<AminoAcids> => "gi.feces-amino-acids",
    Feces<Calcium> => "gi.feces-calcium",
    Feces<Phosphate> => "gi.feces-phosphate",
    Feces<Magnesium> => "gi.feces-magnesium",
    Feces<Iron> => "gi.feces-iron",
}

fn absorption<'b, Q: Substance>(
    laws: &mut FragmentBuilder<'b, Gastrointestinal>,
    ports: &HeldPorts<'b, Gastrointestinal>,
    name: &'static str,
    key: &'static str,
) -> Result<(), AdmissionError> {
    let BoundaryRoute { route, drive } = ports.route::<Q>(name)?;
    let basal = laws.parameter::<Rate<Q>>(ParameterKey::new(key), Admissible::nonnegative());
    let class = hours()?;
    let rate = laws.kernel(|mut k| {
        let (steady, meal) = (k.parameter(&basal), k.read(&drive));
        let total = k.add(steady, meal);
        k.finish(total, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, rate));
    Ok(())
}

fn exchange<'b, Q: Ingested>(
    laws: &mut FragmentBuilder<'b, Gastrointestinal>,
    ports: &GastrointestinalPorts<'b>,
) -> Result<(), AdmissionError> {
    let route = ports.transit::<Q>(laws);
    let pace = laws
        .parameter::<Rate<Dimensionless>>(ParameterKey::new("gi.transit-exchange"), Admissible::positive());
    let class = hours()?;
    let (from, to) = (route.from_amount::<Q>(), route.to_amount::<Q>());
    let balanced = laws.edge_kernel(|mut k| {
        let (up, down, pace) = (k.at_edge(&from), k.at_edge(&to), k.parameter(&pace));
        let rate = k.relax(up, down, pace);
        k.finish(rate, laws.every_mode(), class)
    });
    laws.transfer_along(&route, balanced);
    Ok(())
}

pub(super) fn gastrointestinal(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let supplies = [&Meal::BOUNDARIES[..], CONSTANT].concat();
    let declaration = HeldDeclaration::<Gastrointestinal>::new(supplies, Vec::new(), FIXED)
        .pool::<Triglyceride, Beds<Gut>>(Space::Interstitium)
        .pool::<AminoAcids, CapillaryBed>(Space::Interstitium);
    let mut held = body.hold(declaration)?;
    let (laws, ports) = held.stand_ins();
    enclosure_law(laws, ports, ParameterKey::new("gi.abdominal-pressure"))?;
    absorption::<Water>(laws, ports, "gut water", "gi.basal-water")?;
    absorption::<Sodium>(laws, ports, "gut sodium", "gi.basal-sodium")?;
    absorption::<Potassium>(laws, ports, "gut potassium", "gi.basal-potassium")?;
    absorption::<Chloride>(laws, ports, "gut chloride", "gi.basal-chloride")?;
    absorption::<Glucose>(laws, ports, "gut glucose", "gi.basal-glucose")?;
    absorption::<Triglyceride>(laws, ports, "gut triglyceride", "gi.basal-triglyceride")?;
    absorption::<AminoAcids>(laws, ports, "gut amino acids", "gi.basal-amino-acids")?;
    absorption::<DietBase>(laws, ports, "gut diet base", "gi.basal-diet-base")?;
    absorption::<Calcium>(laws, ports, "gut calcium", "gi.basal-calcium")?;
    absorption::<Phosphate>(laws, ports, "gut phosphate", "gi.basal-phosphate")?;
    absorption::<Magnesium>(laws, ports, "gut magnesium", "gi.basal-magnesium")?;
    absorption::<Iron>(laws, ports, "gut iron", "gi.basal-iron")?;
    constants(laws, ports)?;
    let (mut laws, ports) = body.gastrointestinal(Some(held))?.parts();
    exchange::<Water>(&mut laws, &ports)?;
    exchange::<Sodium>(&mut laws, &ports)?;
    exchange::<Potassium>(&mut laws, &ports)?;
    exchange::<Chloride>(&mut laws, &ports)?;
    exchange::<Glucose>(&mut laws, &ports)?;
    exchange::<Triglyceride>(&mut laws, &ports)?;
    exchange::<AminoAcids>(&mut laws, &ports)?;
    exchange::<DietBase>(&mut laws, &ports)?;
    exchange::<Calcium>(&mut laws, &ports)?;
    exchange::<Phosphate>(&mut laws, &ports)?;
    exchange::<Magnesium>(&mut laws, &ports)?;
    exchange::<Iron>(&mut laws, &ports)?;
    body.add(laws.finish());
    Ok(())
}
