# 12: Heat balance

Type: task
Status: open
Blocked by: 05, 09, `.scratch/research/issues/12-law-forms-heat.md`

**What to build:** Two heat nodes at rung 0, the core and the skin, with blood carrying heat between them. Skin loses heat to the room by radiation, convection and evaporation, and evaporation removes water and its latent heat as one transfer. The thermostat reads core and skin temperature, the skin's through a skin thermoreceptor. It acts on the tone of skin vessels and on sweat through autonomic release at the skin junction, and on shivering through a somatic motor drive, arrow 87's shiver drive, which the muscle bed's heat reads. Sweat carries sodium and chloride with its water. Its set point follows the clock and steps down in sleep. Heat production comes from [ticket 09](09-tissues-respire.md) and changes with state. The body starts at its own heat balance, because a start at a textbook temperature drifts for hours.

**Releases:** the musculoskeletal card's hold on skin water loss, the slice's single heat exit, and coupling arrows 87 (its shiver drive; its set point and sweat drive stay held) and 91 (core temperature to the nervous system). Arrows 92 to 94 carry temperature to blood, liver and blood chemistry, whose laws stay at a reference temperature at rung 0.

- [ ] Tests: each loss term matches its form at stated points, and evaporation moves water and heat in the fixed proportion
- [ ] Run: at admission, heat made equals heat lost within a stated tolerance, so the heat book does not drift from the start
- [ ] Run: a warmer room raises skin flow and then sweat. A cooler room narrows skin vessels and then starts shivering. The set point's daily swing shows in core temperature, with no law setting a temperature
- [ ] The certificate passes on the whole body with the heat book held by this loop, and the body's trace is updated
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget
- [ ] From [ticket 05](05-the-days-inputs.md): each law here that reads a lasting input is reviewed against the rule by reading: the input scales a gain or a set point and never stands for a produced quantity. Compile's `UnscaledInput` is only a stand-in: it passes a law that reads any parameter, such as `0 = k (T - room_air())`, and refuses one that reads only inputs

**Non-goals:** more than two nodes, fever, exercise, and clothing as anything but one insulation value.
