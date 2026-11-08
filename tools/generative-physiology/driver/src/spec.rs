use core::fmt;
use core::num::{NonZeroU32, NonZeroU64};
use core::str::{FromStr, SplitWhitespace};
use ng_contract::{
    AdmissionError, BoundaryEvent, BoundaryInput, CoupledSolver, Digest, Dimensionless, ErrorScales, InputId,
    Legs, Measure, Posture, SimTime,
};
use ng_validate::{ReferenceIntegrator, anchor_moment};

/// Bumped whenever what a run writes changes meaning.
pub const CAPTURE_VERSION: u32 = 1;

const GRAMMAR: u32 = 1;
const NS_PER_MS: u64 = 1_000_000;
const NS_PER_US: u64 = 1_000;
/// crates/ng-contract/src/admission.rs:162: the fence opens one cycle, 1 s, after the anchor.
const FENCE_MS: u32 = 1_000;

/// Whole ms after the anchor, at or after the fence.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Stamp(u32);
impl Stamp {
    pub fn after_anchor_ms(ms: u32) -> Option<Self> {
        (ms >= FENCE_MS).then_some(Self(ms))
    }
    pub fn ms(self) -> u32 {
        self.0
    }
    pub fn sim_time(self) -> SimTime {
        SimTime::from_ns(anchor_moment().nanoseconds() + u64::from(self.0) * NS_PER_MS)
    }
    pub fn at_or_after(at: SimTime) -> Option<Self> {
        let after = at.nanoseconds().checked_sub(anchor_moment().nanoseconds())?;
        Self::after_anchor_ms(u32::try_from(after.div_ceil(NS_PER_MS)).ok()?)
    }
}

/// The trunk's sine in thousandths; the engine reads k / 1000.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Trunk(i16);
impl Trunk {
    pub fn thousandths(k: i16) -> Option<Self> {
        (-1000..=1000).contains(&k).then_some(Self(k))
    }
    pub fn get(self) -> i16 {
        self.0
    }
}

fn legs_word(legs: Legs) -> &'static str {
    match legs {
        Legs::Horizontal => "horizontal",
        Legs::SeatedDependent => "seated",
        Legs::Standing => "standing",
    }
}

fn legs_of(word: &str) -> Option<Legs> {
    [Legs::Horizontal, Legs::SeatedDependent, Legs::Standing]
        .into_iter()
        .find(|&legs| legs_word(legs) == word)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Setting {
    pub trunk: Trunk,
    pub legs: Legs,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PostureEvent {
    pub at: Stamp,
    pub setting: Setting,
}
impl PostureEvent {
    /// Content addressed: one instant and one setting give one id in every run, so runs that share a prefix
    /// share ids, and the seal covers ids (crates/ng-contract/src/boundary.rs:111-130).
    pub fn id(&self) -> InputId {
        let mut bytes = b"gp-posture\n".to_vec();
        bytes.extend_from_slice(self.words().as_bytes());
        InputId::new(Digest::of_canonical(&bytes))
    }
    pub fn boundary_event(&self) -> Result<BoundaryEvent, AdmissionError> {
        let trunk = Measure::<Dimensionless>::new(f64::from(self.setting.trunk.0) / 1000.0)?;
        let posture = Posture::new(trunk, self.setting.legs)?;
        Ok(BoundaryEvent::new(
            self.id(),
            self.at.sim_time(),
            BoundaryInput::Posture(posture),
        ))
    }
    /// The spec's event line, which is also a closed loop's answer.
    pub fn line(&self) -> String {
        format!("event {}", self.words())
    }
    pub fn parse_line(text: &str) -> Result<Self, SpecError> {
        let Some(rest) = text.strip_prefix("event ") else {
            return Err(SpecError::Malformed { line: 1 });
        };
        let Line::Event(event) = event(Words::of(rest, 1))? else {
            return Err(SpecError::Malformed { line: 1 });
        };
        if event.line() != text {
            return Err(SpecError::NotCanonical { line: 1 });
        }
        Ok(event)
    }
    fn words(&self) -> String {
        let Self { at, setting } = self;
        format!(
            "at-ms {} trunk {} legs {}",
            at.0,
            setting.trunk.0,
            legs_word(setting.legs)
        )
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CoreChoice {
    /// `ng_solver::Core::new`.
    Picked { cap_ms: NonZeroU32 },
    /// Every field enters the resolution digest, so every field enters the run identity.
    Reference {
        step_us: NonZeroU32,
        floor_us: NonZeroU32,
        absolute: f64,
        relative: f64,
    },
}
impl CoreChoice {
    pub fn build(&self) -> Result<Box<dyn CoupledSolver>, AdmissionError> {
        match *self {
            Self::Picked { cap_ms } => Ok(Box::new(ng_solver::Core::new(
                u64::from(cap_ms.get()) * NS_PER_MS,
            ))),
            Self::Reference {
                step_us,
                floor_us,
                absolute,
                relative,
            } => {
                let scales = ErrorScales::uniform(Measure::new(absolute)?, Measure::new(relative)?)?;
                let floor =
                    NonZeroU64::from(floor_us).saturating_mul(NonZeroU64::MIN.saturating_add(NS_PER_US - 1));
                Ok(Box::new(
                    ReferenceIntegrator::new(u64::from(step_us.get()) * NS_PER_US)
                        .with_floor(floor)
                        .with_scales(scales),
                ))
            }
        }
    }
    fn words(&self) -> String {
        match self {
            Self::Picked { cap_ms } => format!("ng-solver cap-ms {cap_ms}"),
            Self::Reference {
                step_us,
                floor_us,
                absolute,
                relative,
            } => format!(
                "reference step-us {step_us} floor-us {floor_us} absolute {absolute} relative {relative}"
            ),
        }
    }
}

/// A variant is part of the canonical text, so variants never share a spec id.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    None,
    /// No event is in the admission port: each is enqueued at the first accepted head at or after its stamp
    /// minus the lead.
    Late {
        lead_ms: NonZeroU32,
    },
    /// Checkpoint at the first accepted head at or after `at`. Every event stamped at least the body's longest
    /// step after `at`, which no step up to that head can reach, is then enqueued on the original and on a copy
    /// resumed into a freshly compiled body and a fresh core, and both run to the end.
    CheckpointResume {
        at: Stamp,
    },
    /// As `CheckpointResume`, but the copy is a branch rewound from the original's end, in its core.
    Rewind {
        at: Stamp,
    },
    ObserverOff,
    Replica(u8),
}
impl Variant {
    pub fn words(&self) -> String {
        match self {
            Self::None => "none".to_string(),
            Self::Late { lead_ms } => format!("late lead-ms {lead_ms}"),
            Self::CheckpointResume { at } => format!("checkpoint-resume at-ms {}", at.0),
            Self::Rewind { at } => format!("rewind at-ms {}", at.0),
            Self::ObserverOff => "observer-off".to_string(),
            Self::Replica(copy) => format!("replica {copy}"),
        }
    }
    fn feeds_alone(&self) -> bool {
        matches!(self, Self::None | Self::ObserverOff | Self::Replica(_))
    }
}

/// The digest of the capture version and the canonical text, which names the run's directory. It covers the
/// spec, not the engine: `status` reports a present run as stale unless its identity digests equal the audit's.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct SpecId(Digest);
impl SpecId {
    pub fn digest(self) -> Digest {
        self.0
    }
}
impl fmt::Display for SpecId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex(&self.0))
    }
}

pub fn hex(digest: &Digest) -> String {
    digest
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn digest_of_hex(text: &str) -> Option<Digest> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let nibble = |digit: u8| match digit {
        b'0'..=b'9' => Some(digit - b'0'),
        b'a'..=b'f' => Some(digit - b'a' + 10),
        _ => None,
    };
    let mut digest = [0u8; 32];
    let (pairs, _) = bytes.as_chunks::<2>();
    for (byte, &[high, low]) in digest.iter_mut().zip(pairs) {
        *byte = (nibble(high)? << 4) | nibble(low)?;
    }
    Some(Digest::from_bytes(digest))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpecError {
    NoFinalNewline,
    UnknownLine {
        line: usize,
    },
    OutOfOrder {
        line: usize,
    },
    Missing {
        keyword: &'static str,
    },
    Malformed {
        line: usize,
    },
    BeforeFence {
        line: usize,
    },
    /// The engine would keep the larger id of two postures at one stamp (boundary.rs:149, 654-659).
    EqualStamps {
        line: usize,
    },
    Unsorted {
        line: usize,
    },
    EndBeforeEvent {
        line: usize,
    },
    InstantAfterEnd {
        line: usize,
    },
    ClosedLoopVariant {
        line: usize,
    },
    NotCanonical {
        line: usize,
    },
}
impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFinalNewline => write!(f, "the spec does not end with a newline"),
            Self::UnknownLine { line } => write!(f, "line {line}: no such line kind"),
            Self::OutOfOrder { line } => write!(f, "line {line}: out of order or repeated"),
            Self::Missing { keyword } => write!(f, "no {keyword} line"),
            Self::Malformed { line } => write!(f, "line {line}: malformed"),
            Self::BeforeFence { line } => write!(f, "line {line}: a stamp before the fence at 1000 ms"),
            Self::EqualStamps { line } => write!(f, "line {line}: the stamp of the event before it"),
            Self::Unsorted { line } => write!(f, "line {line}: a stamp before the event before it"),
            Self::EndBeforeEvent { line } => write!(f, "line {line}: an event after the end"),
            Self::InstantAfterEnd { line } => write!(f, "line {line}: an instant after the end"),
            Self::ClosedLoopVariant { line } => write!(f, "line {line}: a variant that moves the feed"),
            Self::NotCanonical { line } => write!(f, "line {line}: not the canonical text"),
        }
    }
}

enum Line {
    Grammar,
    Core(CoreChoice),
    End(Stamp),
    Event(PostureEvent),
    ClosedLoop(String),
    Variant(Variant),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Count {
    One,
    Optional,
    Many,
}

struct Kind {
    keyword: &'static str,
    count: Count,
    parse: fn(Words<'_>) -> Result<Line, SpecError>,
}

/// Every line kind, in the order a spec holds them.
const KINDS: [Kind; 6] = [
    Kind {
        keyword: "gp-run",
        count: Count::One,
        parse: grammar,
    },
    Kind {
        keyword: "core",
        count: Count::One,
        parse: core_line,
    },
    Kind {
        keyword: "end-ms",
        count: Count::One,
        parse: end,
    },
    Kind {
        keyword: "event",
        count: Count::Many,
        parse: event,
    },
    Kind {
        keyword: "closed-loop",
        count: Count::Optional,
        parse: closed_loop,
    },
    Kind {
        keyword: "variant",
        count: Count::One,
        parse: variant,
    },
];

struct Words<'t> {
    rest: &'t str,
    words: SplitWhitespace<'t>,
    line: usize,
}
impl<'t> Words<'t> {
    fn of(rest: &'t str, line: usize) -> Self {
        Self {
            rest,
            words: rest.split_whitespace(),
            line,
        }
    }
    fn malformed(&self) -> SpecError {
        SpecError::Malformed { line: self.line }
    }
    fn word(&mut self) -> Result<&'t str, SpecError> {
        self.words.next().ok_or(SpecError::Malformed { line: self.line })
    }
    fn number<T: FromStr>(&mut self) -> Result<T, SpecError> {
        self.word()?.parse().map_err(|_| self.malformed())
    }
    fn keyed<T: FromStr>(&mut self, key: &str) -> Result<T, SpecError> {
        if self.word()? != key {
            return Err(self.malformed());
        }
        self.number()
    }
    fn stamp(&mut self, key: &str) -> Result<Stamp, SpecError> {
        Stamp::after_anchor_ms(self.keyed(key)?).ok_or(SpecError::BeforeFence { line: self.line })
    }
    fn nonzero(&mut self, key: &str) -> Result<NonZeroU32, SpecError> {
        NonZeroU32::new(self.keyed(key)?).ok_or(self.malformed())
    }
    fn finish(mut self, line: Line) -> Result<Line, SpecError> {
        match self.words.next() {
            None => Ok(line),
            Some(_) => Err(self.malformed()),
        }
    }
}

fn grammar(mut words: Words<'_>) -> Result<Line, SpecError> {
    if words.number::<u32>()? != GRAMMAR {
        return Err(words.malformed());
    }
    words.finish(Line::Grammar)
}

fn core_line(mut words: Words<'_>) -> Result<Line, SpecError> {
    let choice = match words.word()? {
        "ng-solver" => CoreChoice::Picked {
            cap_ms: words.nonzero("cap-ms")?,
        },
        "reference" => {
            let (step_us, floor_us) = (words.nonzero("step-us")?, words.nonzero("floor-us")?);
            let absolute: f64 = words.keyed("absolute")?;
            let relative: f64 = words.keyed("relative")?;
            let scaled = absolute.is_finite() && absolute > 0.0;
            if !scaled || !relative.is_finite() || relative.is_sign_negative() {
                return Err(words.malformed());
            }
            CoreChoice::Reference {
                step_us,
                floor_us,
                absolute,
                relative,
            }
        }
        _ => return Err(words.malformed()),
    };
    words.finish(Line::Core(choice))
}

fn end(mut words: Words<'_>) -> Result<Line, SpecError> {
    let ms = words.number()?;
    let at = Stamp::after_anchor_ms(ms).ok_or(SpecError::BeforeFence { line: words.line })?;
    words.finish(Line::End(at))
}

fn event(mut words: Words<'_>) -> Result<Line, SpecError> {
    let at = words.stamp("at-ms")?;
    let trunk = Trunk::thousandths(words.keyed("trunk")?).ok_or(words.malformed())?;
    if words.word()? != "legs" {
        return Err(words.malformed());
    }
    let legs = legs_of(words.word()?).ok_or(words.malformed())?;
    let setting = Setting { trunk, legs };
    words.finish(Line::Event(PostureEvent { at, setting }))
}

fn closed_loop(words: Words<'_>) -> Result<Line, SpecError> {
    let rule = words.rest.trim();
    if rule.is_empty() {
        return Err(words.malformed());
    }
    Ok(Line::ClosedLoop(rule.to_string()))
}

fn variant(mut words: Words<'_>) -> Result<Line, SpecError> {
    let chosen = match words.word()? {
        "none" => Variant::None,
        "late" => Variant::Late {
            lead_ms: words.nonzero("lead-ms")?,
        },
        "checkpoint-resume" => Variant::CheckpointResume {
            at: words.stamp("at-ms")?,
        },
        "rewind" => Variant::Rewind {
            at: words.stamp("at-ms")?,
        },
        "observer-off" => Variant::ObserverOff,
        "replica" => Variant::Replica(words.number()?),
        _ => return Err(words.malformed()),
    };
    words.finish(Line::Variant(chosen))
}

fn render(
    core: &CoreChoice,
    end: Stamp,
    events: &[PostureEvent],
    closed_loop: Option<&str>,
    variant: &Variant,
) -> String {
    let mut text = format!("gp-run {GRAMMAR}\ncore {}\nend-ms {}\n", core.words(), end.0);
    for event in events {
        text.push_str(&event.line());
        text.push('\n');
    }
    if let Some(rule) = closed_loop {
        text.push_str(&format!("closed-loop {rule}\n"));
    }
    text.push_str(&format!("variant {}\n", variant.words()));
    text
}

/// `closed_loop` names the study's rule and its parameters; the driver never interprets it, and the id covers it.
#[derive(Clone, PartialEq, Debug)]
pub struct RunSpec {
    core: CoreChoice,
    end: Stamp,
    events: Vec<PostureEvent>,
    closed_loop: Option<String>,
    variant: Variant,
}
impl RunSpec {
    /// The only constructor. It takes only canonical text, so a spec, its text and its id cannot disagree.
    pub fn parse(text: &str) -> Result<Self, SpecError> {
        let body = text.strip_suffix('\n').ok_or(SpecError::NoFinalNewline)?;
        let mut seen = [0usize; KINDS.len()];
        let mut last = 0;
        let (mut core, mut end, mut closed_loop, mut variant) = (None, None, None, None);
        let mut events: Vec<(PostureEvent, usize)> = Vec::new();
        let mut lines = [0usize; KINDS.len()];
        for (at, text) in body.split('\n').enumerate() {
            let line = at + 1;
            let keyword = text.split_whitespace().next().unwrap_or_default();
            let kind = (KINDS.iter())
                .position(|kind| kind.keyword == keyword)
                .ok_or(SpecError::UnknownLine { line })?;
            let repeated = seen[kind] > 0 && KINDS[kind].count != Count::Many;
            if kind < last || repeated {
                return Err(SpecError::OutOfOrder { line });
            }
            last = kind;
            seen[kind] += 1;
            lines[kind] = line;
            let rest = text.trim_start().strip_prefix(keyword).unwrap_or_default();
            match (KINDS[kind].parse)(Words::of(rest, line))? {
                Line::Grammar => {}
                Line::Core(choice) => core = Some(choice),
                Line::End(at) => end = Some(at),
                Line::Event(event) => events.push((event, line)),
                Line::ClosedLoop(rule) => closed_loop = Some(rule),
                Line::Variant(chosen) => variant = Some(chosen),
            }
        }
        if let Some((missing, _)) =
            (KINDS.iter().zip(seen)).find(|(kind, seen)| kind.count == Count::One && *seen == 0)
        {
            return Err(SpecError::Missing {
                keyword: missing.keyword,
            });
        }
        let (Some(core), Some(end), Some(variant)) = (core, end, variant) else {
            return Err(SpecError::Missing { keyword: "core" });
        };
        for pair in events.windows(2) {
            let [(before, _), (after, line)] = pair else {
                continue;
            };
            if after.at == before.at {
                return Err(SpecError::EqualStamps { line: *line });
            }
            if after.at < before.at {
                return Err(SpecError::Unsorted { line: *line });
            }
        }
        if let Some((last, line)) = events.last()
            && last.at > end
        {
            return Err(SpecError::EndBeforeEvent { line: *line });
        }
        let variant_line = lines[KINDS.len() - 1];
        if let Variant::CheckpointResume { at } | Variant::Rewind { at } = variant
            && at > end
        {
            return Err(SpecError::InstantAfterEnd { line: variant_line });
        }
        if closed_loop.is_some() && !variant.feeds_alone() {
            return Err(SpecError::ClosedLoopVariant { line: variant_line });
        }
        let spec = Self {
            core,
            end,
            events: events.into_iter().map(|(event, _)| event).collect(),
            closed_loop,
            variant,
        };
        let canonical = spec.canonical();
        if canonical != text {
            let differs =
                (text.split('\n').zip(canonical.split('\n'))).position(|(given, wanted)| given != wanted);
            let line = differs.unwrap_or(text.split('\n').count().min(canonical.split('\n').count()));
            return Err(SpecError::NotCanonical { line: line + 1 });
        }
        Ok(spec)
    }
    pub fn canonical(&self) -> String {
        let Self {
            core,
            end,
            events,
            closed_loop,
            variant,
        } = self;
        render(core, *end, events, closed_loop.as_deref(), variant)
    }
    pub fn id(&self) -> SpecId {
        let text = format!("gp-run-id\n{CAPTURE_VERSION}\n{}", self.canonical());
        SpecId(Digest::of_canonical(text.as_bytes()))
    }
    pub fn core(&self) -> &CoreChoice {
        &self.core
    }
    pub fn end(&self) -> Stamp {
        self.end
    }
    pub fn events(&self) -> &[PostureEvent] {
        &self.events
    }
    pub fn closed_loop(&self) -> Option<&str> {
        self.closed_loop.as_deref()
    }
    pub fn variant(&self) -> Variant {
        self.variant
    }
    pub fn realized(&self, actions: &[PostureEvent]) -> Result<Self, SpecError> {
        let mut events: Vec<PostureEvent> = self.events.iter().chain(actions).copied().collect();
        events.sort_by_key(|event| event.at);
        Self::parse(&render(&self.core, self.end, &events, None, &Variant::None))
    }
}
