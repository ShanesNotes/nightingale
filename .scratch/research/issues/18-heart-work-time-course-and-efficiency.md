# 18: How fast the heart's oxygen use follows its work, and the efficiency of muscle work

Type: research
Status: open
Blocked by: 11

**Question:** How fast does the heart's oxygen use follow a change in its work, and what share of the energy its fuel frees does muscle work take? [Body ticket 09](../../body/issues/09-tissues-respire.md) states the first as a model default with no source and holds the second at one.

Today's forms:

- Arrow 78's work, in `crates/ng-cardiovascular/src/walls/stroke_work.rs`: a memory `m` that follows the stroke work in systole, `dm/dt = k (S r - m)`, and falls in diastole, `dm/dt = -k m`, with `k` = 0.25 a second. `S r` is the summed contractile store times the activation's rate, the work every stroke-work route moves.
- The paying bed's demand, in `crates/ng-metabolism/src/burn.rs`: `D = D_basal + s W e`, with `e` the oxygen a unit of energy takes from the burns' contents. All the energy the burns free can pay work, so the efficiency of muscle work is one.

- [ ] A cited time course of myocardial oxygen consumption after a step in the heart's work, as a time constant or a form, with its source and tier, or the evidence that none exists
- [ ] The efficiency of the heart's work and of vascular smooth muscle's, as the work done over the energy the fuel frees, with its source and tier
- [ ] Each constant's row for [research 16](16-constants-and-what-binds-them.md)'s list: what measurable quantity binds it

**Non-goals:** coronary flow's answer to the heart's oxygen use, which arrow 90's debt carries at rung 0, and the work of breathing on arrow 84 (body ticket 10).
