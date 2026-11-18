use super::FIXED;
use ng_contract::catalog::{LungGases, Reads};
use ng_contract::*;
use ng_respiratory::{Ventilation, exchange};

constant_routes! {Respiratory;
    Expired<Water> => "rs.expired-water",
    AirwayHeat => "rs.airway-heat",
}

pub(super) fn respiratory(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Respiratory>::new(CONSTANT.to_vec(), Vec::new(), FIXED)
        .pool::<Water, Beds<Lung>>(Space::Gas);
    let mut held = body.hold(declaration)?;
    {
        let (laws, ports) = held.stand_ins();
        constants(laws, ports)?;
    }
    let (mut laws, ports) = body.respiratory(Some(held))?.parts();
    let ventilation = Ventilation::declare(&mut laws, &ports)?;
    let plasma: LungGases<Reads<'_, Respiratory>> = laws.receive();
    exchange(&mut laws, &ports, &ventilation, &plasma)?;
    ventilation.pleural(&mut laws, &ports)?;
    body.add(laws.finish());
    Ok(())
}
