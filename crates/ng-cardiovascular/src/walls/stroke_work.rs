use super::Walls;
use crate::rhythm::Heart;
use ng_contract::catalog::{Defines, MyocardialWorkAndBedFlow};
use ng_contract::*;

impl<'b, 'h> Walls<'b, 'h> {
    pub(crate) fn stroke_work(
        &self,
        cv: &mut FragmentBuilder<'b, Cardiovascular>,
        ports: &CardiovascularPorts<'b>,
        heart: &Heart<'b>,
        half: &Parameter<'b, Cardiovascular, Dimensionless>,
        below: &SiteTerm<'b, Cardiovascular, Veins, Energy>,
        wave: RateClass,
    ) {
        let work = ports.stroke_work(cv);
        let into_wall = cv.site_kernel::<Blood, _, _>(|mut k| {
            let contractile = self.contractile(&mut k);
            let per_unit = self.stored(&mut k, half, contractile, below);
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

    /// Line 78's work is a memory that follows the stroke work in systole and falls in diastole at one speed, so its
    /// mean over a beat is the stroke work's: the instant work holds by the rhythm's modes, which no algebraic row may.
    /// Each site's stroke work is its store per unit of activation times the activation's rate, so the sum of the
    /// stores times that rate is the work every stroke-work route moves.
    pub(crate) fn myocardial_work(
        &self,
        cv: &mut FragmentBuilder<'b, Cardiovascular>,
        heart: &Heart<'b>,
        half: &Parameter<'b, Cardiovascular, Dimensionless>,
        below: &SiteTerm<'b, Cardiovascular, Veins, Energy>,
        wave: RateClass,
    ) {
        let line: MyocardialWorkAndBedFlow<Defines<'b, Cardiovascular>> = cv.emit();
        let store = cv.algebraic::<Energy>("contractile store", Admissible::nonnegative());
        let row = cv.kernel(|mut k| {
            let value = k.algebraic(&store);
            k.finish(value, cv.every_mode(), wave)
        });
        let each = cv.site_kernel::<Blood, _, _>(|mut k| {
            let contractile = self.contractile(&mut k);
            let value = self.stored(&mut k, half, contractile, below);
            k.finish(value, cv.every_mode(), wave)
        });
        cv.sum_at(&store, row, each);
        let memory = cv.state::<Rate<Energy>>("work memory", Admissible::nonnegative());
        let speed =
            cv.parameter::<Rate<Dimensionless>>(ParameterKey::new("cv.work-speed"), Admissible::positive());
        let rising = cv.kernel(|mut k| {
            let (target, current, rise) = (
                k.parameter(&heart.peak),
                k.state(&heart.activation),
                k.parameter(&heart.rise),
            );
            let rate = k.relax(target, current, rise);
            let held = k.algebraic(&store);
            let product = k.multiply(held, rate);
            let work = k.as_dimension(product);
            let (now, pace) = (k.state(&memory), k.parameter(&speed));
            let value = k.relax(work, now, pace);
            k.finish(value, heart.systole.only(), wave)
        });
        cv.memory_rate(&memory, rising);
        let falling = cv.kernel(|mut k| {
            let (none, now, pace) = (k.zero(), k.state(&memory), k.parameter(&speed));
            let value = k.relax(none, now, pace);
            k.finish(value, heart.diastole.only(), wave)
        });
        cv.memory_rate(&memory, falling);
        let work = cv.kernel(|mut k| {
            let value = k.state(&memory);
            k.finish(value, cv.every_mode(), wave)
        });
        cv.define(&line.myocardial_work, work);
    }
}
