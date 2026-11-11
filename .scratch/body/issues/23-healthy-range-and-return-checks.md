# 23: The healthy-range check and the return check

Type: task
Status: resolved
Claimed by: 23-range-return-checks
Blocked by: none

**What to build:** The two checks of record 0004 that no code holds yet. The healthy-range check tests each reading in the lens against its healthy range, over every sliding window of its study's length, for the activity that window lies in. The return check tests each anchor 24 and 48 hours after admission against the change its within-subject variation allows. Both run in validation on a synthetic healthy-range set, read a run through the lens with no observation error, and name the run and the set they read (record 0005). The [foundation map](../../foundation/map.md) lists them as unticketed.

- [x] A synthetic run built to leave its range for one window fails the healthy-range check, and the failure names the reading and the window
- [x] A run that stays in range but ends a day away from its anchor fails the return check and passes the other. A run that returns to its anchor but leaves its range between fails the healthy-range check and passes the other
- [x] A reading that leaves its range between two sampling instants, a window that crosses a change of activity, and a study that averaged a whole period are each judged as record 0004 says. A change with no range of its own blocks the verdict
- [x] A corrected set rechecks a finished replay and gives a new result without changing the run's identity
- [x] A reading with no range in the set blocks the verdict and is named, never skipped
- [x] Each rule has a must-fail row or a mutant row

**Non-goals:** any clinical range (the research map sources them), the observation error a learner sees, and the monitor's display.

This lands the healthy-range set item of [foundation 07](../../foundation/issues/07-contract.md) in the crates. It needs no system and can start today.

## Answer

Both checks are built in `ng-validate` as one fold over a session from admission, `ReadingChecks`. It reads every accepted revision through a `BareLens`: the run lens's rows and methods, with no window, delay or error. The healthy-range set is in `ng-contract`, beside the packet and the lens, because only there can a range's dimension be checked against its row. It has its own digest, nothing that builds a run's identity takes it, and only the synthetic kind can be built. `ParameterPacket::anchors` lists each anchor's id, row, method, statistic and window, never its value. Every rule has a test on a synthetic stream or a synthetic run made to break it alone. Every rule that one mutation can break has a row in `tools/gate/mutants.toml`, 67 rows in all. The type rules have compile-fail doctests that `gate.py codes` holds.

Model defaults, each a reading of record 0004 that admits two:

- **Windows end on grid instants.** The contract already read the rule this way ("reads a bare channel and slides its own mean"). A sampled method has no value between grid instants. A time-mean window that ends between them differs from the nearest judged window by at most the grid period over the window's length, times the reading's swing. This sits in tension with record 0004, which rejected reading what a learner sees partly because the verdict "would change ... with how often the lens samples": here a sampled reading's verdict depends on the lens's grid too, within that bound for a time mean. Alternative: windows ending anywhere, from each step's chord.
- **A change's own range covers the time its shift lasts.** The record's example is the pressure dip after sitting up. A change range states how long its source shows the shift lasting. The change owns the time from the change until its longest shift ends, and a window that crosses the change or starts in that time takes only the change's range of its length. Alternative: only windows that cross the change instant, which fails the record's own example when the dip outlasts them.
- **A change with no range of its own blocks** when its two activities' ranges differ. It is judged by the every-activity range when both activities fall back to it. Alternative: leave such windows unjudged, which record 0004 rejects as a gap.
- **A whole period that the run's start or end cuts** is counted, not judged. A range that judged no whole window or whole period blocks, by the coordinator's ruling.
- **The return check compares two mornings.** It compares each later 06:55 reading with the run's own admission-morning reading by the same method and window, which admission solved onto the anchor. The coordinator ruled this reading.
- **A check that tested nothing never passes.** A range whose activity or change the run holds but which judged nothing blocks. So does a range check that judged nothing at all, and a packet with no anchors.

Handoffs:

- **Ticket 24 and the milestone.** A cycle anchor's morning window runs one cycle past 24 h and 48 h, so a run that must show the 48 h return runs one cycle past 48 h. Until the packet's day can give the day's activities ([ticket 05](05-the-days-inputs.md)), the caller states them. A sliding window holds one value per grid period, so a 24 h range on a 10 ms grid holds 8.64 million values per reading.
- **Binding the anchors to the run.** The return report names the packet whose anchors it read. Refusing another packet's anchors needs the run to keep its packet's own hash, because the run's identity seals only the hash admission closes. [Native-contract 37](../../native-contract/issues/37-the-return-check-reads-its-runs-own-anchors.md) does that before ticket 24.
- **The reviewed kind.** Its I16 refusals belong to foundation 07's inference-records item and to [research 10](../../research/issues/10-reviewed-packet-builder.md). Matching a range to the patient's sex and age waits for a packet that states them.
