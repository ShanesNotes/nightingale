# 33: The slice's leg veins empty within 30 s of head-down, and every core faults

Spec: `crates/ng-cardiovascular/src/valves.rs` (`valves`) and `crates/ng-cardiovascular/src/shared.rs` (`drive`), the upwind instruction in `crates/ng-contract/src/evaluate.rs`, and the posture's heights in `crates/ng-contract/src/boundary.rs`

Type: research

Status: resolved

Claimed by: nc-33

Blocked by: none

**Question:** Why do the slice's leg veins drain to empty within 30 s of the head-down posture, and what should stop them?

[Generative ticket 04](../../generative-physiology/issues/04-teacher-audit.md)'s teacher audit ran the slice at bd38c0a5360addc7d5c2be2ffec4338a14e5c402 on the heavy-run machine. Every run that goes head-down (trunk -0.2, horizontal legs) faults with `NoAdmissibleStep` 27 to 30 s later, whatever came before, and every run that stays out of head-down completes. A run that goes head-down at 1 s reproduces the fault in seconds:

```text
gp-run 1
core ng-solver cap-ms 10
end-ms 61000
event at-ms 1000 trunk -200 legs horizontal
variant none
```

| Core | Fault at | Accepted steps | Leg-vein water at the last edge |
|---|---|---|---|
| `ng_solver::Core`, 10 ms cap | 29.494690 s | 5,150 | 0.0031, of 470.7 at admission |
| The reference integrator, 1 ms, floor 0.1 ms | 29.495798 s | 29,661 | 0.00028 |

Both faults are the same refusal at the 0.1 ms floor: `TrialDomain { source: SourceId { index: 258 } }`. `CompiledBody::refusal_sources` names source 258 `Divisor { law: CV/valves }`. The valves law divides explicitly only by `cv.inertance`, a positive parameter. So the refusing divide is the upwind transfer `flow * (amount / water)` in `crates/ng-contract/src/evaluate.rs`, which refuses when the upstream pool holds no water. Here that pool is the leg veins, upstream of the valve to the central veins. The reference integrator faults within 1.2 ms of Core, so the cause is in the model, and no pick of a core changes it.

- [x] The leg veins' water and pressure, the head across their valve with its gravity term, and the valve's flow, through the first 30 s of head-down on main
- [x] How much water a healthy adult's leg veins lose in the first minutes of a comparable head-down tilt, with its source
- [x] The cause, named as one of: the vein's law (nothing slows the drain as the vein empties), the valve's drive, the posture's heights, or the synthetic numbers. One change to that input moves the drain as the cause predicts
- [x] A recommendation and what it changes. A change of form meets [research 02](../../research/issues/02-law-forms.md)'s standard and names its source
- [x] The spec above completes on Core and on the reference integrator once the cause is fixed

**Non-goals:** changing a law or a number in this ticket. The valve timing of [native-contract 32](32-valves-shut-inside-ejection.md), which reads the same law.

## Answer

**The cause is the vein's law.** Below its unstressed volume a vein's stressed volume is held at zero by a soft floor (`Walls::stressed` in `crates/ng-cardiovascular/src/walls.rs`), so its pressure stays near zero however much water it loses. Head-down at trunk -0.2 puts the leg veins 0.1 m above the central veins, which adds 7 mmHg of gravity to the head across their valve (70 mmHg/m times 0.1 m). The leg veins can lower that head only by their own pressure, which was 1.9 mmHg. They reach the floor 0.5 s after the tilt. From then on the head stays at 2 to 3.4 mmHg, the valve passes 45 to 58 mL/s, and the leg bed sends in 15 to 32 mL/s. No term in the vein or the valve reads how much water is left, so the drain keeps its rate until the pool is empty and the upwind divide refuses.

**The fault on main.** The probe adds a trace writer and a list of refusal sources to `gp-driver`, and changes nothing the run computes. All runs are on the beelink at 68d28b0, the slice's model hash `8e461394`.

| Core | Fault at | Accepted steps | Refusal | Runs |
|---|---|---|---|---|
| `ng_solver::Core::new` (the halved core), 10 ms cap | 29.494149 s | 5,290 | `TrialDomain` at source 258, `Divisor { law: CV/valves }` | 2, the same to the ns |
| The reference integrator, 1 ms, floor 0.1 ms | 29.495798 s | 29,661 | the same | 1 |

**The shape on Core.** Pressures are in mmHg and flows in mL/s. The gravity term starts with the tilt at 1 s. The reference integrator's trace agrees within 0.3 mL and 0.1 mmHg.

| t (s) | Leg-vein water (mL) | Leg-vein pressure | Central-vein pressure | Gravity term | Head across the valve | Valve flow | Inflow from the leg bed |
|---|---|---|---|---|---|---|---|
| 0.9 | 471.1 | 1.64 | 1.24 | 0 | 0.40 | 19.8 | 17.3 |
| 1.5 | 450.8 | 0.006 | 4.05 | 7 | 2.96 | 54.1 | 19.6 |
| 3 | 396.7 | 0.001 | 3.90 | 7 | 3.10 | 55.8 | 15.2 |
| 10 | 238.4 | 0.000 | 4.46 | 7 | 2.54 | 50.6 | 21.3 |
| 20 | 104.7 | 0.000 | 4.00 | 7 | 3.00 | 54.7 | 28.5 |
| 28 | 12.2 | 0.000 | 4.48 | 7 | 2.52 | 50.3 | 25.8 |
| 29.494 | 0.0014 | 0.000 | 4.95 | 7 | 2.05 | 45.4 | 22.8 |

**One change to each input.** Each row is one change in a scratch clone of 68d28b0, run once on Core with the spec above unless it says otherwise.

| Input | Change | Result | Reading |
|---|---|---|---|
| The vein's law | Below the unstressed volume the pressure keeps falling: `E s_c`, with `s_c = share (V_u - V_u^2 / V)` and a share of 0.05 bound at the three veins | Completes on Core and on the reference integrator, which agree within 0.4 mL. The leg veins lose 86 mL in 4 s and settle at 377 to 390 mL, at -2.6 to -2.9 mmHg. It also completes at trunk -0.1 (462 mL) and -0.5 (255 mL) | The drain stops where the cause says: once the vein's own pressure falls to the central veins' less the gravity term |
| The valve's drive | Gravity taken out of the drive (`cv.head-per-length` near zero) | Completes, with the leg veins flat at 470 mL | Gravity drives the drain, but the column is real: this change also removes the pooling that standing needs. It does not explain why the drain never slows |
| The posture's heights | Trunk -0.1, then -0.5 and -1.0 | At -0.1 the run completes, because the central veins rise above 3.5 mmHg for part of each beat and shut the valve. At -0.5 the leg veins empty by 7.6 s, at -1.0 by 4.5 s | The heights follow the posture: the slice's 0.5 m from hip to heart times the trunk's sine. Any head larger than the central veins' pressure empties the vein, at a rate set by the head |
| The synthetic numbers | The leg valve's orifice from 1e-3 to 1e-2. A leg-vein elastance of 2.0 in place of 0.4 does not admit, because the start state no longer holds | At -0.2 the run completes. At -1.0 the leg veins still fall, from 477 to 231 mL by 14 s, when the splanchnic veins empty first | A number moves the tilt at which the drain starts and its rate. It does not stop the drain |

**The healthy figure: none on hand.** No source in the evidence store, the evidence library or the curation study gives the volume a healthy adult's legs lose in a head-down tilt, a passive leg raise or leg elevation, or its time course. About 25 full-text queries and a text search of the store found none. Rutlen 1981 and Gauer and Thron 1963 are the works to fetch. What is on hand is the reverse move. About 500 mL pools in the leg veins on standing, over about 45 s (Levick, An Introduction to Cardiovascular Physiology, 6th edition, printed p.143, Figure 8.25, after Gauer and Thron; full passage read; Textbook). Levick also says, at PDF p.168 of the same book, that deeper veins "do not collapse completely". So a healthy leg vein above the heart collapses and holds, and it never runs dry.

**Recommendation: give each vein a collapse branch.** Below the unstressed volume, the stressed volume falls on `s_c = share (V_u - V_u^2 / V)` in place of the floor at zero. Near `V_u` its slope is `share`, and it falls without bound as `V` goes to zero, so no head empties a vein.

| Law | Form | Constants | Holds for | Source and reading |
|---|---|---|---|---|
| Vein wall below its unstressed volume | `p = E s` with `s = smooth_max(V - V_u, 0) + share (V_u - V_u^2 / V)` taken below `V_u` only | `share` per site: 0.05 for veins, which is a compliance 20 times higher once collapsed. 0 for a chamber or a bed, which keeps today's floor | Model default: no posture range is stated. The shape is checked here at trunk -0.1 to -0.5 | Toro and others 2021, Int J Numer Method Biomed Eng, Eqs 3 to 5, full text, PrimaryFitted: `p - p_ext = K((A/A0)^m - (A/A0)^n) + P0`, with `n = -1.5` for veins. Model adaptation: volume in place of area at fixed length, and `n = -1`, because the kernel has a divide and no power. The slope at `V_u` is Snyder and Rideout's twentyfold collapsed compliance, as Shi, Lawford and Hose 2011 restate it (full text of the paragraph, Compiled). Upgrade: Müller and Toro 2014, Snyder and Rideout |

What it changes. The wall law takes a divide by each blood site's volume and one new site parameter, `cv.collapse-share`. So the slice's model hash moves (from `8e461394` to `cd7984e2` in the scratch run) and the arena's packet gains one parameter. Every pinned trace that reads the slice moves with them, though a run that never takes a vein below its unstressed volume barely moves: sitting up for 120 s kept the leg veins' water within 1 mL of main's. The throwaway patch is `patches/6-collapse-branch-three-veins.diff` and its runs are `runs/allveins-*`, in the evidence folder named below.

What stays open:

- At trunk -1.0 the branch at the three veins moves the fault upstream. The leg veins hold at 168 mL and -23 mmHg, and they pull the leg bed's plasma dry by 17.2 s, where the bed keeps the floor. Head straight down is no clinical posture. A bed's collapse, and a waterfall outflow at the collapsed outlet, are the next step if a steeper posture is ever needed.
- The share of 0.05 and the `n = -1` shape are model defaults. The healthy figure for head-down tilt is not on hand, so nothing yet checks how much the legs lose. With the branch, the legs lose 81 to 84 mL by 30 to 60 s at trunk -0.2.

Sitting up does not engage this cause: no vein sits above its drain, and the leg veins fill from 471 to 562 mL and never reach their unstressed volume.

Evidence: `~/lanes/logs/ng-body-wave-3/nc33-leg-veins/`, with `REPORT.md`, every run's trace and summary under `runs/`, the patches under `patches/`, and the evidence search in `evidence-search.md`.
