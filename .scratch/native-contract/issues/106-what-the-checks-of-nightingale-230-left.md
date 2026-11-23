# 106: What the checks of nightingale#230 left

From: the checks of nightingale#230, [native-contract 102](102-the-skins-reads-a-design-round.md)'s contract PR

Spec: the read grants in `crates/ng-contract/src/compile.rs` and `crates/ng-contract/src/catalog.rs`, their tests in `crates/ng-contract/src/publication.rs`, `carriage.rs`, `sweat.rs` and `skin_vessels.rs`, and their rows in `tools/gate/mutants.toml` and `tools/gate/must-fail.toml`

Type: task

Status: open

Blocked by: none

Blocks: none

**Why.** I19's body reads the skin's temperature, carries heat at each bed and makes sweat through the read grants native-contract 102 built, and a grant is only as strong as the tests that would see it change. The checks of its contract PR left the things below. Each box is one a later PR must build or decide, and each note is one a reader must know. Every value is synthetic.

- [ ] The `compile_fail` doctest on `SkinThermoreceptor::temperature` (`crates/ng-contract/src/catalog.rs`, lines 691 to 698) states the sampled site, never the height site, while rustc refuses the doctest's call because it hands a junction kernel where a body kernel is due. The sentence it hangs on says what the refusal is
- [ ] Two mutant rows in `tools/gate/mutants.toml` are caught by a panic in their test's setup, not by the test's assertion. `thermoreceptor-reads-at-the-height-site` panics in `bound` (`crates/ng-contract/src/publication.rs`, lines 403 to 405), which compiles the bench, before `the_carriages_tissue_end_the_skin_thermoreceptor_and_the_skin_exit_read_the_one_skin_row` (line 549) reaches a value. `skin-edge-reads-every-downstream-site` panics in `bench` (`crates/ng-contract/src/skin_vessels.rs`, lines 230 and 231) before `the_skin_edge_scales_its_resistance_by_the_exponential_of_the_field_and_every_other_edge_is_unmoved` (line 382) reaches its assertions. Two more reach their assertion with the body refused as `OrphanPool`, which does not show which row the mutant bound: `missing-store-reads-the-core-row`, under `a_store_read_whose_store_is_missing_or_no_tissue_heat_store_is_refused` (`publication.rs`, lines 1053 to 1065), and `store-temperature-read-body-first`, under `the_core_read_finds_only_the_bodys_row_and_a_store_read_only_its_stores_row` (lines 951 to 962). Each of the four is caught by an assertion that shows the row its mutant changes, or carries its reason
- [ ] The skin edge's test compares every other edge's slope in pressure between two benches, one that reads the skin's field and one that does not (`skin_vessels.rs`, lines 413 to 437), so a change that moves both benches' slopes alike passes it. A slope is pinned to a value from outside the two benches, or the test carries its reason
- [ ] `a_plain_core_temperature_read_inlined_at_a_muscle_bed_is_refused` (`publication.rs`, lines 685 to 694) accepts `UnwiredRead` under either of two law names, `CV.explicit core` or `CV.muscle temperature`. It names the one law the refusal is due under, or says why either stands
- [ ] The carried-flow fixture's case with no flow row (`Feed::NoFlowRow`, `crates/ng-contract/src/carriage.rs`, lines 93 to 96, asserted in `two_feeders_a_sinusoid_and_an_edge_with_no_flow_row_refuse_the_bed_inflow`, lines 295 to 301) asserts its refusal, `NoSuchRead`, by name, and does not show which check gave it: the feeders' count and family or the flow row's own check in `Rows::inflow_row` (`compile.rs`, lines 956 to 960). The test shows which, or carries its reason
- [ ] "Payload" stands in the solver contract's section 1, in the compile-fail table's row "Read the skin temperature from the core temperature's payload" (`docs/architecture/solver-contract.md`, line 236), against the glossary's _Avoid_ line for Content (`GLOSSARY.md`, line 732). The row takes the glossary's word
- [ ] For [body ticket 12](../../body/issues/12-heat-balance.md)'s PR 12b: section 7 of the solver contract has a row for a granted temperature read that finds no row exactly, with its entry in `tools/gate/must-fail.toml`. Its witnesses differ by form, and the row names the cases they exercise. `plain-core-read-body-first` changes a read of `MT.core_temperature` by name (`compile.rs`, line 2447), the case `Lookup::CoreAtASite` of `the_core_read_finds_only_the_bodys_row_and_a_store_read_only_its_stores_row`. A read of `MT.tissue_temperature` by name (line 2448) has its own test, `a_site_temperature_read_finds_its_own_row_and_never_the_bodys_of_that_name` (`publication.rs`, lines 941 to 948). `store-temperature-read-body-first` changes the typed port's read (`compile.rs`, line 2473), the case of the carriage's tissue end with the sender's row at the body only

**Notes.**

- `AGENTS.md` stands at its cap of 60 lines, so a rule that adds a row to its table of rules takes a line from elsewhere in the file.
- Ten comment blocks on the public contract are none of a doctest's sentence, a non-obvious why or a cited source: in `crates/ng-contract/src/catalog.rs` lines 129, 132, 135, 137, 653 and 1224 to 1227; in `failure.rs` lines 377 to 381; in `scope/metabolism.rs` lines 46 and 321; and in `scope.rs` lines 45 and 46. A check judged them by a description and did not read all of their text word for word. The later `/no-comments` pass covered only the comments the fix rounds added or changed, and it kept the block in `failure.rs` as the public defect's contract. None of the ten is called a fault here.

**Non-goals:** what the fix rounds of nightingale#230 closed, which [native-contract 102](102-the-skins-reads-a-design-round.md)'s "What the PR built" records.
