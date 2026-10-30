#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParameterKey {
    key: &'static str,
}
impl ParameterKey {
    pub const fn new(key: &'static str) -> Self {
        Self { key }
    }
    pub(crate) fn key(&self) -> &'static str {
        self.key
    }
}

/// What a validity range rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Basis {
    /// A synthetic fixture's range, by name.
    Fixture(&'static str),
}
