# 99: Fick's check runs over whole breaths

Spec: [native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)'s paragraph "Fick" and its open item 5, and the [solver contract](../../../docs/architecture/solver-contract.md), section 7

Type: task

Status: open

Blocked by: 85

Blocks: none

**Why.** Fick's check runs over nine beats, which are not whole breaths. So it takes the change of each gas over every non-gas amount, the tissues' among them, as slack added to its tolerance: oxygen 0.88% of the rate and carbon dioxide 7.6%. Native-contract 85 gives the body windows between breath edges, and its design has Fick read those windows with the change of store as a signed term. The box and open item 5 moved here from native-contract 85 when the measurement before that ticket's second PR tried the summed form and it did not close.

**What the measurement found**, at main `e2d5c6ba` on a run of the whole body to 2,900 s, every value synthetic. Flow times oxygen content, and flow times carbon dioxide content, were summed at the 8 circulation edges out of the systemic arteries and the 2 out of the pulmonary artery, over two and over four whole breaths from 50 to 300 s. No set of stores tried closes it. Across those windows and sets the oxygen residual runs from -29 to -86 a second against a systemic burn of 95, and the carbon dioxide residual from +60 to +199. The pulmonary artery's summed flow is 6 to 12% above the arteries'. One of the sets named a pool wrongly, `pulmonary veins` for `pulmonary vein`, and so held a pool it meant to leave out. _Inferred:_ the flows the circulation's fields give are not the flows the advection books. The check needs each reading place's flow from what the advection books, or from the valves' flows, with one set of edges on both sides.

- [ ] Fick's check runs over the windows the body's policy opens between breath edges, two or three breaths at the body's measured periods, with integrals and not means, and with each gas's change of store as a signed term in place of slack. The certificate's result keeps one worst window a rated quantity, so the PR says where the check gets every judged window from
- [ ] The PR first measures each gas's residual on those windows, on flows it names, and says why the circulation's fields did not close it. The tolerance is set from that measurement, as native-contract 85's paragraph "Fick" has it
- [ ] Each residual is judged through `at_most`, and a mutant row holds each gas's sign

**Non-goals:** the windows themselves and the body's bounds ([native-contract 85](85-the-certificates-windows-end-on-breath-edges.md)).
