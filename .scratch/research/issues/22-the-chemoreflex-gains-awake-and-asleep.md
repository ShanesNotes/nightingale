# 22: The chemoreflex's gains awake and in NREM sleep

Type: research
Status: open
Blocked by: none

The forms it sources values for are on main since nightingale#211 and nightingale#214, and the question reads the literature and needs no more merged code. The PR that binds the gains waits on this Answer: [body ticket 11](../../body/issues/11-breathing-is-controlled.md) states each value as synthetic and points here, and [body 38](../../body/issues/38-the-drive-moves-the-breaths-depth.md) binds none until it lands.

**Question:** Which cited values set the central and peripheral chemoreflex gains of a healthy adult at rest, awake and in NREM sleep, so the resting loop gives the measured ventilation slopes? [Research 11](11-law-forms-gas-loop.md) decided the controller's gains are calibrated until the resting loop gives Dahan's and Douglas's slopes, which are ventilation per mmHg of the whole loop, and bound no value. [Body ticket 11](../../body/issues/11-breathing-is-controlled.md) states each as a synthetic value.

Today's forms, on main as body 11 built them:

- The drive scale is `C = R - W s + (1 - L s) (G_p (x_p - r_p) / w_p - G_c (pH - r_c) / w_c)`, as `crates/ng-nervous/src/chemoreflex.rs` has it, with `R` the scale's rest, 1 in the body packet. The body packet's gains are synthetic: `G_c = 2 / 0.778` for the central arm and `G_p = 1 / 0.778` for the peripheral, relative gains 2 and 1 raised by 1 / 0.778 since the scale moves the breath's rate.
- The peripheral sensor reads `x_p = (1 - S_a) smooth_max(PaCO2 - B_p, 0, w)`, research 11's arm with its gain `G_p` in the scale above, and the peripheral offset `B_p` at 20.
- Sleep removes the wake drive `W`, 0.15, and lowers both gains by `L`, 0.4.

- [ ] The whole loop's ventilation slope against carbon dioxide and against desaturation, awake and in NREM sleep, with the population and the method
- [ ] Each arm's share of the carbon dioxide response, and the peripheral offset
- [ ] The wake drive's size, read as the shift of the apnoeic level between waking and NREM sleep, and each arm's sleep factor
- [ ] Whether a gain that meets these slopes makes the loop swing at rest, since periodic breathing is a non-goal
- [ ] Each constant's row for [research 16](16-constants-and-what-binds-them.md)'s list: what measurable quantity binds it
- [ ] The steady change in arterial carbon dioxide a healthy adult shows for a stated change in carbon dioxide production or in inspired carbon dioxide, with the population and the method, so that once [body 38](../../body/issues/38-the-drive-moves-the-breaths-depth.md) has built depth, the whole loop, depth and rate together, can be sized against a cited figure and not by its slopes alone
- [ ] The wake drive the cited values give stays below the drive scale's rest, so the sleeping baseline, the scale with no deviation at either sensor, stays above zero. It does not keep the scale above zero, since the arms still move it. The check of nightingale#214 showed the fault on a fixture admitted awake that slept at one second: a wake drive that reaches the rest admitted and faulted at sleep's onset. A body can also be admitted asleep, which that check did not run

The form of the sleep law that the cited gains land in is in body ticket 11's 11c bullet "The law's form", and [body 38](../../body/issues/38-the-drive-moves-the-breaths-depth.md) takes it.

**Non-goals:** REM sleep, the transport delays research 11 dropped, and how the drive splits into rate and depth ([research 23](23-how-the-drive-splits-into-rate-and-depth.md)).
