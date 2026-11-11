use ng_contract::*;

/// The fraction of what reaches a segment that crosses back: `f_min + (f_max - f_min) E_v2`, research 02's form for
/// collecting duct water. With its two limits equal, the same law serves a tubule that ignores V2.
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

pub(crate) fn declare<'b>(rn: &mut FragmentBuilder<'b, Renal>, ports: &RenalPorts<'b>, class: RateClass) {
    let v2 = ports.effect::<Vasopressin>();
    let water = ports.reclaim::<Water>(rn);
    let water_share = Fraction::declare(rn, "rn.water-least", "rn.water-most");
    let back = rn.edge_kernel(|mut k| {
        let share = water_share.read(&mut k, &v2);
        let inflow = k.inflow();
        let rate = k.scale(inflow, share);
        k.finish(rate, rn.every_mode(), class)
    });
    rn.transfer_along(&water, back);
}
