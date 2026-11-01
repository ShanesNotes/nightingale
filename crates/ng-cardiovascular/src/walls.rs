use crate::rhythm::Heart;
use ng_contract::*;

type Port<'b, U> = ReadPort<'b, Cardiovascular, PerSite<Blood, U>>;
type Effect<'b> = ReadPort<'b, Cardiovascular, PerSite<Junction, Dimensionless>>;
type Knob<'b, U> = SiteParameter<'b, Cardiovascular, Blood, U>;

/// One wall law for a vein, a bed and a ventricle: a site with no active elastance is passive, and a site no junction
/// supplies reads no effect.
pub(crate) struct Walls<'b, 'h> {
    water: Port<'b, Water>,
    pub(crate) outside: Port<'b, Pressure>,
    alpha: Effect<'b>,
    passive: Knob<'b, Elastance>,
    active: Knob<'b, Elastance>,
    inotropy: Knob<'b, Dimensionless>,
    unstressed: Knob<'b, Volume>,
    unstressed_gain: Knob<'b, Volume>,
    activation: &'h StateSlot<'b, Cardiovascular, Dimensionless>,
}
impl<'b, 'h> Walls<'b, 'h> {
    pub(crate) fn declare(
        cv: &mut FragmentBuilder<'b, Cardiovascular>,
        ports: &CardiovascularPorts<'b>,
        heart: &'h Heart<'b>,
    ) -> Self {
        let mut knob =
            |key| cv.site_parameter::<Blood, Elastance>(ParameterKey::new(key), Admissible::nonnegative());
        let (passive, active) = (knob("cv.elastance"), knob("cv.active-elastance"));
        Self {
            water: ports.amount::<Water>(),
            outside: ports.extravascular_pressure(),
            alpha: ports.effect::<Norepinephrine>(),
            passive,
            active,
            inotropy: cv.site_parameter(ParameterKey::new("cv.inotropy"), Admissible::nonnegative()),
            unstressed: cv.site_parameter(ParameterKey::new("cv.unstressed"), Admissible::nonnegative()),
            unstressed_gain: cv
                .site_parameter(ParameterKey::new("cv.unstressed-gain"), Admissible::nonnegative()),
            activation: &heart.activation,
        }
    }
    /// `V - V_u`, with `V_u = V_u_base - S E_a1`.
    pub(crate) fn stressed<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Blood>,
    ) -> Expr<'b, 'k, Cardiovascular, Volume> {
        let water = k.at_site(&self.water);
        let volume = k.volume(water);
        let base = k.site_parameter(&self.unstressed);
        let gain = k.site_parameter(&self.unstressed_gain);
        let effect = k.at_junction(&self.alpha);
        let moved = k.scale(gain, effect);
        let unstressed = k.subtract(base, moved);
        k.subtract(volume, unstressed)
    }
    /// The active part of the wall's stiffness at full activation: `E_active (1 + S E_b1)`.
    fn contractile<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Blood>,
    ) -> Expr<'b, 'k, Cardiovascular, Elastance> {
        let active = k.site_parameter(&self.active);
        let gain = k.site_parameter(&self.inotropy);
        let effect = k.at_junction(&self.alpha);
        let tuned = k.scale(active, gain);
        let strengthened = k.scale(tuned, effect);
        k.add(active, strengthened)
    }
    /// `E_passive + a E_active (1 + S E_b1)`.
    pub(crate) fn elastance<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Blood>,
    ) -> Expr<'b, 'k, Cardiovascular, Elastance> {
        let passive = k.site_parameter(&self.passive);
        let contractile = self.contractile(k);
        let activation = k.state(self.activation);
        let driven = k.scale(contractile, activation);
        k.add(passive, driven)
    }
    /// `E s^2 / 2`.
    fn stored<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Blood>,
        half: &Parameter<'b, Cardiovascular, Dimensionless>,
        stiffness: Expr<'b, 'k, Cardiovascular, Elastance>,
    ) -> Expr<'b, 'k, Cardiovascular, Energy> {
        let stressed = self.stressed(k);
        let product = k.multiply(stiffness, stressed);
        let pressure: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
        let product = k.multiply(pressure, stressed);
        let doubled: Expr<'_, '_, _, Energy> = k.as_dimension(product);
        let share = k.parameter(half);
        k.scale(doubled, share)
    }
    /// One mechanical store and one thermal store. Each blood site's wall holds `E s^2 / 2`, and the store is held to
    /// their sum. Work enters where activation stiffens a stretched wall, in systole only, and what the store then
    /// holds over the sum leaves as heat. That rate is no law's choice: it is what keeps the store equal to the sum,
    /// so closure sits in the stage residual and `verify` judges it with the rest.
    pub(crate) fn energy(
        &self,
        cv: &mut FragmentBuilder<'b, Cardiovascular>,
        ports: &CardiovascularPorts<'b>,
        heart: &Heart<'b>,
        wave: RateClass,
    ) {
        let half = cv.parameter::<Dimensionless>(ParameterKey::new("cv.half"), Admissible::positive());
        let elastic = cv.algebraic::<Energy>("elastic energy", Admissible::nonnegative());
        let row = cv.kernel(|mut k| {
            let value = k.algebraic(&elastic);
            k.finish(value, cv.every_mode(), wave)
        });
        let held = cv.site_kernel::<Blood, _, _>(|mut k| {
            let stiffness = self.elastance(&mut k);
            let held = self.stored(&mut k, &half, stiffness);
            k.finish(held, cv.every_mode(), wave)
        });
        cv.sum_at(&elastic, row, held);

        let work = ports.stroke_work(cv);
        let into_wall = cv.site_kernel::<Blood, _, _>(|mut k| {
            let contractile = self.contractile(&mut k);
            let per_unit = self.stored(&mut k, &half, contractile);
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

        let store = ports.dissipation(cv);
        let dissipation = cv.algebraic::<Rate<Energy>>("dissipated power", Admissible::finite());
        let closing = cv.kernel(|mut k| {
            let held = k.read(&store.amount::<Energy>());
            let sum = k.algebraic(&elastic);
            let gap = k.subtract(held, sum);
            k.finish(gap, cv.every_mode(), wave)
        });
        cv.closes(&dissipation, closing);
        let leaving = cv.kernel(|mut k| {
            let value = k.algebraic(&dissipation);
            k.finish(value, cv.every_mode(), wave)
        });
        cv.transfer(Transfer::new(&store, leaving));
    }
}
