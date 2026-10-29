use crate::AnatomyDefect;

/// Why a declaration was refused before any run, naming the key it concerns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    /// A measure that is NaN or infinite.
    NonFinite,
    Anatomy(AnatomyDefect),
}
impl From<AnatomyDefect> for AdmissionError {
    fn from(defect: AnatomyDefect) -> Self {
        Self::Anatomy(defect)
    }
}
