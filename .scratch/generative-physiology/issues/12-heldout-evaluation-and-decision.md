# 12: The held-out evaluation and the decision

Type: research
Status: open
Blocked by: 11
Runs on: the heavy-run machine for scoring and replays; the workstation only reads the report

**What it answers:** once, on the held-out set, does the named model carry the posture response better than C2, out of policy and in a closed loop, and does the first experiment earn the next step? It scores every fitted model and every seed, replays every seed's closed-loop actions through the teacher, applies the falsifier and the list of what earns the next step, and fills the outside-check table.

## Usage

```text
python3 -I tools/generative-physiology/gp.py evaluate --data DATA --models MODELS > DATA/report.txt
```

```python
report = study.evaluate(models, partition=Partition.HELDOUT)    # refuses to run twice under one FROZEN digest
print(report.verdict())
```

## The verdict

Out of policy, a call compares M over the held-out free runs; in the closed loop, it compares M_cl (ticket 09). Both apply ticket 11's rule, with its bootstrap over the 8 held-out start histories.

**The falsifier.** The claimed advantage of named state is falsified for this task if C2 wins or is too close to call against the named model, on the held-out start histories and the out-of-policy paired branches over the full horizon. If R was added and beats C2, the falsification applies to the sixteen totals, not to named state.

**What earns the next step.** All of these must hold, each with its verdict in the Answer:
- The named model beats C1 and C2, out of policy and in the closed loop.
- Balance holds and every total stays nonnegative in every named run.
- The piecewise-input, step-halving and cadence results pass.
- The order of responses across start histories survives: a significant tau on at least two of ticket 04's four response features, unless the audit removed this test.
- The screen left no gap unresolved, or each accepted gap has its reason (ticket 07).
- The P-rand C4a beats the P-pol C4a out of policy, on states and actions both training sets cover. A P-pol model that fails only outside that shared support lacks support; one that fails inside it is biased.

If C1 matches C4a, the linear model is kept. That is a result, and the next experiment needs a harder question. If all of these hold, fluid deliveries (ticket 13) and the planted-discrepancy experiment come next.

**The outside check.** The four-row table from ticket 01, for the teacher and for each model. A teacher that misses the asymmetry is an engine gap or a protocol the slice cannot express, and a question for the engine's process once the protocol is ruled out. A model that shows it where its teacher does not is investigated for leakage.

## Acceptance

- [ ] The Answer holds the decision, every field ticket 09 lists for a result, the closed-loop replays' identities, and the outside-check table.
- [ ] Failed runs stay in every denominator.
- [ ] `map.md` gains a pointer to the decision under its decisions.

## Non-goals

- Retuning after the held-out read. A retune is a new ticket and a refreeze, and the old results stand, superseded.
- Any claim about human physiology. Every number is synthetic, and levels 1 to 4 are judged by the teacher that trained the model.
