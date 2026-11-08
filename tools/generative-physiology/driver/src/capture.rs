use crate::npy::NpyWriter;
use crate::spec::Stamp;
use core::fmt;
use ng_contract::{
    AcceptedStateView, AnatomyBuilder, CapillaryBed, Column, ColumnId, Compensated, CompiledBody, Context,
    Digest, InputId, LawName, Measure, Position, QuantityId, Row, RowId, RowKind, SimTime, SiteClass, SiteId,
    Slot, Species, Target, VesselSegment,
};
use ng_observe::{Observation, ObservedValue};
use ng_record::{Curve, Running, Session, StepReceipt};
use ng_validate::anchor_moment;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::Path;

pub const STEP_COLUMNS: usize = 7;
const BEAT_COLUMNS: usize = 7;
const LENS_COLUMNS: usize = 6;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CatalogueError {
    /// No instance targets this column, so no public item gives its id (`ColumnId::at` is crate-private).
    UnreadColumn {
        index: usize,
    },
    DuplicateName(String),
    Missing(&'static str),
}
impl fmt::Display for CatalogueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnreadColumn { index } => write!(f, "column {index} has no instance target"),
            Self::DuplicateName(name) => write!(f, "two entries named {name}"),
            Self::Missing(what) => write!(f, "the body has no {what}"),
        }
    }
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = || Measure::new(0.0).expect("zero is finite");
    AnatomyBuilder::new()
        .site::<C>(key, Position::new(origin(), origin(), origin()))
        .id()
}

fn place(context: Context) -> String {
    match context {
        Context::Body => "body".to_string(),
        Context::Site(site) => site.key().to_string(),
        Context::Edge { from, to } => format!("{}>{}", from.key(), to.key()),
    }
}

fn row_name(kind: RowKind) -> String {
    match kind {
        RowKind::Amount { site, space, species } => format!("amount/{}/{space:?}/{species:?}", site.key()),
        RowKind::Memory(Slot { law, at }) => format!("memory/{}/{}/{}", law.owner, law.local, place(at)),
        RowKind::Algebraic(Slot { law, at }) => {
            format!("algebraic/{}/{}/{}", law.owner, law.local, place(at))
        }
    }
}

fn column_name(body: &CompiledBody, column: &Column) -> String {
    let species = (column.terms().first()).and_then(|&(row, _)| match body.rows()[row.index()].kind() {
        RowKind::Amount { species, .. } => Some(format!("{species:?}")),
        RowKind::Memory(_) | RowKind::Algebraic(_) => None,
    });
    let LawName { owner, local } = column.origin();
    let exterior = column.receipt().map_or("inside", |receipt| receipt.boundary());
    format!(
        "{owner}/{local}/{}/{}/{exterior}",
        place(column.context()),
        species.as_deref().unwrap_or("none")
    )
}

fn distinct(names: &[String]) -> Result<(), CatalogueError> {
    let mut seen = BTreeSet::new();
    match names.iter().find(|name| !seen.insert(name.as_str())) {
        Some(name) => Err(CatalogueError::DuplicateName(name.clone())),
        None => Ok(()),
    }
}

pub struct Catalogue {
    rows: Vec<String>,
    columns: Vec<String>,
    row_ids: Vec<RowId>,
    column_ids: Vec<ColumnId>,
    quantities: Vec<QuantityId>,
    phase: RowId,
    heart_rate: RowId,
    pressure: RowId,
    filtration: RowId,
    aortic_water: ColumnId,
}
impl Catalogue {
    pub fn of(body: &CompiledBody) -> Result<Self, CatalogueError> {
        let mut column_ids: Vec<ColumnId> = (body.instances().iter())
            .filter_map(|instance| match instance.target() {
                Target::Column(column) => Some(column),
                Target::Row(_) => None,
            })
            .collect();
        column_ids.sort();
        column_ids.dedup();
        let unread =
            (0..body.columns().len()).find(|&at| column_ids.get(at).map(|id| id.index()) != Some(at));
        if let Some(index) = unread {
            return Err(CatalogueError::UnreadColumn { index });
        }
        let rows: Vec<String> = body.rows().iter().map(|row| row_name(row.kind())).collect();
        let columns: Vec<String> = (column_ids.iter())
            .map(|id| column_name(body, &body.columns()[id.index()]))
            .collect();
        distinct(&rows)?;
        distinct(&columns)?;
        let find = |kind: RowKind, what: &'static str| {
            (body.rows().iter())
                .find(|row| row.kind() == kind)
                .map(Row::id)
                .ok_or(CatalogueError::Missing(what))
        };
        let slot = |owner, local, at| Slot {
            law: LawName { owner, local },
            at,
        };
        let arteries = site::<VesselSegment>("systemic arteries");
        let glomerulus = Context::Edge {
            from: site::<CapillaryBed>("glomerular bed"),
            to: site::<VesselSegment>("renal tubule"),
        };
        let aortic = Context::Edge {
            from: site::<VesselSegment>("left ventricle"),
            to: arteries,
        };
        let valves = LawName {
            owner: "CV",
            local: "valves",
        };
        let water = |row: RowId| {
            matches!(
                body.rows()[row.index()].kind(),
                RowKind::Amount {
                    species: Species::Water,
                    ..
                }
            )
        };
        let aortic_water = (column_ids.iter().copied())
            .find(|id| {
                let column = &body.columns()[id.index()];
                let terms = column.terms();
                column.origin() == valves
                    && column.context() == aortic
                    && !terms.is_empty()
                    && terms.iter().all(|&(row, _)| water(row))
            })
            .ok_or(CatalogueError::Missing("aortic valve water column"))?;
        Ok(Self {
            phase: find(
                RowKind::Memory(slot("CV", "phase", Context::Body)),
                "CV phase row",
            )?,
            heart_rate: find(
                RowKind::Algebraic(slot("CV", "rate", Context::Body)),
                "CV rate row",
            )?,
            pressure: find(
                RowKind::Algebraic(slot("CV", "blood pressure", Context::Site(arteries))),
                "CV arterial pressure row",
            )?,
            filtration: find(
                RowKind::Algebraic(slot("RN", "filtration", glomerulus)),
                "RN filtration row",
            )?,
            aortic_water,
            rows,
            columns,
            row_ids: body.rows().iter().map(Row::id).collect(),
            column_ids,
            quantities: body.conserved_basis().ids().collect(),
        })
    }
    pub fn rows(&self) -> &[String] {
        &self.rows
    }
    pub fn columns(&self) -> &[String] {
        &self.columns
    }
    pub fn column_ids(&self) -> &[ColumnId] {
        &self.column_ids
    }
    pub fn quantities(&self) -> &[QuantityId] {
        &self.quantities
    }
    pub fn phase(&self) -> RowId {
        self.phase
    }
    pub fn heart_rate(&self) -> RowId {
        self.heart_rate
    }
    pub fn pressure(&self) -> RowId {
        self.pressure
    }
    pub fn filtration(&self) -> RowId {
        self.filtration
    }
    pub fn aortic_water(&self) -> ColumnId {
        self.aortic_water
    }
    pub fn edge_columns(&self, modes: usize) -> usize {
        2 + modes + 4 * self.rows.len() + 4 * self.columns.len() + 6 * self.quantities.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeatReading {
    pub ordinal: u64,
    pub start: SimTime,
    pub end: SimTime,
    pub heart_rate: f64,
    pub arterial_pressure: f64,
    pub stroke: f64,
    pub filtration: f64,
}
impl BeatReading {
    fn between(ordinal: u64, start: &Edge, end: &Edge) -> Self {
        let length = (end.at.nanoseconds() - start.at.nanoseconds()) as f64;
        let mean = |total: Compensated, before: Compensated| leading(total.minus(before)) / length;
        Self {
            ordinal,
            start: start.at,
            end: end.at,
            heart_rate: 1e9 / length,
            arterial_pressure: mean(end.pressure, start.pressure),
            stroke: leading(end.ejected.minus(start.ejected)),
            filtration: mean(end.filtration, start.filtration),
        }
    }
    pub fn row(&self) -> [f64; BEAT_COLUMNS] {
        let Self {
            ordinal,
            start,
            end,
            heart_rate,
            arterial_pressure,
            stroke,
            filtration,
        } = *self;
        [
            ordinal as f64,
            seconds_after_anchor(start),
            seconds_after_anchor(end),
            heart_rate,
            arterial_pressure,
            stroke,
            filtration,
        ]
    }
    pub fn json(&self, earliest: Stamp) -> String {
        let number = |value: f64| {
            if value.is_finite() {
                format!("{value}")
            } else {
                "null".to_string()
            }
        };
        format!(
            "{{\"beat\":{},\"start_s\":{},\"end_s\":{},\"heart_rate\":{},\"arterial_pressure\":{},\"stroke\":{},\
             \"filtration\":{},\"earliest_ms\":{}}}",
            self.ordinal,
            number(seconds_after_anchor(self.start)),
            number(seconds_after_anchor(self.end)),
            number(self.heart_rate),
            number(self.arterial_pressure),
            number(self.stroke),
            number(self.filtration),
            earliest.ms()
        )
    }
}

fn seconds_after_anchor(at: SimTime) -> f64 {
    (at.nanoseconds() - anchor_moment().nanoseconds()) as f64 / 1e9
}

fn leading(value: Compensated) -> f64 {
    f64::from_bits(value.bits()[0])
}

pub fn words(digest: Digest) -> [u64; 4] {
    let (chunks, _) = digest.as_bytes().as_chunks::<8>();
    core::array::from_fn(|at| u64::from_be_bytes(chunks[at]))
}

pub fn digest_of_words(words: [u64; 4]) -> Digest {
    let mut bytes = [0u8; 32];
    let (chunks, _) = bytes.as_chunks_mut::<8>();
    for (chunk, word) in chunks.iter_mut().zip(words) {
        *chunk = word.to_be_bytes();
    }
    Digest::from_bytes(bytes)
}

pub fn step_row(revision: u64, end: SimTime, refusals: u64, seal: Digest) -> [u64; STEP_COLUMNS] {
    let [first, second, third, fourth] = words(seal);
    [
        revision,
        end.nanoseconds(),
        refusals,
        first,
        second,
        third,
        fourth,
    ]
}

#[derive(Clone, Copy)]
struct Edge {
    at: SimTime,
    pressure: Compensated,
    filtration: Compensated,
    ejected: Compensated,
}

fn ejected<S>(session: &Session<'_, S>, column: ColumnId) -> Compensated {
    match session.conservation().head() {
        Some(head) => {
            let (forward, reverse) = head.exchange(column);
            let interval = head.interval();
            let at_end = |curve: Curve<'_>| curve.on(interval).and_then(|ends| ends.last().copied());
            let (forward, reverse) = (at_end(forward), at_end(reverse));
            let expected = "a segment holds its own interval";
            forward.expect(expected).minus(reverse.expect(expected))
        }
        None => Compensated::ZERO,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventIndex(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arrival {
    pub event: EventIndex,
    pub revision: u64,
}

pub struct Capture<'c> {
    body: &'c CompiledBody,
    catalogue: &'c Catalogue,
    edges: NpyWriter<u64>,
    beats: NpyWriter<f64>,
    steps: NpyWriter<u64>,
    applied: NpyWriter<u64>,
    lens: NpyWriter<u64>,
    chord: Vec<Compensated>,
    phase: Compensated,
    beat_edge: Option<Edge>,
    inputs: BTreeMap<InputId, EventIndex>,
    enqueued: Vec<Arrival>,
}
impl<'c> Capture<'c> {
    pub fn create(
        directory: &Path,
        body: &'c CompiledBody,
        catalogue: &'c Catalogue,
        start: &Session<'_, Running>,
        inputs: BTreeMap<InputId, EventIndex>,
    ) -> io::Result<Self> {
        let view = start.record().accepted();
        let width = catalogue.edge_columns(view.modes().len());
        let mut capture = Self {
            body,
            catalogue,
            edges: NpyWriter::create(&directory.join("edges.npy"), width)?,
            beats: NpyWriter::create(&directory.join("beats.npy"), BEAT_COLUMNS)?,
            steps: NpyWriter::create(&directory.join("steps.npy"), STEP_COLUMNS)?,
            applied: NpyWriter::create(&directory.join("applied.npy"), 2)?,
            lens: NpyWriter::create(&directory.join("lens.npy"), LENS_COLUMNS)?,
            chord: vec![Compensated::ZERO; catalogue.rows().len()],
            phase: view.value(catalogue.phase()),
            beat_edge: None,
            inputs,
            enqueued: Vec::new(),
        };
        capture.edge(start)?;
        Ok(capture)
    }
    pub fn register(&mut self, id: InputId, event: EventIndex) {
        self.inputs.insert(id, event);
    }
    pub fn enqueued(&mut self, arrival: Arrival) {
        self.enqueued.push(arrival);
    }
    pub fn late(&self) -> &[Arrival] {
        &self.enqueued
    }
    pub fn beats(&self) -> u64 {
        self.beats.rows()
    }
    fn integrate(&mut self, view: AcceptedStateView<'_>) {
        let segment = view.last_segment();
        let step = segment.interval().expect("an accepted step has a chord");
        let length = i64::try_from(step.length_ns()).expect("a step is shorter than 2^63 ns");
        for (total, &row) in self.chord.iter_mut().zip(self.catalogue.row_ids.iter()) {
            let ends = (segment.sample(row, step.start()), segment.sample(row, step.end()));
            let (Some(start), Some(end)) = ends else {
                unreachable!("a chord holds both of its ends");
            };
            let twice = start.plus(end).scaled(length);
            *total = total.plus(Compensated::between(Compensated::ZERO, twice, 1, 2));
        }
    }
    pub fn accepted(
        &mut self,
        session: &Session<'_, Running>,
        receipt: &StepReceipt,
        refusals: u64,
    ) -> io::Result<Option<BeatReading>> {
        let view = session.record().accepted();
        self.integrate(view);
        let row = step_row(receipt.revision(), view.time(), refusals, receipt.seal().digest());
        self.steps.push(&row)?;
        for id in receipt.applied() {
            let index = self.inputs.get(id).copied();
            let index = index.ok_or_else(|| io::Error::other("a step applied an input no spec names"))?;
            self.applied.push(&[index.0 as u64, receipt.revision()])?;
        }
        let phase = view.value(self.catalogue.phase());
        let fell = !self.phase.at_most(phase);
        self.phase = phase;
        if !fell {
            return Ok(None);
        }
        self.edge(session)?;
        let edge = Edge {
            at: view.time(),
            pressure: self.chord[self.catalogue.pressure().index()],
            filtration: self.chord[self.catalogue.filtration().index()],
            ejected: ejected(session, self.catalogue.aortic_water()),
        };
        let Some(start) = self.beat_edge.replace(edge) else {
            return Ok(None);
        };
        let beat = BeatReading::between(self.beats.rows(), &start, &edge);
        self.beats.push(&beat.row())?;
        Ok(Some(beat))
    }
    pub fn observed(&mut self, observations: &[Observation]) -> io::Result<()> {
        for observation in observations {
            let (tag, [hi, lo]) = match observation.value() {
                ObservedValue::Present(value) => (0, value.bits()),
                ObservedValue::NotCollected => (1, [0, 0]),
                ObservedValue::Pending => (2, [0, 0]),
                ObservedValue::InvalidSignal => (3, [0, 0]),
            };
            let row = [
                observation.channel() as u64,
                observation.collected().nanoseconds(),
                observation.available().nanoseconds(),
                tag,
                hi,
                lo,
            ];
            self.lens.push(&row)?;
        }
        Ok(())
    }
    pub fn end<S>(&mut self, session: &Session<'_, S>) -> io::Result<()> {
        self.edge(session)
    }
    pub fn finish(self) -> io::Result<()> {
        let Self {
            edges,
            beats,
            steps,
            applied,
            lens,
            ..
        } = self;
        edges.finish()?;
        beats.finish()?;
        steps.finish()?;
        applied.finish()?;
        lens.finish()
    }
    fn edge<S>(&mut self, session: &Session<'_, S>) -> io::Result<()> {
        let record = session.record();
        let view = record.accepted();
        let mut row = Vec::with_capacity(self.catalogue.edge_columns(view.modes().len()));
        row.extend([view.time().nanoseconds(), record.revision()]);
        row.extend(view.modes().iter().map(|&mode| mode as u64));
        for &id in &self.catalogue.row_ids {
            row.extend(view.value(id).bits());
        }
        for total in &self.chord {
            row.extend(total.bits());
        }
        match session.conservation().head() {
            Some(head) => {
                let interval = head.interval();
                let at_end = |curve: Curve<'_>| {
                    let ends = curve.on(interval).and_then(|ends| ends.last().copied());
                    ends.expect("a segment holds its own interval").bits()
                };
                for &column in &self.catalogue.column_ids {
                    let (forward, reverse) = head.exchange(column);
                    row.extend(at_end(forward));
                    row.extend(at_end(reverse));
                }
                for &quantity in &self.catalogue.quantities {
                    row.extend(at_end(head.inventory(quantity)));
                    row.extend(at_end(head.inflow(quantity)));
                    row.extend(at_end(head.outflow(quantity)));
                }
            }
            None => {
                let zero = Compensated::ZERO.bits();
                for _ in &self.catalogue.column_ids {
                    row.extend(zero);
                    row.extend(zero);
                }
                for combination in self.body.conserved_basis().combinations() {
                    let inventory = (combination.terms().iter())
                        .fold(Compensated::ZERO, |sum, &(id, weight)| {
                            sum.plus(view.value(id).scaled(weight))
                        });
                    row.extend(inventory.bits());
                    row.extend(zero);
                    row.extend(zero);
                }
            }
        }
        self.edges.push(&row)
    }
}
