# 06: Healthy ranges and the lens

Type: research
Status: open
Blocked by: 01
Data: obs.range.*, bc.anion_gap.range

**Question:** Which readings does the milestone's lens carry, and what is each one's healthy range by activity, method, sex and age where a source shows a shift (record 0004)?

- [ ] Each lens reading names its method, site and averaging window
- [ ] A range computed from a public dataset names the dataset's version and query
- [ ] A waveform reading is held by named features of its curve, each with a range
- [ ] A reading with no sourced range is listed as blocking the claim

## Comments

- The first 15 ranges are NHANES men 20 to 39, as percentiles of a survey-weighted sample. Each record's method comes from the file's documentation.
- The electrolyte and chemistry rows have no postprandial range of their own. NHANES's fed men (n 47, under 4 h since food) are not a meal study: a few men set their percentiles, so they neither show a shift nor rule one out. Record 0004's default is one range unless a source shows a shift, so a binding agent uses the fasted range after a meal and lists that as resting on the default. Glucose and potassium are the exceptions, because a source shows the shift: Guyton 12e p.361 describes plasma potassium rising after a meal, held down by insulin. No source on hand gives either a range, so both postprandial readings stay open and block the claim (record 0004).
- Against the country intervals in Tate 2016 Table 3: Na 137 to 145 matches the Nordic interval, K 3.6 to 4.8 matches Japan's and runs past the Nordic 4.6, and Ca and phosphate sit inside the Nordic intervals. The Nordic column has no chloride. NHANES chloride, 98 to 105, sits inside the UK's 95 to 108 and Australasia's 95 to 110, and its lower limit is under Australia's, Japan's and Canada's, 101 to 102. Countries differ by more than that gap, so the method on the record matters. The anion gap closes on the NHANES value at both levels.
