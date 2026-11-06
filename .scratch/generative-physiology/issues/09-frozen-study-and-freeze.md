# 09: The frozen study and the freeze

Type: task
Status: open
Blocked by: 06, 07, 08
Runs on: written on the workstation; the freeze and the teacher runs happen on the heavy-run machine

**What it builds:** everything that judges a model, fixed before any model is tuned: the validation and held-out cases, the metrics, the call rule's code, the outside check, the freeze record and the checks that hold it. After the freeze merges, it makes the validation cases and the held-out set, and only then.

## Usage

```text
python3 -I tools/generative-physiology/gp.py freeze --data DATA > tools/generative-physiology/FROZEN   # committed in its own PR
python3 -I tools/generative-physiology/gp.py freeze --check          # the gate's check: the tree matches FROZEN
python3 -I tools/generative-physiology/gp.py run cases --data DATA --workers W      # after FROZEN merges: 48 runs
python3 -I tools/generative-physiology/gp.py run heldout --data DATA --workers W    # after FROZEN merges: 104 runs
```

```python
study = Study.frozen(tool_dir)               # refuses a tree unlike FROZEN and names the first file that differs
score = study.score(rollout, case)           # one case: NRE, features, support, faults
report = study.report(scores, models)        # per case, per family, per start history, per model
```

## The cases

**Validation cases**, on the 4 validation start histories, used only by ticket 11's call: the 4 out-of-policy families and the 7 paired branches below: 44 runs.

**The held-out set**, on 8 new start histories: 4 recombine training holds (interpolation in the start state) and 4 hold stand or sit for at least 240 s, longer than ticket 06 lets any training hold last (extrapolation), each labelled. Each history gets the 4 out-of-policy families, the 7 paired branches, a closed loop and a tilt: 104 runs.

Case times are offsets from H, the start history's end (300 s unless the audit lengthened it).

- **Out of policy.** o1: the `Inverted` rule as a closed loop on the teacher, scored open loop with its realized events. o2: ramps 0.2 s apart. o3: changes 3 to 10 s apart. o4: head-down (trunk -0.2, horizontal legs) and long-sitting (trunk 1, horizontal legs), two settings no training family reaches, because ticket 06's ramps keep horizontal legs below a trunk sine of 0.5 and no family uses a negative trunk.
- **Paired branches.** None; stand at H + 30 s; a sit ramp at H + 30 s; stand at H + 90 s; stand as a 10 s ramp from H + 30 s; head-down at H + 30 s; long-sitting at H + 30 s. Each lies flat at H + 360 s, and all seven share every event before H + 30 s, so their seals must agree up to that cut.
- **Closed loop.** The `Corrects` rule with scheduled stands, one case seed per start history.
- **Tilt.** Tilt at H + 30 s and flat at H + 330 s, the audit's F1 shifted into the encounter.

Every scored run is a free run from the encounter's first beat edge to its end, with no reset to a teacher state. A named model starts from the teacher's totals at that edge; C2 starts from the last 20 noiseless beats before it. That difference is the one the C2 against named comparison tests, and every result states it.

## The metrics

Every metric reads noiseless beat readings unless it says otherwise.

- **Normalized reading error (NRE) of a case.** Over the case's horizon, the time mean of the absolute difference between the model's and the teacher's heart rate, and the same for arterial pressure. Each reading holds its beat's value over its beat, so the integral over the merged beat partitions is exact even when the beats' timing differs. Each is divided by its channel's training SD from FROZEN, and the two are averaged.
- **M**, the primary metric: the mean NRE over a set's free-run cases, the out-of-policy families and the paired branches, per model and per training seed.
- **Totals error** (named models): the time mean of each named total's absolute error at the teacher's beat edges, over its training SD.
- **One-beat errors**, from teacher states: the transfers' net rates and the readings (level 1, fit only).
- **Balance residual and nonnegativity** of every named run. A `ModelFault` fails the case and stays in the denominator.
- **Rest drift**: the NRE of the "none" branch, beside the teacher's own drift.
- **Response features**: the four ticket 04 defines, on paired branches, each against the "none" branch over the same times. Reported as sign agreement and error.
- **Order across start histories**: Kendall's tau between model and teacher, per feature, over the 8 held-out histories, with a one-sided exact permutation test at 5%. The patient is the same packet throughout, so this is order across start histories, not across patients.
- **Closed loop**: M_cl, the model's mean NRE over the held-out closed-loop cases against the teacher's replay of that seed's own actions, for each model and each of its seeds; ticket 12 calls it by ticket 11's rule. Every seed's actions are replayed, one teacher run per seed per closed-loop case: 120 runs for C1, C2 and C4a, and 40 more if R is added. The agreement of a model's actions with the teacher's own closed loop (actions within 10 s with the same setting, as an F1 score) is reported beside it.
- **Support**: the seed spread, the distance from a named model's totals to its nearest training beats, errors inside and outside support (inside means within the training set's own 95th-percentile distance), and the rank correlation between seed spread and error.
- **Teacher response size** per case: the same integral against the start readings held flat. It is persistence's error, a scale and not a model.
- **P-rand against P-pol** (C4a only): errors on held-out beats inside both training sets' support, and outside it.

## The outside check

On the tilt cases, per beat: stroke (the teacher's aortic valve net extent, and a named model's stroke transfer), heart rate, pressure, cardiac output (stroke times rate) and conductance (cardiac output over pressure). The direction targets come from the abstract of the passive tilt study ticket 01 cites, and only from it. Stroke takes longer to reach 63% of its change after tilt-up than after tilt-back, and heart rate's up-to-back ratio is closer to one than stroke's. Ticket 01's four-row table is filled for the teacher (from ticket 04) and for each model; C2 has no stroke and reports heart rate only. Magnitudes and human timings are never targets (SRS rule 16).

## The freeze

**What FROZEN records.**
- The frozen tree: every file the freeze commit tracks under `tools/generative-physiology/`, as `git ls-files` lists it, except FROZEN and `train.py`, each with its SHA-256, and the digest of that list. That covers the driver, its `Cargo.lock`, the study, the model forms with R's, the specs, the schemas and ticket 06's requirements file. A check hashes exactly the listed files, so build output, caches and any other untracked file stay outside it (`.gitignore:1-3`), and the snapshot on the heavy-run machine, which has no git, checks the same set.
- The training and validation-tuning manifest digests, and the reduction digests, named and richer.
- The reading scales, each channel's training SD.
- The screen's decision, the four response features (ticket 04) and the call rule's parameters: 2,000 bootstrap draws over start histories with seed 7301, the seed spread entering through w (ticket 11).
- The rule's thresholds and the measurement-error convention.

**How it is checked.**
1. `fit`, `call` and `evaluate` refuse a tree whose digest differs from FROZEN, and name the first file that differs.
2. A check in the merge gate fails a PR that changes a frozen file, or adds a tracked file under the tool, without changing FROZEN; in a git checkout it compares `git ls-files` with the list. This ticket wires `gp.py freeze --check` into a verify step and names it.
3. Every result carries the FROZEN digest it ran under. Changing FROZEN is a visible refreeze, and the results under the old digest stand superseded.
4. The validation cases and the held-out set are made only after FROZEN merges, and this ticket's Answer records both manifest digests.

**How it is recorded.** FROZEN merges in its own PR. This ticket's Answer records its digest and main's merge commit, and `map.md` gains a pointer under its decisions.

## What every result reports

The model's digest, family, seed and training set; `train.py`'s SHA-256 and the budget used; FROZEN's digest, the source commit and the held-out manifest digest. Per case: its labels (interpolation or extrapolation in the start state; input, timing or policy extrapolation), NRE, totals error, the teacher's response size, the features, the share inside support, faults and refused steps. Per model: M and the closed-loop metrics; the step-halving order and the piecewise-input and cadence results; the balance residual and rest drift; tau per feature; the outside-check row; support calibration. Per family and per start history as well, so an aggregate cannot hide a failed trajectory. And two plain statements: levels 1 to 4 are judged by the teacher that trained the model, and every number is synthetic.

## Acceptance

- [ ] Two runs of `gp.py freeze` print identical text.
- [ ] A planted change to a frozen file fails the gate's check, and passes again once restored.
- [ ] Known answers: a model fed the teacher's own beat readings scores 0, and a model whose readings stay at the teacher's start readings scores exactly the teacher's response size on every case.
- [ ] The validation and held-out runs complete with no fault, and within each start history the paired branches' seals agree up to H + 30 s.
- [ ] The Answer records FROZEN's digest, main's merge commit and both manifest digests.

The cases are 148 runs of 900 to 1,200 s, 37 to 49 simulated hours: 8 to 20 process-hours and 2 to 5 hours of wall time at 4 workers.

## Non-goals

- Fitting any model (ticket 10), and any change after the freeze without a refreeze.
