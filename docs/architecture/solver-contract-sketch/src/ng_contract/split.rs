//! Split qualification is measured evidence. It is produced only by `SplitMeasurement` over the
//! four-family split metric (state, exchange integral, observable, event time; CB numerics.rs
//! 122-260), with the reference's own self-convergence error included. It is never a byte array.
use super::{AdmissionError, Digest, SimTime};
use core::marker::PhantomData;

mod sealed {
    pub trait MetricFamily {}
}
pub trait MetricFamily: sealed::MetricFamily {}
pub enum StateFamily {}
pub enum ExchangeFamily {}
pub enum ObservableFamily {}
pub enum EventTimeFamily {}
impl sealed::MetricFamily for StateFamily {}
impl MetricFamily for StateFamily {}
impl sealed::MetricFamily for ExchangeFamily {}
impl MetricFamily for ExchangeFamily {}
impl sealed::MetricFamily for ObservableFamily {}
impl MetricFamily for ObservableFamily {}
impl sealed::MetricFamily for EventTimeFamily {}
impl MetricFamily for EventTimeFamily {}

/// A shared physical sample grid; both traces of a comparison must use the same one.
pub struct TraceGrid {
    _start: SimTime,
    _period_ns: u64,
    _samples: u32,
}
impl TraceGrid {
    pub fn new(_start: SimTime, _period_ns: u64, _samples: u32) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub struct FamilySamples<F: MetricFamily> {
    _keys: Vec<Digest>,
    _values: Vec<f64>,
    _family: PhantomData<F>,
}
impl<F: MetricFamily> FamilySamples<F> {
    pub fn new(_keys: Vec<Digest>, _values: Vec<f64>) -> Result<Self, AdmissionError> {
        todo!()
    }
}
/// One run's samples on a grid, all four families required, bound to that run's identity.
/// Produced only by `PatientRecord::split_trace` from accepted history; there is no constructor
/// from samples, so a qualification cannot rest on invented traces.
pub struct SplitTrace {
    _identity: Digest,
    _grid: TraceGrid,
    _state: FamilySamples<StateFamily>,
    _exchange: FamilySamples<ExchangeFamily>,
    _observable: FamilySamples<ObservableFamily>,
    _event_time: FamilySamples<EventTimeFamily>,
}
/// The coupled reference's own error, measured against a refined reference; never assumed zero.
/// It keeps the binding (identity, grid and samples digest) of the reference trace it was measured
/// on, so an easier run's small error cannot stand in for this reference's.
pub struct ReferenceError {
    _reference: Digest,
    _per_family: [f64; 4],
}
impl ReferenceError {
    pub fn self_convergence(_reference: &SplitTrace, _refined: &SplitTrace) -> Result<Self, SplitRejected> {
        todo!()
    }
}
pub struct SplitBudget {
    _scales: [f64; 4],
    _reference_fraction: f64,
}
impl SplitBudget {
    pub fn new(_scales: [f64; 4], _reference_fraction: f64) -> Result<Self, AdmissionError> {
        todo!()
    }
}
pub struct SplitMeasurement {
    _reference: SplitTrace,
    _split: SplitTrace,
    _reference_error: ReferenceError,
    _budget: SplitBudget,
}
impl SplitMeasurement {
    /// Refuses traces on different grids, models, packets, seeds or boundary schedules, and a
    /// reference error that was not measured on `reference`.
    pub fn new(
        _reference: SplitTrace,
        _split: SplitTrace,
        _reference_error: ReferenceError,
        _budget: SplitBudget,
    ) -> Result<Self, SplitRejected> {
        todo!()
    }
    /// The only producer of `SplitQualification`: worst normalized error per family plus the
    /// reference error must fit the budget.
    pub fn qualify(self) -> Result<SplitQualification, SplitRejected> {
        todo!()
    }
}
pub struct SplitAssessment {
    _maxima: [f64; 4],
    _reference_error: f64,
    _worst_total: f64,
}
/// Bound to the measured plan, model, packet and validated domain.
pub struct SplitQualification {
    _plan: Digest,
    _model: Digest,
    _packet: Digest,
    _domain: Digest,
    _assessment: SplitAssessment,
}
impl SplitQualification {
    pub fn assessment(&self) -> &SplitAssessment {
        todo!()
    }
}
pub struct SplitRejected {
    _reason: String,
}
