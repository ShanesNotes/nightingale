use ng_contract::*;

type Bound<'b, U> = Parameter<'b, Respiratory, U>;
type Memory<'b> = StateSlot<'b, Respiratory, Dimensionless>;

/// One breath is a pass of the phase from zero to the breath's end: inspiration while the pump drive rises toward its
/// peak, expiration while it falls. Admission solves one heartbeat, and a phase that ran free could not return to
/// itself inside one, so the body admits in `rest`, where the phase rests at zero, the drive at the breath's mean and
/// the mixing share at the rest share, and a hazard starts the first breath.
pub(crate) struct Breath<'b> {
    pub(crate) drive: Memory<'b>,
    pub(crate) share: Memory<'b>,
}

impl<'b> Breath<'b> {
    pub(crate) fn declare(
        laws: &mut FragmentBuilder<'b, Respiratory>,
        class: RateClass,
    ) -> Result<Self, AdmissionError> {
        let key = ParameterKey::new;
        let [rest, inspiration, expiration] = laws.modes("breath", ["rest", "inspiration", "expiration"]);
        let phase = laws.state::<Dimensionless>("breath phase", Admissible::finite());
        let drive = laws.state::<Dimensionless>("pump drive", Admissible::finite());
        let share = laws.state::<Dimensionless>("mixing share", Admissible::finite());
        let hazard = laws.state::<Dimensionless>("breath start", Admissible::nonnegative());
        let speed = |laws: &mut FragmentBuilder<'b, Respiratory>, name| -> Bound<'b, Rate<Dimensionless>> {
            laws.parameter(key(name), Admissible::positive())
        };
        let rate = speed(laws, "rs.breath-rate");
        let rise = speed(laws, "rs.drive-rise");
        let fall = speed(laws, "rs.drive-fall");
        let settle = speed(laws, "rs.rest-speed");
        let mixing_fall = speed(laws, "rs.share-fall");
        let start = laws.parameter::<Rate<Dimensionless>>(key("rs.breath-start"), Admissible::nonnegative());
        let inspiration_end =
            laws.parameter::<Dimensionless>(key("rs.inspiration-end"), Admissible::positive());
        let breath_end = laws.parameter::<Dimensionless>(key("rs.breath-end"), Admissible::positive());
        let peak = laws.parameter::<Dimensionless>(key("rs.drive-peak"), Admissible::positive());
        let rest_drive = laws.parameter::<Dimensionless>(key("rs.rest-drive"), Admissible::nonnegative());
        let rest_share = laws.parameter::<Dimensionless>(key("rs.rest-share"), Admissible::nonnegative());
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
            let (now, end) = (k.state(&phase), k.parameter(&inspiration_end));
            let gap = k.subtract(now, end);
            k.finish(gap, inspiration.only(), class)
        });
        laws.event(Guard::new(
            past_inspiration,
            Crossing::Rising,
            inspiration.to(&expiration)?,
        ));
        let past_breath = laws.kernel(|mut k| {
            let (now, end) = (k.state(&phase), k.parameter(&breath_end));
            let gap = k.subtract(now, end);
            k.finish(gap, expiration.only(), class)
        });
        laws.event(Guard::new(past_breath, Crossing::Rising, expiration.to(&inspiration)?).resetting(&phase));
        laws.hazard(HazardGuard::new(&hazard, &dice, rest.to(&inspiration)?));
        Ok(Self { drive, share })
    }
}

fn toward<'b, 'k>(
    k: &mut Kernel<'b, 'k, Respiratory>,
    target: Expr<'b, 'k, Respiratory, Dimensionless>,
    slot: &Memory<'b>,
    speed: &Bound<'b, Rate<Dimensionless>>,
) -> Expr<'b, 'k, Respiratory, Rate<Dimensionless>> {
    let (now, speed) = (k.state(slot), k.parameter(speed));
    k.relax(target, now, speed)
}
