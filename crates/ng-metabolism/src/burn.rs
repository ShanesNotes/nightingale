use crate::diffusion::CellOxygen;
use ng_contract::catalog::{Defines, OxygenDebtPerBed};
use ng_contract::*;

type Bed<'b, 'k> = Kernel<'b, 'k, Metabolism, CapillaryBed>;
type SiteKey<'b, U> = SiteParameter<'b, Metabolism, CapillaryBed, U>;

const BURN_NS: u64 = 1_000_000_000;

/// The oxygen a bed's cells burn each second: `B = smooth_max(D, 0, s D) - smooth_max(D - k P, 0, s D)`, its demand
/// until its cells' partial pressure can no longer pass that much. It is the smooth minimum `smooth_min(D, k P, s D)`
/// less its value at no cell oxygen, written as two smooth maxima so that a reach far above the demand cannot lose the
/// demand to rounding. Both terms read the same operands at `P = 0`, so the rate is exactly zero there; it never runs a
/// burn backward and passes the demand by at most `D (sqrt(1 + s^2) - 1) / 2`. The width is a share of the demand, so a
/// zero demand burns nothing. These hold while `(s D)^2` stays a normal float: from a demand of 1e-150 up at `s` = 0.02.
struct OxygenRate<'a, 'b> {
    cells: &'a CellOxygen<'b>,
    demand: SiteKey<'b, Rate<Oxygen>>,
    reach: SiteKey<'b, Conductance<Oxygen>>,
    width_share: SiteKey<'b, Dimensionless>,
}

impl<'b> OxygenRate<'_, 'b> {
    fn at<'k>(&self, k: &mut Bed<'b, 'k>) -> Expr<'b, 'k, Metabolism, Rate<Oxygen>> {
        let pressure = self.cells.pressure(k);
        let reach = k.site_parameter(&self.reach);
        let allowed = k.conductance_flow(reach, pressure);
        let (demand, share) = (
            k.site_parameter(&self.demand),
            k.site_parameter(&self.width_share),
        );
        let width = k.scale(demand, share);
        let none = k.zero();
        let ceiling = k.smooth_max(demand, none, width);
        let short = k.subtract(demand, allowed);
        let shortfall = k.smooth_max(short, none, width);
        k.subtract(ceiling, shortfall)
    }
}

/// Each bed's cells burn glucose, `glucose + 6 O2 -> 6 CO2 + 6 H2O`, and fatty acids, `fatty acid + 23 O2 -> 16 CO2 +
/// 16 H2O`, each releasing its energy into the bed's heat. The glucose burn takes a share `s` of the bed's oxygen rate
/// and the fatty acid burn the rest, so the share fixes the respiratory quotient at `s + 16 (1 - s) / 23`.
pub fn burn<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    cells: &CellOxygen<'b>,
) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Reflex, BURN_NS)?;
    let key = ParameterKey::new;
    let rate = OxygenRate {
        cells,
        demand: laws.site_parameter(key("mt.oxygen-demand"), Admissible::nonnegative()),
        reach: laws.site_parameter(key("mt.oxygen-reach"), Admissible::positive()),
        width_share: laws.site_parameter(key("mt.oxygen-rate-width-share"), Admissible::positive()),
    };
    let share: SiteKey<'b, Dimensionless> =
        laws.site_parameter(key("mt.glucose-share"), Admissible::unit_interval());
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
    lactate(laws, &rate, class)
}

/// A smooth maximum of any width would split glucose at rest, so the split's maximum has no width and is exact.
fn lactate<'b>(
    laws: &mut FragmentBuilder<'b, Metabolism>,
    rate: &OxygenRate<'_, 'b>,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let key = ParameterKey::new;
    let per_short: SiteKey<'b, Quotient<Glucose, Oxygen>> =
        laws.site_parameter(key("mt.glucose-per-oxygen-short"), Admissible::nonnegative());
    let speed: SiteKey<'b, Rate<Dimensionless>> =
        laws.site_parameter(key("mt.lactate-burn-speed"), Admissible::nonnegative());
    let half: SiteKey<'b, Pressure> =
        laws.site_parameter(key("mt.lactate-burn-half-pressure"), Admissible::positive());
    let per_lactate: SiteKey<'b, Quotient<Oxygen, Lactate>> =
        laws.site_parameter(key("mt.oxygen-per-lactate"), Admissible::positive());
    let at = Space::Cells;
    let glycolysis = laws.reaction::<CapillaryBed, Glucose, 1>(
        "glycolysis",
        Proportions::<Glucose>::using(at)
            .makes::<Lactate, 2>(at)
            .energy_into(Space::Thermal),
    );
    let burned = laws.reaction::<CapillaryBed, Lactate, 1>(
        "lactate burn",
        Proportions::<Lactate>::using(at)
            .uses::<Oxygen, 3>(at)
            .makes::<CarbonDioxide, 3>(at)
            .makes::<Water, 3>(at)
            .energy_into(Space::Thermal),
    );
    let split = laws.site_kernel(|mut k| {
        let (oxygen, demand) = (rate.at(&mut k), k.site_parameter(&rate.demand));
        let gap = k.subtract(demand, oxygen);
        let none = k.zero();
        let short = k.smooth_max(gap, none, none);
        let per = k.site_parameter(&per_short);
        let glucose = k.multiply(short, per);
        let value = k.as_dimension(glucose);
        k.finish(value, laws.every_mode(), class)
    });
    laws.react_at(&glycolysis, split);
    let held = burned.amount::<Lactate>();
    let burn_rate = laws.site_kernel(|mut k| {
        let (held, none, speed) = (k.at_site(&held), k.zero(), k.site_parameter(&speed));
        let first_order = k.relax(held, none, speed);
        let (pressure, half) = (rate.cells.pressure(&mut k), k.site_parameter(&half));
        let reach = k.add(pressure, half);
        let share = k.divide(pressure, reach);
        let share = k.as_dimension(share);
        let value = k.scale(first_order, share);
        k.finish(value, laws.every_mode(), class)
    });
    laws.react_at(&burned, burn_rate);
    let debt: OxygenDebtPerBed<Defines<'b, Metabolism>> = laws.emit();
    let held = burned.amount::<Lactate>();
    let owed = laws.site_kernel(|mut k| {
        let (held, per) = (k.at_site(&held), k.site_parameter(&per_lactate));
        let oxygen = k.multiply(per, held);
        let value = k.as_dimension(oxygen);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&debt.oxygen_debt_per_bed, owed);
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
