# 12: Heat balance

Type: task
Status: open
Blocked by: 05, 09, `.scratch/research/issues/12-law-forms-heat.md`

**What to build:** Two heat nodes at rung 0, the core and the skin, with blood carrying heat between them. Skin loses heat to the room by radiation, convection and evaporation, and evaporation removes water and its latent heat as one transfer. The thermostat reads core and skin temperature, the skin's through a skin thermoreceptor. It acts on the tone of skin vessels and on sweat through autonomic release at the skin junction, and on shivering through a somatic motor drive, arrow 87's shiver drive, which the muscle bed's heat reads. Sweat carries sodium and chloride with its water. Its set point follows the clock and steps down in sleep. Heat production comes from [ticket 09](09-tissues-respire.md) and changes with state. The body starts at its own heat balance, because a start at a textbook temperature drifts for hours.

**Releases:** the musculoskeletal card's hold on skin water loss, the slice's single heat exit, and coupling arrows 87 (its shiver drive; its set point and sweat drive stay held) and 91 (core temperature to the nervous system), and the respiratory card's hold on airway heat (`rs.airway-heat`). Arrows 92 to 94 carry temperature to blood, liver and blood chemistry, whose laws stay at a reference temperature at rung 0.

- [ ] Tests: each loss term matches its form at stated points, and evaporation moves water and heat in the fixed proportion
- [ ] Run: at admission, heat made equals heat lost within a stated tolerance, so the heat book does not drift from the start
- [ ] Run: a warmer room raises skin flow and then sweat. A cooler room narrows skin vessels and then starts shivering. The set point's daily swing shows in core temperature, with no law setting a temperature
- [ ] The certificate passes on the whole body with the heat book held by this loop, and the body's trace is updated
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget
- [ ] From [ticket 05](05-the-days-inputs.md): each law here that reads a lasting input is reviewed against the rule by reading: the input scales a gain or a set point and never stands for a produced quantity. Compile's `UnscaledInput` is only a stand-in: it passes a law that reads any parameter, such as `0 = k (T - room_air())`, and refuses one that reads only inputs
- [ ] From [ticket 10](10-lungs-breathe.md): the lung gas's water share is held at its vapour limit, shown on the body at five minutes. Today the airway's wash-out sets the share, and on the body it stands at 1.0574 of the saturated share at five minutes. [Native-contract 89](../../native-contract/issues/89-the-eliminations-order-inside-its-largest-block.md) records it and leaves the change to a later ticket, and no ticket owned it

**Non-goals:** more than two nodes, fever, exercise, and clothing as anything but one insulation value.

From [ticket 04](04-scopes-ports-and-census.md): MT's conduction feeds the conducting airway's heat store, and [ticket 10](10-lungs-breathe.md)'s airway heat exit leaves from that store. Until this ticket lands, MT's stand-in in [ticket 06](06-whole-body-on-holds.md)'s body serves conduction at the airway. This ticket's conduction law reaches the airway too, and replaces that stand-in there.

From [ticket 10](10-lungs-breathe.md): the lung's water leaves with expired gas, and its heat stays held at zero. `AirwayHeat` is a constant zero route on the respiratory card's held declaration, and the hold table names this ticket as its release. Kingma's 8% of the metabolic rate (research 12, Eq A2) lumps the evaporative and convective parts, and the latent part follows the water at 0.58 kcal/g (Guyton p. 869), so this ticket can check its airway heat against both. The dry heat of warming inspired air has no form on hand (research 11).

From the measurement before [native-contract 85](../../native-contract/issues/85-the-certificates-windows-end-on-breath-edges.md)'s second PR: the held skin water loss runs at 0.01 a second times a forcing, a seeded spline with knots every 300 s. That forcing is what swings the body's water leaving, between 2.86e-2 and 3.41e-2 a second as 50 s means in a run to 2,900 s, and through the sodium concentration that vasopressin's secretion reads it swings vasopressin too. In that run water's worst ten-second change reached 0.78 of its certificate bound. In a 300 s run with a sleep onset, one breath swung vasopressin by 2.57e-2, over half its bound of 5e-2. [Body ticket 01](01-rung-0-specification.md) moves the forcing with the skin's loss to the evaporation law when this ticket releases the hold, so the swing stays. The PR that releases the hold says what water and vasopressin do under the new law, and their bounds are sized again on its run.
