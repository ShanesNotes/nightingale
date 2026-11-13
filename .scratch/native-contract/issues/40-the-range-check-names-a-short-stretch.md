# 40: The healthy-range check counts and names a short stretch at the run's ends

Spec: [solver contract](../../../docs/architecture/solver-contract.md), the paragraph "The healthy-range check and the return check read a bare lens" and the tradeoff on a stretch at the run's start or end, and `crates/ng-validate/src/healthy/range.rs`

Type: task

Status: open

Blocked by: none

[Body ticket 23](../../body/issues/23-healthy-range-and-return-checks.md) built the healthy-range check. A change with a range of its own owns every window that crosses it, so at the run's start a window that crosses the change has no earlier window of its length before the change. The values before the change are then judged only where the change's own window reaches back to them, and the same holds at the run's end. The last check of nightingale#133 found it in a probe: admission at 0 s, a change at 3 s with a range over 2 s, an every-activity range over 4 s, and three values far outside that range before the change. The verdict was Pass, with no gap. In 400,000 random cases, 30 passes held a value no range judged, each in the first or the last span the run reads. On record 0004's day, admitted at 06:55 and sitting up at 07:00, the first five minutes of any reading whose every-activity window is longer than five minutes are judged by nothing.

The contract accepts the stretch as a tradeoff. What it lacks is the trace a cut period has: a cut period is counted (`cut()`), and this stretch is neither counted nor named. The glossary's "none leaves a reading unchecked silently" is not true at a run's two ends until it is.

**What to build:** the range report counts, for each range, the values at the run's start or end that no judged window held, and names each such stretch by its reading, its range and its span.

- [ ] The probe above gives a report that counts its three values and names the stretch, with a test. The same probe with the change at 4 s, where one window lies before the change, counts none
- [ ] The same at the run's end: a run that ends inside a window's length of an activity's start counts and names its tail, with a test
- [ ] A mutant row that drops the count is CAUGHT by those tests, on the PR's head (`tools/mutate.py --only`)
- [ ] The contract's tradeoff and the glossary say the stretch is counted and named, and whether a counted stretch blocks the verdict is stated as a model default

**Non-goals:** judging the stretch against any range, and any change to which windows a range judges.

This lands before [body ticket 24](../../body/issues/24-the-synthetic-day.md), which reads the range report on a 24 h run.
