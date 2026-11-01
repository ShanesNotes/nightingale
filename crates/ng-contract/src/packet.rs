use crate::body::{Bound, Column, Instance, Layout, ParameterId, Target};
use crate::chance::{holds_a_period, sampled_unit};
use crate::identity::{Canonical, Seal};
use crate::law::{Family, Op};
use crate::provenance::seal_trace;
use crate::quantity::dimension;
use crate::{
    AdmissionError, Anchor, AnchorId, AnchorStatistic, Binding, Citation, Compensated, CompiledBody, Digest,
    Dimension, Dimensionless, ErrorScales, LawName, Measure, MethodHistory, ModelHash, PacketSeed,
    ParameterKey, RateBand, RowId, RowKind, SimTime, Slot, UnknownId, ValueName, ValueProvenance,
};
use core::marker::PhantomData;
use core::num::{NonZeroU32, NonZeroU64};

mod sealed {
    pub trait PacketKind {
        const TAG: u64;
        /// What backs a value in this kind of packet.
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
    /// The hash a run names, closed at admission: the packet's, the moment it starts, the start and fits it reached
    /// and the core's history there, so one identity has one start whatever made it. Events stay out of it.
    pub(crate) fn reading(
        self,
        moment: SimTime,
        start: &[f64],
        fitted: &[f64],
        history: &MethodHistory,
    ) -> Self {
        let mut out = Canonical::new("ng-contract packet and start 1");
        self.digest.seal(&mut out);
        moment.seal(&mut out);
        start.seal(&mut out);
        fitted.seal(&mut out);
        history.digest().seal(&mut out);
        Self { digest: out.digest() }
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
    /// Refuses a period `intervals` does not divide, so every point sits on the clock.
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
/// A row's natural size, in its own unit.
struct ScaleEntry {
    row: RowKind,
    dimension: String,
    bits: u64,
}
struct ModeEntry {
    variable: Slot,
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
/// A parameter's range in a stated population: an interval, the form `authored` and `sampled` read.
struct PopulationEntry {
    key: ParameterKey,
    dimension: String,
    lower: f64,
    upper: f64,
    trace: [&'static str; 2],
}
pub struct ParameterPacket<K: PacketKind> {
    cycle: Option<Cycle>,
    tolerance: Option<ErrorScales>,
    parameters: Vec<ParameterEntry>,
    starts: Vec<StartEntry>,
    scales: Vec<ScaleEntry>,
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
            tolerance: None,
            parameters: Vec::new(),
            starts: Vec::new(),
            scales: Vec::new(),
            modes: Vec::new(),
            anchors: Vec::new(),
            populations: Vec::new(),
            kind: PhantomData,
        }
    }
    /// A packet whose start state returns after `cycle`: the periodic goal. Its start values are the guess
    /// at every point, and each conserved combination's total over them is what the solve cannot move.
    pub fn periodic(cycle: Cycle) -> Self {
        Self {
            cycle: Some(cycle),
            ..Self::new()
        }
    }
    /// How near every row of the admission system must come. The hash covers it, so no initializer can widen it
    /// under a run's identity. A packet that states none is held exactly.
    pub fn within(&mut self, tolerance: ErrorScales) {
        self.tolerance = Some(tolerance);
    }
    /// A row's natural size, where the tolerance and a core's scales read their absolute term. A row with none is
    /// one unit in size, so a fixture near unit size states none, and rows of mixed units state their own. `bind`
    /// refuses one on no row, in another dimension or not positive.
    pub fn row_scale<U: Dimension>(&mut self, row: RowKind, size: Measure<U>) -> Result<(), AdmissionError> {
        if self.scales.iter().any(|entry| entry.row == row) {
            return Err(AdmissionError::RowScale { row });
        }
        self.scales.push(ScaleEntry {
            row,
            dimension: dimension::<U>(),
            bits: size.bits(),
        });
        Ok(())
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
        let width = upper.value() - lower.value();
        if repeated || width < 0.0 || !width.is_finite() {
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
    /// binding, so adding a parameter moves no other draw. Reads the record, so it follows `population`.
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
            note: "",
        })
    }
    /// A value an identity gives: `scale` times the entry at `source`, computed at `bind`, so it is never a free
    /// number. `bind` refuses a source the packet does not hold, or one derived or fitted, since no identity runs
    /// again when a fit moves.
    pub fn derived(
        &mut self,
        key: ParameterKey,
        binding: Binding,
        identity: &'static str,
        source: (ParameterKey, Binding),
        scale: Measure<Dimensionless>,
    ) -> Result<(), AdmissionError> {
        let (source, source_binding) = source;
        self.entry(ParameterEntry {
            key,
            binding,
            dimension: String::new(),
            bits: scale.bits(),
            provenance: ValueProvenance::Derived {
                identity,
                source,
                binding: source_binding,
            },
            note: "",
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
    /// Each entry's value and dimension, as a law reads them: a derived entry's are its source's, scaled.
    fn settled(&self) -> Result<Vec<(f64, &str)>, AdmissionError> {
        let mut settled = Vec::with_capacity(self.parameters.len());
        for entry in &self.parameters {
            let own = f64::from_bits(entry.bits);
            settled.push(match entry.provenance {
                ValueProvenance::Derived { source, binding, .. } => {
                    let found = |known: &&ParameterEntry| known.key == source && known.binding == binding;
                    let usable = |known: &&ParameterEntry| {
                        !matches!(
                            known.provenance,
                            ValueProvenance::Calibrated { .. } | ValueProvenance::Derived { .. }
                        )
                    };
                    let Some(known) = self.parameters.iter().find(found).filter(usable) else {
                        return Err(AdmissionError::IdentitySource {
                            key: entry.key,
                            source,
                        });
                    };
                    (own * f64::from_bits(known.bits), known.dimension.as_str())
                }
                _ => (own, entry.dimension.as_str()),
            });
        }
        Ok(settled)
    }
    fn hash(&self) -> PacketHash {
        let Self {
            cycle,
            tolerance,
            parameters,
            starts,
            scales,
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
        tolerance.seal(&mut out);
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
        let mut scales: Vec<&ScaleEntry> = scales.iter().collect();
        scales.sort_by_key(|entry| entry.row);
        out.length(scales.len());
        for ScaleEntry { row, dimension, bits } in scales {
            row.seal(&mut out);
            out.text(dimension);
            out.number(*bits);
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
    /// A whole-body variable takes its name; a route's takes a `Slot` at its edge.
    /// Every row the packet has not started yet, at `guess`. A guess at an algebraic row is only where the solve begins,
    /// and each row's dimension comes from the row, so a fixture of hundreds of rows states only those that matter.
    pub fn start_rest(
        &mut self,
        body: &CompiledBody,
        guess: impl Fn(RowKind) -> f64,
        note: &'static str,
    ) -> Result<(), AdmissionError> {
        for row in body.rows() {
            if self.starts.iter().any(|entry| entry.row == row.kind) {
                continue;
            }
            let value = Measure::<Dimensionless>::new(guess(row.kind))?;
            self.starts.push(StartEntry {
                row: row.kind,
                dimension: row.dimension.clone(),
                bits: value.bits(),
                note,
            });
        }
        Ok(())
    }
    pub fn start_mode(
        &mut self,
        variable: impl Into<Slot>,
        value: &'static str,
        note: &'static str,
    ) -> Result<(), AdmissionError> {
        let variable = variable.into();
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
    /// The rows behind its anchor, the only ones whose answer to the fit can reach it.
    pub(crate) rows: Vec<RowId>,
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
    pub(crate) tolerance: Option<ErrorScales>,
    pub(crate) anchors: Vec<BoundAnchor>,
    pub(crate) fits: Vec<Fit>,
    /// One per parameter entry: its name, its provenance and its settled value.
    pub(crate) entries: Vec<(ValueName, ValueProvenance, f64)>,
}

/// Packet values in compiled parameter order with the packet's hash.
pub struct BoundParameters {
    pub(crate) model: ModelHash,
    pub(crate) packet: PacketHash,
    pub(crate) values: Vec<f64>,
    pub(crate) starts: Vec<f64>,
    /// One per row: its natural size, one unit unless the packet says.
    pub(crate) sizes: Vec<f64>,
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
            sizes,
            modes,
            owed: _,
        } = self;
        let mut out = Canonical::new("ng-contract bound parameters 1");
        model.digest().seal(&mut out);
        packet.digest().seal(&mut out);
        values.seal(&mut out);
        starts.seal(&mut out);
        sizes.seal(&mut out);
        modes.seal(&mut out);
        out.digest()
    }
}

impl CompiledBody {
    /// A firing leaves algebraic values as they were and nothing here solves them again, so an algebraic law may not
    /// hold by a fired mode, or read a row a firing resets: then no algebraic row jumps at a firing, and no landing
    /// needs a solve. A route's own flow row is the one exception: only stages read it, and each stage solves it, so at
    /// that instant it keeps the left limit the dense segment ends on and no later value rests on it. A variable no
    /// guard moves never leaves its start, and a hazard integral only grows until it fires, so it never returns over a
    /// cycle.
    fn unrun_mode(&self, periodic: bool) -> Option<Slot> {
        let Layout {
            rows,
            modes,
            guards,
            hazards,
            instances,
            ..
        } = &self.layout;
        let algebraic = |target: Target| match target {
            Target::Row(row) => match rows[row.index()].kind {
                RowKind::Algebraic(slot) => !Family::flows(slot.law.local),
                _ => false,
            },
            Target::Column(_) => false,
        };
        let plain = |at| guards.iter().any(|guard| guard.variable == at);
        let hazard = |at| hazards.iter().any(|hazard| hazard.variable == at);
        let selects = |at| {
            (instances.iter()).any(|law| algebraic(law.target) && law.mode.is_some_and(|mode| mode.0 == at))
        };
        let reset_reads = |at| {
            let resets = (guards.iter())
                .filter(|guard| guard.variable == at)
                .filter_map(|guard| guard.resets);
            resets.into_iter().any(|row| {
                let read = |law: &&Instance| {
                    law.inputs
                        .iter()
                        .any(|input| input.unknown() == Some(UnknownId::at(row.index())))
                };
                instances
                    .iter()
                    .filter(|law| algebraic(law.target))
                    .any(|law| read(&law))
            })
        };
        (modes.iter().enumerate())
            .find(|&(at, _)| {
                !(plain(at) || hazard(at)) || (periodic && hazard(at)) || selects(at) || reset_reads(at)
            })
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
        let settled = packet.settled()?;
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
            let (value, dimension) = settled[at];
            if dimension != slot.dimension {
                return Err(AdmissionError::ParameterDimension { key, binding });
            }
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
        let sizes = self.sizes(&packet)?;
        let owed = self.owed(&packet, &settled, &filled_by)?;
        Ok(BoundParameters {
            model: self.model_hash(),
            packet: packet.hash(),
            values,
            starts,
            sizes,
            modes,
            owed,
        })
    }
    fn sizes<K: PacketKind>(&self, packet: &ParameterPacket<K>) -> Result<Vec<f64>, AdmissionError> {
        let rows = &self.layout.rows;
        let mut sizes = vec![1.0; rows.len()];
        for entry in &packet.scales {
            let row = entry.row;
            let found = rows.iter().position(|known| known.kind == row);
            let at = found.ok_or(AdmissionError::RowScale { row })?;
            let size = f64::from_bits(entry.bits);
            if rows[at].dimension != entry.dimension || size <= 0.0 {
                return Err(AdmissionError::RowScale { row });
            }
            sizes[at] = size;
        }
        Ok(sizes)
    }
    /// Every row a row's value can feel and the laws that set them: the row, each row its laws read, and so on
    /// back. At an instant only algebraic rows move, so a walk passes through no other.
    fn behind(&self, row: RowId, moving: bool) -> (Vec<RowId>, Vec<&Instance>) {
        let Layout {
            rows,
            columns,
            instances,
            ..
        } = &self.layout;
        let sets = |instance: &Instance, at: RowId| match instance.target {
            Target::Row(target) => target == at,
            Target::Column(column) => (columns[column.index()].terms.iter()).any(|(row, _)| *row == at),
        };
        let mut seen = vec![false; rows.len()];
        let mut waiting = vec![row];
        let mut behind = Vec::new();
        let mut laws = Vec::new();
        while let Some(at) = waiting.pop() {
            let moves = moving || matches!(rows[at.index()].kind, RowKind::Algebraic(_));
            if seen[at.index()] || !moves {
                continue;
            }
            seen[at.index()] = true;
            behind.push(at);
            for instance in instances.iter().filter(|instance| sets(instance, at)) {
                let reads = instance.inputs.iter().filter_map(Bound::unknown);
                waiting.extend(reads.map(|unknown| RowId::at(unknown.index())));
                laws.push(instance);
            }
        }
        (behind, laws)
    }
    /// Whether a row reads the body: it is an amount a law moves, or the laws behind it read one or a drive. A row
    /// that does not is a function of parameters alone, held forcings and draws included, and a held pool is the
    /// packet's own start, so an anchor on either would be a parameter's value (SRS section 2, I14).
    fn reads_the_body(&self, row: RowId) -> bool {
        let moved = |at: usize| {
            let moves = |column: &Column| column.terms.iter().any(|(row, _)| row.index() == at);
            self.layout.columns.iter().any(moves)
        };
        let (_, laws) = self.behind(row, true);
        let reads = laws.iter().flat_map(|law| &law.inputs);
        moved(row.index())
            || reads.into_iter().any(|input| match *input {
                Bound::Boundary(_) => true,
                Bound::Unknown(unknown) => moved(unknown.index()),
                Bound::Parameter(_)
                | Bound::Draw(_)
                | Bound::Forcing(_)
                | Bound::Gauge
                | Bound::Height(_) => false,
            })
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
        settled: &[(f64, &str)],
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
            let (value, _) = settled[at_entry];
            match entry.provenance {
                ValueProvenance::Calibrated { anchor } => {
                    let Some(at) = anchors.iter().position(|bound| bound.id == anchor) else {
                        return Err(AdmissionError::UnknownAnchor { key, binding });
                    };
                    if fits.iter().any(|fit: &Fit| fit.anchor == at) {
                        return Err(AdmissionError::AnchorFittedTwice { anchor });
                    }
                    let slots: Vec<ParameterId> = (filled_by.iter().enumerate())
                        .filter(|(_, filled)| **filled == at_entry)
                        .map(|(slot, _)| ParameterId::at(slot))
                        .collect();
                    let (rows, laws) = self.behind(anchors[at].row, packet.cycle.is_some());
                    let reads = |law: &&Instance| law.parameters.iter().any(|read| slots.contains(read));
                    if !laws.iter().any(reads) {
                        return Err(AdmissionError::FitUnread { key, binding });
                    }
                    if (self.layout.forcings.iter()).any(|forcing| slots.contains(&forcing.period)) {
                        return Err(AdmissionError::ForcingPeriod { key });
                    }
                    fits.push(Fit {
                        slots,
                        rows,
                        anchor: at,
                        guess: value,
                        key,
                        binding,
                    });
                }
                ValueProvenance::Fixture
                | ValueProvenance::Derived { .. }
                | ValueProvenance::Sampled
                | ValueProvenance::Authored { .. }
                | ValueProvenance::Solved => {}
            }
            let name = ValueName::Parameter { key, binding };
            entries.push((name, entry.provenance, value));
        }
        Ok(Owed {
            cycle: packet.cycle,
            tolerance: packet.tolerance.clone(),
            anchors,
            fits,
            entries,
        })
    }
}

/// One number per node, equal for two nodes that compute one expression: the same operation on operands of the
/// same numbers, with `as_dimension` no operation at all, and in either order for `add`, `multiply` and a smooth
/// limit's two sides. Separate reads of one parameter already compare equal, so `(p + p) / (p + p)` from four reads
/// gets one number on each side. It folds nothing: `p - p` stays a law on a parameter, as finding 15's backed test
/// holds it.
fn numbered(code: &[Op]) -> Vec<usize> {
    let mut numbers: Vec<usize> = Vec::with_capacity(code.len());
    let mut distinct: Vec<Op> = Vec::with_capacity(code.len());
    for op in code {
        let plain = match op.map(|operand| numbers[operand]) {
            Op::AsDimension([x]) => {
                numbers.push(x);
                continue;
            }
            Op::Add([a, b]) => Op::Add([a.min(b), a.max(b)]),
            Op::Multiply([a, b]) => Op::Multiply([a.min(b), a.max(b)]),
            Op::SmoothMax([a, b, width]) => Op::SmoothMax([a.min(b), a.max(b), width]),
            Op::SmoothMin([a, b, width]) => Op::SmoothMin([a.min(b), a.max(b), width]),
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
