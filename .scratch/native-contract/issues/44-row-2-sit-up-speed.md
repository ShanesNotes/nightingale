# 44: Row 2 holds a 10 s sit-up ramp to an active stand's dip

Spec: [native-contract 21](21-day-checks-read-the-trace.md)'s table and comments (row 2, Smith 1994), `crates/ng-validate/src/day.rs` (`Occurrence::sit_up`, `RAMP`) and `crates/ng-validate/src/day/table.rs` (row 2)

Type: research

Status: open

Blocked by: none

Row 2 wants the arterial pressure at heart level to dip within 10 s of sitting up and recover within 30 s. It fails on every core: in the pick dry run at `43afdb7`, and on main `c4760e8` for 06:55 to 07:30 (`engine-bench day 10 ng-solver 2100`, ryzen, 286 s), with the same line both times: no dip, then up +2.45 from 30 s.

The day plan's sit-up is 21 posture events half a second apart: the trunk rises over 10 s and the legs hang from 5 s. The baroreflex follows within a beat, so the pressure never sits more than 0.2024 below its baseline for more than one beat, and its lowest beat is -0.21 at 6.4 s (per-beat trace, `Core::new` and `Core::doubled` alike). The same body and core given the whole sit-up as one event at 07:00 dips 5.4 at 4.7 s, three beats past the declared tolerance from 3.4 s, and is back by 7.5 s: row 2's rule passes that trace. So the slice holds a dip; the 10 s ramp hides it.

Ticket 21's comments already say the posture sources study standing or head-up tilt, not sitting. Smith 1994 is not in the evidence store, so how fast a healthy man sits up, and whether sitting up (not standing) dips the pressure at heart level, rest on no source on hand.

This ticket lands before [body 24](../../body/issues/24-the-synthetic-day.md) runs the day on the whole body.

Evidence: `~/lanes/logs/ng-wave-4/sodium/` (`REPORT.md`, and `probe/rows-p2-plan-new.txt` and `probe/rows-p6-jump-new.txt`).

**What to find, then build:**

- [ ] A source for how long an active sit-up from supine takes, and whether it dips the arterial pressure at heart level, read and cited with how far it was read
- [ ] If it does dip, the day plan's sit-up takes the source's duration, and row 2 is checked again on a short run. The arena's event digests move once, with the day ticket that moves them (body 31)
- [ ] If it does not, row 2 is rescoped for a sit-up or marked not available yet with its reason, in the table and in ticket 21's registry test

**Non-goals:** the baroreflex's laws and gains; standing; the other sit-up rows.
