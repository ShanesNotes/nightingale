# 07: What limits the step

Type: task
Status: resolved
Claimed by: body-07-step-limit
Blocked by: none

**What to build:** A measured account, on the slice, of where a step's cost goes and what limits its length. The aim is 100 times real time with full waveforms (SRS section 6), which is 10 ms of wall time for each simulated second.

Where it stands, from [native-contract 25](../../native-contract/issues/25-history-on-disk.md)'s run on the heavy-run machine (one repeat): 212 accepted steps a simulated second at 0.69 ms each. That is 145 ms for each simulated second, 14.5 times over. Three facts shape the work:

- The record's own share of a step (its replay of the stages, the hash and the ledger) is about 37% in the one profile on record. At today's step count that share alone allows about 21 times real time, whatever the core costs.
- The mean step is 4.7 ms under a 10 ms cap, about 156 steps a beat. Four in five refusals are accuracy refusals on two rows that an earlier dump of the slice names as the left heart's valve flows, and each firing restarts the core's step control. What limits the step is not established, and no histogram of accepted steps exists.
- One coupled step covers the whole body, so every new row pays for the heart's step count. A body with three times the rows at today's cost would be about 43 times over.

- [x] A histogram of accepted steps over a beat, and the limiter named from a profile or from counters, not from reading the code: the cap, one row's accuracy, or the restart after a firing
- [x] The cost of a step by part (evaluate, linearize, factor, the record's replay, the hash, the ledger), with how each part grows with rows
- [x] The cost of the full-waveform observer, which no run has measured
- [x] Every number carries its machine, its run count and its spread (pstack `benchmark-checklist`)

**Non-goals:** any change to a core or to the record ([ticket 27](27-step-plan.md)), and the budget ([ticket 26](26-cost-budget.md)).

Evidence so far: `~/lanes/logs/ng-body-plan/body-cost.md`.

## Answer

The accuracy estimate limits the step. In the first half of each beat it sets 106 of 135 steps, on the heart's rows during ejection and relaxation, and no single row dominates. In the second half the 10 ms cap sets every step. That cap is the body's own wave rate class, so a longer cap on the core changes nothing. The restart after a firing never binds. Firings cost wall time instead. A landing takes 3.8 solves, so the 15 landings a beat take 21% of the wall time.

Every number below is from the heavy-run machine, an AMD Ryzen 5 2600 under WSL2, with Rust 1.98.1 and release builds. Each run held the machine's lock, and the probe was the only busy process. Each run admits the arena at 06:55 with `ng_solver::Core` at 10 ms and no boundary input, on main `85d1e6e`. The evidence and the probe are in `~/lanes/logs/ng-body-wave-1/07-step-limit/`, and its `REPORT.md` names the log behind each number.

### How the probe reads a step

The probe runs a copy of today's core with counters, a record of each decision and settings it can change. A script writes the copy from `crates/ng-solver/src/lib.rs` and stops if any line it edits has moved. With today's settings the copy reproduces the shipped core's seal bit for bit, over the first 120 s (22,482 steps) and over the day window (392,922 steps). A sampling profile (perf at 1999 Hz on a build with frame pointers) splits the record's work. On the core's parts, the copy's timers and the profile agree within 2 µs a step.

### Where the steps fall in the beat

The first 120 s are periodic. The heart beats 60 times a minute, and every beat takes the same 187 steps. The whole window from 06:55 to 07:30 is the same. It takes 392,922 steps in 2,100 s (187.1 a second), with 60 beats a minute, 187 steps a beat and the same reasons in every five-minute stretch. The record on file for that window, from before native-contract 28 merged, had 2,859 beats (about 82 a minute), 212 steps a second and 156 steps a beat. On today's main the heart beats slower, so a second takes 12% fewer steps and a beat takes 20% more.

Each step's length has one reason, read where the core or the record set it.

- Accuracy means the core's error estimate set the span the core asked for.
- Cap means the 10 ms longest step set it.
- Landing means the record cut the step at a guard's crossing, so a mode changed at its end.
- Retry means the record refused one or more attempts, and the step passed at a halved span.
- Growth means the core's limit of twice the last span set it, in the steps after a landing.

Steps a beat by phase, over the 120 beats of the first 120 s. Phase 0 starts systole, which ends at 0.3.

| Phase | Steps | Mean length, ms | Accuracy | Cap | Landing | Retry | Growth |
|---|---|---|---|---|---|---|---|
| 0.0 to 0.1 | 57.3 | 1.77 | 48.3 | 0 | 4.0 | 3.0 | 2.0 |
| 0.1 to 0.2 | 25.0 | 3.86 | 15.0 | 0 | 5.0 | 5.0 | 0 |
| 0.2 to 0.3 | 11.0 | 7.91 | 7.0 | 3.0 | 1.0 | 0 | 0 |
| 0.3 to 0.4 | 25.0 | 3.97 | 20.0 | 0 | 3.0 | 1.0 | 1.0 |
| 0.4 to 0.5 | 17.0 | 5.87 | 15.0 | 0 | 1.0 | 1.0 | 0 |
| 0.5 to 0.6 | 11.0 | 9.88 | 1.0 | 10.0 | 0 | 0 | 0 |
| 0.6 to 1.0 | 41.0 | 9.93 | 0 | 40.0 | 1.0 | 0 | 0 |
| Whole beat | 187.3 | 5.34 | 106.3 | 53.0 | 15.0 | 10.0 | 3.0 |

Over the whole beat, accuracy sets 57% of the steps and takes 46% of the wall time. The cap sets 28% and takes 22%, landings 8% and 21%, retries 5% and 9%, and growth 2% and 1%. A landing takes 3.8 solves and a retried step 2.5, against 1.0 for the rest. Half of the retried steps start right after a firing.

Accepted step lengths over the 120 s, by reason:

| Reason | Under 0.5 ms | 0.5 to 1 | 1 to 2 | 2 to 4 | 4 to 8 | 8 to 10 | 10 ms |
|---|---|---|---|---|---|---|---|
| Accuracy | 0 | 1,089 | 2,177 | 3,608 | 5,040 | 840 | 0 |
| Cap | 0 | 0 | 0 | 0 | 0 | 0 | 6,360 |
| Landing | 360 | 243 | 480 | 601 | 120 | 0 | 0 |
| Retry | 0 | 363 | 120 | 600 | 120 | 0 | 0 |
| Growth | 0 | 0 | 121 | 120 | 120 | 0 | 0 |

One landing of the 1,804 is the stir's chance firing. The rest land the 16 mode changes of each beat. The rhythm, the mitral valve and the tricuspid valve change twice a beat each. The aortic valve opens and shuts three times a beat and the pulmonary valve twice, all between phases 0.01 and 0.31. The leg veins' valve never changes. A valve's shut resets its own flow row to zero, and that reset names each mode in the trace.

### What limits the step

The accuracy estimate names the heart's rows. On the steps it set, the worst row is the aortic valve's flow 39% of the time, the left ventricle's pressure 20% and its chloride 17%. The pulmonary valve's flow is the worst row 11% of the time, the mitral valve's flow 6%, the right ventricle's chloride 6% and the tricuspid valve's flow 2%. A ventricle's chloride ties with its sodium on every one of those steps, because a lumen's species move together. No one row dominates. Were the worst row out of the estimate, the runner-up would allow a median step 1.19 times longer (tenth percentile 1.00, ninetieth 2.05).

The cap that binds is the cardiovascular crate's wave rate class, `WAVE_NS` at `crates/ng-cardiovascular/src/lib.rs:22`. `CompiledBody::longest_step_ns` applies it to every ticket, whatever the core asks.

The restart after a firing does not bind. In the step after a firing, the core grows its next span from that step's own length instead of the span it had committed to. Growing from the committed span instead gives the same seal over 120 s. On the slice the step after a firing always runs its whole committed span, so the two lengths are equal.

Each experiment changes one setting and runs 120 s from the same admission. Each prediction came from today's trace before the run, from the length the estimate allowed each step. The accuracy target is the estimate the core aims a span at, 0.7 against the refusal at 1.0. The wave class is a body setting, so its runs used a throwaway clone with `WAVE_NS` changed. That clone admits to the same state digest as main.

| Change | Predicted steps | Measured steps | What it shows |
|---|---|---|---|
| None | | 22,482 | The copy's seal equals the shipped core's |
| The core's cap to 5 ms | 30,162 (+34.2%) | 29,926 (+33.1%) | The cap binds where steps reach it |
| The core's cap to 20 or 40 ms | 19,836 or 19,337 | 22,482 both | The body still cuts each ticket at 10 ms |
| The wave class and the core's cap to 20 ms | 19,836 (-11.8%) | 19,841 (-11.7%) | The body's wave class is the cap that binds |
| The wave class and the core's cap to 40 ms | 19,337 (-14.0%) | 19,236 (-14.4%) | Past the cap, the estimate sets 81% of the steps |
| The accuracy target to 0.35 | 28,050 (+24.8%) | 28,736 (+27.8%) | The estimate sets the accuracy steps |
| The accuracy target to 0.9 | 20,993 (-6.6%) | 21,765 (-3.2%) | Retried steps rise from 1,203 to 3,249 and take half the gain |
| The growth base kept across a firing | 22,440 (-0.2%) | 22,482, the same seal | The restart never binds |

The first prediction for a longer core cap missed, and the miss found the body's cap. Lifting it to 40 ms removes 14% of the steps. After that, the estimate sets 81% of the steps that remain.

### A step's cost by part, and how it grows with rows

On the slice a step costs 0.640 ms, the median of 5 paired runs (range 0.639 to 0.642 ms), which is 8.33 times real time over 60 s. Two small fixtures in `ng-validate` give the cost of a body with almost no rows. Each body's ms a step is the median of 5 runs. Each part is the median share of 3 profiles times that, with the range of the 3 profiles.

| Part, µs a step | Slice, 241 rows | Chance fixture, 5 rows | Two reservoirs, 2 rows | Added per row |
|---|---|---|---|---|
| Core: evaluate | 146.6 (143.4 to 147.8) | 2.1 | 1.2 | 0.61 |
| Core: linearize | 49.9 (48.6 to 50.6) | 0.6 | 0.3 | 0.21 |
| Core: factor | 69.8 (68.5 to 70.4) | 0.2 | 0.1 | 0.29 |
| Core: back solves | 67.2 (65.3 to 68.5) | 0.4 | 0.2 | 0.28 |
| Core: its own arithmetic | 64.6 (60.8 to 67.2) | 3.5 | 2.9 | 0.26 |
| Record: replay of the stages | 126.7 (124.8 to 127.4) | 3.4 | 2.1 | 0.52 |
| Hash | 57.6 (56.3 to 58.9) | 18.5 | 18.0 | 0.17 |
| Ledger | 52.5 (50.6 to 53.8) | 5.1 | 4.6 | 0.20 |
| Link store and the rest | 7.0 | 1.5 | 1.1 | 0.03 |
| Whole step | 640 (639 to 642) | 35.1 (34.9 to 37.1) | 30.8 (30.5 to 31.2) | 2.55 |

The core takes 62% of a step on the slice and the record 38%. The core's own arithmetic is its Krylov vectors, norms, copies and quadrature. The slice has 380 instances, 257 columns and 1,578 Jacobian entries. Per call on the slice, an evaluate takes 19.1 µs (7.6 a step), a linearize 36.9 µs and a factor 52.2 µs (1.31 a step each), and a back solve 5.5 µs (12.4 a step, 6.8 of them inside the Krylov solve). Per call, an evaluate grows with instances (0.05 µs each), a linearize with Jacobian entries (0.023 µs each) and a back solve with rows (0.023 µs each). How a factor grows depends on its fill, which no public item reports.

A step has a fixed cost of 31 µs on a body with two rows. Of that, 16 µs is SHA-256 of the run identity (7.5 µs), the resolution (5.8 µs) and the ticket (2.8 µs), and both the identity and the resolution are fixed for the whole run. The slice spends 18 µs a step on the same three digests. Its other 40 µs of hash is BLAKE2b, 22 µs for the link's seal and 18 µs for the image. At the slice's 187 steps a second, the fixed cost alone takes 5.8 ms of each simulated second's wall time, 58% of the budget at 100 times real time.

Two small bodies and the slice cannot show whether a part grows faster than its rows. Ticket 06's body gives the next point.

### The cost of a full-waveform observer

An observer costs about 0.1 µs a sample, one channel read at one output-grid instant. The runs below had observers take turns after one admission, 5 rounds of 60 s. The extra wall time is the median of the 5 paired differences with no observer. Those 5 runs with no observer spread over 0.6 ms a simulated second.

| Lens | Samples a simulated second | Extra wall time a simulated second | Rounds slower than none |
|---|---|---|---|
| The arena's 3 means | 3 | Under the noise | 3 of 5 |
| 8 channels at 125 Hz | 1,000 | Under the noise | 2 of 5 |
| 8 channels at 500 Hz | 4,000 | 0.52 ms | 5 of 5 |
| All 241 rows at 125 Hz | 30,129 | 3.7 ms | 5 of 5 |
| All 241 rows at 500 Hz | 120,504 | 11.7 ms | 5 of 5 |

A monitor's waveforms, 8 channels at 500 Hz, take about 5% of the 10 ms budget. Every state row at 125 Hz takes 37% of it.

### Numbers no host-side probe reads today

- Why a step had its length. The shipped core's method history holds the span it chose and its estimate. It does not hold which limit chose the span or which row the estimate named. The probe read both from its copy. The hook is a decision the core publishes with each proposal, or the per-row estimate the record already recomputes in `StepProposal::verify`.
- The record's parts. Only a sampling profile splits the replay, the hash and the ledger, to about a point of share. A timer for each part inside the record would read them exactly.
- Calls a step. The counts of evaluate, linearize and factor calls come from the copy. A counter on `CoupledProblem` would read them from the host.

The copy works because the core is one file, unchanged since nightingale#54. Once ticket 27 changes the core, the copy's script stops until its edits match the new core.

### For tickets 26 and 27

These are measurements, not picks. At 187 steps a second and 0.640 ms a step, the slice takes 120 ms of wall time for each simulated second, 12 times the budget. Lifting the wave class to 40 ms would remove 14% of the steps. Landings take 21% of the wall time, and the fixed cost of a step takes 58% of the budget at today's step count.
