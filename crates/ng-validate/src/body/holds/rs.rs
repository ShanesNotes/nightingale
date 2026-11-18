use super::{FIXED, enclosure_law};
use ng_contract::catalog::{LungGases, Reads};
use ng_contract::*;
use ng_respiratory::{Ventilation, exchange};

constant_routes! {Respiratory;
    Expired<Water> => "rs.expired-water",
    AirwayHeat => "rs.airway-heat",
}

/// The alveolar unit's gas and its exchange are the respiratory card's own laws. Its stand-ins hold the pleural
/// pressure until the breath is built, and the water and heat that leave with expired gas.
pub(super) fn respiratory(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Respiratory>::new(CONSTANT.to_vec(), Vec::new(), FIXED)
        .pool::<Water, Beds<Lung>>(Space::Gas);
    let mut held = body.hold(declaration)?;
    {
        let (laws, ports) = held.stand_ins();
        enclosure_law(laws, ports, ParameterKey::new("rs.pleural-pressure"))?;
        constants(laws, ports)?;
    }
    let (mut laws, ports) = body.respiratory(Some(held))?.parts();
    let ventilation = Ventilation::declare(&mut laws, &ports)?;
    let plasma: LungGases<Reads<'_, Respiratory>> = laws.receive();
    exchange(&mut laws, &ports, &ventilation, &plasma)?;
    body.add(laws.finish());
    Ok(())
}
