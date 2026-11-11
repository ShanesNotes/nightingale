# 33: The slice's leg veins empty within 30 s of head-down, and every core faults

Spec: `crates/ng-cardiovascular/src/lib.rs` (`valves` and `drive`), the upwind instruction in `crates/ng-contract/src/evaluate.rs`, and the posture's heights in `crates/ng-contract/src/boundary.rs`

Type: research

Status: open

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

- [ ] The leg veins' water and pressure, the head across their valve with its gravity term, and the valve's flow, through the first 30 s of head-down on main
- [ ] How much water a healthy adult's leg veins lose in the first minutes of a comparable head-down tilt, with its source
- [ ] The cause, named as one of: the vein's law (nothing slows the drain as the vein empties), the valve's drive, the posture's heights, or the synthetic numbers. One change to that input moves the drain as the cause predicts
- [ ] A recommendation and what it changes. A change of form meets [research 02](../../research/issues/02-law-forms.md)'s standard and names its source
- [ ] The spec above completes on Core and on the reference integrator once the cause is fixed

**Non-goals:** changing a law or a number in this ticket. The valve timing of [native-contract 32](32-valves-shut-inside-ejection.md), which reads the same law.
