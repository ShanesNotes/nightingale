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
    conductance: Bound<'b, Conductance<Q>>,
    mixing: Bound<'b, Rate<Q>>,
    inspired_share: Bound<'b, Dimensionless>,
}

impl<'b, Q: Substance> Gas<'b, Q> {
    fn declare(laws: &mut FragmentBuilder<'b, Respiratory>, keys: [&'static str; 4]) -> Self {
        let [volume, conductance, mixing, inspired] = keys.map(ParameterKey::new);
        Self {
            volume: laws.parameter(volume, Admissible::positive()),
            conductance: laws.parameter(conductance, Admissible::positive()),
            mixing: laws.parameter(mixing, Admissible::nonnegative()),
            inspired_share: laws.parameter(inspired, Admissible::unit_interval()),
        }
    }
}

pub(crate) trait Dry: Breathed + Plain + Sized {
    fn gas<'r, 'b>(ventilation: &'r Ventilation<'b>) -> &'r Gas<'b, Self>;
    fn of<'b, 'k>(amounts: Amounts<'b, 'k>) -> Expr<'b, 'k, Respiratory, Self>;
}
impl Dry for Oxygen {
    fn gas<'r, 'b>(ventilation: &'r Ventilation<'b>) -> &'r Gas<'b, Self> {
        &ventilation.oxygen
    }
    fn of<'b, 'k>(amounts: Amounts<'b, 'k>) -> Expr<'b, 'k, Respiratory, Self> {
        amounts.0
    }
}
impl Dry for CarbonDioxide {
    fn gas<'r, 'b>(ventilation: &'r Ventilation<'b>) -> &'r Gas<'b, Self> {
        &ventilation.carbon_dioxide
    }
    fn of<'b, 'k>(amounts: Amounts<'b, 'k>) -> Expr<'b, 'k, Respiratory, Self> {
        amounts.1
    }
}
impl Dry for NitrogenGas {
    fn gas<'r, 'b>(ventilation: &'r Ventilation<'b>) -> &'r Gas<'b, Self> {
        &ventilation.nitrogen
    }
    fn of<'b, 'k>(amounts: Amounts<'b, 'k>) -> Expr<'b, 'k, Respiratory, Self> {
        amounts.2
    }
}

/// The room's side reads the dead space's own head, not the alveolar unit's: with the alveolar unit's head there, the
/// dead space's total is conserved by these laws and nothing sets it.
pub struct Ventilation<'b> {
    oxygen: Gas<'b, Oxygen>,
    carbon_dioxide: Gas<'b, CarbonDioxide>,
    nitrogen: Gas<'b, NitrogenGas>,
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
        let ventilation = Self {
            oxygen: Gas::declare(
                laws,
                [
                    "rs.oxygen-volume",
                    "rs.oxygen-conductance",
                    "rs.oxygen-mixing",
                    "rs.oxygen-inspired",
                ],
            ),
            carbon_dioxide: Gas::declare(
                laws,
                [
                    "rs.carbon-dioxide-volume",
                    "rs.carbon-dioxide-conductance",
                    "rs.carbon-dioxide-mixing",
                    "rs.carbon-dioxide-inspired",
                ],
            ),
            nitrogen: Gas::declare(
                laws,
                [
                    "rs.nitrogen-volume",
                    "rs.nitrogen-conductance",
                    "rs.nitrogen-mixing",
                    "rs.nitrogen-inspired",
                ],
            ),
            lung_elastance: laws.parameter(key("rs.lung-elastance"), Admissible::positive()),
            wall_elastance: laws.parameter(key("rs.wall-elastance"), Admissible::positive()),
            lung_rest: laws.parameter(key("rs.lung-rest-volume"), Admissible::positive()),
            wall_rest: laws.parameter(key("rs.wall-rest-volume"), Admissible::positive()),
            dead_space_elastance: laws.parameter(key("rs.dead-space-elastance"), Admissible::positive()),
            dead_space: laws.parameter(key("rs.dead-space-volume"), Admissible::positive()),
            width: laws.parameter(key("rs.head-width"), Admissible::positive()),
        };
        let class = class()?;
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
            k.parameter(&self.wall_rest),
        );
        let (stretched, sprung) = (k.subtract(v, l0), k.subtract(v, w0));
        let (lung_recoil, wall_recoil) = (k.multiply(el, stretched), k.multiply(ew, sprung));
        let (lung_recoil, wall_recoil): (Expr<'b, 'k, Respiratory, Pressure>, _) =
            (k.as_dimension(lung_recoil), k.as_dimension(wall_recoil));
        let recoil = k.add(lung_recoil, wall_recoil);
        let zero = k.zero();
        k.subtract(zero, recoil)
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

    fn carried<'k, C, Q: Dry>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        head: Expr<'b, 'k, Respiratory, Pressure>,
        [up, down]: [Expr<'b, 'k, Respiratory, Dimensionless>; 2],
    ) -> Expr<'b, 'k, Respiratory, Rate<Q>> {
        let gas = Q::gas(self);
        let [inward, outward] = self.parts(k, head);
        let (carried_in, carried_out) = (k.scale(inward, up), k.scale(outward, down));
        let g = k.parameter(&gas.conductance);
        let (bulk_in, bulk_out) = (
            k.conductance_flow(g, carried_in),
            k.conductance_flow(g, carried_out),
        );
        let bulk = k.subtract(bulk_in, bulk_out);
        let m = k.parameter(&gas.mixing);
        let gap = k.subtract(up, down);
        let mixed = k.scale(m, gap);
        k.add(bulk, mixed)
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
        let gas = I::Quantity::gas(self);
        let rate = laws.kernel(|mut k| {
            let airway = (k.read(&held.0), k.read(&held.1), k.read(&held.2));
            let head = self.dead_space_head(&mut k, airway);
            let [inward, _] = self.parts(&mut k, head);
            let fraction = k.parameter(&gas.inspired_share);
            let carried = k.scale(inward, fraction);
            let g = k.parameter(&gas.conductance);
            let bulk = k.conductance_flow(g, carried);
            let m = k.parameter(&gas.mixing);
            let mixed = k.scale(m, fraction);
            let value = k.add(bulk, mixed);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&into, rate));
        let rate = laws.kernel(|mut k| {
            let airway = (k.read(&held.0), k.read(&held.1), k.read(&held.2));
            let f = self.share::<_, I::Quantity>(&mut k, airway);
            let head = self.dead_space_head(&mut k, airway);
            let [_, outward] = self.parts(&mut k, head);
            let carried = k.scale(outward, f);
            let g = k.parameter(&gas.conductance);
            let bulk = k.conductance_flow(g, carried);
            let m = k.parameter(&gas.mixing);
            let mixed = k.scale(m, f);
            let value = k.add(bulk, mixed);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&out, rate));
        Ok(())
    }
}
