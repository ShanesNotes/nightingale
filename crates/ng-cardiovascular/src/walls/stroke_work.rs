use super::Walls;
use crate::rhythm::Heart;
use ng_contract::*;

impl<'b, 'h> Walls<'b, 'h> {
    pub(crate) fn stroke_work(
        &self,
        cv: &mut FragmentBuilder<'b, Cardiovascular>,
        ports: &CardiovascularPorts<'b>,
        heart: &Heart<'b>,
        half: &Parameter<'b, Cardiovascular, Dimensionless>,
        wave: RateClass,
    ) {
        let work = ports.stroke_work(cv);
        let into_wall = cv.site_kernel::<Blood, _, _>(|mut k| {
            let contractile = self.contractile(&mut k);
            let per_unit = self.stored(&mut k, half, contractile);
            let (target, current, speed) = (
                k.parameter(&heart.peak),
                k.state(&heart.activation),
                k.parameter(&heart.rise),
            );
            let rate = k.relax(target, current, speed);
            let power = k.multiply(per_unit, rate);
            let power = k.as_dimension(power);
            k.finish(power, heart.systole.only(), wave)
        });
        cv.transfer_at(&work, into_wall);
        let none = cv.site_kernel::<Blood, _, _>(|mut k| {
            let zero = k.zero::<Rate<Energy>>();
            k.finish(zero, heart.diastole.only(), wave)
        });
        cv.transfer_at(&work, none);
    }
}
