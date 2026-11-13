use super::{FIXED, enclosure_law};
use ng_contract::*;

constant_routes! {Respiratory;
    Inspired<Oxygen> => "rs.inspired-oxygen",
    Inspired<CarbonDioxide> => "rs.inspired-carbon-dioxide",
    Inspired<NitrogenGas> => "rs.inspired-nitrogen-gas",
    Expired<Oxygen> => "rs.expired-oxygen",
    Expired<CarbonDioxide> => "rs.expired-carbon-dioxide",
    Expired<NitrogenGas> => "rs.expired-nitrogen-gas",
    Expired<Water> => "rs.expired-water",
    AirwayHeat => "rs.airway-heat",
}

pub(super) fn respiratory(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Respiratory>::new(CONSTANT.to_vec(), Vec::new(), FIXED)
        .pool::<Oxygen, Beds<Lung>>(Space::Gas)
        .pool::<CarbonDioxide, Beds<Lung>>(Space::Gas)
        .pool::<NitrogenGas, Beds<Lung>>(Space::Gas)
        .pool::<Water, Beds<Lung>>(Space::Gas);
    let (mut laws, ports) = body.hold(declaration)?.parts();
    enclosure_law(&mut laws, &ports, ParameterKey::new("rs.pleural-pressure"))?;
    constants(&mut laws, &ports)?;
    body.add(laws.finish());
    Ok(())
}
