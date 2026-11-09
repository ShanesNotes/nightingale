use super::{Arithmetic, Instr, Reads, read};
use crate::evaluate::Inputs;
use crate::pinned;

/// Rounding in one operation, with room for the contract's own `exp` and `ln`, which are within a few units of their
/// last place: each bound moves outward by this share of itself and the smallest normal number.
const SLACK: f64 = 32.0 * f64::EPSILON;

/// A closed set of values `[lo, hi]` that holds every value its expression takes, widened outward after each
/// operation so that rounding cannot shrink it past one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Bounds {
    pub(crate) lo: f64,
    pub(crate) hi: f64,
}
impl Bounds {
    pub(crate) const ZERO: Self = Self { lo: 0.0, hi: 0.0 };
    pub(crate) fn exact(value: f64) -> Self {
        Self { lo: value, hi: value }
    }
    pub(crate) fn around(lo: f64, hi: f64) -> Option<Self> {
        Self::widened(lo, hi)
    }
    fn widened(lo: f64, hi: f64) -> Option<Self> {
        let out = |value: f64| value.abs() * SLACK + f64::MIN_POSITIVE;
        let bounds = Self {
            lo: lo - out(lo),
            hi: hi + out(hi),
        };
        (bounds.lo.is_finite() && bounds.hi.is_finite() && bounds.lo <= bounds.hi).then_some(bounds)
    }
    pub(crate) fn hull(self, other: Self) -> Self {
        Self {
            lo: if other.lo < self.lo { other.lo } else { self.lo },
            hi: if other.hi > self.hi { other.hi } else { self.hi },
        }
    }
    fn plus(self, other: Self) -> Option<Self> {
        Self::widened(self.lo + other.lo, self.hi + other.hi)
    }
    fn minus(self, other: Self) -> Option<Self> {
        Self::widened(self.lo - other.hi, self.hi - other.lo)
    }
    fn times(self, other: Self) -> Option<Self> {
        let products = [
            self.lo * other.lo,
            self.lo * other.hi,
            self.hi * other.lo,
            self.hi * other.hi,
        ];
        let lo = products
            .iter()
            .copied()
            .fold(f64::INFINITY, |least, p| if p < least { p } else { least });
        let hi = products
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, |most, p| if p > most { p } else { most });
        Self::widened(lo, hi)
    }
    /// `None` where the divisor may reach zero or below, as a law's divisor may not.
    fn over(self, divisor: Self) -> Option<Self> {
        if divisor.lo <= 0.0 {
            return None;
        }
        self.times(Self::widened(1.0 / divisor.hi, 1.0 / divisor.lo)?)
    }
    fn negated(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
    fn monotone(self, f: impl Fn(f64) -> Option<f64>) -> Option<Self> {
        Self::widened(f(self.lo)?, f(self.hi)?)
    }
}

/// An expression's values over a stretch of the curve it reads, and its slopes along that curve.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Jet {
    pub(crate) value: Bounds,
    pub(crate) slope: Bounds,
}
impl Jet {
    pub(crate) fn exact(value: f64) -> Self {
        Self {
            value: Bounds::exact(value),
            slope: Bounds::ZERO,
        }
    }
    pub(crate) fn negated(self) -> Self {
        Self {
            value: self.value.negated(),
            slope: self.slope.negated(),
        }
    }
}

fn product(a: Jet, b: Jet) -> Option<Jet> {
    Some(Jet {
        value: a.value.times(b.value)?,
        slope: (a.slope.times(b.value)?).plus(a.value.times(b.slope)?)?,
    })
}

fn quotient(a: Jet, b: Jet) -> Option<Jet> {
    let value = a.value.over(b.value)?;
    Some(Jet {
        value,
        slope: (a.slope.minus(value.times(b.slope)?)?).over(b.value)?,
    })
}

/// `0.5 (a + b + sign r)` with `r = sqrt((a - b)^2 + w^2)`. At no width it is the larger or smaller of the two, whose
/// slope is one of theirs.
fn smooth(a: Jet, b: Jet, width: Jet, sign: f64) -> Option<Jet> {
    let half = Bounds::exact(0.5);
    if width.value == Bounds::ZERO && width.slope == Bounds::ZERO {
        let pick = |x: f64, y: f64| if (x > y) == (sign > 0.0) { x } else { y };
        let value = Bounds::widened(pick(a.value.lo, b.value.lo), pick(a.value.hi, b.value.hi))?;
        let a_wins = if sign > 0.0 {
            a.value.lo > b.value.hi
        } else {
            a.value.hi < b.value.lo
        };
        let b_wins = if sign > 0.0 {
            b.value.lo > a.value.hi
        } else {
            b.value.hi < a.value.lo
        };
        let slope = match (a_wins, b_wins) {
            (true, _) => a.slope,
            (_, true) => b.slope,
            _ => a.slope.hull(b.slope),
        };
        return Some(Jet { value, slope });
    }
    let difference = a.value.minus(b.value)?;
    let squared = |x: Bounds| {
        let product = x.times(x)?;
        Some(if x.lo <= 0.0 && x.hi >= 0.0 {
            Bounds {
                lo: 0.0,
                hi: product.hi,
            }
        } else {
            product
        })
    };
    let inside = squared(difference)?.plus(squared(width.value)?)?;
    let floor = |x: f64| if x > 0.0 { x } else { 0.0 };
    let root = Bounds {
        lo: floor(inside.lo),
        hi: inside.hi,
    }
    .monotone(|x| Some(x.sqrt()))?;
    let root = Bounds {
        lo: floor(root.lo),
        hi: root.hi,
    };
    let signed = |x: Bounds| if sign > 0.0 { x } else { x.negated() };
    let value = (a.value.plus(b.value)?).plus(signed(root))?.times(half)?;
    let unit = Bounds { lo: -1.0, hi: 1.0 };
    let clipped = |share: Bounds| Bounds {
        lo: if share.lo > -1.0 { share.lo } else { -1.0 },
        hi: if share.hi < 1.0 { share.hi } else { 1.0 },
    };
    let along = difference.over(root).map_or(unit, clipped);
    let across = width.value.over(root).map_or(unit, clipped);
    let turned = (along.times(a.slope.minus(b.slope)?)?).plus(across.times(width.slope)?)?;
    let slope = (a.slope.plus(b.slope)?).plus(signed(turned))?.times(half)?;
    Some(Jet { value, slope })
}

fn step(instr: Instr, jets: &[Jet], partial_volume: f64) -> Option<Jet> {
    let at = |slot: usize| jets[slot];
    Some(match instr {
        Instr::Zero => Jet::exact(0.0),
        Instr::Add([a, b]) => Jet {
            value: at(a).value.plus(at(b).value)?,
            slope: at(a).slope.plus(at(b).slope)?,
        },
        Instr::Subtract([a, b]) => Jet {
            value: at(a).value.minus(at(b).value)?,
            slope: at(a).slope.minus(at(b).slope)?,
        },
        Instr::Multiply([a, b]) => product(at(a), at(b))?,
        Instr::Divide([a, b]) => quotient(at(a), at(b))?,
        Instr::Copy([x]) => at(x),
        Instr::SmoothMax([a, b, width]) => smooth(at(a), at(b), at(width), 1.0)?,
        Instr::SmoothMin([a, b, width]) => smooth(at(a), at(b), at(width), -1.0)?,
        Instr::Exp([x]) => {
            let value = at(x).value.monotone(|x| Some(pinned::exp(x)))?;
            Jet {
                value,
                slope: value.times(at(x).slope)?,
            }
        }
        Instr::Logistic([x]) => {
            let logistic = |x: f64| Some(1.0 / (1.0 + pinned::exp(-x)));
            let value = at(x).value.monotone(logistic)?;
            let ends = [value.lo * (1.0 - value.lo), value.hi * (1.0 - value.hi)];
            let low = if ends[0] < ends[1] { ends[0] } else { ends[1] };
            let high = if value.lo <= 0.5 && value.hi >= 0.5 {
                0.25
            } else if ends[0] > ends[1] {
                ends[0]
            } else {
                ends[1]
            };
            Jet {
                value,
                slope: Bounds::widened(low, high)?.times(at(x).slope)?,
            }
        }
        Instr::Ln([x]) => {
            if at(x).value.lo <= 0.0 {
                return None;
            }
            Jet {
                value: at(x).value.monotone(pinned::ln)?,
                slope: at(x).slope.over(at(x).value)?,
            }
        }
        Instr::Relax([target, current, speed]) => {
            let gap = Jet {
                value: at(target).value.minus(at(current).value)?,
                slope: at(target).slope.minus(at(current).slope)?,
            };
            product(at(speed), gap)?
        }
        Instr::Volume([amount]) => product(Jet::exact(partial_volume), at(amount))?,
        Instr::Upwind([flow, from, from_water, to, to_water]) => {
            let carried = |amount: usize, water: usize| product(at(flow), quotient(at(amount), at(water))?);
            let flowing = at(flow).value;
            if flowing.lo > 0.0 {
                carried(from, from_water)?
            } else if flowing.hi < 0.0 {
                carried(to, to_water)?
            } else {
                let (one, other) = (carried(from, from_water)?, carried(to, to_water)?);
                Jet {
                    value: one.value.hull(other.value).hull(Bounds::ZERO),
                    slope: one.slope.hull(other.slope),
                }
            }
        }
    })
}

/// Every value `arithmetic` takes while each unknown it reads stays inside its jet, and every slope it takes there, or
/// `None` where a divisor, a logarithm or the finite range cannot be held.
pub(crate) fn enclose(
    arithmetic: Arithmetic<'_>,
    reads: Reads<'_>,
    inputs: &Inputs<'_>,
    unknown: &dyn Fn(usize) -> Jet,
    partial_volume: f64,
) -> Option<Jet> {
    let mut jets = vec![Jet::exact(0.0); arithmetic.length];
    for &(slot, index) in reads.parameters {
        jets[slot] = Jet::exact(*inputs.parameters.get(index)?);
    }
    for &(slot, index) in reads.unknowns {
        jets[slot] = unknown(index);
    }
    for &(slot, bound) in reads.others {
        jets[slot] = Jet::exact(read(&bound, inputs)?);
    }
    for &(at, instr) in arithmetic.steps {
        jets[at] = step(instr, &jets, partial_volume)?;
    }
    Some(jets[arithmetic.length - 1])
}
