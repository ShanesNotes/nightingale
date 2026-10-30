//! What a stage produces.
use crate::Digest;

/// One stage's evaluation: bound to its ticket and layout and carrying a copy of its point, so it outlives
/// the workspace. Only `CoupledProblem::evaluate` makes one.
pub struct StageEvaluation {
    pub(crate) layout: Digest,
    pub(crate) ticket: Digest,
    pub(crate) offset_ns: u64,
    pub(crate) unknowns: Vec<f64>,
    pub(crate) derivative: Vec<f64>,
    pub(crate) columns: Vec<f64>,
    pub(crate) rows: Vec<f64>,
    pub(crate) values: Vec<f64>,
    pub(crate) residuals: Vec<f64>,
}
impl StageEvaluation {
    /// `F` per row: what a Newton core drives to zero. With the tangent at zero, a differential row's
    /// value is minus its rate, so an explicit core reads its rates here too.
    pub fn residuals(&self) -> &[f64] {
        &self.residuals
    }
}
