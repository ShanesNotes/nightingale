# 03: The teacher driver and its run format

Type: task
Status: open
Blocked by: 02
Runs on: written on the workstation; every build, test and run happens on the heavy-run machine

**What it builds:** `tools/generative-physiology/driver/`, a Rust program outside the workspace members, as `tools/bench` is (`tools/bench/Cargo.toml:7-13`). It has its own empty `[workspace]`, its own `Cargo.lock`, and path dependencies on `ng-contract`, `ng-record`, `ng-observe`, `ng-solver` and `ng-validate` only, so a locked build needs no network. It makes one teacher run per process from a run spec, through public items only, and writes the run's directory whole or not at all. Each run keeps its history links in a file store of its own, as the bench's link log does (`tools/bench/src/link_log.rs:10-37`), so its memory stays flat (native-contract ticket 11); the file is deleted once the run's directory is complete. It never chooses a schedule, applies measurement error or judges a result.

## Usage

```text
gp-driver smoke --out DIR             # a 20 s run on Core; checks the pinned first second
gp-driver run SPEC --out DIR          # one teacher run from a spec file
gp-driver interact SPEC --out DIR     # a closed-loop run: one JSON line out per beat, one line back
gp-driver status --out DIR --identity A0-SUMMARY SPEC...   # one process: present, stale, fault, partial or missing
```

`run` and `interact` take `--memory-cap-gb N`, which stays outside the run id.

A run spec is one canonical text. Stamps are whole ms after the anchor, 06:55 (`crates/ng-validate/src/slice.rs:501-504`), and the trunk is its sine in thousandths:

```text
gp-run 1
core ng-solver cap-ms 10
end-ms 900000
event at-ms 1000 trunk 1000 legs standing
event at-ms 180000 trunk 0 legs horizontal
event at-ms 330000 trunk 1000 legs standing
variant none
```

Two call sites:

```rust
let spec = RunSpec::parse(&fs::read_to_string(path)?)?;   // the only constructor
let teacher = Teacher::new(spec.core())?;                  // compiles the arena body once
let summary = teacher.run(&spec, MemoryCap::gb(8), &out)?; // out/<run id>/, or out/<run id>.fault/
```

```rust
// The closed loop: the driver asks after every beat edge and enqueues what comes back.
let summary = teacher.interact(&spec, &mut LineLink::stdio(), MemoryCap::gb(8), &out)?;
```

## Types and signatures

```rust
// spec.rs: what a teacher run is told
pub const CAPTURE_VERSION: u32 = 1;   // bumped whenever what a run writes changes meaning

/// Whole ms after the anchor. Refuses anything before the fence at 1,000 ms
/// (crates/ng-contract/src/admission.rs:162).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Stamp(u32);
impl Stamp {
    pub fn after_anchor_ms(ms: u32) -> Result<Self, SpecError> { todo!() }
    pub fn sim_time(self) -> SimTime { todo!("anchor_moment() plus ms times 10^6 ns") }
}
/// The trunk's sine in thousandths, -1000..=1000; the engine reads k / 1000.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Trunk(i16);
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Setting { pub trunk: Trunk, pub legs: Legs }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PostureEvent { pub at: Stamp, pub setting: Setting }
impl PostureEvent {
    /// Content addressed: one instant and one setting give one id in every run, so runs that share a
    /// prefix share ids, and the seal covers ids (crates/ng-contract/src/boundary.rs:111-127).
    pub fn id(&self) -> InputId { todo!() }
}
pub enum CoreChoice {
    Picked { cap_ms: u32 },                                         // ng_solver::Core::new
    Reference { step_us: u32, floor_us: u32, absolute: f64, relative: f64 },   // all enter identity
}
/// Audit variants only. A variant is part of the canonical text, so variants never share a run id.
pub enum Variant {
    None,
    Late { lead_ms: u32 },              // enqueue each event once the head passes its stamp minus the lead
    /// Checkpoint at the first head at or after `at`; every later event is then enqueued on the original
    /// and on a copy resumed into a freshly compiled body and a fresh core, and both run to the end.
    CheckpointResume { at: Stamp },
    /// The parent runs to the end; a branch rewound to its checkpoint at `at` takes the same later events.
    Rewind { at: Stamp },
    ObserverOff,
    Replica(u8),
}
/// `closed_loop` names the study's rule and its parameters. The driver never interprets it; the id covers it.
pub struct RunSpec { core: CoreChoice, end: Stamp, events: Vec<PostureEvent>, closed_loop: Option<String>, variant: Variant }
impl RunSpec {
    /// Refuses equal or unsorted stamps, a stamp before the fence and an end before the last event.
    /// Equal stamps are refused because the engine would keep the larger id (boundary.rs:149, 646-651).
    pub fn parse(text: &str) -> Result<Self, SpecError> { todo!() }
    pub fn canonical(&self) -> String { todo!() }
    /// The id covers the spec and the capture version, not the engine. `status` reports a present run as
    /// stale unless its model, packet, resolution and run-identity digests equal the audit's A0 run's.
    pub fn id(&self) -> RunId { todo!("digest of CAPTURE_VERSION and the canonical text") }
    pub fn core(&self) -> &CoreChoice { todo!() }
}
```

```rust
// teacher.rs: one admission, one run
pub struct Teacher { body: CompiledBody, core: Box<dyn CoupledSolver>, catalogue: Catalogue }
impl Teacher {
    pub fn new(core: &CoreChoice) -> Result<Self, TeachError> { todo!() }
    // admit_at(arena_packet, a port holding every event, arena_lens, anchor_moment(), &links); about 17 s
    // (crates/ng-validate/src/initialize.rs:219-240). Then step by hand, as validate/run.rs:4-15 does:
    //   Accepted: add each row's chord integral; log the seal; when the CV phase row falls, write an edge
    //     and the beat that ended there; advance the diagnostic observer.
    //   Refused: retry and count.
    //   Faulted: finish the directory as <run id>.fault/, with the fault's kind, instant and attempted ns.
    // Every 10,000 steps read VmRSS; above the --memory-cap-gb flag, stop the same way with MemoryCap.
    pub fn run(&self, spec: &RunSpec, cap: MemoryCap, out: &Path) -> Result<RunSummary, TeachError> { todo!() }
    /// After each beat edge, sends the beat and the earliest admissible stamp and waits for the answer.
    /// A refused stamp ends the run with a protocol error; the driver never moves a stamp.
    pub fn interact(&self, spec: &RunSpec, link: &mut dyn PolicyLink, cap: MemoryCap, out: &Path)
        -> Result<RunSummary, TeachError> { todo!() }
}
/// The run's history store: one append-only file of links, read back by seal, deleted when the run's
/// directory is complete. The session keeps it from admission to the end (crates/ng-record/src/checkpoint.rs:58-62).
pub struct LinkFile { /* the file, its end and a seal index */ }
impl HistoryStore for LinkFile { /* resolve, keep */ }
/// The resident-memory limit a run stops at; the summary records it, and the run id does not cover it.
pub struct MemoryCap(u64);
impl MemoryCap {
    pub fn gb(gigabytes: u32) -> Self { todo!() }
}
pub trait PolicyLink {
    fn after_beat(&mut self, beat: &BeatReading, earliest: Stamp) -> Result<Option<PostureEvent>, LinkError>;
}
/// One JSON line per beat on stdout, one answer line on stdin: "none", or an action's stamp and setting.
pub struct LineLink { /* stdin and stdout */ }
impl LineLink { pub fn stdio() -> Self { todo!() } }
impl PolicyLink for LineLink { /* ... */ }
```

```rust
// capture.rs: names, beats and edges
/// Refuses a body whose instance targets miss a column (every column has one: crates/ng-contract/src/compile.rs:1089-1100,
/// 1137-1144, 1191-1229, 1243-1278), or that lacks the phase, rate, pressure, filtration or aortic valve rows.
pub struct Catalogue { rows: Vec<RowName>, columns: Vec<ColumnName>, ids: Vec<ColumnId>, phase: RowId,
                       heart_rate: RowId, pressure: RowId, filtration: RowId, aortic_water: ColumnId }
impl Catalogue { pub fn of(body: &CompiledBody) -> Result<Self, CatalogueError> { todo!() } }
/// One beat, edge to edge, computed once, here. Heart rate is one over the length; the phase integrates
/// the rate row and resets at 1.0 (crates/ng-cardiovascular/src/rhythm.rs:59-89).
pub struct BeatReading {
    pub ordinal: u64, pub start: SimTime, pub end: SimTime,
    pub heart_rate: f64,          // per s
    pub arterial_pressure: f64,   // beat mean of the CV "blood pressure" row at systemic arteries
    pub stroke: f64,              // net extent of the aortic valve's water column over the beat
    pub filtration: f64,          // beat mean of the RN "filtration" row
}
```

```rust
// archive.rs: one run directory
pub struct RunSummary {
    pub id: RunId, pub capture_version: u32, pub source_commit: String,
    pub model: Digest, pub packet: Digest, pub resolution: Digest, pub run_identity: Digest,
    pub first_seal: Digest, pub last_seal: Digest, pub revisions: u64, pub refusals: u64, pub beats: u32,
    pub fault: Option<FaultSummary>, pub admission_s: f64, pub step_wall_s: f64, pub peak_rss_bytes: u64,
    pub files: Vec<(String, Digest)>,
}
```

## The run directory

A run writes `<out>/<run id>.partial-<pid>/` and renames it to `<out>/<run id>/` when every file is flushed. A run that faults, or passes its memory cap, is renamed whole to `<out>/<run id>.fault/` instead, with its spec, its summary and what it captured up to that point. `status` reports it, and no command removes or redoes it. A person reads it first. A partial directory whose process is gone has no summary, and the next run of its spec removes and redoes it.

| File | Content |
|---|---|
| `spec.txt` | the canonical spec |
| `edges.npy` | u64, one row per edge (the start, every beat edge, the end): time ns, revision, mode values, every row's value as hi and lo bits, every row's cumulative chord integral, every column's cumulative forward and reverse extents, every conserved quantity's inventory, inflow and outflow |
| `beats.npy` | f64, one row per complete beat: the fields of `BeatReading` |
| `steps.npy` | u64, one row per accepted step: revision, end ns, refusals before it, the seal's four words |
| `applied.npy` | each event's index in the spec and the revision that applied it |
| `lens.npy` | the arena lens's observations, kept only as a diagnostic |
| `summary.txt` | the fields of `RunSummary`, one per line |
| `realized.txt` | closed-loop runs only: the run's actions as an open-loop spec |

Extents and integrals are read only at edges, from the newest segment, `session.conservation().head()` (`crates/ng-record/src/session.rs:75-77`, `crates/ng-record/src/history.rs:575-579`): `let (forward, reverse) = segment.exchange(id)` returns two curves (`history.rs:624-628`), and the last coefficient of each curve's `on(segment.interval())` is its cumulative value at the step's end (`history.rs:642-648`). A checkpoint for an audit variant is `session.checkpoint()`, with no store argument (`crates/ng-record/src/session.rs:159-163`). So a beat's extent is an exact difference of cumulative values, taken in `Compensated` before rounding. Column ids come from deduplicated `Target::Column` instances (`crates/ng-contract/src/body.rs:442-445, 478`), because `ColumnId::at` is crate-private (`body.rs:21`). Rows are stored as both limbs of `Compensated::bits()` (`crates/ng-contract/src/compensated.rs:110`).

## Acceptance

- [ ] On the heavy-run machine, `cargo build --release --locked --manifest-path tools/generative-physiology/driver/Cargo.toml` succeeds.
- [ ] `gp-driver smoke` (a stand at 10 s, ending at 20 s) writes one complete run. Its packet digest and the seal at the head at 1000.079707 ms equal the values `crates/ng-validate/tests/trace.rs:135-136` pins for Core at a 10 ms cap; it has at least 15 beats, no fault, and the stand in `applied.npy`.
- [ ] A second `gp-driver run` with the same spec and `--out` writes nothing and reports the run present.
- [ ] A driver killed with SIGKILL leaves only a partial directory, which the next run of that spec removes and redoes.
- [ ] A run forced past a small memory cap ends as a `.fault` directory, which `status` reports and a second `run` leaves alone.
- [ ] `gp-driver status` reports present, stale, fault, partial and missing runs for a list of specs in one process.
- [ ] Light tests pass: the spec's round trip, id addressing, refusals at the fence and on equal stamps, the `.npy` round trip, and a catalogue that covers all 257 columns of the compiled arena body.
- [ ] `tools/verify.d/10-rust.sh` gains format, clippy, test and check lines for the driver's manifest beside bench's (`tools/verify.d/10-rust.sh:42`), and the proof from a clean git checkout of the PR head on the heavy-run machine shows `full_proof: yes`. An archive snapshot cannot give that proof, because `.agents/skills/verify-nightingale/prove.sh:6` needs a git checkout.

## Non-goals

- The audit's cases and report (ticket 04), the reduction (ticket 05), schedules and the policy (ticket 06).
- Branching by rewind outside the audit's variant, and any engine change.

## Comments

The root `clippy.toml` reaches this crate, so its bans on host libm, hash maps and wall clocks apply. The one wall clock, for cost evidence in the summary, carries a local allow and its reason.

Question for the developer reviewer: any path under `tools/**` runs every verify step, mutants included (`tools/gate/paths.toml:10-16`). Should a rule ahead of it send `tools/generative-physiology/**` to fewer steps?
