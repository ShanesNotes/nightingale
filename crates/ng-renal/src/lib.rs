//! The kidney as segments of the urinary network: filtration out of a bed, flow down the tract, each species reclaimed
//! by its own law, and tubuloglomerular feedback. The map says which segment does what.

use ng_contract::*;

mod feedback;
mod filtration;
mod lumen_pressure;
mod reclaim_chloride;
mod reclaim_potassium;
mod reclaim_sodium;
mod reclaim_water;
mod shared;
mod tubular_flow;

use feedback::feedback;
use lumen_pressure::lumen_pressure;
use shared::Saturable;

const TUBULE_NS: u64 = 1_000_000_000;

pub fn declare<'b>(scope: RenalScope<'b>) -> Result<Fragment<'b, Renal>, AdmissionError> {
    let (mut rn, ports) = scope.parts();
    let class = RateClass::new(RateBand::Reflex, TUBULE_NS)?;
    let lumen = lumen_pressure(&mut rn, &ports, class);
    filtration::declare(&mut rn, &ports, &lumen, class);
    tubular_flow::declare(&mut rn, &ports, &lumen, class);
    reclaim_water::declare(&mut rn, &ports, class);

    let sodium = ports.reclaim::<Sodium>(&mut rn);
    let potassium = ports.reclaim::<Potassium>(&mut rn);
    let transport = Saturable::declare(&mut rn);
    reclaim_sodium::declare(&mut rn, &sodium, &transport, class);
    reclaim_potassium::declare(&mut rn, &sodium, &potassium, &transport, class);
    reclaim_chloride::declare(&mut rn, &ports, &sodium, &potassium, &transport, class);

    feedback(&mut rn, &ports, class)?;
    Ok(rn.finish())
}
