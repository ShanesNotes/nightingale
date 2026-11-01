use crate::body::{Bound, Layout, ParameterId, Target};
use crate::chance::{holds_a_period, sampled_unit};
use crate::identity::{Canonical, Seal};
use crate::law::Op;
use crate::provenance::seal_trace;
use crate::quantity::dimension;
use crate::{
    AdmissionError, Anchor, AnchorId, AnchorStatistic, Binding, Citation, Compensated, CompiledBody, Digest,
    Dimension, LawName, Measure, ModelHash, PacketSeed, ParameterKey, RateBand, RowId, RowKind, ValueName,
    ValueProvenance,
};
use core::marker::PhantomData;
use core::num::{NonZeroU32, NonZeroU64};

mod sealed {
    pub trait PacketKind {
        const TAG: u64;
        /// What backs a value in this kind of packet. It is the one difference between the kinds.
        type Trace;
        fn words(trace: &Self::Trace) -> [&'static str; 2];
    }
}
pub trait PacketKind: sealed::PacketKind {}
/// Obviously synthetic test values with no citations.
pub enum SyntheticFixture {}
impl sealed::PacketKind for SyntheticFixture {
    const TAG: u64 = 0;
    type Trace = &'static str;
    fn words(trace: &Self::Trace) -> [&'static str; 2] {
        [trace, ""]
    }
}
impl PacketKind for SyntheticFixture {}
/// Clinical values with citations. No value that names no source enters one, and no crate here builds one
/// yet: what a first reviewed packet needs beyond these lands with it.
pub enum Reviewed {}
impl sealed::PacketKind for Reviewed {
    const TAG: u64 = 1;
    type Trace = Citation;
    fn words(trace: &Self::Trace) -> [&'static str; 2] {
        [trace.source, trace.locator]
    }
}
impl PacketKind for Reviewed {}

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

/// One pass of the periodic goal from the anchor moment, in equal intervals. The packet holds it, because it
/// shapes the start state and the packet's hash is what a run's identity covers (I2). A cycle-mean anchor reads
/// it, so a packet with such an anchor and no cycle is refused at `bind`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cycle {
    period_ns: u64,
    intervals: u32,
}
impl Cycle {
    /// Refuses a period `intervals` does not divide, so every node sits on the clock.
    pub fn new(period_ns: NonZeroU64, intervals: NonZeroU32) -> Result<Self, AdmissionError> {
        let (period_ns, intervals) = (period_ns.get(), intervals.get());
        if !period_ns.is_multiple_of(u64::from(intervals)) {
            return Err(AdmissionError::UnevenCycle);
        }
        Ok(Self { period_ns, intervals })
    }
    pub(crate) fn period_ns(self) -> u64 {
        self.period_ns
    }
    pub(crate) fn intervals(self) -> u64 {
        u64::from(self.intervals)
    }
    pub(crate) fn step_ns(self) -> u64 {
        self.period_ns / u64::from(self.intervals)
    }
}
impl Seal for Cycle {
    fn seal(&self, out: &mut Canonical) {
        let Self { period_ns, intervals } = self;
        out.number(*period_ns);
        out.number(u64::from(*intervals));
    }
}

struct ParameterEntry {
    key: ParameterKey,
    binding: Binding,
    dimension: String,
    bits: u64,
    provenance: ValueProvenance,
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
struct AnchorEntry {
    id: AnchorId,
    row: RowKind,
    method: &'static str,
    statistic: AnchorStatistic,
    dimension: String,
    bits: u64,
    trace: [&'static str; 2],
}
/// A parameter's range in a stated population: an interval, the form `authored` and `sampled` read. A mean and
/// spread or quantiles are one more constructor on `ParameterPacket::population` when a source gives one.
struct PopulationEntry {
    key: ParameterKey,
    dimension: String,
    lower: f64,
    upper: f64,
    trace: [&'static str; 2],
}
pub struct ParameterPacket<K: PacketKind> {
    cycle: Option<Cycle>,
    parameters: Vec<ParameterEntry>,
    starts: Vec<StartEntry>,
    modes: Vec<ModeEntry>,
    anchors: Vec<AnchorEntry>,
    populations: Vec<PopulationEntry>,
    kind: PhantomData<K>,
}
impl<K: PacketKind> ParameterPacket<K> {
    /// A packet whose start state stands at one instant.
    pub fn new() -> Self {
        Self {
            cycle: None,
            parameters: Vec::new(),
            starts: Vec::new(),
            modes: Vec::new(),
            anchors: Vec::new(),
            populations: Vec::new(),
            kind: PhantomData,
        }
    }
    /// A packet whose start state returns after `cycle`: the periodic goal. Its start values are the guess
    /// at every node, and each conserved combination's total over them is what the solve cannot move.
    pub fn periodic(cycle: Cycle) -> Self {
        Self {
            cycle: Some(cycle),
            ..Self::new()
        }
    }
    /// An anchor of this packet's kind. Refuses a second anchor under one id.
    pub fn anchor<U: Dimension>(&mut self, anchor: Anchor<K, U>) -> Result<(), AdmissionError> {
        let Anchor { id, snapshot, trace } = anchor;
        if self.anchors.iter().any(|entry| entry.id == id) {
            return Err(AdmissionError::DuplicateAnchor { anchor: id });
        }
        self.anchors.push(AnchorEntry {
            id,
            row: snapshot.row,
            method: snapshot.method,
            statistic: snapshot.statistic,
            dimension: dimension::<U>(),
            bits: snapshot.value.bits(),
            trace: <K as sealed::PacketKind>::words(&trace),
        });
        Ok(())
    }
    /// `key`'s range in the patient's population. Refuses a reversed interval and a second record for a key.
    pub fn population<U: Dimension>(
        &mut self,
        key: ParameterKey,
        lower: Measure<U>,
        upper: Measure<U>,
        trace: K::Trace,
    ) -> Result<(), AdmissionError> {
        let repeated = self.populations.iter().any(|entry| entry.key == key);
        if repeated || lower.value() > upper.value() {
            return Err(AdmissionError::PopulationRange { key });
        }
        self.populations.push(PopulationEntry {
            key,
            dimension: dimension::<U>(),
            lower: lower.value(),
            upper: upper.value(),
            trace: <K as sealed::PacketKind>::words(&trace),
        });
        Ok(())
    }
    fn population_of(&self, key: ParameterKey) -> Result<&PopulationEntry, AdmissionError> {
        (self.populations.iter())
            .find(|entry| entry.key == key)
            .ok_or(AdmissionError::NoPopulation { key })
    }
    fn entry(&mut self, entry: ParameterEntry) -> Result<(), AdmissionError> {
        let (key, binding) = (entry.key, entry.binding);
        if (self.parameters.iter()).any(|known| known.key == key && known.binding == binding) {
            return Err(AdmissionError::DuplicateParameter { key, binding });
        }
        self.parameters.push(entry);
        Ok(())
    }
    /// A value drawn from `key`'s population record. The draw is a pure function of the seed, the key and the
    /// binding, so adding a parameter moves no other draw (I9). Reads the record, so it follows `population`.
    pub fn sampled(
        &mut self,
        key: ParameterKey,
        binding: Binding,
        seed: PacketSeed,
    ) -> Result<(), AdmissionError> {
        let record = self.population_of(key)?;
        let value = record.lower + sampled_unit(seed, key, binding) * (record.upper - record.lower);
        let dimension = record.dimension.clone();
        self.entry(ParameterEntry {
            key,
            binding,
            dimension,
            bits: value.to_bits(),
            provenance: ValueProvenance::Sampled,
            note: "",
        })
    }
    /// A value chosen inside `key`'s population record, with the reason. Refuses a value outside it and one in
    /// another dimension (I16).
    pub fn authored<U: Dimension>(
        &mut self,
        key: ParameterKey,
        binding: Binding,
        value: Measure<U>,
        reason: &'static str,
    ) -> Result<(), AdmissionError> {
        let record = self.population_of(key)?;
        if record.dimension != dimension::<U>() {
            return Err(AdmissionError::ParameterDimension { key, binding });
        }
        if value.value() < record.lower || value.value() > record.upper {
            return Err(AdmissionError::OutsidePopulation { key, binding });
        }
        self.entry(ParameterEntry {
            key,
            binding,
            dimension: dimension::<U>(),
            bits: value.bits(),
            provenance: ValueProvenance::Authored { reason },
            note: reason,
        })
    }
    /// A value an identity gives from other packet values. `bind` refuses an input with no entry, so the
    /// trace holds, and `30-data` recomputes a clinical identity.
    pub fn derived<U: Dimension>(
        &mut self,
        key: ParameterKey,
        binding: Binding,
        value: Measure<U>,
        identity: &'static str,
        inputs: &'static [ParameterKey],
    ) -> Result<(), AdmissionError> {
        self.entry(ParameterEntry {
            key,
            binding,
            dimension: dimension::<U>(),
            bits: value.bits(),
            provenance: ValueProvenance::Derived { identity, inputs },
            note: identity,
        })
    }
    /// A parameter admission fits so `anchor` holds. One fit names one anchor and `bind` refuses a second fit
    /// on it, so each fit is one unknown and one row and the system stays square. Every layout slot this entry
    /// fills takes the one fitted value.
    pub fn calibrated<U: Dimension>(
        &mut self,
        key: ParameterKey,
        binding: Binding,
        guess: Measure<U>,
        anchor: AnchorId,
    ) -> Result<(), AdmissionError> {
        self.entry(ParameterEntry {
            key,
            binding,
            dimension: dimension::<U>(),
            bits: guess.bits(),
            provenance: ValueProvenance::Calibrated { anchor },
            note: "",
        })
    }
    fn hash(&self) -> PacketHash {
        let Self {
            cycle,
            parameters,
            starts,
            modes,
            anchors,
            populations,
            kind: _,
        } = self;
        let mut parameters: Vec<&ParameterEntry> = parameters.iter().collect();
        parameters.sort_by_key(|entry| (entry.key, entry.binding));
        let mut starts: Vec<&StartEntry> = starts.iter().collect();
        starts.sort_by_key(|entry| entry.row);
        let mut out = Canonical::new("ng-contract packet 2");
        out.number(<K as sealed::PacketKind>::TAG);
        cycle.seal(&mut out);
        out.length(parameters.len());
        for entry in parameters {
            let ParameterEntry {
                key,
                binding,
                dimension,
                bits,
                provenance,
                note,
            } = entry;
            out.text(key.key());
            binding.seal(&mut out);
            out.text(dimension);
            out.number(*bits);
            provenance.seal(&mut out);
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
        let mut anchors: Vec<&AnchorEntry> = anchors.iter().collect();
        anchors.sort_by_key(|entry| entry.id);
        out.length(anchors.len());
        for entry in anchors {
            let AnchorEntry {
                id,
                row,
                method,
                statistic,
                dimension,
                bits,
                trace,
            } = entry;
            out.text(id.id());
            row.seal(&mut out);
            out.text(method);
            statistic.seal(&mut out);
            out.text(dimension);
            out.number(*bits);
            seal_trace(trace, &mut out);
        }
        let mut populations: Vec<&PopulationEntry> = populations.iter().collect();
        populations.sort_by_key(|entry| entry.key);
        out.length(populations.len());
        for entry in populations {
            let PopulationEntry {
                key,
                dimension,
                lower,
                upper,
                trace,
            } = entry;
            out.text(key.key());
            out.text(dimension);
            lower.seal(&mut out);
            upper.seal(&mut out);
            seal_trace(trace, &mut out);
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
        self.entry(ParameterEntry {
            key,
            binding,
            dimension: dimension::<U>(),
            bits: value.bits(),
            provenance: ValueProvenance::Fixture,
            note,
        })
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

/// One anchor in compiled terms.
pub(crate) struct BoundAnchor {
    pub(crate) id: AnchorId,
    pub(crate) row: RowId,
    pub(crate) method: &'static str,
    pub(crate) statistic: AnchorStatistic,
    pub(crate) value: f64,
}
/// One calibrated packet entry: one unknown and the anchor row that closes it.
pub(crate) struct Fit {
    pub(crate) slots: Vec<ParameterId>,
    pub(crate) anchor: usize,
    pub(crate) guess: f64,
    pub(crate) key: ParameterKey,
    pub(crate) binding: Binding,
}
impl Fit {
    pub(crate) fn name(&self) -> ValueName {
        ValueName::Parameter {
            key: self.key,
            binding: self.binding,
        }
    }
}
/// What this packet asks of admission, read from the packet at `bind`. The packet hash covers all of it, so
/// neither `digest` nor an image holds it, and a revision's parameters owe nothing: a resume never refits.
#[derive(Default)]
pub(crate) struct Owed {
    pub(crate) cycle: Option<Cycle>,
    pub(crate) anchors: Vec<BoundAnchor>,
    pub(crate) fits: Vec<Fit>,
    /// One per parameter entry, as the packet wrote it.
    pub(crate) entries: Vec<(ValueName, ValueProvenance, f64)>,
}

/// Packet values in compiled parameter order with the packet's hash.
pub struct BoundParameters {
    pub(crate) model: ModelHash,
    pub(crate) packet: PacketHash,
    pub(crate) values: Vec<f64>,
    pub(crate) starts: Vec<f64>,
    pub(crate) modes: Vec<usize>,
    pub(crate) owed: Owed,
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
            owed: _,
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
    /// A firing leaves algebraic values as they were, and nothing here solves them again. A hazard integral only
    /// grows until it fires, so it never returns over a cycle.
    fn unrun_mode(&self, periodic: bool) -> Option<LawName> {
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
            .find(|&(at, mode)| periodic || algebraic || plain(mode.variable) || !moved(at))
            .map(|(_, mode)| mode.variable)
    }
    /// Finding 15's partial check. A literal is zero, so only `exp` and `logistic` make a nonzero constant from
    /// literals, and `divide` makes one from two equal expressions. Equal means one number from `numbered`.
    fn unbacked_constant(&self) -> Option<LawName> {
        let unbacked = |code: &[Op]| {
            let numbers = numbered(code);
            let mut literal = vec![false; code.len()];
            code.iter().enumerate().any(|(at, op)| {
                literal[at] = match op {
                    Op::Zero(_) => true,
                    Op::Input(_) => false,
                    _ => op.operands().iter().all(|&operand| literal[operand]),
                };
                match op {
                    Op::Exp(_) | Op::Logistic(_) => literal[at],
                    Op::Divide([a, b]) => numbers[*a] == numbers[*b],
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
        if let Some(variable) = self.unrun_mode(packet.cycle.is_some()) {
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
        let mut filled_by = Vec::with_capacity(self.layout.parameters.len());
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
            filled_by.push(at);
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
        let owed = self.owed(&packet, &filled_by)?;
        Ok(BoundParameters {
            model: self.model_hash(),
            packet: packet.hash(),
            values,
            starts,
            modes,
            owed,
        })
    }
    /// Whether a row reads the body: it is an amount, or its laws reach an amount or a drive through memory and
    /// algebraic rows. A row that does not is a function of parameters alone, held forcings and draws included,
    /// so an anchor on it would be a parameter's value (SRS section 2, I14).
    fn reads_the_body(&self, row: RowId) -> bool {
        let Layout { rows, instances, .. } = &self.layout;
        let mut seen = vec![false; rows.len()];
        let mut waiting = vec![row];
        while let Some(at) = waiting.pop() {
            if seen[at.index()] {
                continue;
            }
            seen[at.index()] = true;
            if matches!(rows[at.index()].kind, RowKind::Amount { .. }) {
                return true;
            }
            for input in (instances.iter())
                .filter(|instance| instance.target == Target::Row(at))
                .flat_map(|instance| &instance.inputs)
            {
                match *input {
                    Bound::Boundary(_) => return true,
                    Bound::Unknown(unknown) => waiting.push(RowId::at(unknown.index())),
                    Bound::Parameter(_) | Bound::Draw(_) | Bound::Forcing(_) => {}
                }
            }
        }
        false
    }
    /// A pulsatile row is one its block's rate class puts in the wave or beat band.
    fn pulsatile(&self, row: RowId) -> bool {
        (self.layout.blocks.iter())
            .find(|block| block.rows.contains(&row))
            .is_some_and(|block| block.rate.band() <= RateBand::Beat)
    }
    /// Parses what the packet asks of admission against the compiled layout. `bind` holds rule 17's window here:
    /// a statistic the row or the packet cannot give is refused before anything solves.
    fn owed<K: PacketKind>(
        &self,
        packet: &ParameterPacket<K>,
        filled_by: &[usize],
    ) -> Result<Owed, AdmissionError> {
        // In key order, as the hash reads them, so two builds of one packet solve with the same bits.
        let mut ordered: Vec<&AnchorEntry> = packet.anchors.iter().collect();
        ordered.sort_by_key(|entry| entry.id);
        let mut anchors = Vec::with_capacity(ordered.len());
        for entry in ordered {
            let anchor = entry.id;
            let row = (self.layout.rows.iter())
                .find(|row| row.kind == entry.row && row.dimension == entry.dimension)
                .ok_or(AdmissionError::AnchorRow { anchor })?
                .id();
            if !self.reads_the_body(row) {
                return Err(AdmissionError::AnchorOnParameters { anchor });
            }
            match entry.statistic {
                AnchorStatistic::Instant if self.pulsatile(row) => {
                    return Err(AdmissionError::InstantOnPulsatile { anchor });
                }
                AnchorStatistic::CycleMean if packet.cycle.is_none() => {
                    return Err(AdmissionError::NoCycle { anchor });
                }
                AnchorStatistic::Instant | AnchorStatistic::CycleMean => {}
            }
            anchors.push(BoundAnchor {
                id: anchor,
                row,
                method: entry.method,
                statistic: entry.statistic,
                value: f64::from_bits(entry.bits),
            });
        }
        let mut fits = Vec::new();
        let mut ordered: Vec<(usize, &ParameterEntry)> = packet.parameters.iter().enumerate().collect();
        ordered.sort_by_key(|(_, entry)| (entry.key, entry.binding));
        let mut entries = Vec::with_capacity(ordered.len());
        for (at_entry, entry) in ordered {
            let ParameterEntry { key, binding, .. } = *entry;
            let value = f64::from_bits(entry.bits);
            match entry.provenance {
                ValueProvenance::Calibrated { anchor } => {
                    let Some(at) = anchors.iter().position(|bound| bound.id == anchor) else {
                        return Err(AdmissionError::UnknownAnchor { key, binding });
                    };
                    if fits.iter().any(|fit: &Fit| fit.anchor == at) {
                        return Err(AdmissionError::AnchorFittedTwice { anchor });
                    }
                    let slots = (filled_by.iter().enumerate())
                        .filter(|(_, filled)| **filled == at_entry)
                        .map(|(slot, _)| ParameterId::at(slot))
                        .collect();
                    fits.push(Fit {
                        slots,
                        anchor: at,
                        guess: value,
                        key,
                        binding,
                    });
                }
                ValueProvenance::Derived { inputs, .. } => {
                    // A fit moves at admission and no identity runs again, so resting on one would leave it false.
                    let settled = |input: &&ParameterKey| {
                        let known = || packet.parameters.iter().filter(|known| known.key == **input);
                        let fitted = |known: &ParameterEntry| {
                            matches!(known.provenance, ValueProvenance::Calibrated { .. })
                        };
                        known().count() > 0 && !known().any(fitted)
                    };
                    if let Some(input) = inputs.iter().find(|input| !settled(input)) {
                        return Err(AdmissionError::IdentityInput { key, input: *input });
                    }
                }
                ValueProvenance::Fixture
                | ValueProvenance::Sampled
                | ValueProvenance::Authored { .. }
                | ValueProvenance::Solved => {}
            }
            let name = ValueName::Parameter { key, binding };
            entries.push((name, entry.provenance, value));
        }
        Ok(Owed {
            cycle: packet.cycle,
            anchors,
            fits,
            entries,
        })
    }
}

/// One number per node, equal for two nodes that compute one expression: the same operation on operands of the
/// same numbers, in either order for `add`, `multiply` and `scale`. Separate reads of one parameter already compare
/// equal, so `(p + p) / (p + p)` from four reads gets one number on each side, and so does `(p q) / (q p)`. It
/// folds nothing: `p - p` stays a law on a parameter, as finding 15's backed test holds it.
fn numbered(code: &[Op]) -> Vec<usize> {
    let mut numbers: Vec<usize> = Vec::with_capacity(code.len());
    let mut distinct: Vec<Op> = Vec::with_capacity(code.len());
    for op in code {
        let plain = match op.map(|operand| numbers[operand]) {
            Op::Add([a, b]) => Op::Add([a.min(b), a.max(b)]),
            Op::Multiply([a, b]) => Op::Multiply([a.min(b), a.max(b)]),
            Op::Scale([a, b]) => Op::Scale([a.min(b), a.max(b)]),
            other => other,
        };
        let known = distinct.iter().position(|seen| *seen == plain);
        numbers.push(known.unwrap_or(distinct.len()));
        if known.is_none() {
            distinct.push(plain);
        }
    }
    numbers
}
