# 09: What the return check reads

Type: grilling
Status: open
Blocked by: none

**Question:** Record 0004 says the return check reads each anchor's method and window, at each later 06:55. 17 of the 50 anchor quantities are rhythms, production rates or factors with no snapshot at that moment. What does the check read for each?

Model default: every anchor keeps its return check, read by its own method over its own window, ending at that morning. A day's production is read over the 24 h before it, and a rhythm's amplitude and phase over the same day. This reads the record as written, so no superseding record is needed. A quantity moves to the parameters only if it meets SRS section 2's definition, something laws read and nothing computes, as red cell lifespan and P25 do. A window's length never decides that (I14). A produced quantity whose source window does not fit inside 48 h stays an anchor, and it is listed as blocking the claim until a shorter window has a source, as record 0004 treats any reading with no sourced range.

- [ ] Each of the 17 names the window its source used, moves to the parameters under the SRS definition, or is listed as blocking the claim
- [ ] Each anchor's within-subject variation matches that window (ticket 07)

## Comments

- A cycle-mean anchor such as `cv.hr.supine_rest` reads "Read as a cycle mean", with no duration, because Wolsk 2017 states none. A dry run had to guess it. This ticket owes the duration the return check uses for such a window.
