//! The kidney as segments of the urinary network: filtration out of a bed, flow down the tract, each species reclaimed
//! by its own law, and tubuloglomerular feedback. The map says which segment does what.

use ng_contract::catalog::{Defines, TubuloglomerularFeedback};
use ng_contract::*;

const TUBULE_NS: u64 = 1_000_000_000;

/// Each segment's pressure from its own volume. Model default: linear.
fn lumen_pressure<'b>(
    rn: &mut FragmentBuilder<'b, Renal>,
    ports: &RenalPorts<'b>,
    class: RateClass,
) -> ReadPort<'b, Renal, PerSite<Urinary, Pressure>> {
    let (row, read) = rn.site_field("lumen pressure", Admissible::finite());
    let stiffness = rn.site_parameter::<Urinary, Elastance>(
        ParameterKey::new("rn.lumen-elastance"),
        Admissible::positive(),
    );
    let resting =
        rn.site_parameter::<Urinary, Volume>(ParameterKey::new("rn.lumen-volume"), Admissible::nonnegative());
    let water = ports.lumen::<Water>();
    let value = rn.site_kernel(|mut k| {
        let amount = k.at_site(&water);
        let volume = k.volume(amount);
        let rest = k.site_parameter(&resting);
        let stretched = k.subtract(volume, rest);
        let wall = k.site_parameter(&stiffness);
        let product = k.multiply(wall, stretched);
        let pressure = k.as_dimension(product);
        k.finish(pressure, rn.every_mode(), class)
    });
    rn.define_at(&row, value);
    read
}

/// The fraction of what reaches a segment that crosses back: `f_min + (f_max - f_min) E_v2`, so the same law serves a
/// tubule that ignores V2, with its two limits equal, and a duct whose limits differ (research 02).
struct Fraction<'b> {
    least: EdgeParameter<'b, Renal, Dimensionless, ReclaimEdge>,
    most: EdgeParameter<'b, Renal, Dimensionless, ReclaimEdge>,
}
impl<'b> Fraction<'b> {
    fn declare(rn: &mut FragmentBuilder<'b, Renal>, least: &'static str, most: &'static str) -> Self {
        Self {
            least: rn.edge_parameter(ParameterKey::new(least), Admissible::unit_interval()),
            most: rn.edge_parameter(ParameterKey::new(most), Admissible::unit_interval()),
        }
    }
    fn read<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Renal, ReclaimEdge>,
        v2: &ReadPort<'b, Renal, PerSite<Blood, Dimensionless>>,
    ) -> Expr<'b, 'k, Renal, Dimensionless> {
        let (least, most) = (k.edge_parameter(&self.least), k.edge_parameter(&self.most));
        let span = k.subtract(most, least);
        let effect = k.downstream(v2);
        let moved = k.scale(span, effect);
        k.add(least, moved)
    }
}

/// `f q A_s / A_w`: the ion's share of what reaches the segment.
fn delivered<'b, 'k, Q: Substance>(
    k: &mut Kernel<'b, 'k, Renal, ReclaimEdge>,
    route: &EdgeRoute<'b, Renal, Q, ReclaimEdge>,
    fraction: Expr<'b, 'k, Renal, Dimensionless>,
) -> Expr<'b, 'k, Renal, Rate<Q>>
where
    Quotient<Q, Water>: Dimension,
{
    let inflow = k.inflow();
    let reached = k.scale(inflow, fraction);
    let (amount, water) = (
        k.at_edge(&route.from_amount::<Q>()),
        k.at_edge(&route.from_amount::<Water>()),
    );
    let share = k.divide(amount, water);
    let rate = k.multiply(reached, share);
    k.as_dimension(rate)
}

pub fn declare<'b>(scope: RenalScope<'b>) -> Result<Fragment<'b, Renal>, AdmissionError> {
    let (mut rn, ports) = scope.parts();
    let class = RateClass::new(RateBand::Reflex, TUBULE_NS)?;
    let lumen = lumen_pressure(&mut rn, &ports, class);
    let blood = ports.blood_pressure();

    let filtration = ports.filtration(&mut rn);
    let coefficient = rn.edge_parameter::<FilterEdge, Conductance<Water>>(
        ParameterKey::new("rn.filtration"),
        Admissible::positive(),
    );
    let slope = rn.parameter::<Quotient<Pressure, Concentration<Albumin>>>(
        ParameterKey::new("rn.oncotic-slope"),
        Admissible::positive(),
    );
    let gfr = rn.edge_kernel(|mut k| {
        let (inside, outside) = (k.upstream(&blood), k.downstream(&lumen));
        let head = k.subtract(inside, outside);
        let (albumin, water) = (
            k.at_edge(&filtration.from_amount()),
            k.at_edge(&filtration.from_amount()),
        );
        let volume = k.volume(water);
        let ratio = k.divide(albumin, volume);
        let concentration = k.as_dimension(ratio);
        let per = k.parameter(&slope);
        let product = k.multiply(per, concentration);
        let pull: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
        let net = k.subtract(head, pull);
        let conductance = k.edge_parameter(&coefficient);
        let flow = k.conductance_flow(conductance, net);
        k.finish(flow, rn.every_mode(), class)
    });
    rn.transfer_along(&filtration, gfr);

    let tubular = ports.tubular(&mut rn);
    let drag = rn.edge_parameter::<TubuleEdge, Resistance>(
        ParameterKey::new("rn.tubule-resistance"),
        Admissible::positive(),
    );
    let down = rn.edge_kernel(|mut k| {
        let (above, below) = (k.upstream(&lumen), k.downstream(&lumen));
        let head = k.subtract(above, below);
        let resistance = k.edge_parameter(&drag);
        let quotient = k.divide(head, resistance);
        let flow = k.as_dimension(quotient);
        k.finish(flow, rn.every_mode(), class)
    });
    rn.transfer_along(&tubular, down);

    let v2 = ports.effect::<Vasopressin>();
    let water = ports.reclaim::<Water>(&mut rn);
    let water_share = Fraction::declare(&mut rn, "rn.water-least", "rn.water-most");
    let back = rn.edge_kernel(|mut k| {
        let share = water_share.read(&mut k, &v2);
        let inflow = k.inflow();
        let rate = k.scale(inflow, share);
        k.finish(rate, rn.every_mode(), class)
    });
    rn.transfer_along(&water, back);

    let sodium = ports.reclaim::<Sodium>(&mut rn);
    let potassium = ports.reclaim::<Potassium>(&mut rn);
    let chloride = ports.reclaim::<Chloride>(&mut rn);
    let sodium_share = Fraction::declare(&mut rn, "rn.sodium-share", "rn.sodium-share");
    let potassium_share = Fraction::declare(&mut rn, "rn.potassium-share", "rn.potassium-share");
    let back = rn.edge_kernel(|mut k| {
        let share = sodium_share.read(&mut k, &v2);
        let rate = delivered(&mut k, &sodium, share);
        k.finish(rate, rn.every_mode(), class)
    });
    rn.transfer_along(&sodium, back);
    let back = rn.edge_kernel(|mut k| {
        let share = potassium_share.read(&mut k, &v2);
        let rate = delivered(&mut k, &potassium, share);
        k.finish(rate, rn.every_mode(), class)
    });
    rn.transfer_along(&potassium, back);
    // Chloride follows the cations, one for one, so reclaiming moves no charge. Model default.
    let back = rn.edge_kernel(|mut k| {
        let (share_na, share_k) = (sodium_share.read(&mut k, &v2), potassium_share.read(&mut k, &v2));
        let (na, potassium_load) = (
            delivered(&mut k, &sodium, share_na),
            delivered(&mut k, &potassium, share_k),
        );
        let (na, potassium_load): (Expr<'_, '_, _, Rate<Chloride>>, Expr<'_, '_, _, Rate<Chloride>>) =
            (k.as_dimension(na), k.as_dimension(potassium_load));
        let rate = k.add(na, potassium_load);
        k.finish(rate, rn.every_mode(), class)
    });
    rn.transfer_along(&chloride, back);

    feedback(&mut rn, &ports, class)?;
    Ok(rn.finish())
}

/// `f = f0 + S (1 / (1 + exp((x - x0) / m)) - 0.5)` on the sodium the macula densa sees (research 02). Line 97 carries
/// it to the afferent edge, and no law here says which edge.
fn feedback<'b>(
    rn: &mut FragmentBuilder<'b, Renal>,
    ports: &RenalPorts<'b>,
    class: RateClass,
) -> Result<(), AdmissionError> {
    let tone: TubuloglomerularFeedback<Defines<'_, Renal>> = rn.emit();
    let (sodium, water) = (ports.lumen::<Sodium>(), ports.lumen::<Water>());
    let rest = rn.parameter::<Dimensionless>(ParameterKey::new("rn.feedback-rest"), Admissible::positive());
    let reach = rn.parameter::<Dimensionless>(ParameterKey::new("rn.feedback-gain"), Admissible::positive());
    let middle = rn.parameter::<Dimensionless>(
        ParameterKey::new("rn.feedback-middle"),
        Admissible::unit_interval(),
    );
    let set = rn
        .parameter::<Concentration<Sodium>>(ParameterKey::new("rn.feedback-sodium"), Admissible::positive());
    let width =
        rn.parameter::<Concentration<Sodium>>(ParameterKey::new("rn.feedback-width"), Admissible::positive());
    let value = rn.kernel(|mut k| {
        let amount = k.at_landmark(Landmark::MaculaDensa, &sodium);
        let solvent = k.at_landmark(Landmark::MaculaDensa, &water);
        let volume = k.volume(solvent);
        let ratio = k.divide(amount, volume);
        let seen: Expr<'_, '_, _, Concentration<Sodium>> = k.as_dimension(ratio);
        let center = k.parameter(&set);
        let above = k.subtract(center, seen);
        let spread = k.parameter(&width);
        let ratio = k.divide(above, spread);
        let drive = k.as_dimension(ratio);
        let opened = k.logistic(drive);
        let half = k.parameter(&middle);
        let shift = k.subtract(opened, half);
        let gain = k.parameter(&reach);
        let moved = k.scale(gain, shift);
        let base = k.parameter(&rest);
        let tone = k.add(base, moved);
        k.finish(tone, rn.every_mode(), class)
    });
    rn.define(&tone.afferent_tone, value);
    Ok(())
}
