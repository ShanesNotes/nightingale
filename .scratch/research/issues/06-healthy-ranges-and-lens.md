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
- The electrolyte and chemistry rows have no postprandial range of their own. NHANES's fed men (n 47) show no shift past their spread, so the fasted range covers the meal window (record 0004) until a source shows one. Glucose is different: a meal shifts it, so its postprandial reading stays open.
- Against NORIP (Tate 2016 Table 3), Na matches at 137 to 145, and K, phosphate and Ca overlap with narrower ends. Chloride sits lower, 98 to 105 against 101 to 110, and this is not yet explained. The anion gap closes on the NHANES value at both levels.
