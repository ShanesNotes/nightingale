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
- `XC-36` compares the product of Katona's cohort means with Wolsk's mean, so it is a consistency check, not an identity in one man.
- The receptor half of the sinus node law is still open: `S_b`, `S_m` and the `eps` and occupancy behind `E_b1` and `E_m2`. Until then only the product closes at rest. A dry run on main found that wiring this anchor needs them.
- `S_b` and `S_m` enter as derivations from `m`, `n` and the resting effects, not as calibrated values, because `m` and `n` are parameters and a calibrated record names anchors only.
- Cardiovascular pilot, 2026-11-07. These rows are candidates. None entered `data/`, because none passed appraisal. Each number below was read from a public abstract. Full texts of licensed sources were not read.

  | Parameter | Source | Value | n | Population and method | Verdict |
  |---|---|---|---|---|---|
  | `cv.orifice`, left ventricle to systemic arteries | EACVI and ASE 2017 focused update, doi:10.1016/j.echo.2017.02.009 | none | | stenosis grading | Not fit: its numbers are disease cut-offs, not a healthy valve's EOA |
  | `cv.elastance`, systemic arteries | Gudbrandsson 1992, doi:10.3109/08037059209065121 | SV/PP 1.90 ± 0.58 (men), 2.00 ± 0.62 (women), after adjustment for body surface area; unit and statistic not stated | 801 | Normal adults 30 ± 5.6 y, echo-Doppler SV over cuff pulse pressure | Unresolved: needs the full text; cuff pressure biases SV/PP low |
  | `cv.elastance`, systemic arteries | Chemla 1998, doi:10.1152/ajpheart.1998.274.2.H500 | none in the abstract | 7 controls | Invasive aortic pressure, area method | Method support for SV/PP; values need the full text |
  | `cv.elastance`, systemic arteries | Kelly 1992, doi:10.1161/01.cir.86.2.513 | Ea | 10 | End-systolic pressure over SV | Rejected: effective arterial elastance is a load index, not 1/C |
  | `cv.elastance`, systemic arteries | Stergiopulos 1999, doi:10.1152/ajpheart.1999.276.2.H424 | 0.169 ± 0.007 mL/mmHg (SE) | 7 | Anaesthetised dogs | Rejected: species. Shows that compliance depends on the pressure it is read at |
  | `cv.elastance`, leg veins | Halliwill 1999, doi:10.1152/jappl.1999.87.4.1555 | none in the abstract; day-to-day CV 4.9% | 11 | Calf, deflating venous cuff from 60 mmHg, quadratic fit | Unresolved: values are in the full text. Calf per 100 mL tissue reaches the whole leg only by a justified scaling |
  | `cv.unstressed-gain`, leg veins | Halliwill 1999 | none | 11 | Handgrip and post-exercise ischaemia | Not enough information: no number, and no measure of the effect the shift rode on |
  | `cv.unstressed` | Magder 1998, doi:10.1097/00003246-199806000-00028 | stressed volume 20.2 ± 1.0 mL/kg | 5 | Patients in hypothermic circulatory arrest | Rejected: whole body, patients, no sympathetic contrast |

- The orifice maps by B = ρ / (2 EOA²), so a sourced blood density joins any EOA row. The arterial elastance maps as 1/C only for whole-tree compliance read over the pulse pressure range.
- Held out for testing, never fitted: Sarafian and Miles-Chan 2016, Front Physiol, doi:10.3389/fphys.2016.00656, CC BY. Its protocol was frozen before the search. It reports sustained heart rate and pressure at tilt sines 0.342, 0.643 and 0.866. So it can test arterial compliance through pulse pressure, and the leg venous rows only indirectly. It cannot test the valve, or the first minutes after a tilt.
- From [research 12](12-law-forms-heat.md): the sudomotor command's one weight pair binds between Wenger's 8 and Stolwijk's 15.6, Windred's half-amplitude binds the set point target's half-range and not the 24 h terms alone, Buono's slope per area becomes `k_c`, and one NREM sympathetic gain weights the stages over a night. Gagge, Stolwijk and Nishi's reprint and Kingma's supplement sit outside the store's catalog, and ICRP 89 prints Lamke's insensible loss at 27 °C as 626 mL/day where Lamke's abstract gives 526.
- From [research 13](13-law-forms-gut-and-fuels.md): an inference record carries the emptying row's `V_h`, a model default well under 200 mL, and Hunt and Stubbs's `a` and `b` are read from Appendix II's eqn 2.8, because the store text garbles eqn 2. Sherwin's hepatic insulin clearance of 400 mL/min is `Q_L E` on inflowing plasma, so `CL_L` binds as `Q_L E / (1 - E)`. Schaller's supplement, outside the store's extracted text, holds the only fit of glucagon's secretion and clearance, no measured glucagon clearance is on hand, and Dalla Man 2009 fits none of the uptake constants, so Visentin's fits are their check.
- From [research 14](14-law-forms-volume-and-acid.md): hemoglobin's buffer value of 2.3 is per heme, so `b_Hb` is 4 times 2.3 per molecule or the per-heme value on the mass basis, and Antonius's tables print `Kc` and albumin in units that do not fit their values. `c_thr` adds dissolved carbon dioxide to Ganong's plasma bicarbonate threshold, aldosterone's `K0` and `dK` fit Guyton and Hall's Fig. 29-5 and not the report's 3.3, and the clock's gain on filtration waits for a source or an inference record.
- From [research 15](15-law-forms-slow-books.md): Rude's magnesium maximum is a concentration in filtrate, read against ultrafilterable magnesium, so `T_Mg` binds against whole plasma magnesium as a model default or through a sourced ultrafilterable share. The glucocorticoid receptor's binding has no human source, so an inference record fills it, and PTH's receptor at the tubule needs its binding constants and `eps`.
