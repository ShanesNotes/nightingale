//! Fluid between the spaces: Starling exchange at every capillary bed and lymph back to the veins. No law here names a
//! space's pool, and what crosses with the water follows from what both ends hold.

use ng_contract::*;

mod exchange_albumin;
mod exchange_chloride;
mod exchange_potassium;
mod exchange_sodium;
mod interstitial_pressure;
mod lymph;
mod shared;
mod starling;

use interstitial_pressure::interstitial_pressure;

const EXCHANGE_NS: u64 = 1_000_000_000;

pub fn declare<'b>(scope: FluidsScope<'b>) -> Result<Fragment<'b, Fluids>, AdmissionError> {
    let (mut fl, ports) = scope.parts();
    let class = RateClass::new(RateBand::Reflex, EXCHANGE_NS)?;
    let route = ports.starling(&mut fl);
    let interstitial = interstitial_pressure(&mut fl, &ports, &route, class);
    starling::declare(&mut fl, &route, &interstitial, class);

    let wall = fl.site_parameter::<CapillaryBed, Rate<Water>>(
        ParameterKey::new("fl.ion-exchange"),
        Admissible::positive(),
    );
    exchange_sodium::declare(&mut fl, &ports, &route, &wall, class);
    exchange_potassium::declare(&mut fl, &ports, &route, &wall, class);
    exchange_chloride::declare(&mut fl, &ports, &route, &wall, class);
    exchange_albumin::declare(&mut fl, &ports, &route, class);
    lymph::declare(&mut fl, &ports, &interstitial, class);
    Ok(fl.finish())
}
