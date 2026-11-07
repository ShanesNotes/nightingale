use super::{DayQuantity, EventKind, HOUR, MINUTE, SECOND};
use Bound::{By, Under};
use Course::{AgainstControl, DipThenBack, OneWay, RiseThenFall};
use Direction::{Down, Up};
use EventKind::{LieDownToSleep, Meal, SitUp, Wake};
use Scoring::{DirectionOnly, Scored, Unscored};
use Span::{Hours, Minutes, Seconds};

/// The quantities a scored or direction-only row reads, each one the day trace holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Measured {
    HeartRate,
    ArterialPressure,
    StrokeVolume,
    CardiacOutput,
    PlasmaVolume,
    PlasmaAlbumin,
    SodiumExcretion,
    PlasmaOsmolality,
    PlasmaSodium,
    UrineFlow,
}
impl Measured {
    pub(super) const fn quantity(self) -> DayQuantity {
        match self {
            Measured::HeartRate => DayQuantity::HeartRate,
            Measured::ArterialPressure => DayQuantity::ArterialPressure,
            Measured::StrokeVolume => DayQuantity::StrokeVolume,
            Measured::CardiacOutput => DayQuantity::CardiacOutput,
            Measured::PlasmaVolume => DayQuantity::PlasmaVolume,
            Measured::PlasmaAlbumin => DayQuantity::PlasmaAlbumin,
            Measured::SodiumExcretion => DayQuantity::SodiumExcretion,
            Measured::PlasmaOsmolality => DayQuantity::PlasmaOsmolality,
            Measured::PlasmaSodium => DayQuantity::PlasmaSodium,
            Measured::UrineFlow => DayQuantity::UrineFlow,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Direction {
    Up,
    Down,
}

/// A bound in seconds is a fast check, in minutes or hours a slow one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Span {
    Seconds(u64),
    Minutes(u64),
    Hours(u64),
}
impl Span {
    pub(super) const fn ns(self) -> u64 {
        match self {
            Span::Seconds(count) => count * SECOND,
            Span::Minutes(count) => count * MINUTE,
            Span::Hours(count) => count * HOUR,
        }
    }
    pub(super) const fn fast(self) -> bool {
        matches!(self, Span::Seconds(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Bound {
    Under(Span),
    By(Span),
}
impl Bound {
    pub(super) const fn span(self) -> Span {
        match self {
            Bound::Under(span) | Bound::By(span) => span,
        }
    }
    /// `elapsed` is measured from the event's instant.
    pub(super) const fn meets(self, elapsed: u64) -> bool {
        match self {
            Bound::Under(span) => elapsed < span.ns(),
            Bound::By(span) => elapsed <= span.ns(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Course {
    OneWay(Direction, Bound),
    /// Row 2: a dip, a return to the band, then held up or unchanged from `held_from`.
    DipThenBack {
        dip: Bound,
        back: Bound,
        held_from: Span,
    },
    /// Row 4: a brief rise, then down from `fall_from` with its fall beginning by `fall`.
    RiseThenFall {
        rise: Bound,
        fall_from: Span,
        fall: Bound,
    },
    /// Against a run from the same start without the meal.
    AgainstControl(Direction, Bound),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Lean {
    Down,
    /// Up from the hour before the wake, which must itself lie below the awake hour before the night's lie-down.
    UpAfterNightDip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Scoring {
    Scored(Measured, Course),
    DirectionOnly(&'static [Measured], Lean),
    /// Its reason in words.
    Unscored(&'static [DayQuantity], &'static str),
}

#[derive(Debug)]
pub(super) struct Row {
    pub(super) number: u8,
    pub(super) event: EventKind,
    pub(super) scoring: Scoring,
    /// The table's Source column; every row's citation check is still open.
    pub(super) source: &'static str,
}

/// Ticket 21's table: directions and timing only, never a target value.
pub(super) const ROWS: [Row; 17] = [
    Row {
        number: 1,
        event: SitUp,
        scoring: Scored(Measured::HeartRate, OneWay(Up, Under(Seconds(15)))),
        source: "Smith 1994",
    },
    Row {
        number: 2,
        event: SitUp,
        scoring: Scored(
            Measured::ArterialPressure,
            DipThenBack {
                dip: By(Seconds(10)),
                back: Under(Seconds(30)),
                held_from: Seconds(30),
            },
        ),
        source: "Smith 1994",
    },
    Row {
        number: 3,
        event: SitUp,
        scoring: Scored(Measured::StrokeVolume, OneWay(Down, By(Seconds(30)))),
        source: "Smith 1994",
    },
    Row {
        number: 4,
        event: SitUp,
        scoring: Scored(
            Measured::CardiacOutput,
            RiseThenFall {
                rise: By(Seconds(30)),
                fall_from: Minutes(5),
                fall: By(Minutes(20)),
            },
        ),
        source: "Smith 1994",
    },
    Row {
        number: 5,
        event: SitUp,
        scoring: Scored(Measured::PlasmaVolume, OneWay(Down, By(Minutes(20)))),
        source: "Hagan 1978, Jacob 1998",
    },
    Row {
        number: 6,
        event: SitUp,
        scoring: Scored(Measured::PlasmaAlbumin, OneWay(Up, By(Minutes(20)))),
        source: "Hagan 1978",
    },
    Row {
        number: 7,
        event: SitUp,
        scoring: Scored(Measured::SodiumExcretion, OneWay(Down, By(Minutes(30)))),
        source: "Jacob 1998",
    },
    Row {
        number: 8,
        event: SitUp,
        scoring: Unscored(&[DayQuantity::Filtration], "direction disputed"),
        source: "Kamper 1988 found no change from supine to sitting",
    },
    Row {
        number: 9,
        event: Meal,
        scoring: Scored(Measured::PlasmaOsmolality, AgainstControl(Up, By(Hours(4)))),
        source: "Gill 1985",
    },
    Row {
        number: 10,
        event: Meal,
        scoring: Scored(Measured::PlasmaSodium, AgainstControl(Up, By(Hours(4)))),
        source: "Gill 1985",
    },
    Row {
        number: 11,
        event: Meal,
        scoring: Unscored(&[DayQuantity::Vasopressin], "unsourced"),
        source: "none found for a mixed meal in healthy adults",
    },
    Row {
        number: 12,
        event: Meal,
        scoring: Unscored(
            &[
                DayQuantity::SodiumExcretion,
                DayQuantity::PotassiumExcretion,
                DayQuantity::ChlorideExcretion,
            ],
            "unsourced",
        ),
        source: "not sourced",
    },
    Row {
        number: 13,
        event: Meal,
        scoring: Unscored(&[DayQuantity::PlasmaPotassium], "unsourced"),
        source: "not found; the insulin potassium shift is declared external",
    },
    Row {
        number: 14,
        event: LieDownToSleep,
        scoring: Scored(Measured::PlasmaVolume, OneWay(Up, By(Minutes(35)))),
        source: "Hagan 1978",
    },
    Row {
        number: 15,
        event: LieDownToSleep,
        scoring: DirectionOnly(&[Measured::ArterialPressure, Measured::HeartRate], Lean::Down),
        source: "Somers 1993",
    },
    Row {
        number: 16,
        event: LieDownToSleep,
        scoring: DirectionOnly(&[Measured::UrineFlow, Measured::SodiumExcretion], Lean::Down),
        source: "Kamperis 2010",
    },
    Row {
        number: 17,
        event: Wake,
        scoring: DirectionOnly(
            &[Measured::ArterialPressure, Measured::HeartRate],
            Lean::UpAfterNightDip,
        ),
        source: "Somers 1993",
    },
];
