use core::fmt;
use core::num::NonZeroU64;
use ng_contract::{
    AdmissionError, Atoms, Book, BoundaryDirection, Charge, Compensated, CompiledBody, ConservedBasis,
    CoverageManifest, Digest, Dimensionless, HeldRoute, Interval, Measure, QuantityId, Rate, RhythmId,
    SimTime, Slot, Substance,
};
use ng_record::{CheckpointError, ConservationView, Curve, HistorySeal, HistorySegment, OldestFirst};

const SECOND: u64 = 1_000_000_000;

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
/// One conserved quantity's allowance between a rhythm's edges: a rate in its own units a second, which a window
/// between edges allows times the window's own length. `settling` is the rate of a window whose opening edge comes
/// before the policy's settling time.
#[derive(Clone, Copy, Debug)]
pub struct EdgeRate {
    quantity: QuantityId,
    settled: Compensated,
    settling: Option<Compensated>,
}
impl EdgeRate {
    /// Refuses a dimension that is not what the combination balances.
    pub fn new<U: Substance>(
        quantity: QuantityId,
        settled: Measure<Rate<U>>,
        settling: Option<Measure<Rate<U>>>,
    ) -> Result<Self, AdmissionError> {
        if !quantity.book().measures::<U>() {
            return Err(AdmissionError::QuantityDimension { quantity });
        }
        Ok(Self::of(quantity, &settled, settling.as_ref()))
    }
    /// Refuses a combination that is not an element's atoms.
    pub fn element(
        quantity: QuantityId,
        settled: Measure<Rate<Atoms>>,
        settling: Option<Measure<Rate<Atoms>>>,
    ) -> Result<Self, AdmissionError> {
        if !quantity.book().measures::<Atoms>() {
            return Err(AdmissionError::QuantityDimension { quantity });
        }
        Ok(Self::of(quantity, &settled, settling.as_ref()))
    }
    fn of<U>(quantity: QuantityId, settled: &Measure<U>, settling: Option<&Measure<U>>) -> Self {
        Self {
            quantity,
            settled: Compensated::of(settled),
            settling: settling.map(Compensated::of),
        }
    }
}
/// Windows between the edges of the rhythm whose phase is `clock`. Every edge from `settling_ns` after admission on
/// opens one window, which closes at the first edge at least `length_ns` later. No stretch with no edge, from
/// admission to the first edge and from the last window's close to the head included, may pass `widest_ns`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeWindows {
    clock: Slot,
    settling_ns: u64,
    length_ns: NonZeroU64,
    widest_ns: u64,
}
impl EdgeWindows {
    pub fn new(clock: Slot, settling_ns: u64, length_ns: NonZeroU64, widest_ns: u64) -> Self {
        Self {
            clock,
            settling_ns,
            length_ns,
            widest_ns,
        }
    }
    pub fn clock(&self) -> Slot {
        self.clock
    }
    pub fn settling_ns(&self) -> u64 {
        self.settling_ns
    }
    pub fn length_ns(&self) -> NonZeroU64 {
        self.length_ns
    }
    pub fn widest_ns(&self) -> u64 {
        self.widest_ns
    }
}
struct EdgePolicy {
    windows: EdgeWindows,
    rates: Vec<EdgeRate>,
}
pub struct CertificatePolicy {
    basis: Digest,
    quantities: Vec<QuantityPolicy>,
    expected: ExpectedDrift,
    windows: SlidingWindows,
    rounding: RowRounding,
    edges: Option<EdgePolicy>,
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
            edges: None,
        })
    }
    /// Adds windows between the edges of the rhythm `windows` names, each allowing each rated quantity its rate times
    /// the window's own length. Refuses rates that are not a subset of the basis in basis order, a rate before the
    /// settling time stated for some quantities and not for others, and a second call, since a policy judges one
    /// clock.
    pub fn between_edges(self, windows: EdgeWindows, rates: Vec<EdgeRate>) -> Result<Self, AdmissionError> {
        let mut held = self.quantities.iter().map(|limits| limits.quantity);
        let ordered = rates
            .iter()
            .all(|rate| held.any(|quantity| quantity == rate.quantity));
        let settling = rates.iter().filter(|rate| rate.settling.is_some()).count();
        if self.edges.is_some() || !ordered || (settling != 0 && settling != rates.len()) {
            return Err(AdmissionError::PolicyCoverage);
        }
        let edges = Some(EdgePolicy { windows, rates });
        Ok(Self { edges, ..self })
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
            edges,
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
        // A policy with no windows between edges adds nothing, so its digest is the one it had before they existed.
        if let Some(EdgePolicy {
            windows:
                EdgeWindows {
                    clock,
                    settling_ns,
                    length_ns,
                    widest_ns,
                },
            rates,
        }) = edges
        {
            bytes.extend_from_slice(clock.digest().as_bytes());
            for number in [*settling_ns, length_ns.get(), *widest_ns, rates.len() as u64] {
                bytes.extend_from_slice(&number.to_be_bytes());
            }
            for EdgeRate {
                quantity,
                settled,
                settling,
            } in rates
            {
                bytes.extend_from_slice(&(quantity.index() as u64).to_be_bytes());
                let stated = [Some(*settled), *settling];
                for value in stated {
                    let limbs = value.map_or([0, 0, 0], |value| {
                        let [hi, lo] = value.bits();
                        [1, hi, lo]
                    });
                    for limb in limbs {
                        bytes.extend_from_slice(&limb.to_be_bytes());
                    }
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
/// The window between edges where one rated quantity came nearest its allowance, or passed it by most, the
/// earliest of equal: its change, as a magnitude, and the rate times its length it was allowed.
#[derive(Clone, Copy, Debug)]
pub struct EdgeResult {
    quantity: QuantityId,
    change: Compensated,
    allowance: Compensated,
    window: Interval,
}
impl EdgeResult {
    pub fn quantity(&self) -> QuantityId {
        self.quantity
    }
    pub fn change(&self) -> Compensated {
        self.change
    }
    pub fn allowance(&self) -> Compensated {
        self.allowance
    }
    pub fn window(&self) -> Interval {
        self.window
    }
}
/// What the windows between one rhythm's edges found, to the head.
#[derive(Clone, Debug)]
pub struct RhythmResult {
    windows: EdgeWindows,
    judged: u64,
    results: Vec<EdgeResult>,
    widest: Interval,
    last_close: Option<SimTime>,
}
impl RhythmResult {
    pub fn windows(&self) -> EdgeWindows {
        self.windows
    }
    /// How many windows between edges were judged.
    pub fn judged(&self) -> u64 {
        self.judged
    }
    /// One per rated quantity in basis order, or none when no window was judged.
    pub fn results(&self) -> &[EdgeResult] {
        &self.results
    }
    /// The widest stretch with no edge, from admission to the first edge and from the last close to the head
    /// included, the earliest of equal.
    pub fn widest(&self) -> Interval {
        self.widest
    }
    /// The close of the last window judged, from which the tail is measured.
    pub fn last_close(&self) -> Option<SimTime> {
        self.last_close
    }
}
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
    rhythm: Option<RhythmResult>,
}
/// Prints as the derive did before a certificate could hold a rhythm, and adds the rhythm where one was judged.
impl fmt::Debug for Certificate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            seal,
            basis,
            policy,
            windows,
            span,
            results,
            coverage,
            support,
            pieces,
            rhythm,
        } = self;
        let mut out = f.debug_struct("Certificate");
        out.field("seal", seal)
            .field("basis", basis)
            .field("policy", policy)
            .field("windows", windows)
            .field("span", span)
            .field("results", results)
            .field("coverage", coverage)
            .field("support", support)
            .field("pieces", pieces);
        if let Some(rhythm) = rhythm {
            out.field("rhythm", rhythm);
        }
        out.finish()
    }
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
    /// The windows between a rhythm's edges, where the policy states them.
    pub fn rhythm(&self) -> Option<&RhythmResult> {
        self.rhythm.as_ref()
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
    /// inventory change over a window between a rhythm's edges, past its rate times the window's length
    EdgeDrift,
    /// a stretch with no edge, or from the last window's close to the head, longer than the policy allows
    RhythmStopped,
    /// a clock the policy names whose phase the record's body declares no rhythm of
    UndeclaredRhythm,
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

/// 2^(e - 105), where 2^e is the power of two at or below the total's larger part; never below the smallest float,
/// and zero for a total that is not finite.
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

/// The count enters as a float: exact to 2^53, within one part in 2^53 above that, and held at `i64::MAX` past it.
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

enum Outcome {
    Holds,
    Drifts,
    Open,
}
struct Balance {
    closure: Compensated,
    place: Compensated,
    drift: Compensated,
    outcome: Outcome,
}

fn balance(
    limits: &QuantityPolicy,
    (closing, opening): (Compensated, Compensated),
    (inflow, outflow): (Compensated, Compensated),
    last_places: u64,
) -> Balance {
    let change = closing.minus(opening);
    let closure = change.minus(inflow.minus(outflow)).abs();
    let gross = inflow.plus(outflow);
    let place = last_place(closing);
    let allowed = admitted(
        limits.absolute.plus(gross.times(&limits.relative)),
        place,
        last_places,
    );
    let drift = change.abs();
    let outcome = if !closure.at_most(allowed) {
        Outcome::Open
    } else if !drift.at_most(limits.drift.unwrap_or(allowed)) {
        Outcome::Drifts
    } else {
        Outcome::Holds
    };
    Balance {
        closure,
        place,
        drift,
        outcome,
    }
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
            let flows = (inflow[index], outflow[index]);
            let balance = balance(limits, inventory[index], flows, last_places);
            let failure = |check, residual| CertificateFailure {
                check,
                quantity: Some(quantity),
                window: Some(interval),
                residual,
            };
            match balance.outcome {
                Outcome::Open => return Err(failure(CertificateCheck::Closure, balance.closure)),
                Outcome::Drifts if drifted.is_none() => {
                    drifted = Some(failure(CertificateCheck::Drift, balance.drift));
                }
                Outcome::Drifts | Outcome::Holds => {}
            }
            windows.push(Window {
                quantity: at,
                interval,
                closure: balance.closure,
                last_places,
                place: balance.place,
                drift: balance.drift,
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

enum Need {
    Trailing,
    Leading,
    Finish,
}

struct Rhythm<'p> {
    limits: &'p EdgePolicy,
    rhythm: RhythmId,
    admission: u64,
    early: bool,
    since: u64,
    open: Vec<(u64, Vec<Compensated>)>,
    judged: u64,
    last_close: Option<u64>,
    widest: Option<Interval>,
    worst: Vec<Option<EdgeResult>>,
    stopped: Option<CertificateFailure>,
    drifted: Option<CertificateFailure>,
}
impl<'p> Rhythm<'p> {
    fn new(limits: &'p EdgePolicy, rhythm: RhythmId, admission: u64) -> Self {
        Self {
            limits,
            rhythm,
            admission,
            early: limits.rates.iter().any(|rate| rate.settling.is_some()),
            since: admission,
            open: Vec::new(),
            judged: 0,
            last_close: None,
            widest: None,
            worst: vec![None; limits.rates.len()],
            stopped: None,
            drifted: None,
        }
    }
    fn arrive(&mut self, segment: &HistorySegment) {
        if !segment.ends_on_edge(self.rhythm) {
            return;
        }
        let at = segment.interval().end().nanoseconds();
        self.stretch(self.since, at);
        self.since = at;
        let ends = (self.limits.rates.iter()).map(|rate| {
            let inventory = segment.inventory(rate.quantity).on(segment.interval());
            inventory.and_then(|ends| ends.last().copied())
        });
        let Some(values) = ends.collect::<Option<Vec<_>>>() else {
            self.stopped.get_or_insert(of_history(CertificateCheck::Coverage));
            return;
        };
        let length = self.limits.windows.length_ns.get();
        let closing = (self.open.iter())
            .take_while(|(opened, _)| at - opened >= length)
            .count();
        let closed: Vec<_> = self.open.drain(..closing).collect();
        for (opened, before) in closed {
            self.judge((opened, &before), (at, &values));
        }
        if self.early || at - self.admission >= self.limits.windows.settling_ns {
            self.open.push((at, values));
        }
    }
    fn judge(&mut self, (opened, before): (u64, &[Compensated]), (at, after): (u64, &[Compensated])) {
        let Some(window) = Interval::new(SimTime::from_ns(opened), SimTime::from_ns(at)) else {
            return;
        };
        let settled = opened - self.admission >= self.limits.windows.settling_ns;
        let length = window.length_ns();
        let whole = i64::try_from(length / SECOND).unwrap_or(i64::MAX);
        let rates = self.limits.rates.iter().zip(before.iter().zip(after));
        for ((rate, (&start, &end)), worst) in rates.zip(&mut self.worst) {
            let per_second = match (settled, rate.settling) {
                (false, Some(settling)) => settling,
                _ => rate.settled,
            };
            let rest = Compensated::between(Compensated::ZERO, per_second, length % SECOND, SECOND);
            let allowance = per_second.scaled(whole).plus(rest);
            let change = end.minus(start).abs();
            if !change.at_most(allowance) && self.drifted.is_none() {
                self.drifted = Some(CertificateFailure {
                    check: CertificateCheck::EdgeDrift,
                    quantity: Some(rate.quantity),
                    window: Some(window),
                    residual: change,
                });
            }
            let judged = EdgeResult {
                quantity: rate.quantity,
                change,
                allowance,
                window,
            };
            let held = worst.get_or_insert(judged);
            if !change.minus(allowance).at_most(held.change.minus(held.allowance)) {
                *held = judged;
            }
        }
        self.judged += 1;
        self.last_close = Some(at);
    }
    fn stretch(&mut self, from: u64, to: u64) {
        let Some(stretch) = Interval::new(SimTime::from_ns(from), SimTime::from_ns(to)) else {
            return;
        };
        if stretch.length_ns() > self.limits.windows.widest_ns && self.stopped.is_none() {
            self.stopped = Some(CertificateFailure {
                check: CertificateCheck::RhythmStopped,
                quantity: None,
                window: Some(stretch),
                residual: Compensated::ZERO,
            });
        }
        if self
            .widest
            .is_none_or(|widest| stretch.length_ns() > widest.length_ns())
        {
            self.widest = Some(stretch);
        }
    }
    fn finish(mut self, head: u64) -> Result<RhythmResult, CertificateFailure> {
        self.stretch(self.last_close.unwrap_or(self.admission), head);
        if let Some(failure) = self.stopped.or(self.drifted) {
            return Err(failure);
        }
        Ok(RhythmResult {
            windows: self.limits.windows,
            judged: self.judged,
            results: self.worst.into_iter().flatten().collect(),
            widest: self.widest.ok_or(of_history(CertificateCheck::Coverage))?,
            last_close: self.last_close.map(SimTime::from_ns),
        })
    }
}

struct Windows<'p> {
    policy: &'p CertificatePolicy,
    routes: &'p [HeldRoute],
    first: u64,
    last: u64,
    from: u64,
    worst: Vec<Option<QuantityResult>>,
    most: Vec<Option<[(Compensated, Interval); 2]>>,
    drifted: Option<CertificateFailure>,
    failed: Option<CertificateFailure>,
    pieces: u64,
    done: bool,
    rhythm: Option<Rhythm<'p>>,
}
impl<'p> Windows<'p> {
    fn new(
        policy: &'p CertificatePolicy,
        routes: &'p [HeldRoute],
        (first, last): (u64, u64),
        rhythm: Option<RhythmId>,
    ) -> Self {
        let edges = policy.edges.as_ref().zip(rhythm);
        Self {
            policy,
            routes,
            first,
            last,
            from: first,
            worst: vec![None; policy.quantities.len()],
            most: vec![None; routes.len()],
            drifted: None,
            failed: None,
            pieces: 0,
            done: false,
            rhythm: edges.map(|(limits, rhythm)| Rhythm::new(limits, rhythm, first)),
        }
    }
    fn arrive(&mut self, segment: &HistorySegment) {
        if let Some(rhythm) = &mut self.rhythm {
            rhythm.arrive(segment);
        }
    }
    fn offer(
        &mut self,
        (behind, trailing): (u64, &HistorySegment),
        (ahead, leading): (u64, &HistorySegment),
    ) -> Need {
        let length = self.policy.windows.window_ns.get();
        let end = |segment: &HistorySegment| segment.interval().end().nanoseconds();
        let one_window = self.first == self.last;
        while !self.done {
            if end(trailing) <= self.from {
                return Need::Trailing;
            }
            let reaches = if one_window {
                self.first + length <= end(leading)
            } else {
                self.from + length < end(leading)
            };
            if !reaches {
                return Need::Leading;
            }
            let steps = behind.abs_diff(ahead) + 1;
            let to = (end(trailing).min(end(leading) - length)).min(self.last);
            let stretch = if one_window {
                Stretch::FromAdmission { at: self.first }
            } else {
                Stretch::Between { from: self.from, to }
            };
            self.pieces += 1;
            match pair(self.policy, self.routes, trailing, leading, (stretch, steps)) {
                Err(failure) => (self.failed, self.done) = (Some(failure), true),
                Ok(pair) => {
                    self.drifted = self.drifted.or(pair.drifted);
                    self.keep_the_earliest_worst(pair.windows);
                    self.keep_the_earliest_most(pair.carried);
                }
            }
            self.from = to;
            self.done |= to == self.last;
        }
        Need::Finish
    }
    fn keep_the_earliest_worst(&mut self, windows: Vec<Window>) {
        let policy = self.policy;
        for window in windows {
            let Window {
                quantity,
                interval,
                closure,
                last_places,
                place,
                drift,
            } = window;
            let limits = &policy.quantities[quantity];
            let result = self.worst[quantity].get_or_insert(QuantityResult {
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
            if !closure.at_most(result.worst_closure) {
                result.worst_closure = closure;
                result.closure_window = interval;
                result.closure_last_places = last_places;
                result.closure_place = place;
            }
            if !drift.at_most(result.worst_drift) {
                result.worst_drift = drift;
                result.drift_window = interval;
            }
        }
    }
    fn keep_the_earliest_most(&mut self, carried: Vec<Carried>) {
        for window in carried {
            let Carried {
                route,
                interval,
                inflow,
                outflow,
            } = window;
            let flows = [inflow, outflow];
            match &mut self.most[route] {
                Some(held) => {
                    for (held, flow) in held.iter_mut().zip(flows) {
                        if !flow.at_most(held.0) {
                            *held = (flow, interval);
                        }
                    }
                }
                unseen => *unseen = Some(flows.map(|flow| (flow, interval))),
            }
        }
    }
    fn finish(self, view: &ConservationView<'_>, span: Interval) -> Result<Certificate, CertificateFailure> {
        if let Some(failure) = self.failed.or(self.drifted) {
            return Err(failure);
        }
        let coverage = of_history(CertificateCheck::Coverage);
        let results = self.worst.into_iter().collect::<Option<Vec<_>>>();
        let support = (self.routes.iter().zip(self.most)).map(|(&route, most)| {
            let [(most_inflow, inflow_window), (most_outflow, outflow_window)] = most?;
            Some(HeldSupport {
                route,
                most_inflow,
                inflow_window,
                most_outflow,
                outflow_window,
            })
        });
        let support = support.collect::<Option<Vec<_>>>().ok_or(coverage)?;
        let results = results.ok_or(coverage)?;
        let head = span.end().nanoseconds();
        let rhythm = self.rhythm.map(|rhythm| rhythm.finish(head)).transpose()?;
        Ok(Certificate {
            seal: view.seal(),
            basis: self.policy.basis,
            policy: self.policy.digest(),
            windows: self.policy.windows,
            span,
            results,
            coverage: view.basis().coverage().clone(),
            support,
            pieces: self.pieces,
            rhythm,
        })
    }
}

/// Closure failures win over drift, so a `Drift` failure says the ledger closed everywhere and the body
/// still moved. It reads the history in time order and reports the earliest failing window, and the earliest of
/// equal worst windows. The windows between a rhythm's edges rank after every fixed window: a stretch with no edge
/// past its bound, the earliest, then a window between edges past its allowance, by earliest close, then oldest
/// opening, then basis order.
pub fn certify(
    view: &ConservationView<'_>,
    policy: &CertificatePolicy,
) -> Result<Certificate, CertificateFailure> {
    let coverage = of_history(CertificateCheck::Coverage);
    if policy.basis != view.basis().digest() {
        return Err(coverage);
    }
    let rhythm = match &policy.edges {
        Some(limits) => {
            let clock = limits.windows.clock;
            let declared = view.rhythms().find(|&(_, phase)| phase == clock);
            let undeclared = of_history(CertificateCheck::UndeclaredRhythm);
            Some(declared.ok_or(undeclared)?.0)
        }
        None => None,
    };
    let span = view.span().ok_or(coverage)?;
    if span.length_ns() < policy.windows.coverage_ns {
        return Err(coverage);
    }
    let ExpectedDrift::Zero = policy.expected;
    let history = |error| of_history(CertificateCheck::History(error));
    let order = view.in_time_order().map_err(history)?;
    let length = policy.windows.window_ns.get();
    let first = span.start().nanoseconds();
    let last = if policy.windows.coverage_ns == length {
        first
    } else {
        span.end().nanoseconds() - length
    };
    let routes = view.basis().coverage().held_routes();
    let mut windows = Windows::new(policy, routes, (first, last), rhythm);
    let read = |cursor: &mut OldestFirst<'_>| cursor.next().ok_or(coverage)?.map_err(history);
    let mut early = order.from(span.start());
    let mut late = match rhythm {
        Some(_) => order.from(span.start()),
        None => order.from(SimTime::from_ns(first + length)),
    };
    let (mut trailing, mut leading) = (read(&mut early)?, read(&mut late)?);
    windows.arrive(&leading.1);
    loop {
        match windows.offer((trailing.0, &trailing.1), (leading.0, &leading.1)) {
            Need::Trailing => trailing = read(&mut early)?,
            Need::Leading => {
                leading = read(&mut late)?;
                windows.arrive(&leading.1);
            }
            Need::Finish => break,
        }
    }
    if rhythm.is_some() {
        for segment in late {
            windows.arrive(&segment.map_err(history)?.1);
        }
    }
    windows.finish(view, span)
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
