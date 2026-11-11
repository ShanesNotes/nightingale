# 10: Reviewed packet builder

Type: task
Status: open
Blocked by: 01, ../../native-contract/issues/08-admission-with-anchor-statistics.md

**What to build:** The contract gains the `Reviewed` packet kind, and a host tool builds it from `data/`. A crate reads no file, so the tool writes one Rust source, and the check fails when that source differs from the data. Rebuilding gives the same hash (I12).

- [ ] The crate table names where a generated packet lives
- [ ] A value without its trace, an anchor resting on an inference and an inference record naming no search are refused when building (I16)
- [ ] The validation crate still accepts only the fixture kind until the milestone
- [ ] From [body 05](../../body/issues/05-the-days-inputs.md): building a reviewed packet refuses one with no day or an empty one, because a clinical run takes its day's events only from the day in its packet (records 0004 and 0005), never from plain events. Interventions still reach the run as events of their own. The type refuses a reviewed packet with no day (`with_day`), and an empty `Day` passes it, so the builder refuses that
