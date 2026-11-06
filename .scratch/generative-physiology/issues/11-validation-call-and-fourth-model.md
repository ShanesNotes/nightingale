# 11: The validation call and the fourth model

Type: research
Status: open
Blocked by: 10
Runs on: the heavy-run machine

**What it answers:** on the validation cases, which model wins each comparison, and whether the results are too close to call. If they are, it adds R, the one fourth model, and calls again. The held-out set stays closed until ticket 12, so the decision to add a model never reads the cases that judge it. This rule was written before any tuning, and `gp.py call` applies it mechanically.

## Usage

```text
python3 -I tools/generative-physiology/gp.py call --data DATA --models MODELS     # prints every number and the decision
python3 -I tools/generative-physiology/gp.py fit R --training P-rand --seed 1 --data DATA --out MODELS/r-prand-1   # only after "add R"
```

```python
decision = study.call(validation_scores)
# Decision(named="C4a", named_against_readings="too close", add="R", calls=[...])
```

## The rule

**The metric.** M is the mean NRE over the validation cases' free runs, the out-of-policy families and the paired branches, for each model and each of its five training seeds (ticket 09).

**A call between two models X and Y.**
- D is the median of Y's five values of M minus the median of X's.
- w is the larger of the two models' seed ranges, the largest minus the smallest of its five values.
- [lo, hi] is the 95% interval of D over 2,000 bootstrap draws that resample the 4 validation start histories with replacement (seed 7301).
- X wins when D > w and lo > 0. Y wins when D < -w and hi < 0. Otherwise the two are too close to call.

The grounds are measured variation, the spread across retraining and across start histories, never a chosen margin. With only 4 validation histories the interval is coarse, which makes "too close" more likely, not less.

**Order of calls.**
1. C1 against C4a. C4a becomes the named model only if it wins. A tie, or a win for C1, keeps C1, because a linear win is a result. The simpler generator is kept, and the next experiment needs a harder question.
2. The named model against C2.
3. If step 2 is too close to call, add R, and only R. It is trained under ticket 10's budgets, on the same data, with seeds 1 to 5. Then R is called against C2 and against the named model by the same rule. No other tie adds a model.

**Why R.** A tie between the named model and C2 means either that the named totals discard something that matters, at the start or along the run, or that this posture task does not need named state. R carries every amount row and every memory, with each quantity supervised by the teacher and named, so its result separates the two. If R beats C2, the totals discarded what matters, and R against the named model, with the screen's table, names it. If R ties C2 as well, the result is inconclusive for named state on this task, and the report names the limiting evidence. A model with a learned latent part (C4b) would start from the named totals alone, so it could not recover start information they discard, and its latent is defined only up to a transformation. C3 and C4c test balance by construction and ledger supervision, which a tie between C4a and C2 does not implicate. Persistence needs no model, because every case reports the teacher's response size.

**After R.** No fifth model. If every model fails, or the uncertainty remains after R, the result is inconclusive, and the Answer names the evidence that limits it.

## Types and signatures

```python
@dataclass(frozen=True)
class Call:
    x: Family
    y: Family
    d: float
    w: float
    interval: tuple[float, float]
    outcome: Literal["x wins", "y wins", "too close"]

@dataclass(frozen=True)
class Decision:
    named: Family                       # C4a, or C1 when C4a does not win
    named_against_readings: Call
    add: Optional[Family]               # R or None
    calls: tuple[Call, ...]

def call(scores: "ValidationScores", draws: int = 2000, seed: int = 7301) -> Decision: ...
```

## Acceptance

- [ ] The Answer gives every model's five values of M, every call's D, w and interval, the calls in order and the decision.
- [ ] R is fitted only when step 2 is too close to call, and is then called as the rule says.
- [ ] No held-out path is opened. The loader refuses one.

## Non-goals

- Reading held-out cases, changing the rule after a result, or any model beyond R.
