//! The certificate: closure and drift over every sliding 24 h window, read from one view and policy data.
//! It shows stability, not accuracy, and takes no vital, reading or second history.
use ng_contract::{
    AdmissionError, Compensated, ConservedBasis, Digest, Dimension, Dimensionless, Interval, Measure,
    QuantityId, SimTime,
};
use ng_record::{ConservationView, Curve, HistorySeal, HistorySegment};

const DAY_NS: u64 = 86_400_000_000_000;

/// One combination's limits. `absolute + relative * gross exchange in the window` bounds closure, so a tiny
/// exchange needs a tiny residual. `drift` stands in for 1.96 times the within-subject variation until a
/// healthy-range set exists: it is fixture data, not a policy value.
pub struct QuantityPolicy {
    quantity: QuantityId,
    absolute: Compensated,
    relative: Measure<Dimensionless>,
    drift: Compensated,
}
impl QuantityPolicy {
    /// Refuses a dimension that is not what the combination balances.
    pub fn new<U: Dimension>(
        quantity: QuantityId,
        absolute: Measure<U>,
        relative: Measure<Dimensionless>,
        drift: Measure<U>,
    ) -> Result<Self, AdmissionError> {
        if !quantity.book().measures::<U>() {
            return Err(AdmissionError::QuantityDimension { quantity });
        }
        Ok(Self {
            quantity,
            absolute: Compensated::of(&absolute),
            relative,
            drift: Compensated::of(&drift),
        })
    }
}
/// What drift is compared with. A later fasting protocol adds a stated curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectedDrift {
    Zero,
}
/// Every sliding 24 h window over at least `coverage_ns` of history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sliding24Hours {
    coverage_ns: u64,
}
impl Sliding24Hours {
    /// Refuses a coverage that does not exceed one window, which could slide nowhere.
    pub fn covering(coverage_ns: u64) -> Result<Self, AdmissionError> {
        if coverage_ns <= DAY_NS {
            return Err(AdmissionError::ShortCoverage);
        }
        Ok(Self { coverage_ns })
    }
}
pub struct CertificatePolicy {
    basis: Digest,
    digest: Digest,
    quantities: Vec<QuantityPolicy>,
    expected: ExpectedDrift,
    windows: Sliding24Hours,
}
impl CertificatePolicy {
    /// Refuses unless every combination of the basis has exactly one entry, in basis order.
    pub fn new(
        basis: &ConservedBasis,
        quantities: Vec<QuantityPolicy>,
        expected: ExpectedDrift,
        windows: Sliding24Hours,
    ) -> Result<Self, AdmissionError> {
        let held = quantities.iter().map(|quantity| quantity.quantity);
        if !held.eq(basis.ids()) {
            return Err(AdmissionError::PolicyCoverage);
        }
        let basis = basis.digest();
        let mut bytes = basis.as_bytes().to_vec();
        let ExpectedDrift::Zero = expected;
        let Sliding24Hours { coverage_ns } = windows;
        bytes.extend_from_slice(&coverage_ns.to_be_bytes());
        for quantity in &quantities {
            let QuantityPolicy {
                quantity: _,
                absolute,
                relative,
                drift,
            } = quantity;
            for value in [*absolute, Compensated::of(relative), *drift] {
                for limb in value.bits() {
                    bytes.extend_from_slice(&limb.to_be_bytes());
                }
            }
        }
        Ok(Self {
            basis,
            digest: Digest::of_canonical(&bytes),
            quantities,
            expected,
            windows,
        })
    }
    pub fn digest(&self) -> Digest {
        self.digest
    }
}

/// The worst window of one combination, named by instant. Every window was checked.
#[derive(Clone, Copy, Debug)]
pub struct QuantityResult {
    quantity: QuantityId,
    worst_closure: Compensated,
    closure_window: Interval,
    worst_drift: Compensated,
    drift_window: Interval,
}
impl QuantityResult {
    pub fn quantity(&self) -> QuantityId {
        self.quantity
    }
    /// The largest `|inventory change - net boundary exchange|` over any window.
    pub fn worst_closure(&self) -> Compensated {
        self.worst_closure
    }
    pub fn closure_window(&self) -> Interval {
        self.closure_window
    }
    /// The largest `|inventory change - expected change|` over any window.
    pub fn worst_drift(&self) -> Compensated {
        self.worst_drift
    }
    pub fn drift_window(&self) -> Interval {
        self.drift_window
    }
}
#[derive(Debug)]
pub struct Certificate {
    seal: HistorySeal,
    basis: Digest,
    policy: Digest,
    results: Vec<QuantityResult>,
    pieces: u64,
}
impl Certificate {
    pub fn seal(&self) -> HistorySeal {
        self.seal
    }
    pub fn basis(&self) -> Digest {
        self.basis
    }
    pub fn policy(&self) -> Digest {
        self.policy
    }
    pub fn results(&self) -> &[QuantityResult] {
        &self.results
    }
    /// How many stretches of window starts the walk covered, so a run that checked nothing shows zero.
    pub fn pieces(&self) -> u64 {
        self.pieces
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CertificateCheck {
    /// inventory change minus net boundary exchange
    Closure,
    /// inventory change minus expected change
    Drift,
    /// the history is shorter than the coverage, or the policy's basis is not the view's
    Coverage,
}
#[derive(Clone, Copy, Debug)]
pub struct CertificateFailure {
    check: CertificateCheck,
    quantity: Option<QuantityId>,
    window: Option<Interval>,
    residual: Compensated,
}
impl CertificateFailure {
    pub fn check(&self) -> CertificateCheck {
        self.check
    }
    pub fn quantity(&self) -> Option<QuantityId> {
        self.quantity
    }
    pub fn window(&self) -> Option<Interval> {
        self.window
    }
    pub fn residual(&self) -> Compensated {
        self.residual
    }
}

/// Window starts: every segment boundary and its 24 h shift, inside `[first, last]`, ascending and without
/// repeats. A window `[a, a + 24 h]` depends on `a` only through which segment holds each end, so its
/// differences are one polynomial between consecutive members of this list.
fn window_starts(segments: &[HistorySegment<'_>], first: u64, last: u64) -> Vec<u64> {
    let mut boundaries = vec![first];
    boundaries.extend(
        segments
            .iter()
            .map(|segment| segment.interval().end().nanoseconds()),
    );
    let (mut direct, mut shifted) = (0, 0);
    let mut starts: Vec<u64> = Vec::with_capacity(boundaries.len());
    loop {
        let plain = boundaries.get(direct).copied();
        let moved = boundaries
            .get(shifted)
            .map(|boundary| boundary.saturating_sub(DAY_NS));
        let next = match (plain, moved) {
            (Some(plain), Some(moved)) if plain <= moved => {
                direct += 1;
                plain
            }
            (_, Some(moved)) => {
                shifted += 1;
                moved
            }
            (Some(plain), None) => {
                direct += 1;
                plain
            }
            (None, None) => break,
        };
        if (first..=last).contains(&next) && starts.last() != Some(&next) {
            starts.push(next);
        }
    }
    starts
}

fn differences(
    later: &Curve<'_>,
    late: Interval,
    earlier: &Curve<'_>,
    early: Interval,
) -> Option<Vec<Compensated>> {
    let (later, earlier) = (later.on(late)?, earlier.on(early)?);
    let difference = |(later, earlier): (&Compensated, &Compensated)| later.minus(*earlier);
    Some(later.iter().zip(&earlier).map(difference).collect())
}

/// Closure failures win over drift, so a `Drift` failure says the ledger closed everywhere and the body
/// still moved. Everything is compensated and nothing is rounded.
pub fn certify(
    view: &ConservationView<'_>,
    policy: &CertificatePolicy,
) -> Result<Certificate, CertificateFailure> {
    let coverage = CertificateFailure {
        check: CertificateCheck::Coverage,
        quantity: None,
        window: None,
        residual: Compensated::ZERO,
    };
    if policy.basis != view.basis().digest() {
        return Err(coverage);
    }
    let span = view.span().ok_or(coverage)?;
    if span.length_ns() < policy.windows.coverage_ns {
        return Err(coverage);
    }
    let ExpectedDrift::Zero = policy.expected;
    let segments: Vec<HistorySegment<'_>> = view.segments().collect();
    let first = span.start().nanoseconds();
    let last = span.end().nanoseconds() - DAY_NS;
    let starts = window_starts(&segments, first, last);
    let mut worst: Vec<Option<QuantityResult>> = vec![None; policy.quantities.len()];
    let mut drifted: Option<CertificateFailure> = None;
    let (mut trail, mut lead) = (0, 0);
    let mut pieces = 0;
    for pair in starts.windows(2) {
        let [from, to] = [pair[0], pair[1]];
        let early = Interval::new(SimTime::from_ns(from), SimTime::from_ns(to)).ok_or(coverage)?;
        let late =
            Interval::new(SimTime::from_ns(from + DAY_NS), SimTime::from_ns(to + DAY_NS)).ok_or(coverage)?;
        while segments[trail].interval().end() <= early.start() {
            trail += 1;
        }
        while segments[lead].interval().end() <= late.start() {
            lead += 1;
        }
        let (trailing, leading) = (&segments[trail], &segments[lead]);
        pieces += 1;
        for (limits, held) in policy.quantities.iter().zip(&mut worst) {
            let quantity = limits.quantity;
            let change = differences(
                &leading.inventory(quantity),
                late,
                &trailing.inventory(quantity),
                early,
            )
            .ok_or(coverage)?;
            let inflow = differences(&leading.inflow(quantity), late, &trailing.inflow(quantity), early)
                .ok_or(coverage)?;
            let outflow = differences(
                &leading.outflow(quantity),
                late,
                &trailing.outflow(quantity),
                early,
            )
            .ok_or(coverage)?;
            for (index, &start) in pair.iter().enumerate() {
                let window = Interval::new(SimTime::from_ns(start), SimTime::from_ns(start + DAY_NS))
                    .ok_or(coverage)?;
                let closure = change[index].minus(inflow[index].minus(outflow[index])).abs();
                let gross = inflow[index].plus(outflow[index]);
                let allowed = limits.absolute.plus(gross.times(&limits.relative));
                if closure > allowed {
                    return Err(CertificateFailure {
                        check: CertificateCheck::Closure,
                        quantity: Some(quantity),
                        window: Some(window),
                        residual: closure,
                    });
                }
                let drift = change[index].abs();
                if drift > limits.drift && drifted.is_none() {
                    drifted = Some(CertificateFailure {
                        check: CertificateCheck::Drift,
                        quantity: Some(quantity),
                        window: Some(window),
                        residual: drift,
                    });
                }
                let result = held.get_or_insert(QuantityResult {
                    quantity,
                    worst_closure: closure,
                    closure_window: window,
                    worst_drift: drift,
                    drift_window: window,
                });
                if closure > result.worst_closure {
                    result.worst_closure = closure;
                    result.closure_window = window;
                }
                if drift > result.worst_drift {
                    result.worst_drift = drift;
                    result.drift_window = window;
                }
            }
        }
    }
    if let Some(failure) = drifted {
        return Err(failure);
    }
    let results = worst.into_iter().collect::<Option<Vec<_>>>().ok_or(coverage)?;
    Ok(Certificate {
        seal: view.seal(),
        basis: policy.basis,
        policy: policy.digest,
        results,
        pieces,
    })
}
