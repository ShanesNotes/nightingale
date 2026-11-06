# 08: The model forms and their runtime

Type: task
Status: open
Blocked by: 05, 06
Runs on: written on the workstation; every test runs on the heavy-run machine, in the environment ticket 06 pins

**What it builds:** `models.py`, the forward meaning of C1, C2 and C4a, and of R, which is defined and frozen here but trained only under ticket 11's rule. It also builds the runtime that carries a learned patient through an encounter: a start, events, a free run, beat readings, a checkpoint and a resume. Training lives in ticket 10. One implementation serves training and judging, so no model's forward pass or integrator exists twice.

## Usage

```python
model = models.load(models_dir / "c4a-seed3", reduction)       # refuses another reduction digest or family
run = model.start(case.named_start)                              # C2 takes a ReadingsStart instead
for event in case.events:
    run.enqueue(event)
run.advance(until=case.end_ns)                                   # a free run: no reset, no teacher value
beats = run.beats()                                              # noiseless beat readings, never fed back
saved = run.checkpoint()
again = models.resume(saved, model)                              # continues with identical bits
```

## The forms

Every named model carries the named totals, plus any named memory the screen added (ticket 07). Its totals move only through the reduction's transfers. The totals equal the start plus B times the cumulative net extents, so balance holds by construction. Each transfer has two nonnegative channels. The forward channel is proportional to the amount at the total it drains, and so is the reverse channel at its other end, so an empty total stops supplying. A channel whose donor would be a receipt is absent, except intake, whose rate takes no donor factor.

| Model | Channel coefficients | Readings | What it isolates |
|---|---|---|---|
| C1 linear | `softplus(a + b . u)`: the drive u alone, so the model is linear in its state for any fixed posture | affine in the standardized totals and the drive | whether the tested range needs nonlinearity |
| C4a named | `softplus` of one network per transfer origin, which reads the standardized totals, the drive and a learned per-transfer embedding: two hidden layers of 16 to 64 with tanh | a network of the totals and the drive | the candidate |
| C2 readings only | none: a recurrent network over beats, 32 to 64 hidden units. Each beat it reads its own previous noiseless reading, the drive at the beat's start and the beat's mean drive, and it predicts the next beat's log heart rate and pressure | its own predictions | access to named state |
| R richer state | C4a's form on the each-row reduction (ticket 05) with the nine memories carried by learned rates | as C4a | whether the named totals discard what matters |

The drive u is the public posture payload, the trunk's sine and two leg indicators. No engine law form is copied into any model. Every rate, its dependence on state and drive, and every reading is learned. The parameter count of every model stays at most 100,000; R stays inside it because its networks are shared per origin.

**A named model's beats.** Its decoder gives the heart rate and the pressure at every instant. A phase integrates the decoded rate and wraps at one, as the teacher's does (`crates/ng-cardiovascular/src/rhythm.rs:59-89`); a wrap is located within 1 microsecond by bisection. A beat reading is one over the beat's length and the beat's mean pressure, the same definitions the driver uses. C2's beat length is one over its own heart rate.

**Integration.** Classical fourth-order Runge-Kutta at 0.1 s, on integer nanoseconds, with every interval split at an event and at a beat edge. A step that would leave any total negative is refused and halved, down to 1/64 of the step; past that the run stops with `ModelFault`. Nothing is clamped. The learned object is a rate per simulated second, never a per-step increment.

**Determinism.** Rollouts run in float64 on one CPU thread, with deterministic algorithms and a fixed loop order. A rollout repeats bit for bit within the pinned environment; nothing is promised across environments.

## Types and signatures

```python
@dataclass(frozen=True)
class NamedStart:
    edge_ns: int
    totals: tuple[float, ...]       # at the teacher's first beat edge of the encounter
    memories: tuple[float, ...]
    setting: Setting

@dataclass(frozen=True)
class ReadingsStart:
    edge_ns: int
    beats: tuple[BeatReading, ...]   # the last 20 noiseless teacher beats before the encounter, with their drives
    setting: Setting

class Family(Enum): C1 = "C1"; C2 = "C2"; C4A = "C4a"; R = "R"

class LearnedRun:
    def enqueue(self, event: PostureEvent) -> None: ...
    def advance(self, until_ns: int) -> "AdvanceEvidence": ...      # steps, refusals, faults
    def beats(self) -> list[BeatReading]: ...                         # since the last call; reading moves nothing
    def checkpoint(self) -> "LearnedCheckpoint": ...                  # state, time, continuation, event log

class Model:
    family: Family
    digest: str
    def start(self, start: NamedStart | ReadingsStart) -> LearnedRun: ...   # refuses the other family's start type

class ModelFault(Exception): ...

def load(path: Path, reduction: "Reduction") -> Model: ...
def resume(saved: "LearnedCheckpoint", model: Model) -> LearnedRun: ...
```

`Model.start` takes only its family's start type, so C2's interface carries no named total, and the named models' carries no teacher reading. Only the study's rule sees noisy readings; `advance` takes none.

## Acceptance

All on manufactured cases, with no teacher data:

- [ ] A two-reservoir C1 with set coefficients matches its closed form within the integrator's error. This is the brief's smoke test against mathematics. The engine's closed form is crate-private (`crates/ng-validate/src/fixture.rs:253-296`), so the test writes its own.
- [ ] Balance holds to rounding for random weights over 1,000 random steps.
- [ ] Step halving on a smooth synthetic named model shows an observed order of at least 3.5 against a run at 1/16 of the step. C2 has no rate and is exempt.
- [ ] The piecewise-input test: one posture event, and the same setting sent again 1 s later, give identical bits.
- [ ] Reading at another cadence leaves the state's bits unchanged.
- [ ] A step driven negative is refused and halved, and an unrecoverable one raises `ModelFault`.
- [ ] A checkpoint and a resume continue with identical bits, and `start` refuses the other family's start type.

## Non-goals

- Training (ticket 10), the call (ticket 11) and judging (ticket 12).
- C3, C4b and C4c, which the first experiment does not run.
