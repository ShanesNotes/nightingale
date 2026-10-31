use crate::body::Layout;
use crate::chance::holds_a_period;
use crate::identity::{Canonical, Seal};
use crate::law::Op;
use crate::quantity::dimension;
use crate::{
    AdmissionError, Binding, Compensated, CompiledBody, Digest, Dimension, LawName, Measure, ModelHash,
    ParameterKey, RowKind,
};
use core::marker::PhantomData;

mod sealed {
    pub trait PacketKind {
        const TAG: u64;
    }
}
pub trait PacketKind: sealed::PacketKind {}
/// Obviously synthetic test values with no citations.
pub enum SyntheticFixture {}
impl sealed::PacketKind for SyntheticFixture {
    const TAG: u64 = 0;
}
impl PacketKind for SyntheticFixture {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketHash {
    digest: Digest,
}
impl PacketHash {
    pub(crate) fn of_digest(digest: Digest) -> Self {
        Self { digest }
    }
    pub fn digest(&self) -> Digest {
        self.digest
    }
}

struct ParameterEntry {
    key: ParameterKey,
    binding: Binding,
    dimension: String,
    bits: u64,
    note: &'static str,
}
struct StartEntry {
    row: RowKind,
    dimension: String,
    bits: u64,
    note: &'static str,
}
struct ModeEntry {
    variable: LawName,
    value: &'static str,
    note: &'static str,
}
pub struct ParameterPacket<K: PacketKind> {
    parameters: Vec<ParameterEntry>,
    starts: Vec<StartEntry>,
    modes: Vec<ModeEntry>,
    kind: PhantomData<K>,
}
impl<K: PacketKind> ParameterPacket<K> {
    pub fn new() -> Self {
        Self {
            parameters: Vec::new(),
            starts: Vec::new(),
            modes: Vec::new(),
            kind: PhantomData,
        }
    }
    fn hash(&self) -> PacketHash {
        let Self {
            parameters,
            starts,
            modes,
            kind: _,
        } = self;
        let mut parameters: Vec<&ParameterEntry> = parameters.iter().collect();
        parameters.sort_by_key(|entry| (entry.key, entry.binding));
        let mut starts: Vec<&StartEntry> = starts.iter().collect();
        starts.sort_by_key(|entry| entry.row);
        let mut out = Canonical::new("ng-contract packet 1");
        out.number(<K as sealed::PacketKind>::TAG);
        out.length(parameters.len());
        for entry in parameters {
            let ParameterEntry {
                key,
                binding,
                dimension,
                bits,
                note,
            } = entry;
            out.text(key.key());
            binding.seal(&mut out);
            out.text(dimension);
            out.number(*bits);
            out.text(note);
        }
        out.length(starts.len());
        for entry in starts {
            let StartEntry {
                row,
                dimension,
                bits,
                note,
            } = entry;
            row.seal(&mut out);
            out.text(dimension);
            out.number(*bits);
            out.text(note);
        }
        let mut modes: Vec<&ModeEntry> = modes.iter().collect();
        modes.sort_by_key(|entry| entry.variable);
        out.length(modes.len());
        for ModeEntry {
            variable,
            value,
            note,
        } in modes
        {
            variable.seal(&mut out);
            out.text(value);
            out.text(note);
        }
        PacketHash { digest: out.digest() }
    }
}
impl<K: PacketKind> Default for ParameterPacket<K> {
    fn default() -> Self {
        Self::new()
    }
}
impl ParameterPacket<SyntheticFixture> {
    /// One parameter's value where it binds: the body, a site or an edge, or as a default a site class or
    /// the network. Refuses the same key and binding twice.
    pub fn synthetic<U: Dimension>(
        &mut self,
        key: ParameterKey,
        binding: Binding,
        value: Measure<U>,
        note: &'static str,
    ) -> Result<(), AdmissionError> {
        if (self.parameters.iter()).any(|entry| entry.key == key && entry.binding == binding) {
            return Err(AdmissionError::DuplicateParameter { key, binding });
        }
        self.parameters.push(ParameterEntry {
            key,
            binding,
            dimension: dimension::<U>(),
            bits: value.bits(),
            note,
        });
        Ok(())
    }
    /// One row's start value, named as the compiled body names the row. The start values live in the
    /// packet, so its hash and the run identity cover them.
    /// Refuses the same row twice.
    pub fn start<U: Dimension>(
        &mut self,
        row: RowKind,
        value: Measure<U>,
        note: &'static str,
    ) -> Result<(), AdmissionError> {
        if self.starts.iter().any(|entry| entry.row == row) {
            return Err(AdmissionError::DuplicateStart { row });
        }
        self.starts.push(StartEntry {
            row,
            dimension: dimension::<U>(),
            bits: value.bits(),
            note,
        });
        Ok(())
    }
    pub fn start_mode(
        &mut self,
        variable: LawName,
        value: &'static str,
        note: &'static str,
    ) -> Result<(), AdmissionError> {
        if self.modes.iter().any(|entry| entry.variable == variable) {
            return Err(AdmissionError::DuplicateStartMode { variable });
        }
        self.modes.push(ModeEntry {
            variable,
            value,
            note,
        });
        Ok(())
    }
}

/// Packet values in compiled parameter order with the packet's hash.
pub struct BoundParameters {
    pub(crate) model: ModelHash,
    pub(crate) packet: PacketHash,
    pub(crate) values: Vec<f64>,
    pub(crate) starts: Vec<f64>,
    pub(crate) modes: Vec<usize>,
}
impl BoundParameters {
    pub fn packet_hash(&self) -> PacketHash {
        self.packet
    }
    pub fn digest(&self) -> Digest {
        let Self {
            model,
            packet,
            values,
            starts,
            modes,
        } = self;
        let mut out = Canonical::new("ng-contract bound parameters 1");
        model.digest().seal(&mut out);
        packet.digest().seal(&mut out);
        values.seal(&mut out);
        starts.seal(&mut out);
        modes.seal(&mut out);
        out.digest()
    }
}

impl CompiledBody {
    /// A firing leaves algebraic values as they were, and nothing here solves them again.
    fn unrun_mode(&self) -> Option<LawName> {
        let Layout {
            rows,
            modes,
            guards,
            hazards,
            ..
        } = &self.layout;
        let algebraic = rows.iter().any(|row| matches!(row.kind, RowKind::Algebraic(_)));
        let plain = |variable| guards.iter().any(|guard| guard.variable == variable);
        let moved = |at| hazards.iter().any(|hazard| hazard.variable == at);
        (modes.iter().enumerate())
            .find(|&(at, mode)| algebraic || plain(mode.variable) || !moved(at))
            .map(|(_, mode)| mode.variable)
    }
    /// Finding 15's partial check. A literal is zero, so only `exp` and `logistic` make a nonzero constant from
    /// literals, and `divide` makes one from two equal nodes.
    fn unbacked_constant(&self) -> Option<LawName> {
        let unbacked = |code: &[Op]| {
            let mut literal = vec![false; code.len()];
            code.iter().enumerate().any(|(at, op)| {
                literal[at] = match op {
                    Op::Zero(_) => true,
                    Op::Input(_) => false,
                    _ => op.operands().iter().all(|&operand| literal[operand]),
                };
                match op {
                    Op::Exp(_) | Op::Logistic(_) => literal[at],
                    Op::Divide([a, b]) => code[*a] == code[*b],
                    _ => false,
                }
            })
        };
        (self.layout.programs.iter())
            .find(|program| unbacked(&program.code.ops))
            .map(|program| program.law)
    }
    pub(crate) fn unheld_period(&self, values: &[f64]) -> Option<ParameterKey> {
        (self.layout.forcings.iter())
            .map(|forcing| forcing.period)
            .find(|period| !holds_a_period(values[period.index()]))
            .map(|period| self.layout.parameters[period.index()].key)
    }
    /// Parses a packet against the compiled layout. Each refusal is by name. What cannot run yet is refused
    /// first, and a mode's start next, because which laws hold decides what every later value means.
    pub fn bind<K: PacketKind>(&self, packet: ParameterPacket<K>) -> Result<BoundParameters, AdmissionError> {
        if let Some(variable) = self.unrun_mode() {
            return Err(AdmissionError::ModesNotYetRun { variable });
        }
        if let Some(law) = self.unbacked_constant() {
            return Err(AdmissionError::UnbackedConstant { law });
        }
        let mut modes = Vec::with_capacity(self.layout.modes.len());
        for mode in &self.layout.modes {
            let variable = mode.variable;
            let Some(entry) = packet.modes.iter().find(|entry| entry.variable == variable) else {
                return Err(AdmissionError::MissingStartMode { variable });
            };
            let value = entry.value;
            let Some(at) = mode.values.iter().position(|known| *known == value) else {
                return Err(AdmissionError::UnknownStartMode { variable, value });
            };
            modes.push(at);
        }
        if let Some(entry) = (packet.modes.iter())
            .filter(|entry| {
                !self
                    .layout
                    .modes
                    .iter()
                    .any(|mode| mode.variable == entry.variable)
            })
            .min_by_key(|entry| entry.variable)
        {
            return Err(AdmissionError::UnknownStartMode {
                variable: entry.variable,
                value: entry.value,
            });
        }
        let mut read = vec![false; packet.parameters.len()];
        let mut values = Vec::with_capacity(self.layout.parameters.len());
        for slot in &self.layout.parameters {
            let (key, binding) = (slot.key, slot.binding);
            let found = |wanted: Binding| {
                (packet.parameters.iter()).position(|entry| entry.key == key && entry.binding == wanted)
            };
            let at = found(binding).or_else(|| slot.default.and_then(found));
            let Some(at) = at else {
                return Err(AdmissionError::UnboundParameter { key, binding });
            };
            read[at] = true;
            let entry = &packet.parameters[at];
            if entry.dimension != slot.dimension {
                return Err(AdmissionError::ParameterDimension { key, binding });
            }
            let value = f64::from_bits(entry.bits);
            if !slot.admissible.holds(Compensated::exact(value)) {
                return Err(AdmissionError::ParameterOutsideSet { key, binding });
            }
            values.push(value);
        }
        if let Some(key) = self.unheld_period(&values) {
            return Err(AdmissionError::ForcingPeriod { key });
        }
        if let Some(entry) = (packet.parameters.iter().zip(&read))
            .filter(|(_, read)| !**read)
            .map(|(entry, _)| entry)
            .min_by_key(|entry| (entry.key, entry.binding))
        {
            return Err(AdmissionError::UnknownParameter {
                key: entry.key,
                binding: entry.binding,
            });
        }
        let mut starts = Vec::with_capacity(self.layout.rows.len());
        for row in &self.layout.rows {
            let kind = row.kind;
            let Some(entry) = packet.starts.iter().find(|entry| entry.row == kind) else {
                return Err(AdmissionError::MissingStart { row: kind });
            };
            if entry.dimension != row.dimension {
                return Err(AdmissionError::StartDimension { row: kind });
            }
            let value = f64::from_bits(entry.bits);
            if !row.admissible.holds(Compensated::exact(value)) {
                return Err(AdmissionError::StartOutsideSet { row: kind });
            }
            starts.push(value);
        }
        if let Some(entry) = (packet.starts.iter())
            .filter(|entry| !self.layout.rows.iter().any(|row| row.kind == entry.row))
            .min_by_key(|entry| entry.row)
        {
            return Err(AdmissionError::UnknownStart { row: entry.row });
        }
        Ok(BoundParameters {
            model: self.model_hash(),
            packet: packet.hash(),
            values,
            starts,
            modes,
        })
    }
}
