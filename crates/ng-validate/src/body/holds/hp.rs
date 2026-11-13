use super::FIXED;
use ng_contract::*;

constant_routes! {Hepatic;
    InsulinFirstPass => "hp.insulin-first-pass",
    AlbuminSynthesis => "hp.albumin-synthesis",
    AlbuminLoss => "hp.albumin-loss",
    FixedAcidMade => "hp.fixed-acid-made",
}

const WAVE_NS: u64 = 10_000_000;

fn sinusoids<'b>(
    laws: &mut FragmentBuilder<'b, Hepatic>,
    ports: &HepaticPorts<'b>,
) -> Result<(), AdmissionError> {
    let route = ports.sinusoids(laws);
    let pressure = ports.blood_pressure();
    let resistance = laws.edge_parameter::<_, Resistance>(
        ParameterKey::new("hp.sinusoid-resistance"),
        Admissible::positive(),
    );
    let class = RateClass::new(RateBand::Wave, WAVE_NS)?;
    let flow = laws.edge_kernel(|mut k| {
        let (up, down) = (k.upstream(&pressure), k.downstream(&pressure));
        let across = k.subtract(up, down);
        let over = k.edge_parameter(&resistance);
        let quotient = k.divide(across, over);
        let flow = k.as_dimension(quotient);
        k.finish(flow, laws.every_mode(), class)
    });
    laws.transfer_along(&route, flow);
    Ok(())
}

pub(super) fn hepatic(body: &mut Assembly<'_>) -> Result<(), AdmissionError> {
    let declaration = HeldDeclaration::<Hepatic>::new(CONSTANT.to_vec(), Vec::new(), FIXED)
        .pool::<Glucose, Beds<Liver>>(Space::Cells)
        .pool::<Lactate, Beds<Liver>>(Space::Cells)
        .pool::<FattyAcids, Beds<Liver>>(Space::Cells)
        .pool::<Glycerol, Beds<Liver>>(Space::Cells)
        .pool::<AminoAcids, Beds<Liver>>(Space::Cells)
        .pool::<Urea, Beds<Liver>>(Space::Cells)
        .pool::<Glycogen, Beds<Liver>>(Space::Cells)
        .pool::<Ketones, Beds<Liver>>(Space::Cells)
        .pool::<Ketones, Besides<Liver>>(Space::Cells)
        .pool::<Glycerol, CapillaryBed>(Space::Interstitium)
        .pool::<Ketones, CapillaryBed>(Space::Interstitium)
        .pool::<Urea, CapillaryBed>(Space::Interstitium);
    let mut held = body.hold(declaration)?;
    let (laws, ports) = held.stand_ins();
    constants(laws, ports)?;
    let (mut laws, ports) = body.hepatic(Some(held))?.parts();
    sinusoids(&mut laws, &ports)?;
    body.add(laws.finish());
    Ok(())
}
