use ng_contract::catalog::{ArterialGases, BrainAcidity, Reads};
use ng_contract::*;

type Value<'b, 'k, U> = Expr<'b, 'k, Nervous, U>;

/// The peripheral sensor is research 11's `x_p = (1 - S_a) smooth_max(PaCO2 - B_p, 0, w)` at the sampled arteries.
/// Each set point follows its sensor slowly, as the baroreflex's does, so admission solves it to the sensor's mean
/// over the beat and the drive scale's mean there is its rest.
pub(crate) fn declare<'b>(
    ns: &mut FragmentBuilder<'b, Nervous>,
    reflex: RateClass,
    slow: RateClass,
) -> AlgebraicSlot<'b, Nervous, Dimensionless> {
    let key = ParameterKey::new;
    let brain: BrainAcidity<Reads<'b, Nervous>> = ns.receive();
    let arterial: ArterialGases<Reads<'b, Nervous>> = ns.receive();
    let central = ns.set_point::<Dimensionless>("central set point", Admissible::finite());
    let peripheral = ns.set_point::<Pressure>("peripheral set point", Admissible::finite());
    let scale = ns.algebraic::<Dimensionless>("drive scale", Admissible::positive());
    let speed = ns.parameter::<Rate<Dimensionless>>(key("ns.chemo-set-point-speed"), Admissible::positive());
    let offset = ns.parameter::<Pressure>(key("ns.peripheral-offset"), Admissible::finite());
    let smoothing = ns.parameter::<Pressure>(key("ns.peripheral-smoothing"), Admissible::positive());
    let central_gain = ns.parameter::<Dimensionless>(key("ns.central-gain"), Admissible::nonnegative());
    let peripheral_gain = ns.parameter::<Dimensionless>(key("ns.peripheral-gain"), Admissible::nonnegative());
    let central_width =
        ns.parameter::<Deviation<Dimensionless>>(key("ns.central-width"), Admissible::positive());
    let peripheral_width =
        ns.parameter::<Deviation<Pressure>>(key("ns.peripheral-width"), Admissible::positive());
    let rest = ns.parameter::<Dimensionless>(key("ns.drive-scale-rest"), Admissible::positive());

    let sensors = Sensors {
        brain,
        arterial,
        offset,
        smoothing,
    };

    let following = ns.kernel(|mut k| {
        let (measured, reference) = (sensors.acidity(&mut k), k.state(&central));
        let off = k.deviation(measured, reference);
        let pace = k.parameter(&speed);
        let product = k.multiply(off, pace);
        let rate = k.as_dimension(product);
        k.finish(rate, ns.every_mode(), slow)
    });
    ns.memory_rate(&central, following);
    let following = ns.kernel(|mut k| {
        let (measured, reference) = (sensors.measure(&mut k), k.state(&peripheral));
        let off = k.deviation(measured, reference);
        let pace = k.parameter(&speed);
        let product = k.multiply(off, pace);
        let rate = k.as_dimension(product);
        k.finish(rate, ns.every_mode(), slow)
    });
    ns.memory_rate(&peripheral, following);

    let residual = ns.kernel(|mut k| {
        let (measured, reference) = (sensors.measure(&mut k), k.state(&peripheral));
        let off = k.deviation(measured, reference);
        let (width, gain) = (k.parameter(&peripheral_width), k.parameter(&peripheral_gain));
        let rising = arm(&mut k, off, width, gain);
        let (measured, reference) = (sensors.acidity(&mut k), k.state(&central));
        let off = k.deviation(measured, reference);
        let (width, gain) = (k.parameter(&central_width), k.parameter(&central_gain));
        let falling = arm(&mut k, off, width, gain);
        let rest = k.parameter(&rest);
        let raised = k.add(rest, rising);
        let value = k.subtract(raised, falling);
        let held = k.algebraic(&scale);
        let residual = k.subtract(held, value);
        k.finish(residual, ns.every_mode(), reflex)
    });
    ns.residual(&scale, residual);
    scale
}

struct Sensors<'b> {
    brain: BrainAcidity<Reads<'b, Nervous>>,
    arterial: ArterialGases<Reads<'b, Nervous>>,
    offset: Parameter<'b, Nervous, Pressure>,
    smoothing: Parameter<'b, Nervous, Pressure>,
}

impl<'b> Sensors<'b> {
    fn acidity<'k>(&self, k: &mut Kernel<'b, 'k, Nervous>) -> Value<'b, 'k, Dimensionless> {
        k.at_landmark(Landmark::CentralChemoreceptor, &self.brain.acidity)
    }
    fn measure<'k>(&self, k: &mut Kernel<'b, 'k, Nervous>) -> Value<'b, 'k, Pressure> {
        let at = Landmark::PeripheralChemoreceptor;
        let pressure = k.at_landmark(at, &self.arterial.carbon_dioxide_partial_pressure);
        let saturation = k.at_landmark(at, &self.arterial.oxygen_saturation);
        let (offset, smoothing, zero) = (k.parameter(&self.offset), k.parameter(&self.smoothing), k.zero());
        let above = k.subtract(pressure, offset);
        let above = k.smooth_max(above, zero, smoothing);
        let saturated = k.scale(above, saturation);
        k.subtract(above, saturated)
    }
}

fn arm<'b, 'k, U: Sensed>(
    k: &mut Kernel<'b, 'k, Nervous>,
    off: Value<'b, 'k, Deviation<U>>,
    width: Value<'b, 'k, Deviation<U>>,
    gain: Value<'b, 'k, Dimensionless>,
) -> Value<'b, 'k, Dimensionless> {
    let ratio = k.divide(off, width);
    let ratio = k.as_dimension(ratio);
    k.scale(ratio, gain)
}
