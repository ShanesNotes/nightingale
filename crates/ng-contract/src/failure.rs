use crate::AnatomyDefect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    NonFinite,
    Anatomy(AnatomyDefect),
}
impl From<AnatomyDefect> for AdmissionError {
    fn from(defect: AnatomyDefect) -> Self {
        Self::Anatomy(defect)
    }
}
