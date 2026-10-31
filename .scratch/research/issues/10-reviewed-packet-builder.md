# 10: Reviewed packet builder

Type: task
Status: open
Blocked by: 01, ../../native-contract/issues/08-admission-with-anchor-statistics.md

**What to build:** The contract gains the `Reviewed` packet kind, and a host tool builds it from `data/`. A crate reads no file, so the tool writes one Rust source, and the check fails when that source differs from the data. Rebuilding gives the same hash (I12).

- [ ] The crate table names where a generated packet lives
- [ ] A value without its trace, an anchor resting on an inference and an inference record naming no search are refused when building (I16)
- [ ] The validation crate still accepts only the fixture kind until the milestone
