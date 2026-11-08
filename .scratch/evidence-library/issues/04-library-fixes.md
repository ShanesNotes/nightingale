# 04: Fix what the tilt slice found in the library's steps

Type: task
Status: open
Blocked by: none

**What to do:** fix the faults [ticket 01](01-tilt-neighborhood.md) found in the library's pipeline before the next slice, then rerun the tilt slice and report what changed.

- [ ] Split chunks longer than Jev's context. Six whole papers sit in one chunk each, so Jev never tagged them; one is the renal autoregulation review (PMC4551215)
- [ ] Drop terms that begin with a verb or name no quantity, such as "stimulates renin secretion", "cytoplasmic concentration" and "excessive level", and merge reflex variants such as "carotid baroreflex" and "prolonged baroreflex" into one term
- [ ] Add "feedback", "resetting" and "set point" to the term step's quantity nouns, since baroreflex resetting never became a term
- [ ] Rewrite the fluids facet so two annotators agree on it (they reached kappa 0.46), then rescore Jev on it
- [ ] Give the two-step test a hub control, since its chains run through arterial pressure and sympathetic activity
