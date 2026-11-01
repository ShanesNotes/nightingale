use crate::rhythm::Heart;
use ng_contract::*;

type Port<'b, U> = ReadPort<'b, Cardiovascular, PerSite<Blood, U>>;
type Effect<'b> = ReadPort<'b, Cardiovascular, PerSite<Junction, Dimensionless>>;
type Knob<'b, U> = SiteParameter<'b, Cardiovascular, Blood, U>;

fn shed<'b, 'k>(
    k: &mut Kernel<'b, 'k, Cardiovascular>,
    net: Expr<'b, 'k, Cardiovascular, Rate<Energy>>,
    soft: &Parameter<'b, Cardiovascular, Rate<Energy>>,
) -> Expr<'b, 'k, Cardiovascular, Rate<Energy>> {
    let (zero, soft) = (k.zero(), k.parameter(soft));
    k.smooth_max(net, zero, soft)
}

/// One wall law for a vein, a bed and a ventricle: a site with no active elastance is passive, and a site no nerve
/// reaches reads no effect.
pub(crate) struct Walls<'b, 'h> {
    water: Port<'b, Water>,
    albumin: Port<'b, Albumin>,
    room: Parameter<'b, Cardiovascular, Quotient<Volume, Albumin>>,
    pub(crate) outside: Port<'b, Pressure>,
    adrenergic: Effect<'b>,
    passive: Knob<'b, Elastance>,
    active: Knob<'b, Elastance>,
    inotropy: Knob<'b, Dimensionless>,
    unstressed: Knob<'b, Volume>,
    unstressed_gain: Knob<'b, Volume>,
    collapse: Parameter<'b, Cardiovascular, Volume>,
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
            albumin: ports.amount::<Albumin>(),
            room: cv.parameter(ParameterKey::new("cv.albumin-volume"), Admissible::nonnegative()),
            outside: ports.extravascular_pressure(),
            adrenergic: ports.effect::<Norepinephrine>(),
            passive,
            active,
            inotropy: cv.site_parameter(ParameterKey::new("cv.inotropy"), Admissible::nonnegative()),
            unstressed: cv.site_parameter(ParameterKey::new("cv.unstressed"), Admissible::nonnegative()),
            unstressed_gain: cv
                .site_parameter(ParameterKey::new("cv.unstressed-gain"), Admissible::nonnegative()),
            collapse: cv.parameter(ParameterKey::new("cv.collapse-width"), Admissible::positive()),
            activation: &heart.activation,
        }
    }
    /// `V - V_u`, with `V` the water's volume and the albumin's, `V_u = V_u_base - S E_a1`, and no less than zero: a
    /// vessel below its unstressed volume collapses and sucks nothing in, so a chamber cannot pull itself dry. Model
    /// default.
    pub(crate) fn stressed<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Blood>,
    ) -> Expr<'b, 'k, Cardiovascular, Volume> {
        let water = k.at_site(&self.water);
        let solvent = k.volume(water);
        let held = k.at_site(&self.albumin);
        let per = k.parameter(&self.room);
        let product = k.multiply(per, held);
        let solute: Expr<'_, '_, _, Volume> = k.as_dimension(product);
        let volume = k.add(solvent, solute);
        let base = k.site_parameter(&self.unstressed);
        let gain = k.site_parameter(&self.unstressed_gain);
        let effect = k.at_junction(&self.adrenergic);
        let moved = k.scale(gain, effect);
        let unstressed = k.subtract(base, moved);
        let stretched = k.subtract(volume, unstressed);
        let (zero, soft) = (k.zero(), k.parameter(&self.collapse));
        k.smooth_max(stretched, zero, soft)
    }
    fn contractile<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Blood>,
    ) -> Expr<'b, 'k, Cardiovascular, Elastance> {
        let active = k.site_parameter(&self.active);
        let gain = k.site_parameter(&self.inotropy);
        let effect = k.at_junction(&self.adrenergic);
        let tuned = k.scale(active, gain);
        let strengthened = k.scale(tuned, effect);
        k.add(active, strengthened)
    }
    /// `E_passive + a E_active (1 + S E_b1)`. Model default.
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
    /// their sum. Work enters where activation stiffens a stretched wall, in systole only, and gravity's enters beside
    /// it. The wall power is what the store must lose to stay on the sum: no law's choice, so closure sits in the stage
    /// residual and `verify` judges it with the rest. What the walls shed leaves as heat, and heat never comes back. What
    /// they gain past those receipts, as a vein's muscle tightens or water enters, is muscle work from the chemical
    /// energy the heart draws on. So the account can run no pool dry. The moving blood's energy and the stretch of tissue
    /// and tubule are not in the sum, so what they hold passes through this rate.
    pub(crate) fn energy(
        &self,
        cv: &mut FragmentBuilder<'b, Cardiovascular>,
        ports: &CardiovascularPorts<'b>,
        heart: &Heart<'b>,
        head: &Parameter<'b, Cardiovascular, Quotient<Pressure, Length>>,
        wave: RateClass,
    ) -> Result<(), AdmissionError> {
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

        let fallen = ports.gravity_work(cv);
        let falling = cv.edge_kernel(|mut k| {
            let per_length = k.parameter(head);
            let fall = k.fall();
            let product = k.multiply(per_length, fall);
            let head: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
            let flow = k.edge_flow();
            let product = k.multiply(head, flow);
            let power = k.as_dimension(product);
            k.finish(power, cv.every_mode(), wave)
        });
        cv.transfer_along(&fallen, falling);

        let store = ports.dissipation(cv);
        let power = cv.algebraic::<Rate<Energy>>("wall power", Admissible::finite());
        let closing = cv.kernel(|mut k| {
            let held = k.read(&store.amount::<Energy>());
            let sum = k.algebraic(&elastic);
            let gap = k.subtract(held, sum);
            k.finish(gap, cv.every_mode(), wave)
        });
        cv.closes(&power, closing);
        let soft = cv.parameter::<Rate<Energy>>(ParameterKey::new("cv.heat-width"), Admissible::positive());
        let leaving = cv.kernel(|mut k| {
            let net = k.algebraic(&power);
            let heat = shed(&mut k, net, &soft);
            k.finish(heat, cv.every_mode(), wave)
        });
        cv.transfer(Transfer::new(&store, leaving));
        let paid = ports.wall_work(cv);
        let gaining = cv.kernel(|mut k| {
            let net = k.algebraic(&power);
            let heat = shed(&mut k, net, &soft);
            let gain = k.subtract(heat, net);
            k.finish(gain, cv.every_mode(), wave)
        });
        cv.transfer(Transfer::new(&paid, gaining));
        Ok(())
    }
}
