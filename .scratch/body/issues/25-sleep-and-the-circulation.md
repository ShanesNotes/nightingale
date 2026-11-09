# 25: Sleep reaches the circulation and the metabolic rate

Type: task
Status: open
Blocked by: 05, 09, `.scratch/research/issues/12-law-forms-heat.md`

**What to build:** Sleep lowers sympathetic outflow as a gain on the nervous system's traffic, and lowers the metabolic rate as a gain on each bed's rate. Both are drivers. The fall in heart rate and pressure at night, and the fall in oxygen use, are outputs the day's checks read. The contract's day table has sleep rows that the slice cannot show, because no payload carries sleep.

- [ ] Tests: each gain reads the sleep state and nothing else, and no law sets a heart rate, a pressure or an oxygen use
- [ ] Run: in sleep, heart rate, arterial pressure and oxygen use fall and then return on waking, by direction. Lying down and sleep together do not overshoot the fall the day's check allows
- [ ] The day table's sleep rows report pass or fail, not "not available yet"
- [ ] The certificate passes on the whole body, and the body's trace is updated with one sentence on why it moved
- [ ] Cost: unknowns and rows before and after, and wall time for each simulated second against ticket 26's budget

**Non-goals:** REM sleep, a sleep law that decides when the body sleeps, and the clock as an oscillator.
