# 03: The anchors

Type: research
Status: open
Blocked by: 01
Data: bc.*, en.*, fl.*, hc.*

**Question:** What does each of P1 to P26 take, with method and averaging window (SRS rule 17), so that the set closes?

- [ ] Every anchor names a source record, a method and a window; none rests on an inference (rule 16)
- [ ] CO, HR, MAP, CVP, pulmonary pressures and wedge pressure close at both levels
- [ ] Plasma Na, K, Cl, phosphate, Ca, Mg, albumin and glucose name a measured distribution, not an interval midpoint, where one exists
- [ ] P9, P14, P19, P20, P21 and P22 are filled, and each pick between method families states why
- [ ] P8 dead space and P25 tissue CO2 capacity move to parameters, since neither is a produced quantity

## Comments

Picks so far, each with its reason:

- **Na, K, Cl, phosphate, albumin, glucose.** NHANES men 20 to 39, fasted. These are the measured distributions the ticket asks for, with the assay each file documents. Ca stays open: ionized Ca is the anchored reading and total Ca is only a range. NHANES has no Mg.
- **Hemoglobin stays on Johnson 2014, 14.3 g/dL.** He is supine at admission and NHANES draws seated, at 15.05 g/dL. Wolsk 2017 reads 14.2 supine, and ICRP red cell volume x MCHC / blood volume gives 14.5, so the supine figure is the one that fits him. A laboratory arterial sample is the method as charted, which is what the SRS anchor definition asks for. NHANES gives the range.
- **Albumin has no supine cohort on hand.** NHANES draws seated with no posture correction, so 44.1 g/L may read high for a supine man, and its certainty stops at moderate. A supine measured distribution replaces it.
- **Glucose takes the plasma tube, 5.52 mmol/L.** The serum panel reads 5.07 in the same men, and NHANES names the plasma result as its reference method.
- **Serum against plasma.** The electrolyte anchors are serum, and their ids say plasma because the body holds plasma. The return check reads them by the serum method named on each record, so the lens needs that reading.
