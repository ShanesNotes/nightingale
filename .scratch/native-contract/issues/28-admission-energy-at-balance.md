# 28: Admission starts the energy store at its own heart-work balance

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4 (the admission solve), and `crates/ng-validate/src/slice.rs:440`

Type: task

Status: resolved

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

- [x] A test in `crates/ng-validate/tests/` admits the arena fixture and fails on main: over the first 30 s, the energy combination's inflow and outflow agree within a stated tolerance of the inflow.
- [x] The tolerance is stated beside the test, with the drift its offset allows over a 24 h window.
- [x] `slice.rs:440` says what admission actually solves.
- [x] A short tower probe on the merged head reports the offset, and the energy drift limit goes back to the developer reviewer with it.

**Non-goals:** setting the energy drift limit; any 24 h or 48 h run; the slice's parameter values.

## Answer

nightingale#91 closed the first three boxes: `admission_energy.rs` holds the gap to 2% of inflow, and `slice.rs` says admission solves energy's total at its own cycle's balance. The probe for the last box ran on a build worker at `eb53f77`, on `ng_solver::Core::new` at 10 ms. Main has changed only docs and tickets since.

- **The offset.** Admission now starts energy 1.0% (1.24e5) below the balance of its first 30 s of heart work. Heart work minus heat is +118/s over the first 30 s and +5.8/s over the first 120 s. `Core::doubled` reproduces nightingale#91's +99.8/s and −2.2/s to the digit. Without inputs, energy then wanders within about 1e5 of admission, and its hourly rate changes sign, so no steady rate projects a day.
- **The measured day.** The arena's own day, with the day plan from 06:55 for 86,400 s, moved energy by +83,433.6 across the pick protocol's window. Inflow minus outflow gives the same figure. Wherever each end falls in the beat, the drift lies between 6.3e4 and 8.4e4. Within the day energy's minute samples reach 5.23e6 below admission and 1.43e6 above it, but both ends lie in the same settled lying state before the 07:00 sit-up.
- **The projections missed.** nightingale#91's method, the 120 s mean gap times a day, gives +5.0e5 on `Core::new` and −1.9e5 on `Core::doubled`, the figure the approval rested on. Neither sees the day plan.
- **The limit.** The developer reviewer approved the oversight seat's recommendation of 2e5, about twice the measured 8.3e4, after the measured day fell below the band their earlier approval of 4e5 assumed. That band was 1.4e5 to 2.4e5. The limit holds for energy alone, and every other quantity keeps its limit. `crates/ng-validate/tests/arena_certificate.rs` holds it on a synthetic body: energy passes just under it and is refused just over, and water is refused just over 1e4. Two mutant rows catch the limit falling back to 1e4 and the limit reaching the other quantities. The second row changes the species and element arms together, and the water test sees only the species arm: the slice has no reaction, so no element is a quantity of its own there. The element arm gets its own row with the first body that has a reaction (body 06 at the earliest).
- **What it has not seen.** Only the window from 06:55 is measured. The milestone protocol reads every sliding 24 h window over 48 h, and their drift is unmeasured until that run. The limit is a synthetic value set from the one measured window. A window above it fails the milestone protocol and reopens the pick, as the contract says. The first question there is whether the synthetic limit or the core is at fault. A prediction to test, not a claim: energy moves up to about 3,100 a second at the 23:00 lie-down, so a sliding window whose ends fall there turns a shift of about 65 s between the two days into 2e5. The 48 h run will show whether any window does that.
- **Still open.** [Native-contract 31](31-cross-interval-drift-check.md), the cross-interval drift check. The Non-goals line above predates the approval, which brought the limit into this ticket.

The probe, its logs and the day's hourly table are outside the repo.

## Comments
