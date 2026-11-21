# 99: Fick's check runs over whole breaths

Spec: [native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)'s paragraph "Fick" and its open item 5, and the [solver contract](../../../docs/architecture/solver-contract.md), section 7

Type: task

Status: open

Blocked by: 85

Blocks: none

**Why.** Fick's check over nine beats takes the blood's change of store as slack, oxygen 0.88% of the rate and carbon dioxide 7.6%, because nine beats are not whole breaths. Native-contract 85 gives the body windows between breath edges, and its design has Fick read those windows with the change of store as a signed term. The box moved here from native-contract 85 when the measurement before that ticket's second PR tried the summed form and it did not close.

**What the measurement found**, at main `e2d5c6ba` on a run of the whole body to 2,900 s, every value synthetic. Flow times content was summed at the 8 circulation edges out of the systemic arteries and the 2 out of the pulmonary artery, over two and over four whole breaths from 50 to 300 s. No set of stores tried closes it. The oxygen residual is -51 to -86 a second against a systemic burn of 95, and the carbon dioxide residual +114 to +199. The pulmonary artery's summed flow is 8.7 to 11.4% above the arteries'. _Inferred:_ the flows the circulation's fields give are not the flows the advection books. The check needs each reading place's flow from what the advection books, or from the valves' flows, with one set of edges on both sides.

- [ ] Fick's window runs over whole breaths, read from the policy's windows between breath edges, with integrals and not means, and with each gas's change of store as a signed term in place of slack
- [ ] The summed form closes on the whole body over two and over four breaths, on flows the PR names, before any tolerance is set, and the PR says why the circulation's fields did not close it
- [ ] Each residual is judged through `at_most`, and a mutant row holds each gas's sign

**Non-goals:** the windows themselves and the body's bounds ([native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)).
