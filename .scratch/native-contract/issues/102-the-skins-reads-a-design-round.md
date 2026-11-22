# 102: The skin's reads, the contract PR

From: [body ticket 12](../../body/issues/12-heat-balance.md)'s plan, its first design question

Type: task

Status: open

Blocked by: none

Blocks: [body ticket 12](../../body/issues/12-heat-balance.md)'s PR 12b, the thermostat's commands and the skin's vessels, and not PR 12a or PR 12e

**What to build.** The contract edits of [record 0011](../../../docs/adr/0011-blood-carries-heat-into-the-store-a-bed-sits-in-and-each-reader-holds-a-grant.md), in one PR with synthetic contract fixtures, landing dormant. The record is the design: read it whole first. Each box below names the consequence it builds as (0011, n), and each of the record's 43 consequences is a box here or a non-goal below. Every value in a fixture is obviously synthetic.

**Why.** PR 12b must carry heat to the skin at the skin bed's own flow, sense the skin's temperature at the thermoreceptor, narrow and widen the skin arteriole, and make sweat at the skin junction. On main none of these reads compiles. A design round settled how, and its record states each consequence. This PR builds the contract's part of each, so 12b only wires the map, the packet and the laws.

**What the code does today, at main.** The contract crate at main is the crate the design was made on. PR 12a, body 12's first PR, is not in main: its MT heat code (`crates/ng-metabolism/src/heat.rs` in that PR) and its shared skin capacity are named by the PR, not by line.

- Carriage pairs the core store with the heat of the bed's parent: the `HeatRoute::CarriedByBlood` arm of `reaches_of` in `crates/ng-contract/src/compile.rs`. In `body_anatomy` (`crates/ng-validate/src/body/anatomy.rs`) the body contains both the skin bed and the skin heat site, so the skin bed has no carriage reach.
- `heat_carried` in `crates/ng-contract/src/scope.rs` returns a raw `SiteRoute<'b, Cardiovascular, CapillaryBed, Energy>`. Its one user is `blood_carries_heat_from_the_core_store_to_the_store_a_bed_sits_in_at_that_bed` in `crates/ng-contract/tests/routes.rs`.
- `inflow` in `crates/ng-contract/src/law.rs` is the urinary edge's, and `edge_flow` is an edge kernel's. A site kernel has no flow read.
- `FragmentBuilder::emit` in `crates/ng-contract/src/law.rs` owes a definition for every field it issues, and `FragmentBuilder::receive` declares no row. MT emits `OxygenDebtPerBed` in `crates/ng-metabolism/src/burn.rs`. No production card emits `CoreTemperature`. The fixtures `a_loop_of_two_couplings_is_one_block_of_coupled_residuals` in `crates/ng-contract/tests/coupling.rs` and `coupled_loop` in `crates/ng-validate/tests/slice.rs` emit and receive it.
- In `crates/ng-contract/src/catalog.rs`, arrow 90 wires `OxygenDebtPerBed { oxygen_debt_per_bed }` for ticket 9 and holds `temperature` with CV as holder, "CV's laws run at a reference temperature". Arrow 91 wires `CoreTemperature { core_temperature }` for ticket 12. Arrow 78 holds `flow_per_bed` with CV as holder. The const checks hold the whole body at 14 live, 6 relations and 18 wholly held, the fields at 16 wired, 33 held and 7 mixed, and the slice at 27 mapped and 11 unmapped.
- `Rows::slot` in `crates/ng-contract/src/compile.rs` tries the body's row of a name before the context's.
- `at_junction_if_supplied` in `crates/ng-contract/src/law.rs` exists only on the CV kernel. CV's `effect` (`crates/ng-contract/src/scope.rs`) and MS's (`crates/ng-contract/src/scope/musculoskeletal.rs`) read at a junction, and renal's in the blood. MT has none.
- `at_landmark` in `crates/ng-contract/src/law.rs` reads a field at the landed site, and the map lands `Landmark::SkinThermoreceptor` on the skin heat site (`body_anatomy`).
- `downstream_if_in` in `crates/ng-contract/src/law.rs` carries a `compile_fail,E0277` doctest of a blood end reading `PerSite<Beds<Muscle>, Dimensionless>`, and `tools/gate/must-fail.toml` quotes it.
- The run identity seals the model's digest (`crates/ng-contract/src/scheme.rs`), and `PatientRecord::admit` in `crates/ng-record/src/record.rs` seals the run identity's digest.
- `tools/bench/run.py`'s cost line names rows, unknowns, columns and Jacobian nonzeros, and no blocks. The whole body's pin prints blocks.
- MT's three sweat routes are holds, `mt.sweat-water`, `mt.sweat-sodium` and `mt.sweat-chloride`, in `crates/ng-validate/src/body/holds/mt.rs`.

**First, before anything else in the PR:**

- [ ] The publication fixture is built and passing. Two peripheral tissue heat stores with unequal capacities, and heats varied apart. Each store's temperature has slope 1/C in its own heat and zero in the other store's. CV's read at the carriage's tissue end, NS's read at the skin thermoreceptor and MT's read at its skin exit lower to the same skin row. The landmark's height site holds another temperature, and one case moves the landmark there. An off-skin or missing landing refuses, and an ungranted field refuses even in a discarded read or a guard. (0011, publication, and 9, 10, 15, 16, 21, 30)

**The relation and the flow:**

- [ ] Carriage at a bed runs from the core store to the nearest tissue heat store above the bed, from the bed's parent toward the root, with a reach only where that store is not the core store. (0011, 1)
- [ ] A fixture shows the five cases: two beds in one store, a site with no heat pool between, a store inside a store, a bed under a bed, and the core's territory with no reach. A store with no bed gets conduction and a temperature with no carriage. (0011, 2)
- [ ] Each bed's own heat still goes from its bed heat to the core store, and `heat_leaves_each_bed_for_the_core_store_and_conducts_from_it_to_each_other_tissue_site` in `crates/ng-contract/src/compile.rs` passes unchanged. (0011, 3)
- [ ] The index of stores is built at compile from the schema's pools and the containment tree, with nothing in accepted state, and a read of a placed store whose root or destination is missing refuses. (0011, 4)
- [ ] `bed_inflow` on a CV `CapillaryBed` kernel binds the one incoming vascular edge's flow row or a valve's flow memory, matched over the whole network by destination and classified as `edge_flow` does. Zero feeders, two feeders, a hepatic sinusoid and an edge with no flow row each refuse in a fixture. (0011, 5)
- [ ] The inflow is signed: a fixture with the outgoing flow held gives a negative, a zero and a positive inflow, and the paired column debits and credits one signed extent. (0011, 6)
- [ ] A fixture's carriage converts the inflow with `volume_flow`, and handing the water flow where a volume flow is due does not compile. (0011, 7)
- [ ] `heat_carried` returns `BloodCarriage` with `post` alone, delegating to `transfer_at`. A dropped token still owes its law. The routes fixture above moves to `post`, and no amount read is reachable from the carriage. (0011, 8)

**The temperatures:**

- [ ] `MetabolismPorts::temperatures` declares the core's `core_temperature` through the existing `CoreTemperature` coupling and `tissue_temperature` at each non-core store over conduction's sites, each defined as `T_0 + H / C` in the store's own kernel. (0011, 9)
- [ ] The contract takes one positive body capacity for the core store and one positive site-keyed capacity for each other store, and a fixture binds two unequal ones. (0011, 10)
- [ ] `OxygenDebtPerBed` and `CoreTemperature` keep their fields, a scalar-only emitter and the ordinary debt emitter each compile alone, and the debt emitter owes no temperature. (0011, 11)
- [ ] Both temperature declarations in one fragment refuse as `NamedTwice`. (0011, 12)
- [ ] Arrow 90's `temperature` is wired for ticket 12 and read by CV as the two ends of a bed's carriage reach, only at a bed with a non-core reach. `oxygen_debt_per_bed` stays wired for ticket 9. (0011, 14)
- [ ] Arrow 91 gains `skin_temperature` for ticket 12, read only by NS at `SkinThermoreceptor` in a body kernel, with no skin row of its own. `core_temperature` keeps its type, name and ticket. (0011, 15)
- [ ] The landmark's sampled site must be a tissue heat store serving a skin bed, and a fixture whose landmark sits elsewhere refuses. (0011, 16)
- [ ] Arrow 78's `flow_per_bed` reason reads "MT's oxygen supply arrives by plasma transport. CV reads its own inflow for carriage." (0011, 17)
- [ ] S10 gains MT's reader of RC's acetylcholine effect at the junction supplying the skin's beds, beside the substrate readers and outside the 38 arrows. Every other arrow's entry stays. (0011, 18)
- [ ] The census's const checks and `crates/ng-contract/tests/census.rs` read 38 arrows, 14 live, 6 relations and 18 wholly held, 18 wired fields, 32 held and 6 mixed (78, 79, 87, 89, 95, 114), and the slice at 27 mapped and 11 unmapped. (0011, 19)
- [ ] The grants and their allowed places are generated from the same arrow 90 and 91 declarations as the census rows, with no second table. (0011, 20)
- [ ] MT reads its own temperature at a bed, the core store's in the core's territory, or at one of its heat exits' inside thermal end, and a gas-space exit given as a tissue heat exit refuses. (0011, 21)
- [ ] The skin reference is one typed body key with one shared descriptor, and two deviations reading it coalesce to one parameter slot. (0011, 22)

**The sweat and the skin arteriole:**

- [ ] `MetabolismPorts::sweat_effect` gives only the acetylcholine effect RC issues, sampled only in an MT kernel on the skin's beds through the strict `at_junction`. `NoSupply`, `UnsuppliedRead` and `NoReceptor` each refuse in a fixture, and a hand-built port asking for the lenient landing refuses. (0011, 23)
- [ ] A fixture bridges a sweat site kernel at the one skin bed to a body row with `only_at`, and a non-draining boundary kernel reads the row. (0011, 24)
- [ ] MT's sweat and loss laws receive only the grants they use, and no raw heat port or output handle reaches them. (0011, 25)
- [ ] `Blood: Includes<Beds<T>>` is stated. A fixture's plain edge reads a CV field on the skin's beds through `downstream_if_in`: the skin edge gains the factor and its slope, and every other edge reads its value to the bit with zero slope. (0011, 26)
- [ ] The skin's read and its operation count cover the plain edges and leave out the hepatic card's sinusoids. (0011, 27)
- [ ] The `Beds<Muscle>` witness on `downstream_if_in` becomes an enclosure witness, the tubule witness stays, a positive skin-bed doctest is added, and `tools/gate/must-fail.toml`'s quoted row changes with it. (0011, 28)

**Compile, seals and types:**

- [ ] Each new read looks up its row by owner, local name and context, and a missing site row refuses where a body scalar of that name exists, across owners. A bound inflow checks hydraulic family, row kind, dimension, edge and owner. (0011, 29)
- [ ] Each field read, kept or discarded, records its dimension, and `UnemittedField`, `FieldDimension` and `UnwiredRead` each refuse in a fixture. (0011, 30)
- [ ] `check_wired` covers the projected fields and the S10 grant on kept and discarded reads, guards, each site-term variant, unrealized sites and each realized instance, and a forged read in the contract's tests refuses. (0011, 31)
- [ ] An explicit field with no row still refuses a place-dependent read. (0011, 32)
- [ ] Appended seal tags for `BedInflow`, the store-heat and store-temperature sources, the projection ends and each exit's identity. Tests show that changing an end, an exit or a landing changes the seal. (0011, 33)
- [ ] Each type rule has a `compile_fail` doctest with its code beside its legal call, and `tools/gate/gate.py codes` passes. The optional junction on the sweat grant declares E0599. (0011, 34)
- [ ] §7's new refusals have `tools/gate/must-fail.toml` entries, and `tools/gate/mutants.toml` has the eight rows the record lists, each replayed. (0011, 35)
- [ ] The sketch's catalog and `docs/architecture/solver-contract-sketch/evidence/census-check.py` move with the crate's: its parser, field fixtures, `BODY_COUNTS`, `WIRED_BY`, `HELD_BY`, the mixed list and the projected landings. (0011, 36)
- [ ] `docs/architecture/solver-contract.md` §§1, 2 and 7, a reader and landing table after the direct-couplings graph in `docs/atlas/coupling-map.md` with lines 77 to 114 unmoved, [body ticket 01](../../body/issues/01-rung-0-specification.md)'s field and hold reasons, and `GLOSSARY.md`. The glossary gains **read grant**, **bed inflow** and **blood carriage**, and **tissue heat stores** if PR 12a has not added it. **Heat route**, **Core store**, **Landmark** and **Supply** say the nearest containing store, the signed incoming flow and the sampled site. The log ratio is a CV field, and the glossary keeps **the skin arteriole's resistance**. (0011, 37)

**Landing:**

- [ ] Dormant. Production anatomy, the packet and every law card stay at the parent revision, so these pins hold unchanged: in `crates/ng-validate/tests/trace.rs`, `the_slices_model_hash_and_conserved_basis_stay_where_they_were` and the three `the_first_second_of_the_slice_…` tests; in `crates/ng-validate/tests/body.rs`, `the_whole_body_holds_its_rows_columns_and_digests`; in `crates/ng-validate/tests/body_run.rs`, `the_whole_body_admits_runs_its_first_minute_and_holds_its_pins` (the fitted packet, the admitted seal and the body's first second) and `the_whole_body_passes_every_replay_gate`; and PR 12a's placement test if it is in the parent. If PR 12a or 12e lands first, the parent is that main. (0011, 38)
- [ ] The PR is merged alone, on the `verify-nightingale` proof with `full_proof: yes`, the compile-fail code check and the `/no-comments` report.

**Non-goals:**

- (0011, 13) Folding MT's two temperature declarations into one, or giving NS one handle on arrow 91. It is looked at again when a third card needs a store's temperature.
- (0011, 39) Reparenting the skin bed in `body_anatomy`. It is PR 12b's first commit, alone, with the pins it moves: the whole body's model hash, and through the run identity the admitted seal and every pin that seals it.
- (0011, 40) PR 12b removes PR 12a's shared skin capacity, `mt.skin-capacity` and the duplicated temperature expressions, and binds the skin and airway capacities apart, the airway's on its own labeled synthetic value with its start recomputed.
- (0011, 41) PR 12b replaces the three `mt.sweat-*` holds with laws, and asserts the sweat exits and the heat exits against the skin bed's store.
- (0011, 42) The cost run, PR 12b's, with blocks from the whole body's pin.
- (0011, 43) PRs 12c, 12d and 12e.
- The record's limits: two feeds into one heated bed, a bed with two stores, more than one skin bed, separate dilator and sudomotor transmitters.
- An amount read at a landmark. The landmark arm of `Source::Amount` in compile stays as it is.
- Every law, evaporation's form and the thermal zero, ruled in body ticket 12.

**What the round's runs show, by log SHA-256.** All on PR 12a's tree with synthetic values: the reparenting (`895386cb7f90…`), the signed inflow and a second feeder (`13c6d40b51e9…`), the skin's ratio, the nearest-store rule and `NamedTwice` (`2b35bf55c961…`), the dormant pieces against the body's, the slice's and 12a's pins (`0e966e7bfd01…`), and the witnesses' codes (`5f3ed4c1fa0e…`, `b1b4cfc348d5…`). Record 0011 states each result. Nothing in this PR is built yet.
