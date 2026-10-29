/// The stable key a parameter is declared and bound under; its value arrives later from a packet.
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
