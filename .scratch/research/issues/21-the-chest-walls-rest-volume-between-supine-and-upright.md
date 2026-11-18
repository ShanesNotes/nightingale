# 21: The chest wall's relaxed volume between supine and upright

Type: research
Status: open
Blocked by: 11

**Question:** How does the chest wall's relaxed volume, and with it the resting lung volume, move with the trunk's angle between lying flat and sitting up, and by how much? [Body ticket 10](../../body/issues/10-lungs-breathe.md)'s second PR states the move as linear in the sine of the trunk's angle with a synthetic slope, and [research 11](11-law-forms-gas-loop.md)'s resting volume row found the direction and the two ends only (Nunn's Figure 11, about 3,500 mL upright and 2,500 mL supine; Katz 2018) and no volume at angles between.

Today's form, in `crates/ng-respiratory/src/airway.rs`:

- `V_W0 = V_W0,supine + dV_W rise / L`, with `rise` the carotid sinus's height over the systemic arteries under the drive frame's posture and `L` its distance up the trunk, 0.2, so `rise / L` is the sine of the trunk's angle. The slope `dV_W` is 10,000 against a supine relaxed volume of 66,997.6 (75,750 less the pump's mean pull, so the breath's means are the body's admitted ones), so sitting up raises the resting volume by 5,000 of 57,000 and lowers the pleural pressure at rest by 0.8.
- The pleural pressure at rest follows: `P_pl,rest = -E_L E_W (V_W0 - V_L0) / (E_L + E_W)`.

- [ ] The resting lung volume (FRC) at angles between supine and upright, as a curve or as values at stated angles, with the population and the method
- [ ] Whether the move is the chest wall's relaxed volume alone, or the lung's or the chest wall's compliance moves with posture too, with the source
- [ ] Whether the legs' position (dependent, horizontal, standing) moves the resting volume at a given trunk angle
- [ ] Each constant's row for [research 16](16-constants-and-what-binds-them.md)'s list: what measurable quantity binds it

**Non-goals:** the lung's and the chest wall's values at rest supine, which [research 19](19-the-alveolar-units-values.md) asks for, and posture's effect on the circulation.
