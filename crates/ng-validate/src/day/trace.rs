use super::table::Measured;
use super::{DayFailure, DayPlan, SECOND};
use ng_contract::{
    AcceptedStateView, AnatomyBuilder, Compensated, CompiledBody, Context, CoupledProblem, Digest,
    ErrorScales, InputId, LawName, Measure, Position, RowId, RowKind, SimTime, SiteClass, SiteId, Slot,
    Space, Species, VesselSegment,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Series {
    HeartRate,
    ArterialPressure,
    AorticFlow,
    PlasmaVolume,
    PlasmaAlbumin,
    PlasmaSodium,
    PlasmaOsmolality,
    SodiumExcretion,
    UrineFlow,
}
pub(super) const SERIES: usize = 9;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Sample {
    pub(super) value: f64,
    pub(super) floor: f64,
}

impl Measured {
    pub(super) const fn series(self) -> Series {
        match self {
            Measured::HeartRate => Series::HeartRate,
            Measured::ArterialPressure => Series::ArterialPressure,
            Measured::StrokeVolume | Measured::CardiacOutput => Series::AorticFlow,
            Measured::PlasmaVolume => Series::PlasmaVolume,
            Measured::PlasmaAlbumin => Series::PlasmaAlbumin,
            Measured::SodiumExcretion => Series::SodiumExcretion,
            Measured::PlasmaOsmolality => Series::PlasmaOsmolality,
            Measured::PlasmaSodium => Series::PlasmaSodium,
            Measured::UrineFlow => Series::UrineFlow,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Beat {
    pub(super) start: SimTime,
    pub(super) end: SimTime,
    pub(super) means: [Sample; SERIES],
}
impl Beat {
    pub(super) fn reading(&self, quantity: Measured) -> Sample {
        let mean = self.means[quantity.series() as usize];
        if quantity != Measured::StrokeVolume {
            return mean;
        }
        let seconds = (self.end.nanoseconds() - self.start.nanoseconds()) as f64 / SECOND as f64;
        Sample {
            value: mean.value * seconds,
            floor: mean.floor * seconds,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct DayTrace {
    pub(super) run: Digest,
    pub(super) seal: Digest,
    pub(super) start: SimTime,
    pub(super) end: SimTime,
    pub(super) beats: Vec<Beat>,
    pub(super) applied: Vec<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Read {
    Rate,
    Pressure,
    Aortic,
    Tubular,
    DuctSodium,
    DuctWater,
    Water,
    Albumin,
    Sodium,
    Potassium,
    Chloride,
}
pub(super) const READS: usize = 11;
pub(super) type Reading = [f64; READS];

fn series(reading: &Reading, allowed: &Reading) -> [Sample; SERIES] {
    let at = |read: Read| (reading[read as usize], allowed[read as usize]);
    let plain = |read: Read| {
        let (value, floor) = at(read);
        Sample { value, floor }
    };
    let ratio = |(top, top_floor): (f64, f64), (bottom, bottom_floor): (f64, f64)| Sample {
        value: top / bottom,
        floor: top_floor / bottom.abs() + top.abs() * bottom_floor / (bottom * bottom),
    };
    let water = at(Read::Water);
    let (sodium, potassium, chloride) = (at(Read::Sodium), at(Read::Potassium), at(Read::Chloride));
    let ions = (
        sodium.0 + potassium.0 + chloride.0,
        sodium.1 + potassium.1 + chloride.1,
    );
    let (flow, flow_floor) = at(Read::Tubular);
    let duct = ratio(at(Read::DuctSodium), at(Read::DuctWater));
    let excretion = Sample {
        value: flow * duct.value,
        floor: duct.value.abs() * flow_floor + flow.abs() * duct.floor,
    };
    [
        plain(Read::Rate),
        plain(Read::Pressure),
        plain(Read::Aortic),
        plain(Read::Water),
        ratio(at(Read::Albumin), water),
        ratio(sodium, water),
        ratio(ions, water),
        excretion,
        plain(Read::Tubular),
    ]
}

struct Open {
    start: SimTime,
    length: f64,
    sums: [Sample; SERIES],
}
impl Open {
    fn at(start: SimTime) -> Self {
        Self {
            start,
            length: 0.0,
            sums: [Sample::default(); SERIES],
        }
    }
    fn add(&mut self, length: f64, samples: &[Sample; SERIES]) {
        for (sum, sample) in self.sums.iter_mut().zip(samples) {
            sum.value += sample.value * length;
            sum.floor += sample.floor * length;
        }
        self.length += length;
    }
    fn close(self, end: SimTime) -> Beat {
        let mean = |sum: Sample| Sample {
            value: sum.value / self.length,
            floor: sum.floor / self.length,
        };
        Beat {
            start: self.start,
            end,
            means: self.sums.map(mean),
        }
    }
}

pub(super) struct Fold {
    scales: ErrorScales,
    sizes: Reading,
    start: SimTime,
    phase: f64,
    open: Option<Open>,
    beats: Vec<Beat>,
    inputs: Vec<(InputId, usize)>,
    remaining: Vec<usize>,
}
impl Fold {
    pub(super) fn new(
        plan: &DayPlan,
        scales: ErrorScales,
        sizes: Reading,
        start: SimTime,
        phase_at_start: f64,
    ) -> Self {
        let occurrences = plan.occurrences.iter().enumerate();
        let planned = occurrences
            .flat_map(|(index, occurrence)| occurrence.inputs.iter().map(move |input| (input.id, index)));
        let mut inputs: Vec<(InputId, usize)> = planned.collect();
        inputs.sort_unstable();
        Self {
            scales,
            sizes,
            start,
            phase: phase_at_start,
            open: None,
            beats: Vec::new(),
            inputs,
            remaining: plan
                .occurrences
                .iter()
                .map(|occurrence| occurrence.inputs.len())
                .collect(),
        }
    }
    pub(super) fn step(&mut self, end: SimTime, length_ns: u64, phase: f64, reading: &Reading) {
        let allowed = core::array::from_fn(|read| self.scales.at(reading[read], self.sizes[read]));
        let samples = series(reading, &allowed);
        if let Some(open) = &mut self.open {
            open.add(length_ns as f64, &samples);
        }
        let edge = phase < self.phase;
        self.phase = phase;
        if edge {
            if let Some(closing) = self.open.take() {
                self.beats.push(closing.close(end));
            }
            self.open = Some(Open::at(end));
        }
    }
    pub(super) fn applied(&mut self, ids: &[InputId]) {
        for id in ids {
            if let Ok(found) = self.inputs.binary_search_by_key(id, |&(id, _)| id) {
                let (_, occurrence) = self.inputs[found];
                self.remaining[occurrence] = self.remaining[occurrence].saturating_sub(1);
            }
        }
    }
    pub(super) fn finish(self, run: Digest, seal: Digest, end: SimTime) -> DayTrace {
        DayTrace {
            run,
            seal,
            start: self.start,
            end,
            beats: self.beats,
            applied: self.remaining.iter().map(|&left| left == 0).collect(),
        }
    }
}

fn site<C: SiteClass>(key: &'static str) -> SiteId {
    let origin = || Measure::new(0.0).expect("zero is finite");
    AnatomyBuilder::new()
        .site::<C>(key, Position::new(origin(), origin(), origin()))
        .id()
}
fn law(owner: &'static str, local: &'static str, at: Context) -> Slot {
    Slot {
        law: LawName { owner, local },
        at,
    }
}

/// Albumin marks blood: the urinary lumens hold water and ions but no albumin.
pub(super) fn blood_spaces(body: &CompiledBody) -> Vec<(SiteId, Space)> {
    let blood = |kind: RowKind| match kind {
        RowKind::Amount {
            site,
            space: space @ (Space::Lumen | Space::Plasma),
            species: Species::Albumin,
        } => Some((site, space)),
        _ => None,
    };
    body.rows().iter().filter_map(|row| blood(row.kind())).collect()
}

fn number(held: Compensated) -> f64 {
    f64::from_bits(held.bits()[0])
}

pub(super) struct Readings {
    phase: RowId,
    rows: [Vec<(RowId, usize)>; READS],
}
impl Readings {
    pub(super) fn bind(body: &CompiledBody) -> Result<Self, DayFailure> {
        let row = |kind: RowKind| {
            let found = body.rows().iter().enumerate().find(|(_, row)| row.kind() == kind);
            found
                .map(|(slot, row)| (row.id(), slot))
                .ok_or(DayFailure::MissingRow(kind))
        };
        let spaces = blood_spaces(body);
        let blood = |species| -> Result<Vec<(RowId, usize)>, DayFailure> {
            let amount = |&(site, space)| row(RowKind::Amount { site, space, species });
            spaces.iter().map(amount).collect()
        };
        let (arteries, ventricle) = (
            site::<VesselSegment>("systemic arteries"),
            site::<VesselSegment>("left ventricle"),
        );
        let (duct, bladder) = (
            site::<VesselSegment>("collecting duct"),
            site::<VesselSegment>("bladder"),
        );
        let duct_amount = |species| {
            row(RowKind::Amount {
                site: duct,
                space: Space::Lumen,
                species,
            })
        };
        let (phase, _) = row(RowKind::Memory(law("CV", "phase", Context::Body)))?;
        let aortic = Context::Edge {
            from: ventricle,
            to: arteries,
        };
        let urinary = Context::Edge {
            from: duct,
            to: bladder,
        };
        Ok(Self {
            phase,
            rows: [
                vec![row(RowKind::Algebraic(law("CV", "rate", Context::Body)))?],
                vec![row(RowKind::Algebraic(law(
                    "CV",
                    "blood pressure",
                    Context::Site(arteries),
                )))?],
                vec![row(RowKind::Memory(law("CV", "valves", aortic)))?],
                vec![row(RowKind::Algebraic(law("RN", "tubular flow", urinary)))?],
                vec![duct_amount(Species::Sodium)?],
                vec![duct_amount(Species::Water)?],
                blood(Species::Water)?,
                blood(Species::Albumin)?,
                blood(Species::Sodium)?,
                blood(Species::Potassium)?,
                blood(Species::Chloride)?,
            ],
        })
    }
    pub(super) fn sizes(&self, problem: &CoupledProblem<'_>) -> Reading {
        core::array::from_fn(|read| {
            self.rows[read]
                .iter()
                .map(|&(_, slot)| problem.row_size(slot))
                .sum()
        })
    }
    pub(super) fn read(&self, view: AcceptedStateView<'_>) -> (f64, Reading) {
        let value = |&(id, _): &(RowId, usize)| number(view.value(id));
        let reading = core::array::from_fn(|read| self.rows[read].iter().map(value).sum());
        (number(view.value(self.phase)), reading)
    }
}
