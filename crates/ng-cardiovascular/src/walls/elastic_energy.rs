use super::Walls;
use ng_contract::*;

impl<'b, 'h> Walls<'b, 'h> {
    pub(crate) fn elastic_energy(
        &self,
        cv: &mut FragmentBuilder<'b, Cardiovascular>,
        half: &Parameter<'b, Cardiovascular, Dimensionless>,
        elastic: &AlgebraicSlot<'b, Cardiovascular, Energy>,
        wave: RateClass,
    ) {
        let row = cv.kernel(|mut k| {
            let value = k.algebraic(elastic);
            k.finish(value, cv.every_mode(), wave)
        });
        let held = cv.site_kernel::<Blood, _, _>(|mut k| {
            let stiffness = self.elastance(&mut k);
            let held = self.stored(&mut k, half, stiffness);
            k.finish(held, cv.every_mode(), wave)
        });
        cv.sum_at(elastic, row, held);
    }
}
