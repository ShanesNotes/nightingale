use ng_contract::*;

/// Sodium crosses by a transporter that saturates, `J A / (K + A)`, so a greater load leaves more of it in the lumen.
/// A fixed share would leave the sodium the macula densa sees blind to filtration, and the feedback with nothing to
/// read. What crosses beside it goes in the proportion the lumen holds them, `J A_q / (K + A)`, so a lumen with no ions
/// reclaims none. Chloride crosses with them, so all of it stops as chloride runs out, `A_cl / (K_cl + A_cl)`, and a
/// lumen of sodium alone keeps its charge and its chloride stays at or above zero. Model default.
pub(crate) struct Saturable<'b> {
    capacity: EdgeParameter<'b, Renal, Rate<Sodium>, ReclaimEdge>,
    half: EdgeParameter<'b, Renal, Sodium, ReclaimEdge>,
    chloride_half: EdgeParameter<'b, Renal, Chloride, ReclaimEdge>,
}
impl<'b> Saturable<'b> {
    pub(crate) fn declare(rn: &mut FragmentBuilder<'b, Renal>) -> Self {
        Self {
            capacity: rn.edge_parameter(ParameterKey::new("rn.sodium-capacity"), Admissible::positive()),
            half: rn.edge_parameter(ParameterKey::new("rn.sodium-half"), Admissible::positive()),
            chloride_half: rn.edge_parameter(ParameterKey::new("rn.chloride-half"), Admissible::positive()),
        }
    }
    pub(crate) fn crossing<'k, Q: Substance>(
        &self,
        k: &mut Kernel<'b, 'k, Renal, ReclaimEdge>,
        sodium: &EdgeRoute<'b, Renal, Sodium, ReclaimEdge>,
        route: &EdgeRoute<'b, Renal, Q, ReclaimEdge>,
    ) -> Expr<'b, 'k, Renal, Rate<Q>> {
        let held = k.at_edge(&route.from_amount::<Q>());
        let (load, half) = (
            k.at_edge(&sodium.from_amount::<Sodium>()),
            k.edge_parameter(&self.half),
        );
        let reach = k.add(load, half);
        let ratio = k.divide(held, reach);
        let capacity = k.edge_parameter(&self.capacity);
        let product = k.multiply(capacity, ratio);
        let rate = k.as_dimension(product);
        let (held, half) = (
            k.at_edge(&sodium.from_amount::<Chloride>()),
            k.edge_parameter(&self.chloride_half),
        );
        let reach = k.add(held, half);
        let ratio = k.divide(held, reach);
        let gate = k.as_dimension(ratio);
        k.scale(rate, gate)
    }
}
