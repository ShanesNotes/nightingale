use crate::walls::Walls;
use ng_contract::*;

pub(crate) fn declare<'b>(
    cv: &mut FragmentBuilder<'b, Cardiovascular>,
    pressure_row: Output<'b, Cardiovascular, PerSite<Blood, Pressure>>,
    walls: &Walls<'b, '_>,
    wave: RateClass,
) {
    let value = cv.site_kernel::<Blood, _, _>(|mut k| {
        let stressed = walls.stressed(&mut k);
        let stiffness = walls.elastance(&mut k);
        let product = k.multiply(stiffness, stressed);
        let elastic = k.as_dimension(product);
        let outside = k.at_site(&walls.outside);
        let total = k.add(elastic, outside);
        k.finish(total, cv.every_mode(), wave)
    });
    cv.define_at(&pressure_row, value);
}
