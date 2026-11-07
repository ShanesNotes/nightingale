use super::SECOND;
use super::table::Measured;
use ng_contract::{Digest, SimTime};

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

/// A value and the declared tolerance carried through its recipe.
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

/// From one beat edge to the next. Each series holds its step-end values' means, weighted by step length.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Beat {
    pub(super) start: SimTime,
    pub(super) end: SimTime,
    pub(super) means: [Sample; SERIES],
}
impl Beat {
    /// Stroke volume is the beat's integral of aortic flow; cardiac output is its mean.
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

/// Folded while the run stepped, never rebuilt from history. `applied` holds one entry per plan occurrence.
#[derive(Clone, Debug)]
pub(super) struct DayTrace {
    pub(super) run: Digest,
    pub(super) seal: Digest,
    pub(super) start: SimTime,
    pub(super) end: SimTime,
    pub(super) beats: Vec<Beat>,
    pub(super) applied: Vec<bool>,
}
