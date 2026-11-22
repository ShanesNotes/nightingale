use ng_contract::*;

const FIXED: &str = "body fixture: synthetic stand-ins until each card's ticket";

fn hours() -> Result<RateClass, AdmissionError> {
    RateClass::new(RateBand::Hours, 3_600_000_000_000)
}

macro_rules! constant_routes {
    ($card:ty; $($process:ty => $key:literal),+ $(,)?) => {
        const CONSTANT: &[&str] = &[$(<$process as BoundaryProcess>::NAME),+];
        fn constants<'b>(
            laws: &mut FragmentBuilder<'b, $card>,
            ports: &HeldPorts<'b, $card>,
        ) -> Result<(), AdmissionError> {
            $(super::constant::<$card, $process>(laws, ports, $key)?;)+
            Ok(())
        }
    };
}
mod bc;
mod en;
mod gi;
mod hc;
mod hp;
mod im;
mod ms;
mod mt;
mod rs;

fn constant<'b, S: OutsideSlice, P: BoundaryProcess>(
    laws: &mut FragmentBuilder<'b, S>,
    ports: &HeldPorts<'b, S>,
    key: &'static str,
) -> Result<(), AdmissionError> {
    let route = ports.route::<P::Quantity>(P::NAME)?.route;
    let rate = laws.parameter::<Rate<P::Quantity>>(ParameterKey::new(key), Admissible::nonnegative());
    let class = hours()?;
    let kernel = laws.kernel(|mut k| {
        let value = k.parameter(&rate);
        k.finish(value, laws.every_mode(), class)
    });
    laws.transfer(Transfer::new(&route, kernel));
    Ok(())
}

fn enclosure_law<'b, S: OutsideSlice>(
    laws: &mut FragmentBuilder<'b, S>,
    ports: &HeldPorts<'b, S>,
    key: ParameterKey,
) -> Result<(), AdmissionError> {
    let held = laws.parameter::<Pressure>(key, Admissible::finite());
    let class = hours()?;
    let pressure = laws.site_kernel(|mut k| {
        let value = k.parameter(&held);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define_at(&ports.enclosure_pressure, pressure);
    Ok(())
}

pub(super) fn hold_every_card(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    rs::respiratory(body)?;
    gi::gastrointestinal(body)?;
    en::endocrine(body)?;
    ms::musculoskeletal(body)?;
    bc::blood_chemistry(body)?;
    mt::metabolism(body)?;
    hp::hepatic(body)?;
    im::immune(body)?;
    hc::hematology(body)
}
