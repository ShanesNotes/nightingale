# 07: The sufficiency screen before the freeze

Type: research
Status: open
Blocked by: 05, 06
Runs on: written on the workstation; runs on the heavy-run machine against the training views

**What it answers:** do the sixteen named totals miss a quantity that changes the next beat? This is a one-round screen on teacher data before the freeze, so an obvious gap can still join the named state. It is a diagnostic. It makes no trajectory, is never scored against the models, and is not a fourth model. It cannot prove the totals sufficient, because it looks one beat ahead; a gap that shows only over many beats is ticket 11's case.

## Usage

```text
python3 -I tools/generative-physiology/gp.py screen --data DATA > DATA/screen.txt
```

```python
report = screen(TrainingViews.read(data / "export"), reduction, CANDIDATES, seed=7302)
print(report.table())     # each candidate against each target, with its permuted control
```

## Method

- **Samples.** Every beat edge of the training runs after their first minute.
- **Features.** The standardized named totals and the drive at the edge, and the beat's mean drive.
- **Targets.** Each transfer's net rate over the beat, the beat's log heart rate and its arterial pressure, each standardized by its training SD.
- **Estimator.** k-nearest-neighbour regression with k = 16, with neighbours drawn only from other start histories: 20,000 query beats, drawn with the seed, against every second training beat. Distances in float64, ties broken by index, in a fixed order.
- **Candidates,** added one at a time to the features: each pool's share of its named total; cleft against bound within each released total; whole-body vasopressin; plasma albumin; each memory row (the afferent filter, the set point, activation and the five valve flows, read at the edge); the previous beat's length. Each candidate also runs with its values permuted across runs, as its control.
- **Flag.** For a candidate c and a target y, let d be the change in mean absolute error when c joins the features, and d0 the change when its permuted control joins. Resample the training start histories 2,000 times. The candidate is needed for y when the upper end of d's 95% interval lies below the lower end of d0's. The grounds are the screen's own variation across start histories, not a chosen ratio.
- **Known answers.** A planted candidate equal to the next beat's net rate of one transfer must be flagged for that transfer. Planted noise must not be flagged.

## Types and signatures

```python
@dataclass(frozen=True)
class Candidate:
    name: str
    kind: Literal["share", "split", "species", "memory", "previous-beat", "planted"]
    source: str                  # the rows or the quantity it reads, by name

@dataclass(frozen=True)
class ScreenRow:
    candidate: str
    target: str
    d: float
    d_interval: tuple[float, float]
    control: float
    control_interval: tuple[float, float]
    needed: bool

def screen(views: "TrainingViews", reduction: "Reduction", candidates: Sequence[Candidate], seed: int) -> "ScreenReport": ...
```

## What follows from it

Before the freeze, each candidate flagged as needed either joins the named schema or enters the schema's accepted-gaps list with its reason. A pool joins as its own total, a split divides a total, and a memory joins as a named memory with a learned rate and no transfers. One round of amendments is allowed. After it, ticket 05's balance checks run again on the amended schema. R's schema does not change.

## Acceptance

- [ ] The Answer reports the base error per target, each candidate's d and d0 with their intervals, the flags and the decision.
- [ ] The planted known answers come out as expected.
- [ ] At most one amendment round, after which balance still holds on every audit beat.

## Non-goals

- Any trained model, and any claim that the totals suffice.
