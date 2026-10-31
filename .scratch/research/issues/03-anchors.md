# 03: The anchors

Type: research
Status: open
Blocked by: 01
Data: bc.*, en.*, fl.*, hc.*, rn.gfr.0655

**Question:** What does each of P1 to P26 take, with method and averaging window (SRS rule 17), so that the set closes?

- [ ] Every anchor names a source record, a method and a window; none rests on an inference (rule 16)
- [ ] CO, HR, MAP, CVP, pulmonary pressures and wedge pressure close at both levels
- [ ] Plasma Na, K, Cl, phosphate, Ca, Mg, albumin and glucose name a measured distribution, not an interval midpoint, where one exists
- [ ] P9, P14, P19, P20, P21 and P22 are filled, and each pick between method families states why
- [ ] P8 dead space and P25 tissue CO2 capacity move to parameters, since neither is a produced quantity

## Comments

Picks so far, each with its reason:

- **Na, K, Cl, phosphate, albumin, glucose.** NHANES men 20 to 39, fasted. These are the measured distributions the ticket asks for, with the assay each file documents. Ca stays open: ionized Ca is the anchored reading and total Ca is only a range. NHANES has no Mg.
- **Hemoglobin stays on Johnson 2014, 14.3 g/dL.** He is supine at admission and NHANES draws seated, at 15.05 g/dL. ICRP red cell volume x MCHC / blood volume gives 14.5, close to the supine figure. Wolsk 2017's 14.2 points the same way, but it pools 10 men and 10 women and does not state the draw's posture. A laboratory arterial sample is the method as charted, which is what the SRS anchor definition asks for. Johnson's men ate a 240 kcal bar first, and each of those records says so. NHANES gives the range.
- **Albumin has no supine cohort on hand.** NHANES draws seated with no posture correction, so 44.1 g/L may read high for a supine man, and its certainty stops at moderate. A supine measured distribution replaces it.
- **Glucose takes the plasma tube, 5.52 mmol/L.** NHANES names the plasma result as its reference method over the serum panel. On the 153 men with both, on fasting weights, plasma reads 5.53 and serum 5.10.
- **Serum against plasma.** The electrolyte anchors are serum, and their ids say plasma because the body holds plasma. The return check reads them by the serum method named on each record, so the lens needs that reading.
- **Fasting insulin is a median, 40.7 pmol/L.** The distribution is skewed, with a mean of 51 and an SD of 36, so a mean and SD would misstate it. The record gives the median and central 95% from NHANES.
- **GFR is Poggio's median scaled by his BSA, 120.8 mL/min.** Poggio's 110 per 1.73 m2 is for non-African-American men 26 to 35, and its 5th to 95th span is a central 90%, which the input record states. Those men took a water load and he does not. Guyton 12e p.312 gives 125.
- **ANP rests on an abstract.** Raine 1986 gives 17 +/- 2 pmol/L as mean +/- SEM in 60 healthy controls, with no posture or time of day, so its certainty stays low.
