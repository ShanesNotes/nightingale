use ng_contract::{
    AdmissionError, Compensated, ConservedBasis, Digest, Dimension, Dimensionless, Interval, Measure,
    QuantityId, SimTime,
};
use ng_record::{CheckpointError, ConservationView, Curve, HistorySeal, HistorySegment, NewestFirst};

const DAY_NS: u64 = 86_400_000_000_000;

/// One combination's limits. `absolute + relative * gross exchange in the window` bounds closure, so a tiny
/// exchange needs a tiny residual. `drift` stands in for 1.96 times the within-subject variation until the
/// healthy-range set holds that variation: it is fixture data, not a policy value.
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
    /// Refuses a coverage that does not exceed one window.
    pub fn covering(coverage_ns: u64) -> Result<Self, AdmissionError> {
        if coverage_ns <= DAY_NS {
            return Err(AdmissionError::ShortCoverage);
        }
        Ok(Self { coverage_ns })
    }
    pub fn coverage_ns(&self) -> u64 {
        self.coverage_ns
    }
}
pub struct CertificatePolicy {
    basis: Digest,
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
        Ok(Self {
            basis: basis.digest(),
            quantities,
            expected,
            windows,
        })
    }
    pub fn digest(&self) -> Digest {
        let Self {
            basis,
            quantities,
            expected,
            windows,
        } = self;
        let mut bytes = basis.as_bytes().to_vec();
        let ExpectedDrift::Zero = expected;
        let Sliding24Hours { coverage_ns } = windows;
        bytes.extend_from_slice(&coverage_ns.to_be_bytes());
        for quantity in quantities {
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
        Digest::of_canonical(&bytes)
    }
}

/// The worst window of one combination, named by instant, beside the limits it was held to. Every window was
/// checked.
#[derive(Clone, Copy, Debug)]
pub struct QuantityResult {
    quantity: QuantityId,
    closure_floor: Compensated,
    closure_relative: Compensated,
    drift_limit: Compensated,
    worst_closure: Compensated,
    closure_window: Interval,
    worst_drift: Compensated,
    drift_window: Interval,
}
impl QuantityResult {
    pub fn quantity(&self) -> QuantityId {
        self.quantity
    }
    /// With `closure_relative`, the policy's closure bound: `floor + relative * gross exchange in the window`.
    pub fn closure_floor(&self) -> Compensated {
        self.closure_floor
    }
    pub fn closure_relative(&self) -> Compensated {
        self.closure_relative
    }
    pub fn drift_limit(&self) -> Compensated {
        self.drift_limit
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
    windows: Sliding24Hours,
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
    pub fn windows(&self) -> Sliding24Hours {
        self.windows
    }
    pub fn results(&self) -> &[QuantityResult] {
        &self.results
    }
    /// How many stretches of window starts the walk covered.
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
    /// a link the store did not give, or one that does not chain to the seal that asked for it
    History(CheckpointError),
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

fn of_history(check: CertificateCheck) -> CertificateFailure {
    CertificateFailure {
        check,
        quantity: None,
        window: None,
        residual: Compensated::ZERO,
    }
}

struct Holding<'r> {
    walk: NewestFirst<'r>,
    at: Option<HistorySegment>,
}
impl<'r> Holding<'r> {
    fn new(view: &ConservationView<'r>) -> Self {
        Self {
            walk: view.newest_first(),
            at: None,
        }
    }
    fn holding(&mut self, instant: u64) -> Result<&HistorySegment, CertificateFailure> {
        let before = |segment: &HistorySegment| segment.interval().start().nanoseconds() < instant;
        while !self.at.as_ref().is_some_and(before) {
            let next = self.walk.next().ok_or(of_history(CertificateCheck::Coverage))?;
            let read = |error| of_history(CertificateCheck::History(error));
            self.at = Some(next.map_err(read)?);
        }
        self.at.as_ref().ok_or(of_history(CertificateCheck::Coverage))
    }
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

struct Window {
    quantity: usize,
    interval: Interval,
    closure: Compensated,
    drift: Compensated,
}

fn pair(
    policy: &CertificatePolicy,
    trailing: &HistorySegment,
    leading: &HistorySegment,
    from: u64,
    to: u64,
) -> Result<(Option<CertificateFailure>, Vec<Window>), CertificateFailure> {
    let coverage = of_history(CertificateCheck::Coverage);
    let early = Interval::new(SimTime::from_ns(from), SimTime::from_ns(to)).ok_or(coverage)?;
    let late =
        Interval::new(SimTime::from_ns(from + DAY_NS), SimTime::from_ns(to + DAY_NS)).ok_or(coverage)?;
    let mut drifted = None;
    let mut windows = Vec::with_capacity(2 * policy.quantities.len());
    for (at, limits) in policy.quantities.iter().enumerate() {
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
        for (index, start) in [from, to].into_iter().enumerate() {
            let interval =
                Interval::new(SimTime::from_ns(start), SimTime::from_ns(start + DAY_NS)).ok_or(coverage)?;
            let closure = change[index].minus(inflow[index].minus(outflow[index])).abs();
            let gross = inflow[index].plus(outflow[index]);
            let allowed = limits.absolute.plus(gross.times(&limits.relative));
            if !closure.at_most(allowed) {
                return Err(CertificateFailure {
                    check: CertificateCheck::Closure,
                    quantity: Some(quantity),
                    window: Some(interval),
                    residual: closure,
                });
            }
            let drift = change[index].abs();
            if !drift.at_most(limits.drift) && drifted.is_none() {
                drifted = Some(CertificateFailure {
                    check: CertificateCheck::Drift,
                    quantity: Some(quantity),
                    window: Some(interval),
                    residual: drift,
                });
            }
            windows.push(Window {
                quantity: at,
                interval,
                closure,
                drift,
            });
        }
    }
    Ok((drifted, windows))
}

/// `windows` came earlier in time than every window `worst` has seen, so a tie replaces.
fn keep_the_earliest_worst(
    worst: &mut [Option<QuantityResult>],
    policy: &CertificatePolicy,
    windows: Vec<Window>,
) {
    for window in windows.into_iter().rev() {
        let Window {
            quantity,
            interval,
            closure,
            drift,
        } = window;
        let limits = &policy.quantities[quantity];
        let result = worst[quantity].get_or_insert(QuantityResult {
            quantity: limits.quantity,
            closure_floor: limits.absolute,
            closure_relative: Compensated::of(&limits.relative),
            drift_limit: limits.drift,
            worst_closure: closure,
            closure_window: interval,
            worst_drift: drift,
            drift_window: interval,
        });
        if result.worst_closure.at_most(closure) {
            result.worst_closure = closure;
            result.closure_window = interval;
        }
        if result.worst_drift.at_most(drift) {
            result.worst_drift = drift;
            result.drift_window = interval;
        }
    }
}

/// Closure failures win over drift, so a `Drift` failure says the ledger closed everywhere and the body
/// still moved. It walks the store newest first and reports what a walk in time order would: the earliest
/// failing window, and the earliest of equal worst windows.
pub fn certify(
    view: &ConservationView<'_>,
    policy: &CertificatePolicy,
) -> Result<Certificate, CertificateFailure> {
    let coverage = of_history(CertificateCheck::Coverage);
    if policy.basis != view.basis().digest() {
        return Err(coverage);
    }
    let span = view.span().ok_or(coverage)?;
    if span.length_ns() < policy.windows.coverage_ns {
        return Err(coverage);
    }
    let ExpectedDrift::Zero = policy.expected;
    let first = span.start().nanoseconds();
    let (mut early, mut late) = (Holding::new(view), Holding::new(view));
    let mut worst: Vec<Option<QuantityResult>> = vec![None; policy.quantities.len()];
    let (mut failed, mut drifted) = (None, None);
    let mut pieces = 0;
    let mut to = span.end().nanoseconds() - DAY_NS;
    while to > first {
        let trailing = early.holding(to)?;
        let leading = late.holding(to + DAY_NS)?;
        let start = |segment: &HistorySegment| segment.interval().start().nanoseconds();
        let from = start(trailing).max(start(leading).saturating_sub(DAY_NS));
        pieces += 1;
        match pair(policy, trailing, leading, from, to) {
            Err(failure) => failed = Some(failure),
            Ok((drift, windows)) => {
                drifted = drift.or(drifted);
                keep_the_earliest_worst(&mut worst, policy, windows);
            }
        }
        to = from;
    }
    if let Some(failure) = failed.or(drifted) {
        return Err(failure);
    }
    let results = worst.into_iter().collect::<Option<Vec<_>>>().ok_or(coverage)?;
    Ok(Certificate {
        seal: view.seal(),
        basis: policy.basis,
        policy: policy.digest(),
        windows: policy.windows,
        results,
        pieces,
    })
}
