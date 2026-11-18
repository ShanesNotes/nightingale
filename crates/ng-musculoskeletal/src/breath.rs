use ng_contract::catalog::{Defines, PumpForce};
use ng_contract::*;

type Bound<'b, U> = Parameter<'b, Musculoskeletal, U>;
type Memory<'b> = StateSlot<'b, Musculoskeletal, Dimensionless>;

/// One breath is a pass of the phase through inspiration's length, while the pump drive rises toward its
/// peak, and then expiration's, while it falls. Two positive lengths leave every breath room to end.
/// Admission solves one heartbeat, and a phase that ran free could not return to itself inside one, so the
/// body admits in `rest`, where the phase rests at zero, the drive at the breath's mean and the mixing share
/// at the rest share, and a hazard starts the first breath. The pump force on line 95 is `P_mus = PImax a` for the
/// pump drive `a` (research 11's pump row). The mixing share rides line 95 too: the respiratory card's mixing exchange
/// carries the breath's mean ventilation while the breath rests, and only the breath's modes say when it starts.
pub fn breathe<'b>(laws: &mut FragmentBuilder<'b, Musculoskeletal>) -> Result<(), AdmissionError> {
    let class = RateClass::new(RateBand::Beat, 1_000_000_000)?;
    let Breath { drive, share } = Breath::declare(laws, class)?;
    let pump = laws.parameter::<Pressure>(ParameterKey::new("ms.pump-pressure"), Admissible::nonnegative());
    let line: PumpForce<Defines<'b, Musculoskeletal>> = laws.emit();
    let force = laws.kernel(|mut k| {
        let (pump, drive) = (k.parameter(&pump), k.state(&drive));
        let value = k.scale(pump, drive);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define(&line.pump_force, force);
    let mixing = laws.kernel(|mut k| {
        let value = k.state(&share);
        k.finish(value, laws.every_mode(), class)
    });
    laws.define(&line.mixing_share, mixing);
    Ok(())
}

struct Breath<'b> {
    drive: Memory<'b>,
    share: Memory<'b>,
}

impl<'b> Breath<'b> {
    fn declare(
        laws: &mut FragmentBuilder<'b, Musculoskeletal>,
        class: RateClass,
    ) -> Result<Self, AdmissionError> {
        let key = ParameterKey::new;
        let [rest, inspiration, expiration] = laws.modes("breath", ["rest", "inspiration", "expiration"]);
        let phase = laws.state::<Dimensionless>("breath phase", Admissible::nonnegative());
        let drive = laws.state::<Dimensionless>("pump drive", Admissible::nonnegative());
        let share = laws.state::<Dimensionless>("mixing share", Admissible::unit_interval());
        let hazard = laws.state::<Dimensionless>("breath start", Admissible::nonnegative());
        let speed =
            |laws: &mut FragmentBuilder<'b, Musculoskeletal>, name| -> Bound<'b, Rate<Dimensionless>> {
                laws.parameter(key(name), Admissible::positive())
            };
        let rate = speed(laws, "ms.breath-rate");
        let rise = speed(laws, "ms.drive-rise");
        let fall = speed(laws, "ms.drive-fall");
        let settle = speed(laws, "ms.rest-speed");
        let mixing_fall = speed(laws, "ms.share-fall");
        let start = laws.parameter::<Rate<Dimensionless>>(key("ms.breath-start"), Admissible::nonnegative());
        let inspiration_length =
            laws.parameter::<Dimensionless>(key("ms.inspiration-length"), Admissible::positive());
        let expiration_length =
            laws.parameter::<Dimensionless>(key("ms.expiration-length"), Admissible::positive());
        let peak = laws.parameter::<Dimensionless>(key("ms.drive-peak"), Admissible::positive());
        let rest_drive = laws.parameter::<Dimensionless>(key("ms.rest-drive"), Admissible::nonnegative());
        let rest_share = laws.parameter::<Dimensionless>(key("ms.rest-share"), Admissible::unit_interval());
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
    k: &mut Kernel<'b, 'k, Musculoskeletal>,
    target: Expr<'b, 'k, Musculoskeletal, Dimensionless>,
    slot: &Memory<'b>,
    speed: &Bound<'b, Rate<Dimensionless>>,
) -> Expr<'b, 'k, Musculoskeletal, Rate<Dimensionless>> {
    let (now, speed) = (k.state(slot), k.parameter(speed));
    k.relax(target, now, speed)
}
