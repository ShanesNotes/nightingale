# 19: The alveolar unit's values, and the exchange that stands for the breath

Type: research
Status: open
Blocked by: 11

**Question:** Which cited values bind the constants of the respiratory card's alveolar unit, its airway and its membrane at rest, and what bounds the mixing exchange that stands for the breath until the breath is built? [Body ticket 10](../../body/issues/10-lungs-breathe.md)'s first PR states every one as a synthetic value, and [research 11](11-law-forms-gas-loop.md) gives their forms and no values.

Today's forms, in `crates/ng-respiratory/src/`:

- The airway, `airway.rs`: the gas moves along the airway edge and across the room's side of the dead space as one volume flow, `G (h+ f_up - h- f_down) + M (f_up - f_down)` for each gas's share `f` of the dry gas at each end, and each gas's amount at that over its molar volume. The alveolar unit's head is `-E_L (V - V_L0) - E_W (V - V_W0)` and the dead space's `E_D (V_D0 - V_D)`.
- The membrane, `membrane.rs`: `J_i = D_i ((P_B - P_H2O) f_i - P_c,i)` for oxygen and carbon dioxide.
- The shunt: a vascular edge beside the pulmonary bed whose resistance is `R_c (1 - f) / f` for a fixed fraction `f`.

- [ ] The lung's and the chest wall's compliances and relaxed volumes at rest (`E_L`, `E_W`, `V_L0`, `V_W0`), whose balance is the resting volume, with the posture each was measured in
- [ ] The anatomic dead space's volume (`V_D0`), and whether a dead space that mixes its gas, as this form does, needs a smaller volume than one that returns its gas in order (research 11's Decisions)
- [ ] The diffusing capacities for oxygen and carbon dioxide (`D_O2`, `D_CO2`) at rest, as conductances from alveolar gas to plasma, and the water vapour pressure of alveolar gas at body temperature (`P_H2O`)
- [ ] The resting shunt fraction (`f`), and the airway's conductance to bulk flow (`G`, volume for each unit of pressure)
- [ ] What the pulmonary bed's inflow answers. Today it reads no oxygen debt, so the shunt keeps its share; hypoxic vasoconstriction, a later rung, would read alveolar oxygen there
- [ ] What bounds the mixing exchange (`M`): the resting alveolar ventilation it must give, so the alveolar gases sit where a tidal breath at the resting rate and depth puts their means
- [ ] Each constant's row for [research 16](16-constants-and-what-binds-them.md)'s list: what measurable quantity binds it

**Non-goals:** the breath's rhythm, drive and pleural pressure (body ticket 10's second PR), more than one alveolar unit, and the dead space's own elastance `E_D` and the head's smoothing width, which are model defaults with no physiology: the first keeps the dead space near its volume and the second splits a head into its inward and outward parts.
