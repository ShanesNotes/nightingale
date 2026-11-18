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
/// `cv.unstressed-gain`. Model default.
fn collapsing<'b, 'k>(
    k: &mut Kernel<'b, 'k, Cardiovascular, Veins>,
    lumen: &Lumen<'b>,
    share: &SiteParameter<'b, Cardiovascular, Veins, Dimensionless>,
    power: &Parameter<'b, Cardiovascular, Dimensionless>,
    width: &Parameter<'b, Cardiovascular, Volume>,
) -> Expr<'b, 'k, Cardiovascular, Volume> {
    let (volume, unstressed) = lumen.volumes(k);
    let shrunk = shrinking(k, unstressed, volume, power);
    let (zero, soft) = (k.zero(), k.parameter(width));
    let below = k.smooth_min(shrunk, zero, soft);
    let share = k.site_parameter(share);
    k.scale(below, share)
}

fn shrinking<'b, 'k>(
    k: &mut Kernel<'b, 'k, Cardiovascular, Veins>,
    unstressed: Expr<'b, 'k, Cardiovascular, Volume>,
    volume: Expr<'b, 'k, Cardiovascular, Volume>,
    power: &Parameter<'b, Cardiovascular, Dimensionless>,
) -> Expr<'b, 'k, Cardiovascular, Volume> {
    let quotient = k.divide(unstressed, volume);
    let ratio: Expr<'_, '_, _, Dimensionless> = k.as_dimension(quotient);
    let power = k.parameter(power);
    let log = k.ln(ratio);
    let scaled = k.scale(log, power);
    let raised = k.exp(scaled);
    let reach = k.scale(unstressed, raised);
    let gap = k.subtract(unstressed, reach);
    let per = k.divide(gap, power);
    k.as_dimension(per)
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
    lumen: Lumen<'b>,
    pub(crate) outside: Port<'b, Pressure>,
    passive: Knob<'b, Elastance>,
    active: Knob<'b, Elastance>,
    inotropy: Knob<'b, Dimensionless>,
    collapse: Parameter<'b, Cardiovascular, Volume>,
    share: SiteParameter<'b, Cardiovascular, Veins, Dimensionless>,
    power: Parameter<'b, Cardiovascular, Dimensionless>,
    collapsed: SiteTerm<'b, Cardiovascular, Veins, Volume>,
    activation: &'h StateSlot<'b, Cardiovascular, Dimensionless>,
}
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
        Self {
            lumen,
            outside: ports.extravascular_pressure(),
            passive,
            active,
            inotropy,
            collapse,
            share,
            power,
            collapsed,
            activation: &heart.activation,
        }
    }
    /// `V - V_u`, with `V` the water's volume and the albumin's, `V_u = V_u_base - S E_a1` (research 02), held no
    /// less than zero off the veins: a chamber or a bed below its unstressed volume collapses and sucks nothing in,
    /// so it cannot pull itself dry. Model default.
    pub(crate) fn stressed<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Blood>,
    ) -> Expr<'b, 'k, Cardiovascular, Volume> {
        let above = self.stretched(k);
        k.add_from(above, &self.collapsed)
    }
    fn stretched<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Blood>,
    ) -> Expr<'b, 'k, Cardiovascular, Volume> {
        let (volume, unstressed) = self.lumen.volumes(k);
        let stretched = k.subtract(volume, unstressed);
        let (zero, soft) = (k.zero(), k.parameter(&self.collapse));
        k.smooth_max(stretched, zero, soft)
    }
    fn contractile<'k, C: SiteSet>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, C>,
    ) -> Expr<'b, 'k, Cardiovascular, Elastance>
    where
        Blood: Includes<C>,
    {
        let active = k.site_parameter_in(&self.active);
        let gain = k.site_parameter_in(&self.inotropy);
        let effect = k.at_junction_if_supplied(&self.lumen.adrenergic);
        let tuned = k.scale(active, gain);
        let strengthened = k.scale(tuned, effect);
        k.add(active, strengthened)
    }
    /// `E_passive + a E_active (1 + S E_b1)`. Model default.
    pub(crate) fn elastance<'k, C: SiteSet>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, C>,
    ) -> Expr<'b, 'k, Cardiovascular, Elastance>
    where
        Blood: Includes<C>,
    {
        let passive = k.site_parameter_in(&self.passive);
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
        below: &SiteTerm<'b, Cardiovascular, Veins, Energy>,
    ) -> Expr<'b, 'k, Cardiovascular, Energy> {
        let stretched = self.stretched(k);
        let product = k.multiply(stiffness, stretched);
        let pressure: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
        let product = k.multiply(pressure, stretched);
        let doubled: Expr<'_, '_, _, Energy> = k.as_dimension(product);
        let share = k.parameter(half);
        let held = k.scale(doubled, share);
        k.add_from(held, below)
    }
    /// Below `V_u` a marked vein's wall holds the work its pressure did, `E share Psi` with
    /// `Psi = (V_u / n) ((V - V_u) - (V_u / (1 - n)) ((V / V_u)^(1 - n) - 1))`, the integral of the collapse branch's
    /// stressed volume from `V_u`. It is written `V_u ((V - V_u) - (V / V_u) shrunk) / (n - 1)`, so a power of one or
    /// less refuses on its divisor. It is read at `V_u + smooth_min(V - V_u, 0, w)`, so it fades above `V_u` and the
    /// store joins `E s^2 / 2` there with its slope. That volume reaches zero at `w^2 / (4 V_u)`, where the term
    /// refuses. With a power above one the store grows without bound as the vein empties. Model default.
    fn emptied<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Cardiovascular, Veins>,
        stiffness: Expr<'b, 'k, Cardiovascular, Elastance>,
    ) -> Expr<'b, 'k, Cardiovascular, Energy> {
        let (volume, unstressed) = self.lumen.volumes(k);
        let stretched = k.subtract(volume, unstressed);
        let (zero, soft) = (k.zero(), k.parameter(&self.collapse));
        let short = k.smooth_min(stretched, zero, soft);
        let smoothed = k.add(unstressed, short);
        let shrunk = shrinking(k, unstressed, smoothed, &self.power);
        let quotient = k.divide(smoothed, unstressed);
        let fraction: Expr<'_, '_, _, Dimensionless> = k.as_dimension(quotient);
        let left = k.scale(shrunk, fraction);
        let gap = k.subtract(short, left);
        let power = k.parameter(&self.power);
        let lifted = k.scale(unstressed, power);
        let excess = k.subtract(lifted, unstressed);
        let quotient = k.divide(unstressed, excess);
        let per: Expr<'_, '_, _, Dimensionless> = k.as_dimension(quotient);
        let reach = k.scale(gap, per);
        let product = k.multiply(stiffness, reach);
        let pressure: Expr<'_, '_, _, Pressure> = k.as_dimension(product);
        let product = k.multiply(pressure, unstressed);
        let work: Expr<'_, '_, _, Energy> = k.as_dimension(product);
        let share = k.site_parameter(&self.share);
        k.scale(work, share)
    }
    /// One mechanical store and one thermal store. Each blood site's wall holds `E s^2 / 2`, a marked vein's below
    /// `V_u` the work its pressure did, and the store is held to their sum. Work enters where activation stiffens a
    /// stretched wall, in systole only, and gravity's enters beside it. The wall power is what the store must lose to
    /// stay on the sum: no law's choice, so closure sits in the stage residual and `verify` judges it with the rest.
    /// What the walls shed leaves as heat, and that heat never comes back to the walls. What they gain past those
    /// receipts, as a vein's muscle tightens or water enters, is muscle work, paid by the heat of the bed the schema
    /// names to pay it or else by the chemical energy the heart draws on. So the account runs none of its own pools
    /// dry; work past what a paying bed's heat holds is refused at that bed's thermal pool. The moving blood's energy
    /// and the stretch of tissue and tubule are not in the sum, so what they hold passes through this rate.
    pub(crate) fn energy(
        &self,
        cv: &mut FragmentBuilder<'b, Cardiovascular>,
        ports: &CardiovascularPorts<'b>,
        heart: &Heart<'b>,
        head: &Parameter<'b, Cardiovascular, Quotient<Pressure, Length>>,
        wave: RateClass,
        emit_work: bool,
    ) -> Result<(), AdmissionError> {
        let half = cv.parameter::<Dimensionless>(ParameterKey::new("cv.half"), Admissible::positive());
        let elastic = cv.algebraic::<Energy>("elastic energy", Admissible::nonnegative());
        let below = cv.site_term("vein collapse store", |mut k| {
            let stiffness = self.elastance(&mut k);
            let value = self.emptied(&mut k, stiffness);
            k.finish_term(value)
        });
        let contracting = cv.site_term("vein collapse stroke work", |mut k| {
            let stiffness = self.contractile(&mut k);
            let value = self.emptied(&mut k, stiffness);
            k.finish_term(value)
        });
        self.elastic_energy(cv, &half, &elastic, &below, wave);
        self.stroke_work(cv, ports, heart, &half, &contracting, wave);
        if emit_work {
            self.myocardial_work(cv, heart, &half, &contracting, wave);
        }
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
