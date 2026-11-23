# 28: The whole body lives two synthetic days

Type: task
Status: open
Blocked by: 19, 20, 21, 22, 24, `.scratch/native-contract/issues/95-the-record-trims-its-history.md`

**What to build:** Both passes together, over 48 hours of the synthetic day on one named build worker. This is the map's destination. Every number is synthetic, so it is not the first milestone: it is the body that [foundation 01](../../foundation/issues/01-arena-and-milestone.md), step 4, binds the reviewed packet to.

- [ ] The certificate passes on every sliding 24 h window of the 48 hours
- [ ] The healthy-range check passes on a synthetic set, and the return check passes at 24 and at 48 hours. Missing coverage is named and never counts as a pass
- [ ] From a start pushed off balance (stores 5% low, a salt load, a warm hour), every loop brings its amount back, with all loops on
- [ ] The hold table lists only the holds rung 0 keeps, each with its reason, and the census accounts for every coupling arrow
- [ ] The cost of the 48 hours meets [ticket 26](26-cost-budget.md)'s ceiling under its repeat protocol
- [ ] The course reports what its store keeps during the run and after it, in bytes, beside [record 0008](../../../docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md)'s arithmetic for the slice
- [ ] The Answer lists what must change before the reviewed packet is bound

**Non-goals:** any clinical claim, and the speed aim itself.

A note from the check of native-contract 85's second PR: no 48-hour protocol uses the body's policy yet. The milestone protocol (`ArenaProtocol::milestone` in `crates/ng-validate/src/arena.rs`) runs on the slice under `arena_certificate`, whose drift table states no windows between edges, and only the body's tests read `body_certificate`.

A note from the check of body ticket 12's PR 12a, for this ticket's energy check: the heat loop's test bounds heat made less heat lost by `HEAT_SHARE` (`crates/ng-validate/tests/body_run.rs`, line 3931), whose bound of 1e-1 of heat made is set by the synthetic forcing's swing over the reference run's draw. With the forcing at amplitude 0 the check measured the loop alone balancing to 5.47e-4 of heat made. So while the forcing runs, that test does not refuse a loop imbalance under about a tenth of heat made over a breath window.

From [ticket 06](06-whole-body-on-holds.md)'s plan: the hold table this ticket lists gains the cell anion, which body 01's table of the holds rung 0 keeps (01:635-651) leaves out. BC holds it in the cells of every bed, because no rung 0 law moves it, and [ticket 17](17-volume-hormones.md) reads it (01:547).
