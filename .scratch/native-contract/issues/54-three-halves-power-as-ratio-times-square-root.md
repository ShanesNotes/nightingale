# 54: The collapse branch's three-halves power, written as the ratio times its square root

Spec: the evaluator's op list in `crates/ng-contract`, and the collapse branch of [native-contract 42](42-veins-collapse-branch.md)

Type: task

Status: open

Blocked by: 42

**Why.** The first milestone (SRS invariant I19) runs the healthy reference man for 48 hours, and how long that run takes rests on what a step costs. Native-contract 42 gives the veins a collapse branch. Its form raises a ratio of volumes to a power, written `exp(n ln(V_u / V))`, with `n` the body parameter `cv.collapse-power` at 1.5. The logarithm and the exponential are what that branch costs. With the power at 1.5, the same value is the ratio times its square root, which does neither.

**What was measured.** The plan for part 2 of native-contract 42 (`~/lanes/logs/ng-wave-4/nc42-pr2/PLAN.md`, its cost table) timed the branch on the slice with four veins marked, against main, on the beelink over 31 and 32 pairs of runs:

| Form | Step wall against main, median (95% CI) |
|---|---|
| `exp(n ln(V_u / V))`, shared subexpressions | +2.4% (+1.8 to +3.4) |
| `r * sqrt(r)`, with `r = V_u / V`, and a measurement-only square-root op | +0.2% (-0.1 to +0.9) |

The plan names the logarithm and the exponential as what limits the branch's cost. The square-root op it built was for measurement only and is not on main.

- [ ] The question first: the exponent leaves the packet and becomes part of the law's form. Today `cv.collapse-power` is a body parameter, so a packet can set it. Written as `r * sqrt(r)`, the form holds 1.5 and no packet can change it. The Answer says whether a fixed exponent is acceptable for the form's sources and for the arena, and the ticket stops there if it is not
- [ ] The evaluator gains a square-root op, with its value and its derivatives, and the op is on the exact list the bans probe in `10-rust` holds, or the Answer says why it cannot be
- [ ] A closed-form test: `r * sqrt(r)` equals the branch's value below the unstressed volume, at a vein with a share above zero
- [ ] The step's cost before and after, measured on one named build worker under the same other load
- [ ] The PR lists every pinned value that moves, with one sentence each

**Non-goals:** any other law's power. The collapse branch's other parts. Evaluating a term once for all the laws that add it, which is [native-contract 55](55-a-term-evaluated-once-and-rows-that-are-definitions.md).
