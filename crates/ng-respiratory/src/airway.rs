use ng_contract::catalog::{Defines, PumpForce, Reads, RespiratoryDrive, WorkOfBreathing};
use ng_contract::*;

type Bound<'b, U> = Parameter<'b, Respiratory, U>;
type Amounts<'b, 'k> = (
    Expr<'b, 'k, Respiratory, Oxygen>,
    Expr<'b, 'k, Respiratory, CarbonDioxide>,
    Expr<'b, 'k, Respiratory, NitrogenGas>,
);
type Held<'b> = (
    ReadPort<'b, Respiratory, Oxygen>,
    ReadPort<'b, Respiratory, CarbonDioxide>,
    ReadPort<'b, Respiratory, NitrogenGas>,
    ReadPort<'b, Respiratory, Water>,
);
type PerWater<'b> = Bound<'b, Quotient<Volume, Water>>;
type LatentHeat = Quotient<Rate<Energy>, Rate<Water>>;

pub(crate) fn class() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Beat, 1_000_000_000)
}

fn water_volume<'b>(laws: &mut FragmentBuilder<'b, Respiratory>) -> PerWater<'b> {
    laws.parameter(ParameterKey::new("rs.water-volume"), Admissible::positive())
}

fn held_by<'b>(out: &Pair<'b, Respiratory, Water>) -> Held<'b> {
    (out.amount(), out.amount(), out.amount(), out.amount())
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

/// The room's saturated vapour pressure at its air temperature, `P_sat(T) = P_ref exp(a - b / (T + c))`, the Antoine
/// form the skin's evaporation reads (research 12, the row 'Saturated vapour pressure'). This crate reaches no other
/// card's code, so it carries the form with keys of its own.
struct Room<'b> {
    reference: Bound<'b, Pressure>,
    a: Bound<'b, Dimensionless>,
    b: Bound<'b, Temperature>,
    c: Bound<'b, Temperature>,
}

impl<'b> Room<'b> {
    fn declare(laws: &mut FragmentBuilder<'b, Respiratory>) -> Self {
        let key = ParameterKey::new;
        Self {
            reference: laws.parameter(key("rs.vapour-reference"), Admissible::positive()),
            a: laws.parameter(key("rs.vapour-a"), Admissible::finite()),
            b: laws.parameter(key("rs.vapour-b"), Admissible::positive()),
            c: laws.parameter(key("rs.vapour-c"), Admissible::finite()),
        }
    }

    fn saturated<'k, C>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
    ) -> Expr<'b, 'k, Respiratory, Pressure> {
        let (a, b, c) = (k.parameter(&self.a), k.parameter(&self.b), k.parameter(&self.c));
        let air = k.room_air();
        let shifted = k.add(air, c);
        let ratio = k.divide(b, shifted);
        let ratio = k.as_dimension(ratio);
        let exponent = k.subtract(a, ratio);
        let factor = k.exp(exponent);
        let reference = k.parameter(&self.reference);
        k.scale(reference, factor)
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
    pump: PumpForce<Reads<'b, Respiratory>>,
    drive: RespiratoryDrive<Reads<'b, Respiratory>>,
    wall: AlgebraicSlot<'b, Respiratory, Volume>,
    oxygen: Gas<'b, Oxygen>,
    carbon_dioxide: Gas<'b, CarbonDioxide>,
    nitrogen: Gas<'b, NitrogenGas>,
    oxygen_inspired: Bound<'b, Dimensionless>,
    carbon_dioxide_inspired: Bound<'b, Dimensionless>,
    room: Bound<'b, Pressure>,
    vapour: Bound<'b, Pressure>,
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
            pump: laws.receive(),
            drive: laws.receive(),
            wall: laws.algebraic("chest wall rest volume", Admissible::positive()),
            oxygen: Gas::declare(laws, "rs.oxygen-volume"),
            carbon_dioxide: Gas::declare(laws, "rs.carbon-dioxide-volume"),
            nitrogen: Gas::declare(laws, "rs.nitrogen-volume"),
            oxygen_inspired: laws.parameter(key("rs.oxygen-inspired"), Admissible::unit_interval()),
            carbon_dioxide_inspired: laws
                .parameter(key("rs.carbon-dioxide-inspired"), Admissible::unit_interval()),
            room: laws.parameter(key("rs.room-pressure"), Admissible::positive()),
            vapour: laws.parameter(key("rs.water-vapour-pressure"), Admissible::nonnegative()),
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
        self.share_of(k, &Q::gas(self).volume, Q::of(amounts), amounts)
    }

    fn share_of<'k, C, Q: Breathed + Plain>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        per: &Bound<'b, Quotient<Volume, Q>>,
        held: Expr<'b, 'k, Respiratory, Q>,
        amounts: Amounts<'b, 'k>,
    ) -> Expr<'b, 'k, Respiratory, Dimensionless> {
        let total = self.volume(k, amounts);
        let v = k.parameter(per);
        let part = k.multiply(v, held);
        let part: Expr<'b, 'k, Respiratory, Volume> = k.as_dimension(part);
        let share = k.divide(part, total);
        k.as_dimension(share)
    }

    pub(crate) fn dry_gas_pressure<'k, C>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
    ) -> Expr<'b, 'k, Respiratory, Pressure> {
        let (room, vapour) = (k.parameter(&self.room), k.parameter(&self.vapour));
        k.subtract(room, vapour)
    }

    fn muscle<'k, C>(&self, k: &mut Kernel<'b, 'k, Respiratory, C>) -> Expr<'b, 'k, Respiratory, Pressure> {
        k.read(&self.pump.pump_force)
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
        let (m, share) = (k.parameter(&self.mixing), k.read(&self.drive.mixing_share));
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

    /// Line 84's work of breathing is a memory of the pump's pressure times the breath's inflow at the room's side,
    /// `W = P_mus G h+`, so its mean over whole breaths is the pump's. While the breath rests the memory holds the
    /// breath's stated mean instead, handed over by the mixing share as the mixing exchange is, so the body admits
    /// paying for the breath it will take. The target is written `(W - s W) + s W_mean`, which is exact at a share of
    /// zero and of one, where `W + s (W_mean - W)` loses the mean once the pump's work is far above it. The inflow is
    /// read inside the memory's rate: as a row of its own its kink at zero flow refused every step.
    pub fn work(
        &self,
        laws: &mut FragmentBuilder<'b, Respiratory>,
        ports: &RespiratoryPorts<'b>,
    ) -> Result<(), AdmissionError> {
        let class = class()?;
        let key = ParameterKey::new;
        let line: WorkOfBreathing<Defines<'b, Respiratory>> = laws.emit();
        let speed = laws.parameter::<Rate<Dimensionless>>(key("rs.work-speed"), Admissible::positive());
        let stated = laws.parameter::<Rate<Energy>>(key("rs.work-mean"), Admissible::nonnegative());
        let out = ports.boundary::<Expired<Oxygen>>()?.route;
        let held = (
            out.amount::<Oxygen>(),
            out.amount::<CarbonDioxide>(),
            out.amount::<NitrogenGas>(),
        );
        let memory = laws.state::<Rate<Energy>>("breath work memory", Admissible::nonnegative());
        let rate = laws.kernel(|mut k| {
            let airway = (k.read(&held.0), k.read(&held.1), k.read(&held.2));
            let head = self.dead_space_head(&mut k, airway);
            let [inward, _] = self.parts(&mut k, head);
            let g = k.parameter(&self.conductance);
            let inflow = k.conductance_flow(g, inward);
            let muscle = self.muscle(&mut k);
            let product = k.multiply(muscle, inflow);
            let now = k.as_dimension(product);
            let (mean, share) = (k.parameter(&stated), k.read(&self.drive.mixing_share));
            let lost = k.scale(now, share);
            let kept = k.subtract(now, lost);
            let handed = k.scale(mean, share);
            let target = k.add(kept, handed);
            let (held, pace) = (k.state(&memory), k.parameter(&speed));
            let value = k.relax(target, held, pace);
            k.finish(value, laws.every_mode(), class)
        });
        laws.memory_rate(&memory, rate);
        let work = laws.kernel(|mut k| {
            let value = k.state(&memory);
            k.finish(value, laws.every_mode(), class)
        });
        laws.define(&line.work_of_breathing, work);
        Ok(())
    }

    /// The lung's water rides the dry gas. Its share is its vapour's volume over the dry gas's, `v_w W / V`, so the
    /// airway carries it as it carries each dry gas and it adds nothing to the volume the heads read. The lining
    /// feeds the alveolar gas the water the breath carries out, `V'_A (σ - i) / v_w`, with `V'_A` the alveolar
    /// ventilation the breath is fitted to, `σ = P_H2O / (P_B - P_H2O)` the saturated vapour share and `i` the
    /// room's (research 11, Guyton p. 285). It reads no gas row and no water pool, which keeps the gas's water out of
    /// the elimination's largest block (native-contract 89). The room takes the dead space's water out and gives back
    /// its own at `i = h P_r / (P_B - h P_r)` of the dry gas, with `h` the room's humidity and `P_r` its saturated
    /// vapour pressure at its air temperature, both read from the room.
    pub fn water(
        &self,
        laws: &mut FragmentBuilder<'b, Respiratory>,
        ports: &RespiratoryPorts<'b>,
    ) -> Result<(), AdmissionError> {
        let class = class()?;
        let per = water_volume(laws);
        let ventilated = laws.parameter::<Rate<Volume>>(
            ParameterKey::new("rs.alveolar-ventilation"),
            Admissible::nonnegative(),
        );
        let room = Room::declare(laws);

        let lining = ports.lining(laws);
        let rate = laws.site_kernel(|mut k| {
            let dry = self.dry_gas_pressure(&mut k);
            let vapour = k.parameter(&self.vapour);
            let saturated = k.divide(vapour, dry);
            let saturated = k.as_dimension(saturated);
            let given = self.room_water(&mut k, &room);
            let gap = k.subtract(saturated, given);
            let ventilation = k.parameter(&ventilated);
            let flow = k.scale(ventilation, gap);
            let value = self.amount_rate(&mut k, flow, &per);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer_at(&lining, rate);

        let route = ports.airway::<Water>(laws);
        let ends = [
            (
                route.from_amount::<Oxygen>(),
                route.from_amount::<CarbonDioxide>(),
                route.from_amount::<NitrogenGas>(),
                route.from_amount::<Water>(),
            ),
            (
                route.to_amount::<Oxygen>(),
                route.to_amount::<CarbonDioxide>(),
                route.to_amount::<NitrogenGas>(),
                route.to_amount::<Water>(),
            ),
        ];
        let rate = laws.edge_kernel(|mut k| {
            let [up, down] = ends.each_ref().map(|end| {
                let dry = (k.at_edge(&end.0), k.at_edge(&end.1), k.at_edge(&end.2));
                (dry, k.at_edge(&end.3))
            });
            let shares = [
                self.share_of(&mut k, &per, up.1, up.0),
                self.share_of(&mut k, &per, down.1, down.0),
            ];
            let head = self.alveolar_head(&mut k, down.0);
            let value = self.carried(&mut k, head, shares, &per);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer_along(&route, rate);

        let out = ports.boundary::<Expired<Water>>()?.route;
        let held = held_by(&out);
        let rate = laws.kernel(|mut k| {
            let [leaving, entering] = self.exhaled(&mut k, &held, &per, &room);
            let value = k.subtract(leaving, entering);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&out, rate));
        Ok(())
    }

    /// The water the breath carries out takes its latent heat with it, `Q = lambda W'`, with `W'` the net water the
    /// expired water's law posts, what leaves less what the room gives back (research 11, Guyton p. 869). The heat
    /// leaves the airway store, which conduction from the core feeds.
    pub fn airway_heat(
        &self,
        laws: &mut FragmentBuilder<'b, Respiratory>,
        ports: &RespiratoryPorts<'b>,
    ) -> Result<(), AdmissionError> {
        let class = class()?;
        let (per, room) = (water_volume(laws), Room::declare(laws));
        let latent =
            laws.parameter::<LatentHeat>(ParameterKey::new("rs.latent-heat"), Admissible::positive());
        let held = held_by(&ports.boundary::<Expired<Water>>()?.route);
        let heat = ports.boundary::<AirwayHeat>()?.route;
        let rate = laws.kernel(|mut k| {
            let [leaving, entering] = self.exhaled(&mut k, &held, &per, &room);
            let water = k.subtract(leaving, entering);
            let latent = k.parameter(&latent);
            let product = k.multiply(latent, water);
            let value = k.as_dimension(product);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&heat, rate));
        Ok(())
    }

    fn exhaled<'k>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory>,
        held: &Held<'b>,
        per: &PerWater<'b>,
        room: &Room<'b>,
    ) -> [Expr<'b, 'k, Respiratory, Rate<Water>>; 2] {
        let airway = (k.read(&held.0), k.read(&held.1), k.read(&held.2));
        let water = k.read(&held.3);
        let share = self.share_of(k, per, water, airway);
        let head = self.dead_space_head(k, airway);
        let [inward, outward] = self.parts(k, head);
        let leaving = self.at_the_room(k, outward, share, per);
        let room_share = self.room_water(k, room);
        let entering = self.at_the_room(k, inward, room_share, per);
        [leaving, entering]
    }

    fn room_water<'k, C>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        room: &Room<'b>,
    ) -> Expr<'b, 'k, Respiratory, Dimensionless> {
        let saturated = room.saturated(k);
        let h = k.room_humidity();
        let vapour = k.scale(saturated, h);
        let pressure = k.parameter(&self.room);
        let dry = k.subtract(pressure, vapour);
        let share = k.divide(vapour, dry);
        k.as_dimension(share)
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

    fn amount_rate<'k, C, Q: Breathed + Plain>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        flow: Expr<'b, 'k, Respiratory, Rate<Volume>>,
        per: &Bound<'b, Quotient<Volume, Q>>,
    ) -> Expr<'b, 'k, Respiratory, Rate<Q>> {
        let v = k.parameter(per);
        let rate = k.divide(flow, v);
        k.as_dimension(rate)
    }

    fn carried<'k, C, Q: Breathed + Plain>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        head: Expr<'b, 'k, Respiratory, Pressure>,
        [up, down]: [Expr<'b, 'k, Respiratory, Dimensionless>; 2],
        per: &Bound<'b, Quotient<Volume, Q>>,
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
        self.amount_rate(k, flow, per)
    }

    fn at_the_room<'k, C, Q: Breathed + Plain>(
        &self,
        k: &mut Kernel<'b, 'k, Respiratory, C>,
        head: Expr<'b, 'k, Respiratory, Pressure>,
        share: Expr<'b, 'k, Respiratory, Dimensionless>,
        per: &Bound<'b, Quotient<Volume, Q>>,
    ) -> Expr<'b, 'k, Respiratory, Rate<Q>> {
        let (g, m) = (k.parameter(&self.conductance), self.mixing(k));
        let bulk = k.conductance_flow(g, head);
        let flow = k.add(bulk, m);
        let flow = k.scale(flow, share);
        self.amount_rate(k, flow, per)
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
            let value = self.carried(&mut k, head, shares, &Q::gas(self).volume);
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
            let value = self.amount_rate(&mut k, flow, &I::Quantity::gas(self).volume);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&into, rate));
        let rate = laws.kernel(|mut k| {
            let airway = (k.read(&held.0), k.read(&held.1), k.read(&held.2));
            let f = self.share::<_, I::Quantity>(&mut k, airway);
            let head = self.dead_space_head(&mut k, airway);
            let [_, outward] = self.parts(&mut k, head);
            let value = self.at_the_room(&mut k, outward, f, &I::Quantity::gas(self).volume);
            k.finish(value, laws.every_mode(), class)
        });
        laws.transfer(Transfer::new(&out, rate));
        Ok(())
    }
}
