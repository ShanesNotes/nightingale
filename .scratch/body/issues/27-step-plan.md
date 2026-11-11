# 27: A step plan that can reach the speed aim

Type: prototype
Status: resolved
Claimed by: 27-step-plan
Blocked by: 07

**What to build:** A way of stepping the body that can reach 100 times real time on all 13 systems. [Ticket 07](07-what-limits-the-step.md) says what limits the step today. On its figures no faster core closes the gap alone: the record's own work caps the slice near 21 times real time, and every row the body adds pays for the heart's step count.

- [x] Two or three plans prototyped and measured on the slice, each still passing the replay gates and the accuracy gate. Candidates:
  - A scheme of higher order. Today's scheme is first order: backward Euler, stepped twice at half length (`crates/ng-solver/src/lib.rs`). The seam admits any implicit Runge-Kutta scheme whose weights are nonnegative exact ratios ([solver contract](../../../docs/architecture/solver-contract.md), section 3). The scheme has to damp stiff rows: a trapezoidal stage fails on the valve rows, and two-stage Radau IIA, with weights 3/4 and 1/4, is one that [native-contract 28](../../native-contract/issues/28-admission-energy-at-balance.md) names for admission's cycle. A new scheme is one more entry in the contract's closed set, and its plan says what it asks of admission
  - Step control that survives a firing
  - A split where the heart's rows step apart from the slow rows (the split gate of native-contract 20 exists and no core uses it)
  - A cheaper record that keeps the same guarantees
- [ ] The same plans measured on [ticket 06](06-whole-body-on-holds.md)'s body once it exists: [ticket 30](30-step-plans-on-the-whole-body.md), blocked by 06
- [x] A pick with its measured gain and what it costs, written as a decision record if it changes the contract. Resolution is part of a run's identity (foundation map), so a plan that changes results is a different run and says so. A pick that falls short of the aim says by how much and what would close the rest
- [x] The pick is on main, so [ticket 24](24-the-synthetic-day.md)'s day runs on it. Build work the pick needs beyond this ticket's commits is filed as a ticket that ticket 24 waits on
- [x] Every number carries its machine, its run count and its spread (pstack `benchmark-checklist`)

**Non-goals:** the arena's pick between cores, and any gain bought by dropping a law.

This runs as pstack's Hillclimb playbook: one metric, one hypothesis at a time, a decision log, and one commit for each accepted gain.

## Answer

The pick is the halved backward Euler step, `backward_euler_halved` ([record 0009](../../../docs/adr/0009-a-step-reads-its-error-from-its-two-half-stages.md)). A step solves only the two half stages that carry its increment, and the record reads its error from the change in slope between them times half the ticket, measured from the start. That is their second difference when the halves are equal; on an odd ticket it weighs each half by its length, so a straight line reads no error. A ticket after a breakpoint shows the doubled set, and a ticket under 2 ns one whole stage. `ng_solver::Core::new` steps by it. `Core::doubled` keeps the old run, so a checkpoint it wrote still resumes and replays.

On the slice it takes 19.5% less wall time for each simulated second than the doubled step: 126.4 ms against 157.7. Those are medians of 25 repeats a core, on asrog after its interrupt fix, from 5 alternating rounds of 5 repeats in one binary. The paired difference is -19.49%, range -20.68 to -18.28, lower in 5 of 5 rounds. A step costs 22.0% less (instructions a step -23.4%, 2 counter runs a core), and a second takes 3.2% more steps. Health is the doubled step's, except fewer accuracy refusals on row 158 (361 for 482 in 60 s); `most_refusals` stays 4 and `shortest_retry_ns` 625,000. It passes the certificate, the accuracy gate (worst 0.693) and the replay gates. Over the first 30 s after admission the energy gap is 0.963% of inflow, against 0.814% on the doubled step, under the test's 2%.

It is a different run. The resolution gains method 3, so the run identity changes, and the first-second seal and trace in `trace.rs` and `smoke.rs` move. The slice's model hash and its packet do not.

**The pick falls short of the aim by a factor of 12.6:** 126.4 ms for each simulated second against 10.

### The plans measured

Every row is the slice on asrog (i7-6700, release builds), from paired rounds that alternate the arms, in one binary where the arms share code. Rows before the halved step are against the doubled step; the tuning rows are against the halved step. A row's figure is the median paired difference over its rounds, with the range of the rounds. All rows but the pick's last line ran before asrog's interrupt fix. A plan that lost stays on its branch.

| Plan | Branch | Wall time a simulated second | Kept |
|---|---|---|---|
| The doubled step, the baseline | `6f129e3` | 149.64 ms (148.53 to 157.60, 25 repeats) | baseline |
| The books visit only exchanging columns | `claude/body-27-ledger` | +5.1% (+3.9 to +5.4), 5 rounds; instructions -1.09% | no: code placement |
| After a firing, commit the length the last restart passed at | `claude/body-27-restart` | +3.95% (+3.53 to +4.63), 5 rounds; refusals -59.7% | no |
| Two-stage Radau IIA, order 3 | `claude/body-27-radau` | +15% tuned, 5 rounds; steps -21%, a step +46% | no |
| The halved step | `claude/body-27-lean` | -20.05% (-21.01 to -14.93), 5 rounds | yes |
| The halved step, built, after the interrupt fix | this ticket's PR | -19.49% (-20.68 to -18.28), 5 rounds | the pick |
| One stage read by Milne's device | `claude/body-27-lean` | -30.37% (-33.93 to -26.92), 5 rounds | no: one refusal of floor margin, accuracy worst 0.989, 4 fewer valve mode changes a beat |
| Accuracy target 0.85 | `claude/body-27-tune` | -0.17% (-10.77 to +1.47), 10 rounds | no: retries +113% |
| Growth 4 | `claude/body-27-tune` | -1.75% (-9.26 to +5.83), 10 rounds | no: under the noise |
| Newton stop at 0.3 of the estimate | `claude/body-27-tune` | +0.11% (-12.51 to +3.65), 10 rounds | no: the same work |

The run-to-run spread on asrog, a desktop in use, is about 5% between builds, so each comparison sits inside one binary and reports instructions beside wall time.

### What limits the step now

On the halved step a beat takes 193.5 steps: the estimate sets 103.3, the 10 ms wave class 50.0, landings 16.1, retries 7.1, the doubled set after a breakpoint 11.1, and growth 6.0 (one 20 s trace). It takes 246.7 solves, and the 16 landings take 57 of them. The estimate's and the cap's steps stop at one Newton update a stage, so the step control has no slack left: the three tuning rows above moved nothing past the noise.

### What would close the rest

The step count and a step's cost have to fall together, about 12.6 times between them. In order of size:

1. Landings: 16 a beat take 23% of the solves, at 3.6 solves each. The record's landing rule (`crates/ng-contract/src/landing.rs`) aims a secant at published ends. The half stage's guard reading could aim it nearer. That changes the contract outside this ticket. Six of the landings are valve reopenings inside one ejection ([native-contract 32](../../native-contract/issues/32-valves-shut-inside-ejection.md)).
2. The wave rate class: 50 of the 193 steps a beat are the cardiovascular crate's 10 ms class. [Ticket 07](07-what-limits-the-step.md) measured 14% fewer steps at 40 ms.
3. A split: the heart's rows set every row's step. On the whole body that matters more; [ticket 30](30-step-plans-on-the-whole-body.md) measures the plans there.
4. A step's cost. By ticket 07's parts, measured before nightingale#108 hashed the run identity once, evaluations are about 35% of a step, the Jacobian and its factorization about 19%, and the record's hash and ledger about 17%. A faster evaluator is the body crates' work. A factorization kept across steps would have to travel in the method history, or a resumed run would not replay.

None of these reaches the aim alone. No build work beyond this ticket's PR is needed for ticket 24's day to run on the pick.

## Comments

**What the rhythm's firing costs, read from ticket 07's trace.** The trace is `data/v3/out/trace-today.csv` in ticket 07's evidence folder (sha256 `91a5901070752994…`): 120 s on the heavy-run machine, one deterministic run. Twice a beat the rhythm fires, and the step after it asks for the span the core carried from before the firing, 10 ms both times. At the start of systole that step is refused four times, once at Newton's iteration limit and three times on the accuracy of the mitral valve's flow, and it passes at 0.625 ms. At the end of systole it is refused three times and passes at 1.25 ms. The two steps take 3.5% of the wall time.

For a plan the margin matters more than the cost. A step that passes at 0.625 ms has two refusals left: a third would retry at 0.078 ms, below the floor of 0.1 ms, which is the fault `NoAdmissibleStep`. A 48 h run stands at that point 172,800 times.

Two facts of the seam shape any step control that survives a firing. `CoupledSolver::next_step` takes the accepted view, which has no reading that says the head is a breakpoint, so a core commits its span before it can know that a guard fired. And `StepTicket::refuse` halves the span whatever the estimate was, so a step sixteen times too long takes four refused solves to find its length.

`NumericalHealth::most_refusals` and `NumericalHealth::shortest_retry_ns` report this margin for any run through the arena, so each prototype's run shows its own.
