# 05: Parameters of the body at rung 0

Type: research
Status: open
Blocked by: 02
Data: ns.*

**Question:** What value does each law read, in the order the slice composes them?

Reading order matters more than row count, since one document answers many rows. Bind at the site-class default until a sourced check earns a split (record 0002).

- [ ] CV: elastances and unstressed volumes by vessel class, valves, baroreflex effectors
- [ ] FL: the Starling set by bed class, interstitial compliance, lymph
- [ ] RN: segmental transport for each conserved solute
- [ ] NS and the receptor layer: affinities, on and off rates, junction volumes. An unmeasurable release per firing takes an inference record from spillover
- [ ] RS, BC, EN, HP, GI, HC, MT, MS and IM at rung 0

## Comments

- The sinus node's `R0`, `m` and `n` are in `data/` from Katona 1982, read from its abstract in 10 nonathletes, and `XC-36` closes them against the heart rate anchor. The abstract names no posture, so this cohort's rest and the anchor's supine rest are only assumed to match.
- `XC-36` multiplies cohort means from two studies, so it is a consistency check, not an identity in one man.
- The receptor half of the sinus node law is still open: `S_b`, `S_m` and the `eps` and occupancy behind `E_b1` and `E_m2`. Until then only the product closes at rest. A dry run on main found that wiring this anchor needs them.
