use ng_contract::catalog::{LungGases, Reads};
use ng_contract::*;
use ng_respiratory::{Ventilation, exchange};

/// A held enclosure's fluid must exist, so while the abdomen is held the respiratory card keeps a declaration and the
/// pleural fluid needs no entry in it.
const HELD: &str =
    "body fixture: the pleural fluid while the abdomen is held, and airway heat until conduction";

constant_routes! {Respiratory;
    AirwayHeat => "rs.airway-heat",
}

pub(super) fn respiratory(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Respiratory>::new(CONSTANT.to_vec(), Vec::new(), HELD);
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
    ventilation.work(&mut laws, &ports)?;
    ventilation.water(&mut laws, &ports)?;
    body.add(laws.finish());
    Ok(())
}
