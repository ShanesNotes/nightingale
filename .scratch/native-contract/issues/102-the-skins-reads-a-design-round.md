# 102: The skin's reads, the contract PR

From: [body ticket 12](../../body/issues/12-heat-balance.md)'s plan, its first design question

Type: task

Status: open

Blocked by: none

Blocks: [body ticket 12](../../body/issues/12-heat-balance.md)'s PR 12b, the thermostat's commands and the skin's vessels, and not PR 12a or PR 12e

**What to build.** The contract edits of [record 0011](../../../docs/adr/0011-blood-carries-heat-into-the-store-a-bed-sits-in-and-each-card-reads-through-a-grant.md), in one PR that lands dormant. The record is the design: read it whole first. Each box below names the consequence it builds as (0011, n). Each of the record's 46 consequences is a box here or a non-goal below, and the last box, the merge proof, is the repo's rule and outside that count. Every new interface is tested in a synthetic assembly of the contract's own, and every value in a fixture is obviously synthetic. No production law card, anatomy or packet changes.

**Why.** PR 12b must carry heat to the skin at the skin bed's own flow, sense the skin's temperature at the thermoreceptor, narrow and widen the skin arteriole, and make sweat. NS releases acetylcholine at the skin junction, which supplies the skin bed. MT computes the sweat rate at the skin bed. The water and salts leave the skin bed's interstitium, and the heat leaves the skin heat site's store, the store the skin thermoreceptor reads. On main none of these reads compiles. This PR builds the contract's part of each. PR 12b then reparents the skin bed, adds the skin junction's acetylcholine pools with their release, reuptake and receptor bindings, binds the capacities, and moves the production CV, NS and MT laws onto the grants (0011, 41 to 44).

**What the code does today, at main.** The contract crate at main is the crate the design was made on. PR 12a, body 12's first PR, is not in main: its MT heat code (`crates/ng-metabolism/src/heat.rs` in that PR) and its shared skin capacity are named by the PR, not by line.

- Carriage pairs the core store with the heat of the bed's parent: the `HeatRoute::CarriedByBlood` arm of `reaches_of` in `crates/ng-contract/src/compile.rs`. In `body_anatomy` (`crates/ng-validate/src/body/anatomy.rs`) the body contains both the skin bed and the skin heat site, so the skin bed has no carriage route. `Reaches::of` in `compile.rs` builds routes only for the families the fragments request.
- `heat_carried` in `crates/ng-contract/src/scope.rs` returns a raw `SiteRoute<'b, Cardiovascular, CapillaryBed, Energy>`. Its one user is `blood_carries_heat_from_the_core_store_to_the_store_a_bed_sits_in_at_that_bed` in `crates/ng-contract/tests/routes.rs`.
- `inflow` in `crates/ng-contract/src/law.rs` is the urinary edge's, and `edge_flow` is an edge kernel's. A site kernel has no flow read.
- `FragmentBuilder::emit` in `crates/ng-contract/src/law.rs` owes a definition for every field it issues, and `FragmentBuilder::receive` declares no row. MT emits `OxygenDebtPerBed` in `crates/ng-metabolism/src/burn.rs`. No production card emits `CoreTemperature`. The fixtures `a_loop_of_two_couplings_is_one_block_of_coupled_residuals` in `crates/ng-contract/tests/coupling.rs` and `coupled_loop` in `crates/ng-validate/tests/slice.rs` emit and receive it.
- In `crates/ng-contract/src/catalog.rs`, arrow 90 wires `OxygenDebtPerBed { oxygen_debt_per_bed }` for ticket 9 and holds `temperature` with CV as holder, "CV's laws run at a reference temperature". Arrow 91 wires `CoreTemperature { core_temperature }` for ticket 12. Arrow 78 wires `myocardial_work` for ticket 9 and holds `flow_per_bed` with CV as holder. The const checks hold the whole body at 14 live, 6 relations and 18 wholly held, the fields at 16 wired, 33 held and 7 mixed, and the slice at 27 mapped and 11 unmapped, with 90 and 91 among its inputs.
- `Rows::slot` in `crates/ng-contract/src/compile.rs` looks a row up by owner, local name and context, and tries the body's row of that owner and name before the context's.
- `Kernel::read_at` in `crates/ng-contract/src/law.rs` records the expected dimension of each `Source::Field` read, and `reachable_placed` keeps it for a discarded read.
- `check_wired` in `crates/ng-contract/src/compile.rs` passes every field whose owner is not blood chemistry.
- `explicit_laws` in `crates/ng-contract/src/compile.rs` refuses a site parameter, a flow, a drive, a route pool or a reacting amount in an explicit field with `NoSuchRead`, and lets it read a field with a row at a site.
- `at_junction_if_supplied` in `crates/ng-contract/src/law.rs` exists only on the CV kernel. CV's `effect` (`crates/ng-contract/src/scope.rs`) and MS's (`crates/ng-contract/src/scope/musculoskeletal.rs`) read at a junction, and renal's in the blood. MT has none.
- `at_landmark` in `crates/ng-contract/src/law.rs` reads a field at the landmark's site, and the map puts `Landmark::SkinThermoreceptor` on the skin heat site (`body_anatomy`).
- `downstream_if_in` in `crates/ng-contract/src/law.rs` carries a `compile_fail,E0277` doctest of a blood end reading `PerSite<Beds<Muscle>, Dimensionless>`, and `tools/gate/must-fail.toml` quotes it.
- `plain_edges` in `crates/ng-cardiovascular/src/circulation.rs` reads the alpha effect once and builds its arrow 90 debt read only under its `whole_body` flag.
- The run identity seals the model's digest (`crates/ng-contract/src/scheme.rs`), and `PatientRecord::admit` in `crates/ng-record/src/record.rs` seals the run identity's digest.
- `tools/bench/run.py`'s cost line names rows, unknowns, columns and Jacobian nonzeros, and no blocks. The whole body's census pin prints blocks and Jacobian nonzeros.
- MT's three sweat routes are holds, `mt.sweat-water`, `mt.sweat-sodium` and `mt.sweat-chloride`, in `crates/ng-validate/src/body/holds/mt.rs`. `boundaries` in `crates/ng-validate/src/body/schema.rs` puts the sweat exits on the skin bed's interstitium and the heat exits on the skin heat site's thermal space. `body_schema` there gives acetylcholine pools only to the sinoatrial and respiratory pump junctions.
- In PR 12a the evaporation residual and the dry heat loss kernel read their exit's energy amount and compute the skin's temperature in place with the shared capacity `Stores::shell`, body key `mt.skin-capacity`.

**The first test the PR writes:**

- [ ] The publication fixture, written before the code that publishes, so it fails first, and passing when the PR ends. Two peripheral tissue heat stores with unequal capacities. Each store's heat and each store's capacity is varied alone, including a case that holds the skin's heat and moves only the airway store's capacity. Each store's temperature has slope 1/C in its own heat and zero in the other store's. CV's read at the carriage's tissue end, NS's read at the skin thermoreceptor and MT's read at its skin exit lower to the same skin row. The landmark's height site holds another temperature, and one case moves the landmark there: the read follows the landmark's sampled site, never its height site, and a sampled site that is no tissue heat store serving a skin bed refuses (0011, 16). An off-skin or missing place refuses, and an ungranted field refuses even in a discarded read or a guard. (0011, 40)

**The relation and the flow:**

- [ ] Carriage at a bed runs from the core store to the nearest tissue heat store above the bed, from the bed's parent toward the root, with a carriage route only where that store is not the core store. (0011, 1)
- [ ] A fixture shows the five cases: two beds in one store, a site with no heat pool between, a store inside a store, a bed under a bed, and a bed whose nearest store is the core store, with no route. A store with no bed, such as an airway store, gets conduction and a temperature and no carriage. (0011, 2)
- [ ] Each bed's own heat still goes from its bed heat to the core store, and `heat_leaves_each_bed_for_the_core_store_and_conducts_from_it_to_each_other_tissue_site` in `crates/ng-contract/src/compile.rs` passes unchanged. (0011, 3)
- [ ] Each bed's store is found at compile from the schema's pools and the containment tree, with nothing in accepted state, and a read of a placed store whose root or destination is missing refuses. (0011, 4)
- [ ] `bed_inflow` on a CV `CapillaryBed` kernel binds the one incoming vascular edge's flow row or a valve's flow memory, matched over the whole network by destination and classified as `edge_flow` does. A circulation edge and a valve each bind in a fixture, and zero feeders, two feeders, a hepatic sinusoid and an edge with no flow row each refuse. (0011, 5)
- [ ] The inflow is signed: a fixture with the outgoing flow held gives a negative, a zero and a positive inflow, and the paired column debits and credits one signed extent. (0011, 6)
- [ ] A fixture's carriage law posts `c_blood × volume flow × (T_core − T_tissue)`, converting the inflow with `volume_flow` and reading the two temperatures as arrow 90's two coordinates, and handing the water flow where a volume flow is due does not compile. (0011, 7)
- [ ] `heat_carried` returns `BloodCarriage` with `post` alone, delegating to `transfer_at`. A dropped token still owes its law. The routes fixture above moves to `post`, and no amount read is reachable from the carriage. Every other site route keeps its amount reads, and their tests pass unchanged. (0011, 8)

**The temperatures:**

- [ ] `MetabolismPorts::temperatures` declares the core's `core_temperature` through the existing `CoreTemperature` coupling and `tissue_temperature` at every tissue heat store but the core, and MT's law defines each as `T_0 + H / C` in the store's own kernel. A fixture that asks for the temperatures and no transfer law gets a row at each such store. MT's core heat grant lands at the root, and its store heat grant refuses anything but a thermal pool of a tissue site. (0011, 9)
- [ ] A fixture's MT temperature law takes one positive body capacity parameter for the core store and one positive site-keyed capacity parameter for the other stores, and binds two unequal ones. `temperatures` takes no capacity: MT's law reads it where it defines the rows. (0011, 10)
- [ ] `OxygenDebtPerBed` and `CoreTemperature` keep their fields, a scalar-only emitter and the ordinary debt emitter each compile alone, and the debt emitter owes no temperature. (0011, 11)
- [ ] Two fixtures: a fragment that calls `temperatures` once, declaring the core's and the other stores' temperatures once each, compiles, and a fragment that declares one of them a second time, such as emitting `CoreTemperature` beside `temperatures`, refuses as `NamedTwice`. (0011, 12)
- [ ] Arrow 90's `temperature` is wired for ticket 12 and read by CV as the two ends of a bed's carriage route. A CV read at a bed with no non-core route, and at a vessel, refuses. `oxygen_debt_per_bed` stays wired for ticket 9. (0011, 14)
- [ ] Arrow 91 gains `skin_temperature` for ticket 12, read only by NS at `SkinThermoreceptor` in a body kernel, with no skin row of its own. `core_temperature` keeps its type, name and ticket. (0011, 15)
- [ ] The landmark's sampled site must be a tissue heat store serving a skin bed, and a fixture whose landmark samples a tissue heat store that serves no skin bed, such as an airway store with its own temperature row, refuses. (0011, 16)
- [ ] Arrow 78 keeps `myocardial_work` wired for ticket 9 and `flow_per_bed` held by CV, and the hold's reason reads "MT's oxygen supply arrives by plasma transport. CV reads its own inflow for carriage." (0011, 17)
- [ ] S10 gains MT's reader of RC's acetylcholine effect at the junction supplying the skin's beds, beside the substrate readers and outside the 38 arrows. Every other arrow's entry stays. (0011, 18)
- [ ] The census's const checks and `crates/ng-contract/tests/census.rs` read 38 arrows, 14 live, 6 relations and 18 wholly held, 18 wired fields, 32 held and 6 mixed (78, 79, 87, 89, 95, 114), and the slice at 27 mapped and 11 unmapped with 90 and 91 still among its inputs. The checks that pin each field by line and name move only for arrow 90's `temperature` and arrow 91's `skin_temperature`, each wired for ticket 12, and keep every other holder and ticket. (0011, 19)
- [ ] The grants and their allowed places are generated from the same arrow 90 and 91 declarations as the census rows, with no second table. (0011, 20)
- [ ] MT reads its own temperature at a bed, the core store's where no other store contains the bed, or at one of its heat exits' inside thermal end. A fixture's evaporation residual reads it at a `SkinEvaporativeHeat` exit, and a gas-space exit given as a tissue heat exit refuses. (0011, 21)
- [ ] The skin reference is one typed body key with one descriptor. A fixture's three deviations, an NS skin command on its thermoreceptor grant, a CV skin factor on its carriage temperatures and an MT sweat rate on its sweat grant, read it and coalesce to one descriptor and one parameter slot with one binding. No grant gives a card other than NS the core's set point. (0011, 22)

**The sweat and the skin arteriole, in fixtures:**

- [ ] `MetabolismPorts::sweat_effect` gives only the acetylcholine effect RC issues, sampled only in an MT kernel on the skin's beds through the strict `at_junction`. `NoSupply`, `UnsuppliedRead` and `NoReceptor` each refuse in a fixture, and a hand-built port asking for the lenient junction read refuses. (0011, 23)
- [ ] A fixture bridges a sweat site kernel at the one skin bed, reading the sweat grant and the bed's store temperature, to a body row with `only_at`, and a non-draining boundary kernel reads the row. (0011, 24)
- [ ] The temperature bundle is narrowed: the grant a fixture's sweat or loss law receives holds no raw heat port and no defining handle, with a `compile_fail` doctest of a loss law reaching for one, its code settled by `gate.py codes`. Doctests 10, 11 and 24 below move onto the narrowed items. (0011, 25)
- [ ] `Blood: Includes<Beds<T>>` is stated. A fixture's plain edge reads a CV field on the skin's beds through `downstream_if_in`, a field whose kernel reads the skin's temperature against the fixed skin reference and the acetylcholine effect at the supplying junction. The edge reads the alpha effect once, scales its resistance by the exponential of the field and takes flow from the pressure head over the result. The skin edge gains the factor and its slope in the skin's log ratio, and every other edge reads its value to the bit with zero slope in that ratio and its slope in pressure as before. The field read is the dimensionless log ratio, never a temperature. (0011, 26)
- [ ] The skin's read and its operation count cover the plain edges and leave out the hepatic card's sinusoids. (0011, 27)
- [ ] The `Beds<Muscle>` doctest on `downstream_if_in` becomes the enclosure doctest, the tubule doctest stays, a positive skin-bed doctest is added, a test of the skin field's read reaches `EndOffSet`, and `tools/gate/must-fail.toml`'s quoted row changes with it. (0011, 28)

**Compile, the model hash and types:**

- [ ] Each new read looks up its row by owner, local name and context. A fixture in which MT defines `tissue_temperature` only as a body scalar and a grant reads it at a site refuses, where `Rows::slot` today binds the scalar. A bound inflow checks hydraulic family, row kind, dimension, edge and owner. (0011, 29)
- [ ] Each new grant is tested with its definition and without one, and without one refuses `UnemittedField`. A sender that defines the row in another dimension refuses `FieldDimension`, for a kept read and for a discarded one. A reader or place the catalog does not wire refuses `UnwiredRead`. (0011, 30)
- [ ] `check_wired` checks the reader, the field and the place before inlining or lowering loses who reads. In the contract's own tests it refuses two reads by a card the catalog does not wire: a forged projected read, and a plain `Source::Field` read that spells a protected field's owner and local name. It covers kept and discarded reads, guards, each site-term variant, the structural right where no site is realized, and each realized instance, and a site with no allowed projection refuses. (0011, 31)
- [ ] An explicit field with no row that reads the bed's inflow, a store's heat or a store's temperature through a new grant refuses with `NoSuchRead`, one fixture each. (0011, 32)
- [ ] Appended tags for `BedInflow`, the store-heat and store-temperature sources, the projection ends and each exit's identity. Tests show that changing an end, an exit or a place changes the model hash, and the dormant box's pins show a body using none keeps its bytes. No runtime cache, crate or `tools/gate/paths.toml` exception is added. (0011, 33)
- [ ] Each type rule has a `compile_fail` doctest with its code beside its legal call, and `python3 -I tools/gate/gate.py codes` passes. The design's sketch has 24, listed below with the code each declares, and the PR has all 24. (0011, 34)
- [ ] §7's new refusals have `tools/gate/must-fail.toml` entries, and `tools/gate/mutants.toml` has the eight rows the record lists, each replayed. (0011, 35)
- [ ] The sketch's catalog and `docs/architecture/solver-contract-sketch/evidence/census-check.py` move with the crate's: its parser, field fixtures, `BODY_COUNTS`, `WIRED_BY`, `HELD_BY`, the mixed list and the projected fields' places. (0011, 36)
- [ ] The documents, each sentence as listed below: `docs/architecture/solver-contract.md` §§1, 2 and 7, a table of each reader and the place it reads after the direct-couplings graph in `docs/atlas/coupling-map.md` with lines 77 to 114 unmoved, [body ticket 01](../../body/issues/01-rung-0-specification.md)'s field and hold reasons for arrows 78, 90 and 91, and `GLOSSARY.md`'s entries. (0011, 37)

The 24 `compile_fail` doctests, one line each: what it tries, and its code.

1. MT receives CV's carriage temperatures: E0271.
2. MT emits the carriage temperatures, a read-only projection: E0271.
3. CV reads a carriage temperature in a vessel segment's kernel: E0277.
4. NS samples its skin thermoreceptor grant in a junction kernel: E0308.
5. `bed_inflow` on an MT `CapillaryBed` kernel: E0599.
6. `bed_inflow` taken as a volume flow without `volume_flow`: E0308.
7. A raw amount asked of `BloodCarriage`: E0599.
8. `BloodCarriage::post` given a vascular edge kernel: E0308.
9. NS calls `temperatures`, declaring MT's rows: E0599.
10. A store temperature defined from an energy kernel: E0308.
11. MT's `at_exit` given a water boundary route: E0308.
12. The sweat grant taken as a blood field: E0308.
13. The sweat grant sampled in an MT body kernel: E0308.
14. The sweat grant sampled in an MT `Beds<Muscle>` kernel: E0308.
15. The sweat grant read through `at_junction_if_supplied` in an MT kernel: E0599. The sketch declares E0308, and a probe found E0599.
16. A ligand chosen on the acetylcholine-only sweat grant: E0107.
17. A vascular edge reads an enclosure field through `downstream_if_in`, the enclosure doctest: E0277.
18. NS reads `skin_temperature` from the `CoreTemperature` coupling's struct: E0609.
19. CV receives NS's skin thermoreceptor grant: E0271.
20. `bed_inflow` on a CV body kernel: E0599.
21. `bed_inflow` on a CV vascular edge kernel: E0599.
22. `BloodCarriage::post` given a water rate: E0308.
23. `BloodCarriage::post` given an MT kernel: E0308.
24. MT's `at_exit` given another card's energy boundary route: E0308.

The documents' sentences:

- §1's heat-route sentence says blood carries heat at a bed, at the signed flow of its one incoming vascular edge, from the core store to the nearest tissue heat store above the bed. Its `Includes` sentence adds that blood includes each tissue's beds. Its table of compile-fail moves gains a row for each new type rule with its code.
- §2's sentence on the structs the macro emits says arrow 90's temperature is wired and no member of its arrow's struct: a projected field read through a grant at the two ends of a bed's carriage route, with no row of its own, reading its sender's rows. Arrow 91's skin temperature is the same at the skin thermoreceptor. Its census sentence reads six mixed arrows (78, 79, 87, 89, 95 and 114), 18 fields wired and 32 held. Its S6 entry names blood's carriage at a bed, posted by CV between the core store and the bed's store. Its S10 entry reads CV, RN and MT laws, MT reading the acetylcholine effect at the junction supplying the skin's beds.
- §7's must-fail table gains a row for each new refusal, naming what rejects it.
- The glossary's Heat route, Core store, Landmark and Supply say the nearest containing store, the signed incoming flow and the sampled site. Tissue heat stores says each store's own capacity, if PR 12a has added the entry, and the PR adds it if not. Coupling field says a projected field has no row of its own and is read through a grant from its sender's rows.

**Dormant, and merged alone:**

- [ ] Dormant. Production anatomy, the packet and every law card stay at the parent revision, so these pins hold unchanged: in `crates/ng-validate/tests/trace.rs`, `the_slices_model_hash_and_conserved_basis_stay_where_they_were` and the three `the_first_second_of_the_slice_…` tests; in `crates/ng-validate/tests/body.rs`, `the_whole_body_holds_its_rows_columns_and_digests`; in `crates/ng-validate/tests/body_run.rs`, `the_whole_body_admits_runs_its_first_minute_and_holds_its_pins` (the fitted packet, the admitted seal and the body's first second) and `the_whole_body_passes_every_replay_gate`; and PR 12a's placement test if it is in the parent. If PR 12a or 12e lands first, the parent is that main. The census moves as the boxes above say. (0011, 39)
- [ ] Every new interface is tested in a synthetic assembly of its own, each grant with and without its definition, and no production caller uses one. (0011, 38)
- [ ] The repo's merge rule, outside the count of consequences: the PR is merged alone, on the `verify-nightingale` proof with `full_proof: yes`, the compile-fail code check, the `/no-comments` report and no open review thread.

**Non-goals:**

- (0011, 13) Folding MT's two temperature declarations into one, or giving NS one handle on arrow 91. It is looked at again when a third card needs a store's temperature.
- (0011, 41) Reparenting the skin bed in `body_anatomy`. It is PR 12b's first commit, alone, with the pins it moves: the whole body's model hash, and through the run identity the admitted seal and every pin that seals it.
- (0011, 42) PR 12b removes PR 12a's shared skin capacity, `mt.skin-capacity` and the duplicated temperature expressions, and binds the skin and airway capacities apart, the airway's on its own labeled synthetic value until a reviewed one or an admissible inference exists, with its start recomputed from it.
- (0011, 43) PR 12b replaces the three `mt.sweat-*` holds with laws and their bindings, and asserts that the three sweat exits draw on the skin bed's interstitium and that the heat exits and the skin thermoreceptor agree with the skin bed's store.
- (0011, 44) The production callers, PR 12b's: the three MT temperatures published, NS's commands and release shares, the skin junction's acetylcholine vesicle, cleft and bound pools with their release, reuptake, receptor bindings and starts, CV's carriage and plain-edge law under its whole-body flag, and MT's sweat and loss laws on the grants.
- (0011, 45) The cost run, PR 12b's, with blocks and Jacobian nonzeros from the whole body's census pin, and its predictions with PR 12a as parent.
- (0011, 46) PRs 12c, 12d and 12e.
- The record's limits: two feeds into one heated bed, a bed with two stores, more than one skin bed, separate dilator and sudomotor transmitters.
- An amount read at a landmark. The landmark arm of `Source::Amount` in compile stays as it is.
- Every law, evaporation's form and the thermal zero, ruled in body ticket 12.

**What the design's tests show, by log SHA-256.** All on an earlier revision of PR 12a's tree with synthetic values: the reparenting (`9866fc3043cd…`), the signed inflow and a second feeder (`0df0df749f36…`), the skin's ratio, the nearest-store rule and `NamedTwice` (`2b35bf55c961…`), the dormant pieces against the body's, the slice's and 12a's pins (`0e966e7bfd01…`), the doctests' codes, the second on placeholder items (`b1b4cfc348d5…`, `5f3ed4c1fa0e…`), and the other design's two readers on one row (`1dbfeca4e94f…`). Record 0011 states each result and what each leaves unshown.
