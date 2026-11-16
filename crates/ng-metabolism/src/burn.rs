use crate::diffusion::CellOxygen;
use ng_contract::*;

type Bed<'b, 'k> = Kernel<'b, 'k, Metabolism, CapillaryBed>;
type SiteKey<'b, U> = SiteParameter<'b, Metabolism, CapillaryBed, U>;

const BURN_NS: u64 = 1_000_000_000;

/// The oxygen a bed's cells burn each second: `B = smooth_min(D, k P, w)`, its demand until its cells' partial
/// pressure can no longer pass that much, so the burn cannot draw a cell's oxygen below zero.
struct OxygenRate<'a, 'b> {
    cells: &'a CellOxygen<'b>,
    demand: SiteKey<'b, Rate<Oxygen>>,
    reach: SiteKey<'b, Conductance<Oxygen>>,
    width: SiteKey<'b, Rate<Oxygen>>,
}

impl<'b> OxygenRate<'_, 'b> {
    fn at<'k>(&self, k: &mut Bed<'b, 'k>) -> Expr<'b, 'k, Metabolism, Rate<Oxygen>> {
        let pressure = self.cells.pressure(k);
        let reach = k.site_parameter(&self.reach);
        let allowed = k.conductance_flow(reach, pressure);
        let (demand, width) = (k.site_parameter(&self.demand), k.site_parameter(&self.width));
        k.smooth_min(demand, allowed, width)
    }
}

/// Each bed's cells burn glucose, `glucose + 6 O2 -> 6 CO2 + 6 H2O`, and fatty acids, `fatty acid + 23 O2 -> 16 CO2 +
/// 16 H2O`, each releasing its energy into the bed's heat. The glucose burn takes a share `s` of the bed's oxygen rate
/// and the fatty acid burn the rest, so the share fixes the respiratory quotient at `s + 16 (1 - s) / 23`.
pub fn burn<'b>(laws: &mut FragmentBuilder<'b, Metabolism>, cells: &CellOxygen<'b>) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, BURN_NS)?;
    let key = ParameterKey::new;
    let rate = OxygenRate {
        cells,
        demand: laws.site_parameter(key("mt.oxygen-demand"), Admissible::nonnegative()),
        reach: laws.site_parameter(key("mt.oxygen-reach"), Admissible::positive()),
        width: laws.site_parameter(key("mt.oxygen-rate-width"), Admissible::positive()),
    };
    let share: SiteKey<'b, Dimensionless> = laws.site_parameter(key("mt.glucose-share"), Admissible::unit_interval());
    let at = Space::Cells;
    let glucose = laws.reaction::<CapillaryBed, Oxygen, 6>(
        "glucose burn",
        Proportions::<Oxygen, 6>::using(at)
            .uses::<Glucose, 1>(at)
            .makes::<CarbonDioxide, 6>(at)
            .makes::<Water, 6>(at)
            .energy_into(Space::Thermal),
    );
    let fat = laws.reaction::<CapillaryBed, Oxygen, 23>(
        "fatty acid burn",
        Proportions::<Oxygen, 23>::using(at)
            .uses::<FattyAcids, 1>(at)
            .makes::<CarbonDioxide, 16>(at)
            .makes::<Water, 16>(at)
            .energy_into(Space::Thermal),
    );
    let glucose_rate = laws.site_kernel(|mut k| {
        let (oxygen, share) = (rate.at(&mut k), k.site_parameter(&share));
        let value = k.scale(oxygen, share);
        k.finish(value, laws.every_mode(), class)
    });
    laws.react_at(&glucose, glucose_rate);
    let fat_rate = laws.site_kernel(|mut k| {
        let (oxygen, share) = (rate.at(&mut k), k.site_parameter(&share));
        let glucose = k.scale(oxygen, share);
        let value = k.subtract(oxygen, glucose);
        k.finish(value, laws.every_mode(), class)
    });
    laws.react_at(&fat, fat_rate);
    Ok(())
}

/// Each bed's heat passes to the core store first order, `J = E k`, at one speed for every bed.
pub fn bed_heat<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    ports: &MetabolismPorts<'b>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, BURN_NS)?;
    let route = ports.bed_heat(laws);
    let held = route.from_amount::<Energy>();
    let speed: SiteKey<'b, Rate<Dimensionless>> =
        laws.site_parameter(ParameterKey::new("mt.bed-heat-speed"), Admissible::nonnegative());
    let rate = laws.site_kernel(|mut k| {
        let (held, zero, speed) = (k.at_site(&held), k.zero(), k.site_parameter(&speed));
        let value = k.relax(held, zero, speed);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer_at(&route, rate);
    Ok(())
}
