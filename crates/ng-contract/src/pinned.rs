use core::f64::consts::FRAC_PI_4;
use core::num::NonZeroU64;

const LN2_HI: f64 = f64::from_bits(0x3FE6_2E42_FEE0_0000);
const LN2_LO: f64 = f64::from_bits(0x3DEA_39EF_3579_3C76);
const INV_LN2: f64 = f64::from_bits(0x3FF7_1547_652B_82FE);
const SQRT_2: f64 = f64::from_bits(0x3FF6_A09E_667F_3BCD);
const TWO_54: f64 = f64::from_bits(0x4350_0000_0000_0000);
const EXP_OVERFLOW: f64 = f64::from_bits(0x4086_2E42_FEFA_39EF);
const EXP_UNDERFLOW: f64 = f64::from_bits(0xC087_4910_D52D_3051);
const EXPM1_FLOOR: f64 = f64::from_bits(0xC043_687A_9F1A_F2B1);
const TWO_NEG_54: f64 = f64::from_bits(0x3C90_0000_0000_0000);
const MANTISSA: u64 = 0x000F_FFFF_FFFF_FFFF;
const ONE_BITS: u64 = 0x3FF0_0000_0000_0000;
/// `0.5 - 2^-54`, so adding it before truncation matches `f64::round` on small arguments without calling libm.
const HALF_DOWN: f64 = f64::from_bits(0x3FDF_FFFF_FFFF_FFFF);

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

const COS_SERIES: [f64; 10] = [
    -1.0 / 6_402_373_705_728_000.0,
    1.0 / 20_922_789_888_000.0,
    -1.0 / 87_178_291_200.0,
    1.0 / 479_001_600.0,
    -1.0 / 3_628_800.0,
    1.0 / 40_320.0,
    -1.0 / 720.0,
    1.0 / 24.0,
    -1.0 / 2.0,
    1.0,
];
const SIN_SERIES: [f64; 10] = [
    -1.0 / 121_645_100_408_832_000.0,
    1.0 / 355_687_428_096_000.0,
    -1.0 / 1_307_674_368_000.0,
    1.0 / 6_227_020_800.0,
    -1.0 / 39_916_800.0,
    1.0 / 362_880.0,
    -1.0 / 5_040.0,
    1.0 / 120.0,
    -1.0 / 6.0,
    1.0,
];

#[inline(always)]
fn horner<const N: usize>(series: &[f64; N], x: f64) -> f64 {
    series[1..].iter().fold(series[0], |sum, &term| sum * x + term)
}

#[inline(always)]
fn round_small(y: f64) -> i32 {
    (if y >= 0.0 { y + HALF_DOWN } else { y - HALF_DOWN }) as i32
}

#[inline(always)]
fn power_of_two(exponent: i32) -> f64 {
    f64::from_bits(((exponent + 1023) as u64) << 52)
}

/// `e^x` within one unit in the last place: Cody-Waite reduction by a split `ln 2`, then a degree 13
/// Taylor polynomial. `exp(0)` is exactly 1, the result is `+inf` past the overflow threshold and `+0`
/// past the underflow threshold, and `NaN` returns `NaN`.
#[inline]
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
    let k_int = round_small(x * INV_LN2);
    let k = f64::from(k_int);
    let high = x - k * LN2_HI;
    let low = k * LN2_LO;
    let r = high - low;
    let tail = r * r * (0.5 + r * horner(&EXP_SERIES, r));
    let y = 1.0 + (high + (tail - low));
    let half = k_int / 2;
    // The first factor is exact, so a subnormal result rounds once, at the second.
    y * power_of_two(half) * power_of_two(k_int - half)
}

/// `e^x - 1` within one unit in the last place, by `exp`'s reduction and polynomial with the reduction's rounding
/// carried, and fdlibm's rebuild by `k`, so it keeps its digits as `x` nears zero. `expm1(0)` is exactly 0, below
/// `2^-54` it is `x`, below `-56 ln 2` it is `-1`, past `exp`'s overflow threshold it is `+inf`, and `NaN` returns
/// `NaN`.
pub(crate) fn expm1(x: f64) -> f64 {
    if x.is_nan() || (x > -TWO_NEG_54 && x < TWO_NEG_54) {
        return x;
    }
    if x > EXP_OVERFLOW {
        return f64::INFINITY;
    }
    if x < EXPM1_FLOOR {
        return -1.0;
    }
    let k_int = round_small(x * INV_LN2);
    let k = f64::from(k_int);
    let high = x - k * LN2_HI;
    let low = k * LN2_LO;
    let r = high - low;
    let carried = (high - r) - low;
    let tail = r * r * (0.5 + r * horner(&EXP_SERIES, r));
    let s = tail + carried * (1.0 + r);
    let k = k_int;
    match k {
        0 => r + s,
        -1 => 0.5 * (r + s) - 0.5,
        1 if r < -0.25 => 2.0 * ((r + 0.5) + s),
        1 => 1.0 + 2.0 * (r + s),
        ..0 | 57.. => {
            let half = k / 2;
            ((r + s) + 1.0) * power_of_two(half) * power_of_two(k - half) - 1.0
        }
        ..20 => ((r + s) + (1.0 - power_of_two(-k))) * power_of_two(k),
        _ => (r - (power_of_two(-k) - s) + 1.0) * power_of_two(k),
    }
}

#[inline]
pub(crate) fn ln_positive_finite(x: f64) -> f64 {
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
    k * LN2_HI - ((half_square - (tail + k * LN2_LO)) - f)
}

pub(crate) fn ln(x: f64) -> Option<f64> {
    if x.is_nan() || x <= 0.0 {
        return None;
    }
    if x.is_infinite() {
        return Some(x);
    }
    Some(ln_positive_finite(x))
}

pub(crate) fn turn(k: u64, eighth: NonZeroU64) -> (f64, f64) {
    let eighth = eighth.get();
    let quarter = 2 * eighth;
    let (quadrant, within) = ((k / quarter) % 4, k % quarter);
    let (cos, sin) = if within <= eighth {
        octant(within, eighth)
    } else {
        let (cos, sin) = octant(quarter - within, eighth);
        (sin, cos)
    };
    match quadrant {
        0 => (cos, sin),
        1 => (0.0 - sin, cos),
        2 => (0.0 - cos, 0.0 - sin),
        _ => (sin, 0.0 - cos),
    }
}

fn octant(steps: u64, eighth: u64) -> (f64, f64) {
    let x = steps as f64 * (FRAC_PI_4 / eighth as f64);
    let y = x * x;
    (horner(&COS_SERIES, y), x * horner(&SIN_SERIES, y))
}

#[cfg(test)]
mod tests {
    use super::{exp, expm1, ln, turn};
    use core::num::NonZeroU64;

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

    // Correctly rounded values from Python's `decimal` at 1000 digits, never from the code under test: tiny inputs,
    // each side of the reduction's k = 0, 1 and 2, the far side of -1, and the last binade before overflow.
    const EXPM1_VECTORS: [(u64, u64); 37] = [
        (0x3C30000000000000, 0x3C30000000000000),
        (0xBC30000000000000, 0xBC30000000000000),
        (0x01A56E1FC2F8F359, 0x01A56E1FC2F8F359),
        (0x81A56E1FC2F8F359, 0x81A56E1FC2F8F359),
        (0x3DDB7CDFD9D7BDBB, 0x3DDB7CDFD9DDA4E3),
        (0xBDDB7CDFD9D7BDBB, 0xBDDB7CDFD9D1D693),
        (0x3EE4F8B588E368F1, 0x3EE4F8BC681CDFB6),
        (0xBEE4F8B588E368F1, 0xBEE4F8AEA9ACF2C9),
        (0x3FB999999999999A, 0x3FBAEC7B35A00D3A),
        (0xBFB999999999999A, 0xBFB85C933156A62C),
        (0x3FD0000000000000, 0x3FD22D78F0FA061A),
        (0xBFD0000000000000, 0xBFCC5041854DF7D4),
        (0x3FD5C28F5C28F5C3, 0x3FD9EAA94C8422F5),
        (0xBFD5C28F5C28F5C3, 0xBFD2725AE35E2895),
        (0x3FD6666666666666, 0x3FDAD200B20177B2),
        (0xBFD6666666666666, 0xBFD2E663ED31C11E),
        (0x3FE0000000000000, 0x3FE4C2531C3C0D38),
        (0xBFE0000000000000, 0xBFD92E9A0720D3EC),
        (0x3FF0000000000000, 0x3FFB7E151628AED3),
        (0xBFF0000000000000, 0xBFE43A54E4E98864),
        (0x3FF0A3D70A3D70A4, 0x3FFD44790F5ED3CF),
        (0xBFF0A3D70A3D70A4, 0xBFE4B07FCE8BCE81),
        (0x4000000000000000, 0x40198E64B8D4DDAE),
        (0xC000000000000000, 0xBFEBAB5557101F8D),
        (0x4008000000000000, 0x403315E5BF6FB106),
        (0xC008000000000000, 0xBFEE6824F33314F5),
        (0x4024000000000000, 0x40D5825DCF950560),
        (0xC024000000000000, 0xBFEFFFA0CA192A6E),
        (0x403E000000000000, 0x42A370470AEC26ED),
        (0xC03E000000000000, 0xBFEFFFFFFFFFFCB5),
        (0x4042800000000000, 0x4344D13FBB1A0019),
        (0xC042800000000000, 0xBFEFFFFFFFFFFFFF),
        (0x4044000000000000, 0x438A220D397972EB),
        (0xC044000000000000, 0xBFF0000000000000),
        (0x4049000000000000, 0x44719103E4080B45),
        (0x4085E00000000000, 0x7F0D945DF4F8EC8E),
        (0x40862D999999999A, 0x7FED75AE7A50EE14),
    ];

    #[test]
    fn expm1_is_within_one_ulp_of_the_correctly_rounded_value() {
        for (x, want) in EXPM1_VECTORS {
            let got = expm1(f64::from_bits(x));
            assert!(
                ulps(got, f64::from_bits(want)) <= 1,
                "expm1({x:#x}) = {:#x}",
                got.to_bits()
            );
        }
    }

    // Correctly rounded from the series summed in exact rationals, never from the code under test.
    const EXPM1_CARRIED: [(u64, u64); 8] = [
        (0xBFF0A2B23F3BAB70, 0xBFE4AFB0CCC06217),
        (0xBFE02A23B932AFFF, 0xBFD9619699136743),
        (0xBFD99AAB2BEBDE68, 0xBFD51A3152FC0739),
        (0xBFD7D4FA53463BC2, 0xBFD3E5D84D1C1CD7),
        (0x3FD62E42FEFA39F0, 0x3FDA827999FCEF33),
        (0x3FF0A322353303E9, 0x3FFD42797D9AD3DA),
        (0x3FF12187226FD68A, 0x3FFEAD954F64E869),
        (0x4001CE285E575B60, 0x402084ECD1C995A3),
    ];

    #[test]
    fn expm1_carries_the_reductions_rounding_to_the_correctly_rounded_value() {
        for (x, want) in EXPM1_CARRIED {
            let got = expm1(f64::from_bits(x));
            assert_eq!(got.to_bits(), want, "expm1({x:#x}) = {:#x}", got.to_bits());
        }
    }

    #[test]
    fn expm1_is_exact_at_zero_and_saturates_past_its_thresholds() {
        assert_eq!(expm1(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(expm1(-0.0).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(expm1(709.79), f64::INFINITY);
        assert_eq!(expm1(f64::INFINITY), f64::INFINITY);
        assert_eq!(expm1(709.78).classify(), core::num::FpCategory::Normal);
        assert_eq!(expm1(-745.2).to_bits(), (-1.0_f64).to_bits());
        assert_eq!(expm1(f64::NEG_INFINITY).to_bits(), (-1.0_f64).to_bits());
        assert!(expm1(f64::NAN).is_nan());
    }

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

    const TURN_VECTORS: [(u64, u64, u64); 14] = [
        (1, 0x3FEFFFEC0960384E, 0x3F71DF42EAE296E2),
        (7, 0x3FEFFC2DDE7CB544, 0x3F9F457CEE04C393),
        (89, 0x3FED9E071E7C1F2A, 0x3FD83BC7690D138B),
        (179, 0x3FE6B9D6AE4CC898, 0x3FE68749E32E1B53),
        (180, 0x3FE6A09E667F3BCD, 0x3FE6A09E667F3BCD),
        (181, 0x3FE68749E32E1B53, 0x3FE6B9D6AE4CC898),
        (300, 0x3FD0907DC1930690, 0x3FEEE8DD4748BF15),
        (359, 0x3F71DF42EAE296E2, 0x3FEFFFEC0960384E),
        (361, 0xBF71DF42EAE296E2, 0x3FEFFFEC0960384E),
        (541, 0xBFE6B9D6AE4CC898, 0x3FE68749E32E1B53),
        (719, 0xBFEFFFEC0960384E, 0x3F71DF42EAE296E2),
        (721, 0xBFEFFFEC0960384E, 0xBF71DF42EAE296E2),
        (1000, 0xBFD5E3A8748A0BF5, 0xBFEE11F642522D1C),
        (1439, 0x3FEFFFEC0960384E, 0xBF71DF42EAE296E2),
    ];
    const MINUTES: NonZeroU64 = NonZeroU64::new(180).unwrap();

    #[test]
    fn a_turn_is_within_two_ulps_of_the_correctly_rounded_value() {
        for (k, cos, sin) in TURN_VECTORS {
            let (got_cos, got_sin) = turn(k, MINUTES);
            assert!(
                ulps(got_cos, f64::from_bits(cos)) <= 2,
                "cos {k}: {:#x}",
                got_cos.to_bits()
            );
            assert!(
                ulps(got_sin, f64::from_bits(sin)) <= 2,
                "sin {k}: {:#x}",
                got_sin.to_bits()
            );
        }
    }

    #[test]
    fn a_turn_is_exact_at_its_quarters_and_repeats() {
        let bits = |(cos, sin): (f64, f64)| (cos.to_bits(), sin.to_bits());
        let quarters = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
        for (quarter, want) in (0..).zip(quarters) {
            assert_eq!(
                bits(turn(360 * quarter, MINUTES)),
                bits(want),
                "quarter {quarter}"
            );
        }
        for k in [0, 1, 179, 180, 181, 719, 1439] {
            assert_eq!(bits(turn(k + 1440, MINUTES)), bits(turn(k, MINUTES)), "{k}");
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

    #[test]
    fn round_small_matches_f64_round_at_every_half_integer_boundary() {
        for n in -1076..=1026 {
            for half in [n as f64 + 0.5, n as f64] {
                let bits = half.to_bits();
                for delta in -2_i64..=2 {
                    let y = f64::from_bits(bits.wrapping_add(delta as u64));
                    if y.is_finite() {
                        assert_eq!(
                            super::round_small(y),
                            y.round() as i32,
                            "y={y:?} ({:#x})",
                            y.to_bits()
                        );
                    }
                }
            }
        }
        assert_eq!(super::round_small(0.0), 0);
        assert_eq!(super::round_small(-0.0), 0);
        assert_eq!(super::round_small(-1e-300), 0);
        assert_eq!(super::round_small(1e-300), 0);
    }
}
