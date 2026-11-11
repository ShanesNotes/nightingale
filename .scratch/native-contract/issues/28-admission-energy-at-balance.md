# 28: Admission starts the energy store at its own heart-work balance

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4 (the admission solve), and `crates/ng-validate/src/slice.rs:440`

Type: task

Status: claimed

Claimed by: nc28-drift-limit

Blocked by: none

The 2 h tower probe on main (`d74ab35`, `ng_solver::Core` at 10 ms, from 06:55, no inputs) found the energy store drifting up after admission, from 658/s down to 8/s with a time constant of about 1,000 s. The probe and its logs are outside the repo, in `~/lanes/logs/ng-energy/`. Admission starts the thermal store at 1.184 × 10⁷, about 7.8 × 10⁵ (6%) below what its heart work balances, and the slice comment at `slice.rs:440` says admission solves energy's total. On the pick protocol that offset alone is a 24 h drift of about 7.8 × 10⁵ in the window from admission, so every core would fail the proposed limit of 4 × 10⁵ on admission and not on its own error.

**Cause, measured on the tower (2026-11-08).** Admission does solve energy's total, but on its own cycle: 100 backward-Euler intervals a beat. The total balances the heart work that cycle computes, and the core then integrates the same beat to its own accuracy and gets more. Heart work minus heat over the first 30 s, at each count of intervals, with admission's wall time (`points/run.out` and `points2/run.out`):

| Intervals a beat | Gap, first 30 s | Gap, first 120 s | Admission |
|---|---|---|---|
| 100 (main) | 1,098/s | 778/s | 6.5 s |
| 200 | 507/s | 522/s | |
| 400 | 233/s | 246/s | |
| 800 | 71/s | | 56 s |
| 1,600 | 64/s | | 141 s |

The gap halves as the intervals double up to 400, which is the first-order error of the cycle. Past 800 it stops shrinking. The slow states the cycle places move with it: after a start at 100 intervals the core's heart work holds at about 12,620/s for two hours, and after one at 800 it is about 12,070/s. So energy shows the error only because its store relaxes slowest; every slow state carries it.

**A second-order interval fails.** A trapezoidal stage (`trap/trap.patch`) fails to admit at 100 and 200 intervals with `Inconsistent { row: Body(RowId { index: 1 }) }`. A stiff valve row's trapezoidal step multiplies its deviation by nearly −1, and an even count of such steps returns it to itself, so the periodic problem has no rank there. A second-order stage would have to damp stiff rows (L-stable, such as two-stage Radau IIA or TR-BDF2), which gives each interval stage unknowns of its own.

**Options.**

- **A. More intervals a beat (800).** A one-line change. The gap falls to about 70/s, an offset of about 7 × 10⁴. Admission takes about 9 times longer, about 56 s in release. `SliceCase` admits once for each core resolution it meets, so a debug test binary pays that several times.
- **B. An L-stable second-order interval stage in `InitializationSystem`.** It keeps 100 intervals and should remove most of the gap, but it adds stage unknowns, the ring elimination grows, and `admit` judges the new rows. That is new state in admission, so it runs pstack `architect` first (team contract).
- **C. Guards located inside an interval.** The cycle places a valve's switch on an interval's end, while the core locates it exactly. This may be what stops A short of zero past 800. It is the largest change.

**Done when:**

- [ ] A test in `crates/ng-validate/tests/` admits the arena fixture and fails on main: over the first 30 s, the energy combination's inflow and outflow agree within a stated tolerance of the inflow.
- [ ] The tolerance is stated beside the test, with the drift its offset allows over a 24 h window.
- [ ] `slice.rs:440` says what admission actually solves.
- [ ] A short tower probe on the merged head reports the offset, and the energy drift limit goes back to Shane with it.

**Non-goals:** setting the energy drift limit; any 24 h or 48 h run; the slice's parameter values.

## Comments
