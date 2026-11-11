# 20: Cortisol's daily rhythm

Type: task
Status: open
Blocked by: 05, 14, `.scratch/research/issues/15-law-forms-slow-books.md`

**What to build:** Cortisol is a species secreted on a daily profile that the clock drives, cleared at its own rate, and acting on the liver's glucose output through the receptor layer. The profile is a driver. It never sets a produced quantity. This is pass 2.

- [ ] Tests: the secretion profile and the clearance match their forms at stated points
- [ ] Run: the liver's glucose output rises toward morning with cortisol, and no law sets glucose
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] From [research 15](../../research/issues/15-law-forms-slow-books.md): the receptor layer's effect reads one ligand with a fixed efficacy, so this ticket picks how cortisol lowers insulin's efficacy at the liver: the effect kernel reads a second input, or the layer publishes occupancy and HP's law finishes the effect
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget
- [ ] From [ticket 05](05-the-days-inputs.md): each law here that reads a lasting input is reviewed against the rule by reading: the input scales a gain or a set point and never stands for a produced quantity. Compile's `UnscaledInput` is only a stand-in: it passes a law that reads any parameter, such as `0 = k (T - room_air())`, and refuses one that reads only inputs

**Non-goals:** the stress response, pulsed secretion, and cortisol's other actions.

From [research 15](../../research/issues/15-law-forms-slow-books.md): Scheer counts cortisol's phase from the core temperature minimum, so the secretion law maps that zero to the one [ticket 05](05-the-days-inputs.md) states.

From [research 13](../../research/issues/13-law-forms-gut-and-fuels.md): the liver's glucose output is the sum of its reactions, and only the glycogen rows read insulin's effect, while gluconeogenesis reads glucagon alone, so lowering insulin's efficacy reaches glycogen synthesis and breakdown only. Research 13 splits it, so this ticket can still take the gain on gluconeogenesis that research 15 set aside.
