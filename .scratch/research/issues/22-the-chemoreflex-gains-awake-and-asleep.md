# 22: The chemoreflex's gains awake and in NREM sleep

Type: research
Status: open
Blocked by: 11

**Question:** Which cited values set the central and peripheral chemoreflex gains of a healthy adult at rest, awake and in NREM sleep, so the resting loop gives the measured ventilation slopes? [Research 11](11-law-forms-gas-loop.md) decided the controller's gains are calibrated until the resting loop gives Dahan's and Douglas's slopes, which are ventilation per mmHg of the whole loop, and bound no value. [Body ticket 11](../../body/issues/11-breathing-is-controlled.md) states each as a synthetic value.

Today's forms, from body 11's plan:

- The drive scale is `C = 1 - W s + (1 - L s) ((x_p - r_p) / w_p - (pH - r_c) / w_c)`, with relative gains 2 for the central arm and 1 for the peripheral, times 1/0.778 since the scale moves the breath's rate.
- The peripheral arm is research 11's `G_p (1 - S_a) smooth_max(PaCO2 - B_p, 0, w)`, with the peripheral offset `B_p` at 20.
- Sleep removes the wake drive `W`, 0.15, and lowers both gains by `L`, 0.4.

- [ ] The whole loop's ventilation slope against carbon dioxide and against desaturation, awake and in NREM sleep, with the population and the method
- [ ] Each arm's share of the carbon dioxide response, and the peripheral offset
- [ ] The wake drive's size, read as the shift of the apnoeic level between waking and NREM sleep, and each arm's sleep factor
- [ ] Whether a gain that meets these slopes makes the loop swing at rest, since periodic breathing is a non-goal
- [ ] Each constant's row for [research 16](16-constants-and-what-binds-them.md)'s list: what measurable quantity binds it
- [ ] The steady change in arterial carbon dioxide a healthy adult shows for a stated change in carbon dioxide production or in inspired carbon dioxide, with the population and the method, so that once [body 38](../../body/issues/38-the-drive-moves-the-breaths-depth.md) has built depth, the whole loop, depth and rate together, can be sized against a cited figure and not by its slopes alone

**Non-goals:** REM sleep, the transport delays research 11 dropped, and how the drive splits into rate and depth ([research 23](23-how-the-drive-splits-into-rate-and-depth.md)).
