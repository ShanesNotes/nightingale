use ng_contract::catalog::{Defines, RespiratoryDrive};
use ng_contract::*;

type Bound<'b, U> = Parameter<'b, Nervous, U>;
type Memory<'b> = StateSlot<'b, Nervous, Dimensionless>;

/// One breath is a pass of the phase through inspiration's length, while the pump drive rises toward its
/// peak, and then expiration's, while it falls. Two positive lengths leave every breath room to end.
/// Admission solves one heartbeat, and a phase that ran free could not return to itself inside one, so the
/// body admits in `rest`, where the phase rests at zero, the drive at the breath's mean and the mixing share
/// at the rest share, and a hazard starts the first breath.
/// The mixing share rides line 86: the respiratory card's mixing exchange carries the breath's mean ventilation while
/// the breath rests, and only the breath's modes say when it starts.
pub(crate) fn breathe<'b>(laws: &mut FragmentBuilder<'b, Nervous>) -> Result<Memory<'b>, AdmissionError> {
    let class = RateClass::new(RateBand::Beat, 1_000_000_000)?;
    let Breath { drive, share } = Breath::declare(laws, class)?;
    let line: RespiratoryDrive<Defines<'b, Nervous>> = laws.emit();
    let mixing = laws.kernel(|mut k| {
        let value = k.state(&share);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define(&line.mixing_share, mixing);
    Ok(drive)
}

struct Breath<'b> {
    drive: Memory<'b>,
    share: Memory<'b>,
}

impl<'b> Breath<'b> {
    fn declare(laws: &mut FragmentBuilder<'b, Nervous>, class: RateClass) -> Result<Self, AdmissionError> {
        let key = ParameterKey::new;
        let [rest, inspiration, expiration] = laws.modes("breath", ["rest", "inspiration", "expiration"]);
        let phase = laws.state::<Dimensionless>("breath phase", Admissible::nonnegative());
        let drive = laws.state::<Dimensionless>("pump drive", Admissible::nonnegative());
        let share = laws.state::<Dimensionless>("mixing share", Admissible::unit_interval());
        let hazard = laws.state::<Dimensionless>("breath start", Admissible::nonnegative());
        let speed = |laws: &mut FragmentBuilder<'b, Nervous>, name| -> Bound<'b, Rate<Dimensionless>> {
            laws.parameter(key(name), Admissible::positive())
        };
        let rate = speed(laws, "ns.breath-rate");
        let rise = speed(laws, "ns.drive-rise");
        let fall = speed(laws, "ns.drive-fall");
        let settle = speed(laws, "ns.rest-speed");
        let mixing_fall = speed(laws, "ns.share-fall");
        let start = laws.parameter::<Rate<Dimensionless>>(key("ns.breath-start"), Admissible::nonnegative());
        let inspiration_length =
            laws.parameter::<Dimensionless>(key("ns.inspiration-length"), Admissible::positive());
        let expiration_length =
            laws.parameter::<Dimensionless>(key("ns.expiration-length"), Admissible::positive());
        let peak = laws.parameter::<Dimensionless>(key("ns.drive-peak"), Admissible::positive());
        let rest_drive = laws.parameter::<Dimensionless>(key("ns.rest-drive"), Admissible::nonnegative());
        let rest_share = laws.parameter::<Dimensionless>(key("ns.rest-share"), Admissible::unit_interval());
        let dice = laws.stream("breath dice");

        let advance = laws.kernel(|mut k| {
            let zero = k.zero();
            let value = toward(&mut k, zero, &phase, &settle);
            k.finish(value, rest.only(), class)
        });
        laws.memory_rate(&phase, advance);
        let held = laws.kernel(|mut k| {
            let target = k.parameter(&rest_drive);
            let value = toward(&mut k, target, &drive, &settle);
            k.finish(value, rest.only(), class)
        });
        laws.memory_rate(&drive, held);
        let mixed = laws.kernel(|mut k| {
            let target = k.parameter(&rest_share);
            let value = toward(&mut k, target, &share, &settle);
            k.finish(value, rest.only(), class)
        });
        laws.memory_rate(&share, mixed);
        let accrue = laws.kernel(|mut k| {
            let value = k.parameter(&start);
            k.finish(value, rest.only(), class)
        });
        laws.memory_rate(&hazard, accrue);

        let rising = laws.kernel(|mut k| {
            let (target, now, speed) = (k.parameter(&peak), k.state(&drive), k.parameter(&rise));
            let value = k.relax(target, now, speed);
            k.finish(value, inspiration.only(), class)
        });
        laws.memory_rate(&drive, rising);
        let falling = laws.kernel(|mut k| {
            let (target, now, speed) = (k.zero(), k.state(&drive), k.parameter(&fall));
            let value = k.relax(target, now, speed);
            k.finish(value, expiration.only(), class)
        });
        laws.memory_rate(&drive, falling);
        for mode in [&inspiration, &expiration] {
            let advance = laws.kernel(|mut k| {
                let value = k.parameter(&rate);
                k.finish(value, mode.only(), class)
            });
            laws.memory_rate(&phase, advance);
            let unmixed = laws.kernel(|mut k| {
                let (target, now, speed) = (k.zero(), k.state(&share), k.parameter(&mixing_fall));
                let value = k.relax(target, now, speed);
                k.finish(value, mode.only(), class)
            });
            laws.memory_rate(&share, unmixed);
            let spent = laws.kernel(|mut k| {
                let value = k.zero();
                k.finish(value, mode.only(), class)
            });
            laws.memory_rate(&hazard, spent);
        }

        let past_inspiration = laws.kernel(|mut k| {
            let (now, end) = (k.state(&phase), k.parameter(&inspiration_length));
            let gap = k.subtract(now, end);
            k.finish(gap, inspiration.only(), class)
        });
        laws.event(Guard::new(
            past_inspiration,
            Crossing::Rising,
            inspiration.to(&expiration)?,
        ));
        let past_breath = laws.kernel(|mut k| {
            let (now, first, second) = (
                k.state(&phase),
                k.parameter(&inspiration_length),
                k.parameter(&expiration_length),
            );
            let end = k.add(first, second);
            let gap = k.subtract(now, end);
            k.finish(gap, expiration.only(), class)
        });
        laws.event(Guard::new(past_breath, Crossing::Rising, expiration.to(&inspiration)?).resetting(&phase));
        laws.hazard(HazardGuard::new(&hazard, &dice, rest.to(&inspiration)?));
        Ok(Self { drive, share })
    }
}

fn toward<'b, 'k>(
    k: &mut Kernel<'b, 'k, Nervous>,
    target: Expr<'b, 'k, Nervous, Dimensionless>,
    slot: &Memory<'b>,
    speed: &Bound<'b, Rate<Dimensionless>>,
) -> Expr<'b, 'k, Nervous, Rate<Dimensionless>> {
    let (now, speed) = (k.state(slot), k.parameter(speed));
    k.relax(target, now, speed)
}
