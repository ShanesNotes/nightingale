# 72: A guard's own range is judged, and its own refusal has a source

Spec: `crates/ng-contract/src/evaluate.rs` (`excursions`) and `crates/ng-contract/src/sources.rs` (`refusal_sources`)

Type: task

Status: open

Blocked by: none

On main a validity range on a guard's zero is not judged, because excursions read only the instances' programs. A guard's own refusal, under its mode variable's law, has no refusal source, because the sources come from the programs. So an explicit field's range inside a guard is not judged either. The test `written::a_fields_range_is_judged_in_a_guard_that_reads_it` (from the second break-it pass on nightingale#179) fails for that reason, and it waits on this ticket.

- [ ] A guard's own validity range is judged, and leaving it is reported under the guard's law
- [ ] A guard's own refusal has a refusal source
