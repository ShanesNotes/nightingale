//! `expm1` in a law: its value and slope where `exp(x) - 1` loses its digits, and a refusal by name where it
//! overflows.

use ng_contract::*;

fn at() -> Result<Position, AdmissionError> {
    Ok(Position::new(
        Measure::new(0.0)?,
        Measure::new(0.0)?,
        Measure::new(0.0)?,
    ))
}

const VALUE: LawName = LawName {
    owner: "NS",
    local: "value",
};

#[derive(Clone, Copy)]
enum Argument {
    Row,
    Power,
}

fn expm1_law(argument: Argument) -> Result<CompiledBody, AdmissionError> {
    let mut map = AnatomyBuilder::new();
    map.site::<TissueSite>("body", at()?);
    assemble(Schema::new(map.finish()?)?, |mut body| {
        let class = RateClass::new(RateBand::Reflex, 1_000_000_000)?;
        let (mut laws, _ports) = body.nervous()?.parts();
        let value = laws.algebraic::<Dimensionless>(VALUE.local, Admissible::finite());
        let residual = match argument {
            Argument::Row => {
                let row = laws.algebraic::<Dimensionless>("argument", Admissible::finite());
                let read =
                    laws.parameter::<Dimensionless>(ParameterKey::new("ns.argument"), Admissible::finite());
                let held = laws.kernel(|mut k| {
                    let (own, read) = (k.algebraic(&row), k.parameter(&read));
                    let gap = k.subtract(own, read);
                    k.finish(gap, laws.every_mode(), class)
                });
                laws.residual(&row, held);
                laws.kernel(|mut k| {
                    let (own, argument) = (k.algebraic(&value), k.algebraic(&row));
                    let made = k.expm1(argument);
                    let gap = k.subtract(own, made);
                    k.finish(gap, laws.every_mode(), class)
                })
            }
            Argument::Power => {
                let base =
                    laws.parameter::<Dimensionless>(ParameterKey::new("ns.base"), Admissible::positive());
                let exponent =
                    laws.parameter::<Dimensionless>(ParameterKey::new("ns.exponent"), Admissible::finite());
                laws.kernel(|mut k| {
                    let (own, base, exponent) =
                        (k.algebraic(&value), k.parameter(&base), k.parameter(&exponent));
                    let log = k.ln(base);
                    let argument = k.scale(log, exponent);
                    let made = k.expm1(argument);
                    let gap = k.subtract(own, made);
                    k.finish(gap, laws.every_mode(), class)
                })
            }
        };
        laws.residual(&value, residual);
        body.add(laws.finish());
        body.compile()
    })
}

struct Evaluated {
    value: f64,
    slope: Option<f64>,
}

fn evaluated(
    argument: Argument,
    parameters: &[(&'static str, f64)],
) -> Result<Evaluated, InitializationFailure> {
    let body = expm1_law(argument)?;
    let mut packet = ParameterPacket::new();
    for &(key, value) in parameters {
        let value = Measure::<Dimensionless>::new(value)?;
        packet.synthetic(ParameterKey::new(key), Binding::Body, value, "synthetic")?;
    }
    let read = parameters.first().map_or(0.0, |&(_, value)| value);
    let start = |row: RowKind| match row {
        RowKind::Algebraic(Slot { law, .. }) if law.local == "argument" => read,
        _ => 0.0,
    };
    packet.start_rest(&body, start, "synthetic")?;
    let port = BoundaryPort::new(Vec::new())?;
    let request = InitializationRequest::at(SimTime::from_ns(0));
    let system = body.initialization(body.bind(packet)?, port, request)?;
    let workspace = system.workspace();
    let slot = |local: &'static str| {
        let law = LawName { owner: "NS", local };
        let row = RowKind::Algebraic(Slot {
            law,
            at: Context::Body,
        });
        let found = (body.rows().iter()).find(|known| known.kind() == row);
        workspace.endpoint(0).start + found.expect("a body row").id().index()
    };
    let at = slot(VALUE.local);
    let value = 0.0 - system.evaluate(&workspace)?[at];
    let slope = match argument {
        Argument::Row => {
            let column = slot("argument");
            let jacobian = system.linearize(&workspace)?;
            let view = jacobian.view();
            let entries = view.row_offsets()[at]..view.row_offsets()[at + 1];
            let entry = entries
                .into_iter()
                .find(|&entry| view.columns()[entry].index() == column);
            entry.map(|entry| 0.0 - view.values()[entry])
        }
        Argument::Power => None,
    };
    Ok(Evaluated { value, slope })
}

fn ulps(a: f64, b: f64) -> u64 {
    a.to_bits().abs_diff(b.to_bits())
}

/// `e^a` correctly rounded, from Python's 80-digit `decimal`, never from the code under test.
const SLOPES: [(f64, u64); 4] = [
    (0.5, 0x3FFA61298E1E069C),
    (-1.0, 0x3FD78B56362CEF38),
    (1.0, 0x4005BF0A8B145769),
    (1e-10, 0x3FF000000006DF38),
];

#[test]
fn a_laws_slope_of_expm1_is_e_to_its_argument() -> Result<(), InitializationFailure> {
    for (a, want) in SLOPES {
        let found = evaluated(Argument::Row, &[("ns.argument", a)])?;
        let slope = found.slope.expect("an entry for the argument");
        assert!(ulps(slope, f64::from_bits(want)) <= 1, "{a}: {slope:e}");
    }
    Ok(())
}

#[test]
fn expm1_past_its_overflow_is_refused_by_the_law_that_takes_it() -> Result<(), InitializationFailure> {
    let found = evaluated(Argument::Row, &[("ns.argument", 709.7)])?;
    assert!(found.value.is_finite());
    assert_eq!(
        evaluated(Argument::Row, &[("ns.argument", 710.0)]).err(),
        Some(InitializationFailure::Undefined { law: VALUE })
    );
    Ok(())
}

/// `x^C - 1` for the double `x = 0.55` and each double `C`, correctly rounded from Python's `decimal` at 1000
/// digits, beside the error the law may carry in units in the last place. `ln` is within one unit and the product
/// within half of one, and `expm1` scales a relative error in its argument by `a e^a / (e^a - 1)`, about 1 here and 233
/// at `C = -390`, before its own unit.
const POWERS: [(f64, u64, u64); 5] = [
    (1e-3, 0xBF4395857EA359F1, 3),
    (1e-6, 0xBEA40F611320918F, 3),
    (1e-15, 0xBCC58A12B5113553, 3),
    (1e-17, 0xBC5B920377204444, 3),
    (-390.0, 0x54F4BACC1DF13F87, 351),
];

#[test]
fn expm1_of_c_ln_x_keeps_the_digits_x_to_the_c_minus_one_loses() -> Result<(), InitializationFailure> {
    let mut seen = Vec::new();
    for (exponent, want, bound) in POWERS {
        let found = evaluated(Argument::Power, &[("ns.base", 0.55), ("ns.exponent", exponent)])?;
        let off = ulps(found.value, f64::from_bits(want));
        seen.push((exponent, off, bound));
    }
    assert!(seen.iter().all(|&(_, off, bound)| off <= bound), "{seen:?}");
    Ok(())
}
