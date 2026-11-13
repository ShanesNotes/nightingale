# 37: The return check reads its run's own anchors

Spec: [solver contract](../../../docs/architecture/solver-contract.md), the paragraphs "The healthy-range set is apart from the run too" and "The healthy-range check and the return check read a bare lens", and `crates/ng-contract/src/admission.rs` (`InitialRevision`)

Type: task

Status: open

Blocked by: none

[Body ticket 23](../../body/issues/23-healthy-range-and-return-checks.md) built the return check. It takes a packet's every anchor through `ParameterPacket::anchors`, a view with no values, so a caller cannot drop one anchor. But it takes any packet's view. A run admitted from one packet can be checked against another packet's anchors, and the return report then names that other packet. The check cannot refuse it: `InitialRevision` keeps the packet's own hash in a private field, and a run's identity seals only the hash admission closes over the start. So nothing public lets a session say which packet it was admitted from.

**What to build:** `ReadingChecks::new` refuses anchors from a packet the run was not admitted from.

- [ ] The admitted run, or its record, names the hash of the packet it bound, without moving the run's identity or any pinned seal
- [ ] `ReadingChecks::new` refuses an `Anchors` whose packet is not that one, named, with a test and a mutant row
- [ ] The activities the healthy-range check reads are made from the packet's day, or checked against it. Today `ReadingChecks::new` checks a caller's `DayActivities` only for order, repeats, a first span that holds admission and changes on the grid, so a caller that states one waking span for a run whose packet's day has a night is not refused, and the night's values are judged as waking values. The range report names the stated spans' digest, which cannot be compared with the packet's hash. The last check of nightingale#133 found it. A test states a day that disagrees with the packet's and is refused, named, with a mutant row
- [ ] The contract and its sketch say so

**Non-goals:** any change to what the identity seals, any anchor value leaving the packet (I14), and any code that turns a packet's day into activities beyond what the check reads.

This lands before [body ticket 24](../../body/issues/24-the-synthetic-day.md), which builds on the return report.
