use core::num::NonZeroU64;
use ng_contract::{
    AdmissionError, Atoms, Book, BoundaryDirection, Charge, Compensated, CompiledBody, ConservedBasis,
    CoverageManifest, Digest, Dimensionless, HeldRoute, Interval, Measure, QuantityId, SimTime, Substance,
};
use ng_record::{CheckpointError, ConservationView, Curve, HistorySeal, HistorySegment, NewestFirst};

/// One combination's limits. `absolute + relative * gross exchange in the window` bounds closure, so a tiny
/// exchange needs a tiny residual. `drift` stands in for 1.96 times the within-subject variation until the
/// healthy-range set holds that variation: it is fixture data, not a policy value. Charge has none: its drift
/// holds to the closure bound (record 0004).
pub struct QuantityPolicy {
    quantity: QuantityId,
    absolute: Compensated,
    relative: Measure<Dimensionless>,
    drift: Option<Compensated>,
}
impl QuantityPolicy {
    /// Refuses a dimension that is not what the combination balances.
    pub fn new<U: Substance>(
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
            drift: Some(Compensated::of(&drift)),
        })
    }
    /// Refuses a combination that is not an element's atoms.
    pub fn element(
        quantity: QuantityId,
        absolute: Measure<Atoms>,
        relative: Measure<Dimensionless>,
        drift: Measure<Atoms>,
    ) -> Result<Self, AdmissionError> {
        if !quantity.book().measures::<Atoms>() {
            return Err(AdmissionError::QuantityDimension { quantity });
        }
        Ok(Self {
            quantity,
            absolute: Compensated::of(&absolute),
            relative,
            drift: Some(Compensated::of(&drift)),
        })
    }
    /// Refuses a combination that is not charge.
    pub fn charge(
        quantity: QuantityId,
        absolute: Measure<Charge>,
        relative: Measure<Dimensionless>,
    ) -> Result<Self, AdmissionError> {
        if quantity.book() != Book::Charge {
            return Err(AdmissionError::QuantityDimension { quantity });
        }
        Ok(Self {
            quantity,
            absolute: Compensated::of(&absolute),
            relative,
            drift: None,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpectedDrift {
    Zero,
}
/// Every sliding window of `window_ns` over at least `coverage_ns` of history. A coverage of one window is the one
/// window from admission, however far past its end the history's last step reaches, so every core is judged on the
/// same window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlidingWindows {
    window_ns: NonZeroU64,
    coverage_ns: u64,
}
impl SlidingWindows {
    /// Refuses a coverage shorter than one window, which holds none.
    pub fn new(window_ns: NonZeroU64, coverage_ns: u64) -> Result<Self, AdmissionError> {
        if coverage_ns < window_ns.get() {
            return Err(AdmissionError::ShortCoverage);
        }
        Ok(Self {
            window_ns,
            coverage_ns,
        })
    }
    pub fn window_ns(&self) -> NonZeroU64 {
        self.window_ns
    }
    pub fn coverage_ns(&self) -> u64 {
        self.coverage_ns
    }
}
pub struct CertificatePolicy {
    basis: Digest,
    quantities: Vec<QuantityPolicy>,
    expected: ExpectedDrift,
    windows: SlidingWindows,
    rounding: RowRounding,
}
struct RowRounding {
    last_places_an_update: u64,
    updates_a_step: Vec<u64>,
}
impl CertificatePolicy {
    /// Refuses unless every combination of the basis has exactly one entry, in basis order.
    pub fn new(
        basis: &ConservedBasis,
        quantities: Vec<QuantityPolicy>,
        expected: ExpectedDrift,
        windows: SlidingWindows,
    ) -> Result<Self, AdmissionError> {
        let held = quantities.iter().map(|quantity| quantity.quantity);
        if !held.eq(basis.ids()) {
            return Err(AdmissionError::PolicyCoverage);
        }
        let rounding = RowRounding {
            last_places_an_update: 0,
            updates_a_step: vec![0; quantities.len()],
        };
        Ok(Self {
            basis: basis.digest(),
            quantities,
            expected,
            windows,
            rounding,
        })
    }
    /// Each closure may also be `per_update` last places of its combination's total at the window's late end for each
    /// row update the window's accepted steps make in that combination, counted from `body`'s columns. That bounds
    /// the rows' own rounding only where the weighted rows share one sign, so no row exceeds the total. Where they
    /// have both signs, as charge's do, a row may be far larger than the total, whose last place then bounds nothing.
    /// Refuses a body whose basis is not the policy's.
    pub fn admitting_row_rounding(
        self,
        body: &CompiledBody,
        per_update: u64,
    ) -> Result<Self, AdmissionError> {
        let basis = body.conserved_basis();
        if basis.digest() != self.basis {
            return Err(AdmissionError::PolicyCoverage);
        }
        let mut member = vec![false; body.rows().len()];
        let updates_a_step = (basis.combinations().iter())
            .map(|combination| {
                member.fill(false);
                for &(row, _) in combination.terms() {
                    member[row.index()] = true;
                }
                let terms = body.columns().iter().flat_map(|column| column.terms());
                terms.filter(|(row, _)| member[row.index()]).count() as u64
            })
            .collect();
        let rounding = RowRounding {
            last_places_an_update: per_update,
            updates_a_step,
        };
        Ok(Self { rounding, ..self })
    }
    pub fn digest(&self) -> Digest {
        let Self {
            basis,
            quantities,
            expected,
            windows,
            rounding:
                RowRounding {
                    last_places_an_update,
                    updates_a_step,
                },
        } = self;
        let mut bytes = basis.as_bytes().to_vec();
        bytes.extend_from_slice(&last_places_an_update.to_be_bytes());
        let ExpectedDrift::Zero = expected;
        let SlidingWindows {
            window_ns,
            coverage_ns,
        } = windows;
        bytes.extend_from_slice(&window_ns.get().to_be_bytes());
        bytes.extend_from_slice(&coverage_ns.to_be_bytes());
        for (quantity, updates) in quantities.iter().zip(updates_a_step) {
            bytes.extend_from_slice(&updates.to_be_bytes());
            let QuantityPolicy {
                quantity: _,
                absolute,
                relative,
                drift,
            } = quantity;
            // Only charge has no drift bound, and the basis digest says which combination that is.
            for value in [*absolute, Compensated::of(relative)].into_iter().chain(*drift) {
                for limb in value.bits() {
                    bytes.extend_from_slice(&limb.to_be_bytes());
                }
            }
        }
        Digest::of_canonical(&bytes)
    }
}

/// The worst window of one combination, named by instant. Every window was checked.
#[derive(Clone, Copy, Debug)]
pub struct QuantityResult {
    quantity: QuantityId,
    closure_floor: Compensated,
    closure_relative: Compensated,
    row_rounding: u64,
    worst_closure: Compensated,
    closure_window: Interval,
    closure_last_places: u64,
    closure_place: Compensated,
    worst_drift: Compensated,
    drift_window: Interval,
}
impl QuantityResult {
    pub fn quantity(&self) -> QuantityId {
        self.quantity
    }
    pub fn closure_floor(&self) -> Compensated {
        self.closure_floor
    }
    pub fn closure_relative(&self) -> Compensated {
        self.closure_relative
    }
    /// The policy's last places for one row update.
    pub fn row_rounding(&self) -> u64 {
        self.row_rounding
    }
    /// The largest `|inventory change - net boundary exchange|` over any window.
    pub fn worst_closure(&self) -> Compensated {
        self.worst_closure
    }
    pub fn closure_window(&self) -> Interval {
        self.closure_window
    }
    /// The last places the worst closure's window admitted, each of `closure_place`.
    pub fn closure_last_places(&self) -> u64 {
        self.closure_last_places
    }
    pub fn closure_place(&self) -> Compensated {
        self.closure_place
    }
    /// The largest `|inventory change - expected change|` over any window.
    pub fn worst_drift(&self) -> Compensated {
        self.worst_drift
    }
    pub fn drift_window(&self) -> Interval {
        self.drift_window
    }
}
/// The most one held route moved into and out of the body over any window, each named by instant.
#[derive(Clone, Copy, Debug)]
pub struct HeldSupport {
    route: HeldRoute,
    most_inflow: Compensated,
    inflow_window: Interval,
    most_outflow: Compensated,
    outflow_window: Interval,
}
impl HeldSupport {
    pub fn route(&self) -> HeldRoute {
        self.route
    }
    pub fn most_inflow(&self) -> Compensated {
        self.most_inflow
    }
    pub fn inflow_window(&self) -> Interval {
        self.inflow_window
    }
    pub fn most_outflow(&self) -> Compensated {
        self.most_outflow
    }
    pub fn outflow_window(&self) -> Interval {
        self.outflow_window
    }
}
#[derive(Debug)]
pub struct Certificate {
    seal: HistorySeal,
    basis: Digest,
    policy: Digest,
    windows: SlidingWindows,
    span: Interval,
    results: Vec<QuantityResult>,
    coverage: CoverageManifest,
    support: Vec<HeldSupport>,
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
    pub fn windows(&self) -> SlidingWindows {
        self.windows
    }
    /// Admission to the accepted head the walk read.
    pub fn span(&self) -> Interval {
        self.span
    }
    pub fn results(&self) -> &[QuantityResult] {
        &self.results
    }
    pub fn coverage(&self) -> &CoverageManifest {
        &self.coverage
    }
    /// One per held route, in `CoverageManifest::held_routes` order.
    pub fn support(&self) -> &[HeldSupport] {
        &self.support
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
    admission: u64,
    taken: u64,
}
impl<'r> Holding<'r> {
    fn new(view: &ConservationView<'r>, admission: u64) -> Self {
        Self {
            walk: view.newest_first(),
            at: None,
            admission,
            taken: 0,
        }
    }
    fn holding(&mut self, instant: u64) -> Result<(&HistorySegment, u64), CertificateFailure> {
        let admission = self.admission;
        let before = |segment: &HistorySegment| {
            let start = segment.interval().start().nanoseconds();
            let opens_the_history = start == admission;
            start < instant || opens_the_history
        };
        while !self.at.as_ref().is_some_and(before) {
            let next = self.walk.next().ok_or(of_history(CertificateCheck::Coverage))?;
            let read = |error| of_history(CertificateCheck::History(error));
            self.at = Some(next.map_err(read)?);
            self.taken += 1;
        }
        let taken = self.taken;
        (self.at.as_ref().map(|at| (at, taken))).ok_or(of_history(CertificateCheck::Coverage))
    }
}

/// 2^(e - 105), with 2^e the power of two at or below the total's larger part, never below the smallest float, and
/// zero for a total that is not finite.
fn last_place(total: Compensated) -> Compensated {
    let biased = (total.bits()[0] >> 52) & 0x7ff;
    let place = match biased {
        0x7ff => 0.0,
        106.. => f64::from_bits((biased - 105) << 52),
        54.. => f64::from_bits(1 << (biased - 54)),
        _ => f64::from_bits(1),
    };
    Measure::<Dimensionless>::new(place).map_or(Compensated::ZERO, |place| Compensated::of(&place))
}

/// Scales the place by the count as a float: exact up to 2^53, to within one part in 2^53 above it, and held at the
/// largest signed integer past that.
fn admitted(floor: Compensated, place: Compensated, last_places: u64) -> Compensated {
    floor.plus(place.scaled(i64::try_from(last_places).unwrap_or(i64::MAX)))
}

#[derive(Clone, Copy)]
enum Stretch {
    Between { from: u64, to: u64 },
    FromAdmission { at: u64 },
}
impl Stretch {
    fn starts(self) -> impl Iterator<Item = u64> {
        let (from, to, count) = match self {
            Stretch::Between { from, to } => (from, to, 2),
            Stretch::FromAdmission { at } => (at, at, 1),
        };
        [from, to].into_iter().take(count)
    }
    fn ends(
        self,
        (later, leading): (&Curve<'_>, Interval),
        (earlier, trailing): (&Curve<'_>, Interval),
        length: u64,
    ) -> Option<Vec<(Compensated, Compensated)>> {
        let at = SimTime::from_ns;
        match self {
            Stretch::Between { from, to } => {
                let early = Interval::new(at(from), at(to))?;
                let late = Interval::new(at(from + length), at(to + length))?;
                let (later, earlier) = (later.on(late)?, earlier.on(early)?);
                Some(later.into_iter().zip(earlier).collect())
            }
            Stretch::FromAdmission { at: start } => {
                let late = later.on(Interval::new(leading.start(), at(start + length))?)?;
                let early = earlier.on(Interval::new(at(start), trailing.end())?)?;
                Some(vec![(*late.last()?, *early.first()?)])
            }
        }
    }
}

struct Window {
    quantity: usize,
    interval: Interval,
    closure: Compensated,
    last_places: u64,
    place: Compensated,
    drift: Compensated,
}
struct Carried {
    route: usize,
    interval: Interval,
    inflow: Compensated,
    outflow: Compensated,
}
struct Pair {
    drifted: Option<CertificateFailure>,
    windows: Vec<Window>,
    carried: Vec<Carried>,
}

fn pair(
    policy: &CertificatePolicy,
    routes: &[HeldRoute],
    trailing: &HistorySegment,
    leading: &HistorySegment,
    (stretch, steps): (Stretch, u64),
) -> Result<Pair, CertificateFailure> {
    let coverage = of_history(CertificateCheck::Coverage);
    let length = policy.windows.window_ns.get();
    let (early, late) = (trailing.interval(), leading.interval());
    let ends = |later: &Curve<'_>, earlier: &Curve<'_>| stretch.ends((later, late), (earlier, early), length);
    let differences = |later: &Curve<'_>, earlier: &Curve<'_>| {
        let ends = ends(later, earlier)?;
        Some(
            ends.into_iter()
                .map(|(later, earlier)| later.minus(earlier))
                .collect::<Vec<_>>(),
        )
    };
    let window =
        |start: u64| Interval::new(SimTime::from_ns(start), SimTime::from_ns(start + length)).ok_or(coverage);
    let mut drifted = None;
    let mut windows = Vec::with_capacity(2 * policy.quantities.len());
    for (at, limits) in policy.quantities.iter().enumerate() {
        let quantity = limits.quantity;
        let inventory = ends(&leading.inventory(quantity), &trailing.inventory(quantity)).ok_or(coverage)?;
        let inflow = differences(&leading.inflow(quantity), &trailing.inflow(quantity)).ok_or(coverage)?;
        let outflow = differences(&leading.outflow(quantity), &trailing.outflow(quantity)).ok_or(coverage)?;
        let rounding = &policy.rounding;
        let last_places = (rounding
            .last_places_an_update
            .saturating_mul(rounding.updates_a_step[at]))
        .saturating_mul(steps);
        for (index, start) in stretch.starts().enumerate() {
            let interval = window(start)?;
            let (closing, opening) = inventory[index];
            let change = closing.minus(opening);
            let closure = change.minus(inflow[index].minus(outflow[index])).abs();
            let gross = inflow[index].plus(outflow[index]);
            let place = last_place(closing);
            let allowed = admitted(
                limits.absolute.plus(gross.times(&limits.relative)),
                place,
                last_places,
            );
            if !closure.at_most(allowed) {
                return Err(CertificateFailure {
                    check: CertificateCheck::Closure,
                    quantity: Some(quantity),
                    window: Some(interval),
                    residual: closure,
                });
            }
            let drift = change.abs();
            if !drift.at_most(limits.drift.unwrap_or(allowed)) && drifted.is_none() {
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
                last_places,
                place,
                drift,
            });
        }
    }
    let mut carried = Vec::with_capacity(2 * routes.len());
    for (at, route) in routes.iter().enumerate() {
        let ((forward, reverse), (forward_before, reverse_before)) = (
            leading.exchange(route.column()),
            trailing.exchange(route.column()),
        );
        let forward = differences(&forward, &forward_before).ok_or(coverage)?;
        let reverse = differences(&reverse, &reverse_before).ok_or(coverage)?;
        let (inflow, outflow) = match route.forward() {
            BoundaryDirection::IntoBody => (forward, reverse),
            BoundaryDirection::OutOfBody => (reverse, forward),
        };
        for (index, start) in stretch.starts().enumerate() {
            carried.push(Carried {
                route: at,
                interval: window(start)?,
                inflow: inflow[index],
                outflow: outflow[index],
            });
        }
    }
    Ok(Pair {
        drifted,
        windows,
        carried,
    })
}

fn keep_the_earliest_most(most: &mut [Option<HeldSupport>], routes: &[HeldRoute], carried: Vec<Carried>) {
    for window in carried.into_iter().rev() {
        let Carried {
            route,
            interval,
            inflow,
            outflow,
        } = window;
        let support = most[route].get_or_insert(HeldSupport {
            route: routes[route],
            most_inflow: inflow,
            inflow_window: interval,
            most_outflow: outflow,
            outflow_window: interval,
        });
        if support.most_inflow.at_most(inflow) {
            support.most_inflow = inflow;
            support.inflow_window = interval;
        }
        if support.most_outflow.at_most(outflow) {
            support.most_outflow = outflow;
            support.outflow_window = interval;
        }
    }
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
            last_places,
            place,
            drift,
        } = window;
        let limits = &policy.quantities[quantity];
        let result = worst[quantity].get_or_insert(QuantityResult {
            quantity: limits.quantity,
            closure_floor: limits.absolute,
            closure_relative: Compensated::of(&limits.relative),
            row_rounding: policy.rounding.last_places_an_update,
            worst_closure: closure,
            closure_window: interval,
            closure_last_places: last_places,
            closure_place: place,
            worst_drift: drift,
            drift_window: interval,
        });
        if result.worst_closure.at_most(closure) {
            result.worst_closure = closure;
            result.closure_window = interval;
            result.closure_last_places = last_places;
            result.closure_place = place;
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
    let routes = view.basis().coverage().held_routes();
    let length = policy.windows.window_ns.get();
    let first = span.start().nanoseconds();
    let (mut early, mut late) = (Holding::new(view, first), Holding::new(view, first));
    let mut worst: Vec<Option<QuantityResult>> = vec![None; policy.quantities.len()];
    let mut most: Vec<Option<HeldSupport>> = vec![None; routes.len()];
    let (mut failed, mut drifted) = (None, None);
    let mut pieces = 0;
    let mut to = if policy.windows.coverage_ns == length {
        first
    } else {
        span.end().nanoseconds() - length
    };
    loop {
        let (trailing, behind) = early.holding(to)?;
        let (leading, ahead) = late.holding(to + length)?;
        let steps = behind.abs_diff(ahead) + 1;
        let start = |segment: &HistorySegment| segment.interval().start().nanoseconds();
        let from = start(trailing).max(start(leading).saturating_sub(length));
        pieces += 1;
        let stretch = if to == first {
            Stretch::FromAdmission { at: first }
        } else {
            Stretch::Between { from, to }
        };
        match pair(policy, routes, trailing, leading, (stretch, steps)) {
            Err(failure) => failed = Some(failure),
            Ok(pair) => {
                drifted = pair.drifted.or(drifted);
                keep_the_earliest_worst(&mut worst, policy, pair.windows);
                keep_the_earliest_most(&mut most, routes, pair.carried);
            }
        }
        if from == first {
            break;
        }
        to = from;
    }
    if let Some(failure) = failed.or(drifted) {
        return Err(failure);
    }
    let results = worst.into_iter().collect::<Option<Vec<_>>>().ok_or(coverage)?;
    let support = most.into_iter().collect::<Option<Vec<_>>>().ok_or(coverage)?;
    Ok(Certificate {
        seal: view.seal(),
        basis: policy.basis,
        policy: policy.digest(),
        windows: policy.windows,
        span,
        results,
        coverage: view.basis().coverage().clone(),
        support,
        pieces,
    })
}

#[cfg(test)]
mod tests {
    use super::{admitted, last_place};
    use ng_contract::{Compensated, Dimensionless, Measure};

    fn exact(value: f64) -> Compensated {
        Compensated::of(&Measure::<Dimensionless>::new(value).expect("finite"))
    }

    #[test]
    fn a_closure_at_its_last_places_passes_and_one_place_over_refuses() {
        let total = exact(4254.0).plus(exact(6.39e-14));
        let place = last_place(total);
        assert_eq!(
            place.bits(),
            exact(f64::from_bits((1023 - 93) << 52)).bits(),
            "4,254's last place is 2^-93"
        );
        let allowed = admitted(Compensated::ZERO, place, 7 * 31 * 1_000);
        let at = place.scaled(7 * 31 * 1_000);
        assert!(at.at_most(allowed), "a closure at its allowance passes");
        assert!(!at.plus(place).at_most(allowed), "one last place over refuses");
        let floor = exact(1e-20);
        assert!(
            at.plus(floor).at_most(admitted(floor, place, 7 * 31 * 1_000)),
            "the floor adds to it"
        );
    }

    #[test]
    fn a_total_just_below_a_power_of_two_takes_its_larger_parts_last_place() {
        let total = exact(4096.0).plus(exact(-6.39e-14));
        assert_eq!(total.bits()[0], 4096.0_f64.to_bits(), "the larger part is 2^12");
        assert_eq!(
            last_place(total).bits(),
            exact(f64::from_bits((1023 - 93) << 52)).bits(),
            "so the place is 2^-93, though the sum is below 2^12"
        );
    }

    #[test]
    fn a_count_scales_as_a_float_and_holds_at_the_largest_signed_integer() {
        let one = exact(1.0);
        let two_to = |power: u64| exact(f64::from_bits((1023 + power) << 52));
        assert_eq!(
            admitted(Compensated::ZERO, one, (1 << 53) + 1).bits(),
            two_to(53).bits(),
            "past 2^53 a count rounds down"
        );
        assert_eq!(
            admitted(Compensated::ZERO, one, (1 << 53) + 3).bits(),
            two_to(53).plus(exact(4.0)).bits(),
            "or up, by one part in 2^53 at most"
        );
        assert_eq!(
            admitted(Compensated::ZERO, one, u64::MAX).bits(),
            two_to(63).bits(),
            "past the largest signed integer it holds at that, which scales as 2^63"
        );
    }

    #[test]
    fn a_last_place_holds_from_the_smallest_float_to_the_largest() {
        let smallest = f64::from_bits(1);
        assert_eq!(
            last_place(exact(1.0)).bits(),
            exact(f64::from_bits((1023 - 105) << 52)).bits()
        );
        assert_eq!(
            last_place(exact(1.5)).bits(),
            last_place(exact(1.0)).bits(),
            "the power of two at or below"
        );
        assert_eq!(
            last_place(exact(f64::from_bits(106 << 52))).bits(),
            exact(f64::from_bits(1 << 52)).bits()
        );
        assert_eq!(
            last_place(exact(f64::from_bits(105 << 52))).bits(),
            exact(f64::from_bits(1 << 51)).bits()
        );
        assert_eq!(
            last_place(exact(smallest)).bits(),
            exact(smallest).bits(),
            "never below the smallest float"
        );
        assert_eq!(
            last_place(exact(-2.0)).bits(),
            last_place(exact(2.0)).bits(),
            "on either sign"
        );
        let unbounded = exact(f64::MAX).scaled(2);
        assert_eq!(
            last_place(unbounded).bits(),
            Compensated::ZERO.bits(),
            "zero for a total that is not finite"
        );
    }
}
