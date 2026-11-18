use crate::breath::Breath;
use ng_contract::*;

type Bound<'b, U> = Parameter<'b, Respiratory, U>;
type Amounts<'b, 'k> = (
    Expr<'b, 'k, Respiratory, Oxygen>,
    Expr<'b, 'k, Respiratory, CarbonDioxide>,
    Expr<'b, 'k, Respiratory, NitrogenGas>,
);

pub(crate) fn class() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Beat, 1_000_000_000)
}

pub(crate) struct Gas<'b, Q> {
    volume: Bound<'b, Quotient<Volume, Q>>,
}

impl<'b, Q: Substance> Gas<'b, Q> {
    fn declare(laws: &mut FragmentBuilder<'b, Respiratory>, key: &'static str) -> Self {
        Self {
            volume: laws.parameter(ParameterKey::new(key), Admissible::positive()),
        }
    }
}

pub(crate) trait Dry: Breathed + Plain + Sized {
    fn gas<'r, 'b>(ventilation: &'r Ventilation<'b>) -> &'r Gas<'b, Self>;
    fn of<'b, 'k>(amounts: Amounts<'b, 'k>) -> Expr<'b, 'k, Respiratory, Self>;
    fn inspired<'b, 'k, C>(
        ventilation: &Ventilation<'b>,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        flow: Expr<'b, 'k, Respiratory, Rate<Volume>>,
    ) -> Expr<'b, 'k, Respiratory, Rate<Volume>>;
}
impl Dry for Oxygen {
    fn gas<'r, 'b>(ventilation: &'r Ventilation<'b>) -> &'r Gas<'b, Self> {
        &ventilation.oxygen
    }
    fn inspired<'b, 'k, C>(
        ventilation: &Ventilation<'b>,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        flow: Expr<'b, 'k, Respiratory, Rate<Volume>>,
    ) -> Expr<'b, 'k, Respiratory, Rate<Volume>> {
        let share = k.parameter(&ventilation.oxygen_inspired);
        k.scale(flow, share)
    }
    fn of<'b, 'k>(amounts: Amounts<'b, 'k>) -> Expr<'b, 'k, Respiratory, Self> {
        amounts.0
    }
}
impl Dry for CarbonDioxide {
    fn gas<'r, 'b>(ventilation: &'r Ventilation<'b>) -> &'r Gas<'b, Self> {
        &ventilation.carbon_dioxide
    }
    fn inspired<'b, 'k, C>(
        ventilation: &Ventilation<'b>,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        flow: Expr<'b, 'k, Respiratory, Rate<Volume>>,
    ) -> Expr<'b, 'k, Respiratory, Rate<Volume>> {
        let share = k.parameter(&ventilation.carbon_dioxide_inspired);
        k.scale(flow, share)
    }
    fn of<'b, 'k>(amounts: Amounts<'b, 'k>) -> Expr<'b, 'k, Respiratory, Self> {
        amounts.1
    }
}
impl Dry for NitrogenGas {
    fn gas<'r, 'b>(ventilation: &'r Ventilation<'b>) -> &'r Gas<'b, Self> {
        &ventilation.nitrogen
    }
    fn inspired<'b, 'k, C>(
        ventilation: &Ventilation<'b>,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        flow: Expr<'b, 'k, Respiratory, Rate<Volume>>,
    ) -> Expr<'b, 'k, Respiratory, Rate<Volume>> {
        let oxygen = Oxygen::inspired(ventilation, k, flow);
        let carbon_dioxide = CarbonDioxide::inspired(ventilation, k, flow);
        let rest = k.subtract(flow, oxygen);
        k.subtract(rest, carbon_dioxide)
    }
    fn of<'b, 'k>(amounts: Amounts<'b, 'k>) -> Expr<'b, 'k, Respiratory, Self> {
        amounts.2
    }
}

/// The room's side reads the dead space's own head, not the alveolar unit's: with the alveolar unit's head there, the
/// dead space's total is conserved by these laws and nothing sets it.
pub struct Ventilation<'b> {
    breath: Breath<'b>,
    pump: Bound<'b, Pressure>,
    wall: AlgebraicSlot<'b, Respiratory, Volume>,
    oxygen: Gas<'b, Oxygen>,
    carbon_dioxide: Gas<'b, CarbonDioxide>,
    nitrogen: Gas<'b, NitrogenGas>,
    oxygen_inspired: Bound<'b, Dimensionless>,
    carbon_dioxide_inspired: Bound<'b, Dimensionless>,
    conductance: Bound<'b, Conductance<Volume>>,
    mixing: Bound<'b, Rate<Volume>>,
    lung_elastance: Bound<'b, Elastance>,
    wall_elastance: Bound<'b, Elastance>,
    lung_rest: Bound<'b, Volume>,
    wall_rest: Bound<'b, Volume>,
    dead_space_elastance: Bound<'b, Elastance>,
    dead_space: Bound<'b, Volume>,
    width: Bound<'b, Pressure>,
}

impl<'b> Ventilation<'b> {
    pub fn declare(
        laws: &mut FragmentBuilder<'b, Respiratory>,
        ports: &RespiratoryPorts<'b>,
    ) -> Result<Self, AdmissionError> {
        let key = ParameterKey::new;
        let class = class()?;
        let ventilation = Self {
            breath: Breath::declare(laws, class)?,
            pump: laws.parameter(key("rs.pump-pressure"), Admissible::nonnegative()),
            wall: laws.algebraic("chest wall rest volume", Admissible::positive()),
            oxygen: Gas::declare(laws, "rs.oxygen-volume"),
            carbon_dioxide: Gas::declare(laws, "rs.carbon-dioxide-volume"),
            nitrogen: Gas::declare(laws, "rs.nitrogen-volume"),
            oxygen_inspired: laws.parameter(key("rs.oxygen-inspired"), Admissible::unit_interval()),
            carbon_dioxide_inspired: laws
                .parameter(key("rs.carbon-dioxide-inspired"), Admissible::unit_interval()),
            conductance: laws.parameter(key("rs.airway-conductance"), Admissible::positive()),
            mixing: laws.parameter(key("rs.airway-mixing"), Admissible::nonnegative()),
            lung_elastance: laws.parameter(key("rs.lung-elastance"), Admissible::positive()),
            wall_elastance: laws.parameter(key("rs.wall-elastance"), Admissible::positive()),
            lung_rest: laws.parameter(key("rs.lung-rest-volume"), Admissible::positive()),
            wall_rest: laws.parameter(key("rs.wall-rest-volume"), Admissible::positive()),
            dead_space_elastance: laws.parameter(key("rs.dead-space-elastance"), Admissible::positive()),
            dead_space: laws.parameter(key("rs.dead-space-volume"), Admissible::positive()),
            width: laws.parameter(key("rs.head-width"), Admissible::positive()),
        };
        ventilation.posture(laws, class);
        ventilation.along::<Oxygen>(laws, ports, class);
        ventilation.along::<CarbonDioxide>(laws, ports, class);
        ventilation.along::<NitrogenGas>(laws, ports, class);
        ventilation.room::<Inspired<Oxygen>, Expired<Oxygen>>(laws, ports, class)?;
        ventilation.room::<Inspired<CarbonDioxide>, Expired<CarbonDioxide>>(laws, ports, class)?;
        ventilation.room::<Inspired<NitrogenGas>, Expired<NitrogenGas>>(laws, ports, class)?;
        Ok(ventilation)
    }

    fn volume<'k, C>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        (o, c, n): Amounts<'b, 'k>,
    ) -> Expr<'b, 'k, Respiratory, Volume> {
        let (vo, vc, vn) = (
            k.parameter(&self.oxygen.volume),
            k.parameter(&self.carbon_dioxide.volume),
            k.parameter(&self.nitrogen.volume),
        );
        let (o, c, n) = (k.multiply(vo, o), k.multiply(vc, c), k.multiply(vn, n));
        let (o, c, n): (Expr<'b, 'k, Respiratory, Volume>, _, _) =
            (k.as_dimension(o), k.as_dimension(c), k.as_dimension(n));
        let total = k.add(o, c);
        k.add(total, n)
    }

    pub(crate) fn share<'k, C, Q: Dry>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        amounts: Amounts<'b, 'k>,
    ) -> Expr<'b, 'k, Respiratory, Dimensionless> {
        let total = self.volume(k, amounts);
        let v = k.parameter(&Q::gas(self).volume);
        let part = k.multiply(v, Q::of(amounts));
        let part: Expr<'b, 'k, Respiratory, Volume> = k.as_dimension(part);
        let share = k.divide(part, total);
        k.as_dimension(share)
    }

    /// The pump's pressure, `P_mus = PImax a` for the pump drive `a` (research 11's pump row).
    fn muscle<'k, C>(&self, k: &mut Kernel<'b, 'k, Respiratory, C>) -> Expr<'b, 'k, Respiratory, Pressure> {
        let (pump, drive) = (k.parameter(&self.pump), k.state(&self.breath.drive));
        k.scale(pump, drive)
    }

    /// `V_W0 = V_W0,supine + dV_W rise / L`: sitting up raises the chest wall's relaxed volume (research 11, Nunn
    /// pp. 46 to 47), and the carotid sinus's rise over its distance up the trunk is the sine of the trunk's angle.
    fn posture(&self, laws: &mut FragmentBuilder<'b, Respiratory>, class: RateClass) {
        let key = ParameterKey::new;
        let slope = laws.parameter::<Volume>(key("rs.wall-rest-rise"), Admissible::finite());
        let trunk = laws.parameter::<Length>(key("rs.trunk-length"), Admissible::positive());
        let residual = laws.kernel(|mut k| {
            let (supine, slope, length) = (
                k.parameter(&self.wall_rest),
                k.parameter(&slope),
                k.parameter(&trunk),
            );
            let rise = k.rise(Landmark::CarotidSinus);
            let sine = k.divide(rise, length);
            let sine: Expr<'b, '_, Respiratory, Dimensionless> = k.as_dimension(sine);
            let raised = k.scale(slope, sine);
            let value = k.add(supine, raised);
            let row = k.algebraic(&self.wall);
            let residual = k.subtract(row, value);
            k.finish(residual, laws.every_mode(), class)
        });
        laws.residual(&self.wall, residual);
    }

    fn mixing<'k, C>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
    ) -> Expr<'b, 'k, Respiratory, Rate<Volume>> {
        let (m, share) = (k.parameter(&self.mixing), k.state(&self.breath.share));
        k.scale(m, share)
    }

    fn alveolar_head<'k, C>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        lung: Amounts<'b, 'k>,
    ) -> Expr<'b, 'k, Respiratory, Pressure> {
        let v = self.volume(k, lung);
        let (el, ew, l0, w0) = (
            k.parameter(&self.lung_elastance),
            k.parameter(&self.wall_elastance),
            k.parameter(&self.lung_rest),
            k.algebraic(&self.wall),
        );
        let (stretched, sprung) = (k.subtract(v, l0), k.subtract(v, w0));
        let (lung_recoil, wall_recoil) = (k.multiply(el, stretched), k.multiply(ew, sprung));
        let (lung_recoil, wall_recoil): (Expr<'b, 'k, Respiratory, Pressure>, _) =
            (k.as_dimension(lung_recoil), k.as_dimension(wall_recoil));
        let recoil = k.add(lung_recoil, wall_recoil);
        let muscle = self.muscle(k);
        k.subtract(muscle, recoil)
    }

    /// The pleural pressure is the lung's recoil at the mechanics' static solution, `P_pl = -E_L (V - V_L0)` with
    /// `V - V_L0 = (E_W (V_W0 - V_L0) + P_mus) / (E_L + E_W)`, so `P_pl = P_pl,rest - E_L / (E_L + E_W) P_mus`.
    pub fn pleural(
        &self,
        laws: &mut FragmentBuilder<'b, Respiratory>,
        ports: &RespiratoryPorts<'b>,
    ) -> Result<(), AdmissionError> {
        let class = class()?;
        let pressure = laws.site_kernel(|mut k| {
            let (el, ew, l0, w0) = (
                k.parameter(&self.lung_elastance),
                k.parameter(&self.wall_elastance),
                k.parameter(&self.lung_rest),
                k.algebraic(&self.wall),
            );
            let apart = k.subtract(w0, l0);
            let held = k.multiply(ew, apart);
            let held: Expr<'b, '_, Respiratory, Pressure> = k.as_dimension(held);
            let muscle = self.muscle(&mut k);
            let pull = k.add(held, muscle);
            let both = k.add(el, ew);
            let stretch = k.divide(pull, both);
            let stretch: Expr<'b, '_, Respiratory, Volume> = k.as_dimension(stretch);
            let recoil = k.multiply(el, stretch);
            let recoil: Expr<'b, '_, Respiratory, Pressure> = k.as_dimension(recoil);
            let zero = k.zero();
            let value = k.subtract(zero, recoil);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define_at(&ports.enclosure_pressure, pressure);
        Ok(())
    }

    fn dead_space_head<'k, C>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        airway: Amounts<'b, 'k>,
    ) -> Expr<'b, 'k, Respiratory, Pressure> {
        let v = self.volume(k, airway);
        let (e, v0) = (
            k.parameter(&self.dead_space_elastance),
            k.parameter(&self.dead_space),
        );
        let short = k.subtract(v0, v);
        let head = k.multiply(e, short);
        k.as_dimension(head)
    }

    fn parts<'k, C>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        head: Expr<'b, 'k, Respiratory, Pressure>,
    ) -> [Expr<'b, 'k, Respiratory, Pressure>; 2] {
        let (zero, w) = (k.zero(), k.parameter(&self.width));
        let inward = k.smooth_max(head, zero, w);
        let back = k.subtract(zero, head);
        let outward = k.smooth_max(back, zero, w);
        [inward, outward]
    }

    fn amount_rate<'k, C, Q: Dry>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        flow: Expr<'b, 'k, Respiratory, Rate<Volume>>,
    ) -> Expr<'b, 'k, Respiratory, Rate<Q>> {
        let v = k.parameter(&Q::gas(self).volume);
        let rate = k.divide(flow, v);
        k.as_dimension(rate)
    }

    fn carried<'k, C, Q: Dry>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        head: Expr<'b, 'k, Respiratory, Pressure>,
        [up, down]: [Expr<'b, 'k, Respiratory, Dimensionless>; 2],
    ) -> Expr<'b, 'k, Respiratory, Rate<Q>> {
        let [inward, outward] = self.parts(k, head);
        let g = k.parameter(&self.conductance);
        let (bulk_in, bulk_out) = (k.conductance_flow(g, inward), k.conductance_flow(g, outward));
        let (carried_in, carried_out) = (k.scale(bulk_in, up), k.scale(bulk_out, down));
        let bulk = k.subtract(carried_in, carried_out);
        let m = self.mixing(k);
        let gap = k.subtract(up, down);
        let mixed = k.scale(m, gap);
        let flow = k.add(bulk, mixed);
        self.amount_rate::<_, Q>(k, flow)
    }

    fn at_the_room<'k, C, Q: Dry>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        head: Expr<'b, 'k, Respiratory, Pressure>,
        share: Expr<'b, 'k, Respiratory, Dimensionless>,
    ) -> Expr<'b, 'k, Respiratory, Rate<Q>> {
        let (g, m) = (k.parameter(&self.conductance), self.mixing(k));
        let bulk = k.conductance_flow(g, head);
        let flow = k.add(bulk, m);
        let flow = k.scale(flow, share);
        self.amount_rate::<_, Q>(k, flow)
    }

    fn along<Q: Dry>(
        &self,
        laws: &mut FragmentBuilder<'b, Respiratory>,
        ports: &RespiratoryPorts<'b>,
        class: RateClass,
    ) {
        let route = ports.airway::<Q>(laws);
        let ups = (
            route.from_amount::<Oxygen>(),
            route.from_amount::<CarbonDioxide>(),
            route.from_amount::<NitrogenGas>(),
        );
        let downs = (
            route.to_amount::<Oxygen>(),
            route.to_amount::<CarbonDioxide>(),
            route.to_amount::<NitrogenGas>(),
        );
        let rate = laws.edge_kernel(|mut k| {
            let up = (k.at_edge(&ups.0), k.at_edge(&ups.1), k.at_edge(&ups.2));
            let down = (k.at_edge(&downs.0), k.at_edge(&downs.1), k.at_edge(&downs.2));
            let shares = [self.share::<_, Q>(&mut k, up), self.share::<_, Q>(&mut k, down)];
            let head = self.alveolar_head(&mut k, down);
            let value = self.carried::<_, Q>(&mut k, head, shares);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer_along(&route, rate);
    }

    fn room<I, E>(
        &self,
        laws: &mut FragmentBuilder<'b, Respiratory>,
        ports: &RespiratoryPorts<'b>,
        class: RateClass,
    ) -> Result<(), AdmissionError>
    where
        I: BoundaryProcess<Card = Respiratory>,
        E: BoundaryProcess<Card = Respiratory, Quantity = I::Quantity>,
        I::Quantity: Dry,
    {
        let into = ports.boundary::<I>()?.route;
        let out = ports.boundary::<E>()?.route;
        let held = (
            out.amount::<Oxygen>(),
            out.amount::<CarbonDioxide>(),
            out.amount::<NitrogenGas>(),
        );
        let rate = laws.kernel(|mut k| {
            let airway = (k.read(&held.0), k.read(&held.1), k.read(&held.2));
            let head = self.dead_space_head(&mut k, airway);
            let [inward, _] = self.parts(&mut k, head);
            let (g, m) = (k.parameter(&self.conductance), self.mixing(&mut k));
            let bulk = k.conductance_flow(g, inward);
            let flow = k.add(bulk, m);
            let flow = I::Quantity::inspired(self, &mut k, flow);
            let value = self.amount_rate::<_, I::Quantity>(&mut k, flow);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&into, rate));
        let rate = laws.kernel(|mut k| {
            let airway = (k.read(&held.0), k.read(&held.1), k.read(&held.2));
            let f = self.share::<_, I::Quantity>(&mut k, airway);
            let head = self.dead_space_head(&mut k, airway);
            let [_, outward] = self.parts(&mut k, head);
            let value = self.at_the_room::<_, I::Quantity>(&mut k, outward, f);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&out, rate));
        Ok(())
    }
}
