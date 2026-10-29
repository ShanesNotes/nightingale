use crate::AdmissionError;
use core::marker::PhantomData;

/// Length, the dimension of a position in the body frame.
pub enum Length {}

/// A finite value of dimension `U` in its canonical unit.
pub struct Measure<U> {
    value: f64,
    unit: PhantomData<U>,
}
impl<U> Measure<U> {
    pub fn new(canonical: f64) -> Result<Self, AdmissionError> {
        if !canonical.is_finite() {
            return Err(AdmissionError::NonFinite);
        }
        // Adding +0.0 turns -0.0 into +0.0, so equal values have equal bits in a digest.
        Ok(Self {
            value: canonical + 0.0,
            unit: PhantomData,
        })
    }
    pub(crate) fn bits(&self) -> u64 {
        self.value.to_bits()
    }
}
