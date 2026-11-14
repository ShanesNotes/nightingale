use crate::rhythm::Heart;
use ng_contract::*;

mod dissipation;
mod elastic_energy;
mod gravity_work;
mod stroke_work;
mod wall_power;
mod wall_work;

type Port<'b, U> = ReadPort<'b, Cardiovascular, PerSite<Blood, U>>;
type Effect<'b> = ReadPort<'b, Cardiovascular, PerSite<Junction, Dimensionless>>;
type Knob<'b, U> = SiteParameter<'b, Cardiovascular, Blood, U>;

/// Below `V_u` a marked vein's stressed volume falls on `share (V_u / n) (1 - (V_u / V)^n)`, without bound as it
/// empties, so no head drains it dry. The form is the collapse branch of Toro and others 2021 Eqs 3 to 5, read in
/// volume; its slope at `V_u` is the share, after the collapsed compliance Shi, Lawford and Hose 2011 restate. Neither
/// gives the numbers. The ratio needs `V_u` above zero, so a marked vein needs `cv.unstressed` above
/// `cv.unstressed-gain`, or a run refuses by this term's name. Model default.
fn collapsing<'b, 'k>(
    k: &mut Kernel<'b, 'k, Cardiovascular, Veins>,
    lumen: &Lumen<'b>,
    share: &SiteParameter<'b, Cardiovascular, Veins, Dimensionless>,
    power: &Parameter<'b, Cardiovascular, Dimensionless>,
    width: &Parameter<'b, Cardiovascular, Volume>,
) -> Expr<'b, 'k, Cardiovascular, Volume> {
    let (volume, unstressed) = lumen.volumes(k);
    let quotient = k.divide(unstressed, volume);
    let ratio: Expr<'_, '_, _, Dimensionless> = k.as_dimension(quotient);
    let power = k.parameter(power);
    let log = k.ln(ratio);
    let scaled = k.scale(log, power);
    let raised = k.exp(scaled);
    let reach = k.scale(unstressed, raised);
    let gap = k.subtract(unstressed, reach);
    let per = k.divide(gap, power);
    let shrunk: Expr<'_, '_, _, Volume> = k.as_dimension(per);
    let (zero, soft) = (k.zero(), k.parameter(width));
    let below = k.smooth_min(shrunk, zero, soft);
    let share = k.site_parameter(share);
    k.scale(below, share)
}

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
    collapsed: SiteTerm<'b, Cardiovascular, Veins, Volume>,
    activation: &'h StateSlot<'b, Cardiovascular, Dimensionless>,
}
/// What a blood site holds and how much of it stretches the wall, read by the wall law and by a vein's collapse.
struct Lumen<'b> {
    water: Port<'b, Water>,
    albumin: Port<'b, Albumin>,
    room: Parameter<'b, Cardiovascular, Quotient<Volume, Albumin>>,
    adrenergic: Effect<'b>,
    unstressed: Knob<'b, Volume>,
    unstressed_gain: Knob<'b, Volume>,
}
impl<'b> Lumen<'b> {
    /// `V`, the water's volume and the albumin's, and `V_u = V_u_base - S E_a1` (research 02).
    fn volumes<'k, C: SiteSet>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, C>,
    ) -> (
        Expr<'b, 'k, Cardiovascular, Volume>,
        Expr<'b, 'k, Cardiovascular, Volume>,
    )
    where
        Blood: Includes<C>,
    {
        let water = k.at_site_in(&self.water);
        let solvent = k.volume(water);
        let held = k.at_site_in(&self.albumin);
        let per = k.parameter(&self.room);
        let product = k.multiply(per, held);
        let solute: Expr<'_, '_, _, Volume> = k.as_dimension(product);
        let volume = k.add(solvent, solute);
        let base = k.site_parameter_in(&self.unstressed);
        let gain = k.site_parameter_in(&self.unstressed_gain);
        let effect = k.at_junction_if_supplied(&self.adrenergic);
        let moved = k.scale(gain, effect);
        (volume, k.subtract(base, moved))
    }
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
        let room = cv.parameter(ParameterKey::new("cv.albumin-volume"), Admissible::nonnegative());
        let inotropy = cv.site_parameter(ParameterKey::new("cv.inotropy"), Admissible::nonnegative());
        let unstressed = cv.site_parameter(ParameterKey::new("cv.unstressed"), Admissible::nonnegative());
        let unstressed_gain =
            cv.site_parameter(ParameterKey::new("cv.unstressed-gain"), Admissible::nonnegative());
        let collapse = cv.parameter(ParameterKey::new("cv.collapse-width"), Admissible::positive());
        let lumen = Lumen {
            water: ports.amount::<Water>(),
            albumin: ports.amount::<Albumin>(),
            room,
            adrenergic: ports.effect::<Norepinephrine>(),
            unstressed,
            unstressed_gain,
        };
        let share = cv.site_parameter::<Veins, Dimensionless>(
            ParameterKey::new("cv.collapse-share"),
            Admissible::nonnegative(),
        );
        let power = cv.parameter(ParameterKey::new("cv.collapse-power"), Admissible::positive());
        let collapsed = cv.site_term("vein collapse", |mut k| {
            let value = collapsing(&mut k, &lumen, &share, &power, &collapse);
            k.finish_term(value)
        });
        let Lumen {
            water,
            albumin,
            room,
            adrenergic,
            unstressed,
            unstressed_gain,
        } = lumen;
        Self {
            water,
            albumin,
            room,
            outside: ports.extravascular_pressure(),
            adrenergic,
            passive,
            active,
            inotropy,
            unstressed,
            unstressed_gain,
            collapse,
            collapsed,
            activation: &heart.activation,
        }
    }
    /// `V - V_u`, with `V` the water's volume and the albumin's, `V_u = V_u_base - S E_a1` (research 02), held no
    /// less than zero off the veins: a chamber or a bed below its unstressed volume collapses and sucks nothing in,
    /// so it cannot pull itself dry. At a marked vein the term `vein collapse` carries it below zero. Model default.
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
        let effect = k.at_junction_if_supplied(&self.adrenergic);
        let moved = k.scale(gain, effect);
        let unstressed = k.subtract(base, moved);
        let stretched = k.subtract(volume, unstressed);
        let (zero, soft) = (k.zero(), k.parameter(&self.collapse));
        let above = k.smooth_max(stretched, zero, soft);
        k.add_from(above, &self.collapsed)
    }
    fn contractile<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Blood>,
    ) -> Expr<'b, 'k, Cardiovascular, Elastance> {
        let active = k.site_parameter(&self.active);
        let gain = k.site_parameter(&self.inotropy);
        let effect = k.at_junction_if_supplied(&self.adrenergic);
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
        self.elastic_energy(cv, &half, &elastic, wave);
        self.stroke_work(cv, ports, heart, &half, wave);
        gravity_work::declare(cv, ports, head, wave);

        let store = ports.dissipation(cv);
        let power = cv.algebraic::<Rate<Energy>>("wall power", Admissible::finite());
        wall_power::declare(cv, &store, &power, &elastic, wave);
        let soft = cv.parameter::<Rate<Energy>>(ParameterKey::new("cv.heat-width"), Admissible::positive());
        dissipation::declare(cv, &store, &power, &soft, wave);
        wall_work::declare(cv, ports, &power, &soft, wave);
        Ok(())
    }
}
