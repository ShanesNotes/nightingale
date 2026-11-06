# 10: Training the three models

Type: task
Status: open
Blocked by: 09
Runs on: written on the workstation; every fit runs on the heavy-run machine

**What it builds:** `train.py`, the one editable training file, which FROZEN's tree leaves out, and the fitted models, in the environment ticket 06 pinned and ticket 09 froze; changing that environment is a refreeze. The models are C1, C2 and C4a with five seeds each on P-rand, and C4a with five seeds on P-pol. The P-pol fits are the same model on other data, for the brief's hypothesis that randomized schedules generalize where policy-coupled ones fail. They are not a fourth model.

## Usage

```text
python3 -I tools/generative-physiology/gp.py fit C4a --training P-rand --seed 3 --data DATA --out MODELS/c4a-prand-3
python3 -I tools/generative-physiology/gp.py fit C2 --training P-rand --seed 1 --data DATA --out MODELS/c2-prand-1
python3 -I tools/generative-physiology/gp.py evaluate --load-only --models MODELS      # every model loads under FROZEN
```

```python
views = TrainingViews.read(data / "export", partition=Partition.TRAIN)   # refuses any other partition
model = fit(Family.C4A, views, validation=tuning_views, budget=BUDGET, seed=3)
save(model, out)     # weights, the family, the reduction digest, train.py's SHA-256 and the budget used
```

## Budgets

Equal for every family, and fixed in FROZEN:
- Tuning: at most 6 CPU-hours per family, scored only on the validation tuning runs, which are in policy. The held-out set and the validation cases stay closed.
- A final fit: at most 1 CPU-hour or 20,000 optimizer updates, whichever comes first, for each of the seeds 1 to 5.
- At most 100,000 parameters per model.
- A budget that runs out is reported, never extended.

The losses are one-beat losses first (each transfer's net extent per beat, and the beat readings), then rollouts over 30 beats from teacher states, in which a model consumes its own predictions. Every reading in a loss is noiseless. The Answer keeps the log the brief asks for: every change kept or discarded, with its validation score, budget and model digest.

## Types and signatures

```python
@dataclass(frozen=True)
class Budget:
    tuning_cpu_hours: float = 6.0
    fit_cpu_hours: float = 1.0
    fit_updates: int = 20_000
    max_parameters: int = 100_000

def fit(family: Family, views: "TrainingViews", validation: "TrainingViews", budget: Budget, seed: int) -> "Model": ...
def save(model: "Model", out: Path) -> str: ...      # returns the model's digest
```

## Acceptance

- [ ] Every fit records the environment's requirements digest, which equals FROZEN's. A GPU is used only after `torch.cuda.is_available()` returns true and the Answer records it; the first build does not need one.
- [ ] Every model loads under `gp.py evaluate --load-only` against FROZEN.
- [ ] No fit opens a held-out or validation-case path. The loader refuses one, and a test proves it.
- [ ] The Answer holds the kept and discarded log, and the CPU-hours each family used.

The plan is 4 families of fits (6 tuning hours and 5 fits of at most 1 hour each), about 44 CPU-hours, or about 11 hours of wall time with 4 fits at once.

## Non-goals

- The call (ticket 11) and the held-out evaluation (ticket 12).
- Changing a model's form. The forms are frozen (ticket 08). A new form is a refreeze.

## Comments

Ticket 06 settles whether the heavy-run machine can reach the package index for these wheels; if it cannot, the wheels travel with the snapshot, checked against the same hashes.
