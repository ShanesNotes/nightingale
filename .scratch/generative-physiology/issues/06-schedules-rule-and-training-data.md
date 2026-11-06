# 06: Schedules, the scripted rule and the training data

Type: task
Status: open
Blocked by: 04, 05
Runs on: written on the workstation; the teacher runs happen on the heavy-run machine

**What it builds:** the study's half of a teacher run, the driver's `errors` command, and the pinned Python environment: a requirements file of numpy and CPU PyTorch with their wheel hashes, installed on the heavy-run machine. Tickets 07 to 12 use that environment, and ticket 09 freezes its digest. It draws start histories and posture schedules from seeds, implements the scripted rule once for the teacher and for every model, applies measurement error from one table, runs the training and validation-tuning partitions through `gp-driver`, and exports the training views. The driver owns run ids and beat readings, and Python never computes either.

## Usage

```text
gp-driver errors --seed SEED --beats 4000 --out DATA/noise/CASE.npy        # the engine's own error draws
python3 -I tools/generative-physiology/gp.py run train --data DATA --workers W        # 224 runs
python3 -I tools/generative-physiology/gp.py run validation --data DATA --workers W   # 32 tuning runs
python3 -I tools/generative-physiology/gp.py export --data DATA --out DATA/export     # training views only
```

```python
specs = schedules(Partition.TRAIN)                  # P-rand and P-pol, from frozen seeds
rule = Rule(RuleSpec.corrects(low, high), NoiseTable.read(path), start=Setting.FLAT)
event = rule.react(beat)                            # at a beat's end; None, or an action 2 s later
```

`run` asks `gp-driver status` once which of its specs are present, stale, faulted, partial or missing, then starts one `gp-driver` process per missing or partial run, longest first, as many as the audit's worker count allows. A closed-loop run goes through `gp-driver interact`. For each beat line the driver writes, the rule answers one line.

## What it draws

**Start histories.** A history runs from the fence at 1 s to its end H, 300 s unless the audit lengthens it to 600 s (ticket 04). Its first setting starts at 1 s, every later change falls on a whole multiple of 30 s after the anchor, no hold lasts more than 150 s, and the last hold lasts at least 60 s. Settings are drawn from flat, sit, stand and tilt. Training has 14 histories and validation 4, drawn the same way. Ticket 09 draws the held-out histories after the freeze.

**The encounter** starts at the first beat edge at or after H and runs for the audit's encounter length, 10 or 15 minutes. Every case time in ticket 09 is an offset from H, a whole number of ms.

**P-rand.** Posture changes at state-independent times, with exponential gaps of mean 60 s and at least 5 s. Each change goes to a setting other than the current one, drawn from flat, sit, stand and tilt. Three changes in ten are ramps of an odd number n of events, 5 to 11, 0.5 to 2 s apart. From setting S0 to S1, event k of n sets the trunk's sine to S0's plus (S1's minus S0's) times k/n, and takes S1's legs from k = (n + 1)/2 on. So horizontal legs never meet a trunk sine above 0.5, and no family uses a negative trunk; ticket 09's untrained settings rely on both.

**P-pol.** The `Corrects` rule in a closed loop, plus scheduled stands (exponential gaps of mean 120 s and at least 30 s) that give it something to correct. At each beat's end the rule takes the mean of the last three beats' noisy arterial pressure. Below the audit's low threshold, it lies the patient flat 2 s later; above the high threshold, it sits them up 2 s later. It acts only when its last action is at least 30 s old. The `Inverted` rule makes the opposite choices; only the out-of-policy family o1 uses it, in ticket 09's validation cases and held-out set. These are synthetic rules for an experiment, never care instructions.

**Partitions.** Training: 14 histories with 8 P-rand and 8 P-pol schedules each, 224 runs. Validation tuning: 4 histories with 4 of each, 32 runs. A run and every run that shares its start history stay in one partition.

**Measurement error.** One table per case, made by `gp-driver errors` from a lens of two channels, heart rate with a synthetic SD of 0.02 per s and arterial pressure with 2.0, through the public `Channel::error` (`crates/ng-contract/src/lens.rs:79-85`). The k-th beat of a case takes the error the engine gives at instant k ns. That is a host convention, recorded in FROZEN, and it gives the teacher's k-th beat and every model's k-th beat the same error. The study adds it after a reading is made; no model's evolution and no teacher state ever sees it.

**Seeds.** A schedule's draws are a SHA-256 counter over `("gp schedule 1", partition, history, schedule, draw)`, so a schedule replays without a random-number library, and the stored spec is what counts. A case's observation seed is `ObservationSeed::new([0x6770_0001, partition, case, 0])` (`crates/ng-contract/src/scheme.rs:295, 313`). Partitions are an enum, so training and evaluation seeds never meet. The engine's run seed stays `admit_at`'s fixed one (`crates/ng-validate/src/initialize.rs:234`).

## Types and signatures

```python
class Legs(Enum): HORIZONTAL = "horizontal"; SEATED = "seated"; STANDING = "standing"

@dataclass(frozen=True)
class Setting:
    trunk_milli: int            # the trunk's sine in thousandths, -1000..=1000
    legs: Legs
    FLAT: ClassVar["Setting"]; SIT: ClassVar["Setting"]; STAND: ClassVar["Setting"]; TILT: ClassVar["Setting"]

@dataclass(frozen=True)
class PostureEvent:
    at_ms: int                  # whole ms after the anchor; at least 1,000
    setting: Setting

@dataclass(frozen=True)
class RunSpec:
    core: str
    end_ms: int
    events: tuple[PostureEvent, ...]   # strictly increasing
    closed_loop: Optional["RuleSpec"]
    def text(self) -> str: ...         # what gp-driver parses; the driver owns the canonical form and the id

@dataclass(frozen=True)
class RuleSpec:
    kind: Literal["corrects", "inverted"]
    low: float
    high: float
    case: int

class Rule:
    """The scripted clinician: one implementation for the teacher's closed loop and for every model's."""
    def __init__(self, spec: RuleSpec, noise: "NoiseTable", start: Setting): ...
    def react(self, beat: "BeatReading") -> Optional[PostureEvent]: ...

class Partition(Enum): TRAIN = 1; VALIDATION = 2; HELDOUT = 3; REPLAY = 4

def start_histories(partition: Partition) -> list["StartHistory"]: ...
def schedules(partition: Partition) -> list[RunSpec]: ...
def run(partition: Partition, data: Path, workers: int) -> "RunReport": ...
def export(data: Path, out: Path) -> "ExportManifest": ...   # reads TRAIN and VALIDATION only
```

The frozen constants live in one module, `spec.py`: the reaction delay (2 s), the rule's window (3 beats), its refractory time (30 s), the thresholds from ticket 04, the synthetic reading SDs, the schedule families' parameters and the partition sizes.

## Acceptance

- [ ] All 256 runs complete with no engine fault, or at most 1% end as `.fault` directories, each listed with its spec and fault. Every run shares the audit's A0 identity digests.
- [ ] The `Corrects` rule acts on between 10% and 60% of the scheduled stands; outside that range the thresholds are wrong, and the Answer says so before anything else uses the data.
- [ ] Four sampled closed-loop runs, replayed open loop from their `realized.txt`, give equal seal logs.
- [ ] A second `gp.py run train` starts no teacher run, only the one `gp-driver status` process.
- [ ] The requirements file pins every package with its wheel hash, the environment installs from it on the heavy-run machine, and the Answer records its digest and the Python version.
- [ ] `gp.py export` writes the training views with the reduction's digest and a manifest digest, and the Answer records both, with the time and memory the runs took.

Each run lasts its start history plus the encounter: 900 s with a 10-minute encounter and 1,200 s with a 15-minute one, 300 s more if histories grow to 600 s. The plan is 256 runs, 64 to 85 simulated hours: 13 to 34 process-hours at the audit's assumed speed, 3.3 to 8.5 hours of wall time at 4 workers (or at the 3 the workers rule picks for 1,200 s runs), 16 to 22 GB at once and about 7 to 9 GB on disk. The audit's measured speed replaces these figures before the runs start.

## Non-goals

- The validation cases and the held-out set (ticket 09), and training (ticket 10).
- Any change to the engine's error model. The table only calls its public draw.
