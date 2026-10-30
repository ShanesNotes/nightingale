//! The arithmetic profile's own transcendental routines: correctly rounded add, subtract, multiply and
//! divide, exact scaling by powers of two, no `mul_add` and no host libm. A changed bit is a new
//! `ArithmeticProfile`, never a silent change.

const LN2_HI: f64 = f64::from_bits(0x3FE6_2E42_FEE0_0000);
const LN2_LO: f64 = f64::from_bits(0x3DEA_39EF_3579_3C76);
const INV_LN2: f64 = f64::from_bits(0x3FF7_1547_652B_82FE);
const SQRT_2: f64 = f64::from_bits(0x3FF6_A09E_667F_3BCD);
const TWO_54: f64 = f64::from_bits(0x4350_0000_0000_0000);
const EXP_OVERFLOW: f64 = f64::from_bits(0x4086_2E42_FEFA_39EF);
const EXP_UNDERFLOW: f64 = f64::from_bits(0xC087_4910_D52D_3051);
const MANTISSA: u64 = 0x000F_FFFF_FFFF_FFFF;
const ONE_BITS: u64 = 0x3FF0_0000_0000_0000;

/// `1 / n!` from `n = 13` down to `n = 3`.
const EXP_SERIES: [f64; 11] = [
    1.0 / 6_227_020_800.0,
    1.0 / 479_001_600.0,
    1.0 / 39_916_800.0,
    1.0 / 3_628_800.0,
    1.0 / 362_880.0,
    1.0 / 40_320.0,
    1.0 / 5_040.0,
    1.0 / 720.0,
    1.0 / 120.0,
    1.0 / 24.0,
    1.0 / 6.0,
];
/// `1 / n` for odd `n` from 23 down to 3.
const ATANH_SERIES: [f64; 11] = [
    1.0 / 23.0,
    1.0 / 21.0,
    1.0 / 19.0,
    1.0 / 17.0,
    1.0 / 15.0,
    1.0 / 13.0,
    1.0 / 11.0,
    1.0 / 9.0,
    1.0 / 7.0,
    1.0 / 5.0,
    1.0 / 3.0,
];

fn horner(series: &[f64], x: f64) -> f64 {
    series.iter().fold(0.0, |sum, &term| sum * x + term)
}

fn power_of_two(exponent: i32) -> f64 {
    f64::from_bits(((exponent + 1023) as u64) << 52)
}

/// `e^x` within one unit in the last place: Cody-Waite reduction by a split `ln 2`, then a degree 13
/// Taylor polynomial. `exp(0)` is exactly 1, the result is `+inf` past the overflow threshold and `+0`
/// past the underflow threshold, and `NaN` returns `NaN`.
pub fn exp(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x > EXP_OVERFLOW {
        return f64::INFINITY;
    }
    if x < EXP_UNDERFLOW {
        return 0.0;
    }
    let k = (x * INV_LN2).round();
    let high = x - k * LN2_HI;
    let low = k * LN2_LO;
    let r = high - low;
    let tail = r * r * (0.5 + r * horner(&EXP_SERIES, r));
    let y = 1.0 + (high + (tail - low));
    let k = k as i32;
    let half = k / 2;
    // The first factor is exact, so a subnormal result rounds once, at the second.
    y * power_of_two(half) * power_of_two(k - half)
}

/// `ln x` within one unit in the last place: the atanh series to `z^11` on a mantissa in
/// `[sqrt(1/2), sqrt(2))`. `ln(1)` is exactly +0. `None` for an argument that is not positive, which is
/// the operator's own refusal source.
pub(crate) fn ln(x: f64) -> Option<f64> {
    if x.is_nan() || x <= 0.0 {
        return None;
    }
    if x.is_infinite() {
        return Some(x);
    }
    let (x, shift) = if x < f64::MIN_POSITIVE {
        (x * TWO_54, -54)
    } else {
        (x, 0)
    };
    let bits = x.to_bits();
    let mut exponent = ((bits >> 52) & 0x7FF) as i32 - 1023 + shift;
    let mut mantissa = f64::from_bits((bits & MANTISSA) | ONE_BITS);
    if mantissa >= SQRT_2 {
        mantissa *= 0.5;
        exponent += 1;
    }
    let f = mantissa - 1.0;
    let s = f / (2.0 + f);
    let z = s * s;
    let half_square = 0.5 * f * f;
    let tail = s * (half_square + 2.0 * (z * horner(&ATANH_SERIES, z)));
    let k = f64::from(exponent);
    Some(k * LN2_HI - ((half_square - (tail + k * LN2_LO)) - f))
}

#[cfg(test)]
mod tests {
    use super::{exp, ln};

    fn ulps(a: f64, b: f64) -> u64 {
        a.to_bits().abs_diff(b.to_bits())
    }

    // Correctly rounded values from Python's 80-digit `decimal`, never from the code under test.
    const EXP_VECTORS: [(u64, u64); 14] = [
        (0x3FF0000000000000, 0x4005BF0A8B145769),
        (0xBFF0000000000000, 0x3FD78B56362CEF38),
        (0x3FE0000000000000, 0x3FFA61298E1E069C),
        (0x3FD0000000000000, 0x3FF48B5E3C3E8186),
        (0x4000000000000000, 0x401D8E64B8D4DDAE),
        (0x4024000000000000, 0x40D5829DCF950560),
        (0xC024000000000000, 0x3F07CD79B5647C9B),
        (0x4059200000000000, 0x48FFCC37A76F9E76),
        (0x4085E00000000000, 0x7F0D945DF4F8EC8E),
        (0xC085E00000000000, 0x00D14F2B0FB9307F),
        (0x3EE4F8B588E368F1, 0x3FF0000A7C5E340E),
        (0xC087480000000000, 0x0000000000000001),
        (0xC087200000000000, 0x0000000000000055),
        (0x4086280000000000, 0x7FDD422D2BE5DC9B),
    ];
    const LN_VECTORS: [(u64, u64); 11] = [
        (0x4000000000000000, 0x3FE62E42FEFA39EF),
        (0x4024000000000000, 0x40026BB1BBB55516),
        (0x3FE0000000000000, 0xBFE62E42FEFA39EF),
        (0x4008000000000000, 0x3FF193EA7AAD030B),
        (0x3FF8000000000000, 0x3FD9F323ECBF984C),
        (0x3FE8000000000000, 0xBFD269621134DB92),
        (0x01A56E1FC2F8F359, 0xC085963447F87FB5),
        (0x7E37E43C8800759C, 0x4085963447F87FB5),
        (0x0000000000000001, 0xC0874385446D71C3),
        (0x0010000000000000, 0xC086232BDD7ABCD2),
        (0x3FF000000006DF38, 0x3DDB7CDFFFFA18D8),
    ];

    #[test]
    fn exp_is_within_one_ulp_of_the_correctly_rounded_value() {
        for (x, want) in EXP_VECTORS {
            let got = exp(f64::from_bits(x));
            assert!(
                ulps(got, f64::from_bits(want)) <= 1,
                "exp({x:#x}) = {:#x}",
                got.to_bits()
            );
        }
    }

    #[test]
    fn ln_is_within_one_ulp_of_the_correctly_rounded_value() {
        for (x, want) in LN_VECTORS {
            let got = ln(f64::from_bits(x)).unwrap_or(f64::NAN);
            assert!(
                ulps(got, f64::from_bits(want)) <= 1,
                "ln({x:#x}) = {:#x}",
                got.to_bits()
            );
        }
    }

    #[test]
    fn exact_points_are_exact() {
        assert_eq!(exp(0.0).to_bits(), 1.0_f64.to_bits());
        assert_eq!(exp(-0.0).to_bits(), 1.0_f64.to_bits());
        assert_eq!(ln(1.0).map(f64::to_bits), Some(0.0_f64.to_bits()));
    }

    #[test]
    fn exp_saturates_past_its_thresholds() {
        assert_eq!(exp(709.79), f64::INFINITY);
        assert_eq!(exp(f64::INFINITY), f64::INFINITY);
        assert_eq!(exp(709.78).classify(), core::num::FpCategory::Normal);
        assert_eq!(exp(-745.14).to_bits(), 0.0_f64.to_bits());
        assert_eq!(exp(f64::NEG_INFINITY).to_bits(), 0.0_f64.to_bits());
        assert!(exp(-745.0) > 0.0);
        assert!(exp(f64::NAN).is_nan());
    }

    #[test]
    fn ln_refuses_what_is_not_positive() {
        assert_eq!(ln(0.0), None);
        assert_eq!(ln(-0.0), None);
        assert_eq!(ln(-1.0), None);
        assert_eq!(ln(f64::NEG_INFINITY), None);
        assert_eq!(ln(f64::NAN), None);
        assert_eq!(ln(f64::INFINITY), Some(f64::INFINITY));
    }
}
