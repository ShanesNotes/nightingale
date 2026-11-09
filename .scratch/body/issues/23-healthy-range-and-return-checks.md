# 23: The healthy-range check and the return check

Type: task
Status: open
Blocked by: none

**What to build:** The two checks of record 0004 that no code holds yet. The healthy-range check tests each reading in the lens against its healthy range, over every sliding window of its study's length, for the activity that window lies in. The return check tests each anchor 24 and 48 hours after admission against the change its within-subject variation allows. Both run in validation on a synthetic healthy-range set, read a run through the lens with no observation error, and name the run and the set they read (record 0005). The [foundation map](../../foundation/map.md) lists them as unticketed.

- [ ] A synthetic run built to leave its range for one window fails the healthy-range check, and the failure names the reading and the window
- [ ] A run that stays in range but ends a day away from its anchor fails the return check and passes the other. A run that returns to its anchor but leaves its range between fails the healthy-range check and passes the other
- [ ] A reading that leaves its range between two sampling instants, a window that crosses a change of activity, and a study that averaged a whole period are each judged as record 0004 says. A change with no range of its own blocks the verdict
- [ ] A corrected set rechecks a finished replay and gives a new result without changing the run's identity
- [ ] A reading with no range in the set blocks the verdict and is named, never skipped
- [ ] Each rule has a must-fail row or a mutant row

**Non-goals:** any clinical range (the research map sources them), the observation error a learner sees, and the monitor's display.

This lands the healthy-range set item of [foundation 07](../../foundation/issues/07-contract.md) in the crates. It needs no system and can start today.
