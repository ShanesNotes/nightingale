use crate::airway::{Dry, Ventilation, class};
use ng_contract::catalog::{LungGases, Reads};
use ng_contract::*;

type AtLung<'b, 'k> = Kernel<'b, 'k, Respiratory, Beds<Lung>>;
type Plasmatic<'b, Q> = ReadPort<'b, Respiratory, PerSite<Beds<Lung>, Q>>;

/// The blood side of the gas-exchange surface: each gas's partial pressure in the pulmonary bed's plasma, given the
/// plasma's amount of that gas there.
pub trait Plasma<'b> {
    fn oxygen<'k>(
        &self,
        k: &mut AtLung<'b, 'k>,
        plasma: &Plasmatic<'b, Oxygen>,
    ) -> Expr<'b, 'k, Respiratory, Pressure>;
    fn carbon_dioxide<'k>(
        &self,
        k: &mut AtLung<'b, 'k>,
        plasma: &Plasmatic<'b, CarbonDioxide>,
    ) -> Expr<'b, 'k, Respiratory, Pressure>;
}

/// On the body, blood chemistry's speciation states the plasma's partial pressures.
impl<'b> Plasma<'b> for LungGases<Reads<'b, Respiratory>> {
    fn oxygen<'k>(
        &self,
        k: &mut AtLung<'b, 'k>,
        _: &Plasmatic<'b, Oxygen>,
    ) -> Expr<'b, 'k, Respiratory, Pressure> {
        k.at_site(&self.oxygen_partial_pressure)
    }
    fn carbon_dioxide<'k>(
        &self,
        k: &mut AtLung<'b, 'k>,
        _: &Plasmatic<'b, CarbonDioxide>,
    ) -> Expr<'b, 'k, Respiratory, Pressure> {
        k.at_site(&self.carbon_dioxide_partial_pressure)
    }
}

/// Exchange across the gas-exchange surface by Fick's law, `J_i = D_i ((P_B - P_H2O) f_i - P_c,i)`: the alveolar
/// partial pressure is the gas's share of the dry gas times the room's pressure less water vapour's (research 11,
/// West p. 20), and `P_c,i` is the plasma's. Carbon dioxide leaves at a negative rate.
pub fn exchange<'b>(
    laws: &mut FragmentBuilder<'b, Respiratory>,
    ports: &RespiratoryPorts<'b>,
    ventilation: &Ventilation<'b>,
    plasma: &impl Plasma<'b>,
) -> Result<(), AdmissionError> {
    let key = ParameterKey::new;
    let room = laws.parameter::<Pressure>(key("rs.room-pressure"), Admissible::positive());
    let vapour = laws.parameter::<Pressure>(key("rs.water-vapour-pressure"), Admissible::nonnegative());
    let oxygen_diffusing =
        laws.parameter::<Conductance<Oxygen>>(key("rs.oxygen-diffusing"), Admissible::positive());
    let carbon_dioxide_diffusing = laws
        .parameter::<Conductance<CarbonDioxide>>(key("rs.carbon-dioxide-diffusing"), Admissible::positive());
    let oxygen = ports.alveolar::<Oxygen>(laws);
    let carbon_dioxide = ports.alveolar::<CarbonDioxide>(laws);
    let held = (
        oxygen.from_amount::<Oxygen>(),
        oxygen.from_amount::<CarbonDioxide>(),
        oxygen.from_amount::<NitrogenGas>(),
    );
    let (oxygen_plasma, carbon_dioxide_plasma) = (
        oxygen.to_amount::<Oxygen>(),
        carbon_dioxide.to_amount::<CarbonDioxide>(),
    );
    let class = class()?;
    let dry = (&room, &vapour);
    let rate = laws.site_kernel(|mut k| {
        let gas = alveolar::<Oxygen>(&mut k, ventilation, &held, dry);
        let blood = plasma.oxygen(&mut k, &oxygen_plasma);
        let head = k.subtract(gas, blood);
        let d = k.parameter(&oxygen_diffusing);
        let value = k.conductance_flow(d, head);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer_at(&oxygen, rate);
    let rate = laws.site_kernel(|mut k| {
        let gas = alveolar::<CarbonDioxide>(&mut k, ventilation, &held, dry);
        let blood = plasma.carbon_dioxide(&mut k, &carbon_dioxide_plasma);
        let head = k.subtract(gas, blood);
        let d = k.parameter(&carbon_dioxide_diffusing);
        let value = k.conductance_flow(d, head);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer_at(&carbon_dioxide, rate);
    Ok(())
}

type Held<'b> = (
    Plasmatic<'b, Oxygen>,
    Plasmatic<'b, CarbonDioxide>,
    Plasmatic<'b, NitrogenGas>,
);

/// `(P_B - P_H2O) f_i`, one gas's alveolar partial pressure.
fn alveolar<'b, 'k, Q: Dry>(
    k: &mut AtLung<'b, 'k>,
    ventilation: &Ventilation<'b>,
    held: &Held<'b>,
    (room, vapour): (
        &Parameter<'b, Respiratory, Pressure>,
        &Parameter<'b, Respiratory, Pressure>,
    ),
) -> Expr<'b, 'k, Respiratory, Pressure> {
    let lung = (k.at_site(&held.0), k.at_site(&held.1), k.at_site(&held.2));
    let share = ventilation.share::<_, Q>(k, lung);
    let (room, vapour) = (k.parameter(room), k.parameter(vapour));
    let dry = k.subtract(room, vapour);
    k.scale(dry, share)
}
