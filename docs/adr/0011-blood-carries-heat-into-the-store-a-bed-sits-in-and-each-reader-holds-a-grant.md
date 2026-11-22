# Blood carries heat into the store a bed sits in, and each card reads a store's temperature through a grant

Body ticket 12's thermostat must carry heat to the skin at the skin bed's own flow, sense the skin's temperature at the skin thermoreceptor, narrow and widen the skin arteriole, and make sweat where the skin junction is. On main none of these reads compiles (native-contract 102). The decision: the map puts the skin bed inside the skin heat site, so containment gives blood's carriage its reach from the core store to the skin store. CV posts that carriage at the bed from a read of the bed's one arterial inflow. MT is the one owner of each tissue heat store's temperature, `T_0 + H / C` with that store's own capacity. CV and NS read a store's temperature, and MT reads the skin junction's acetylcholine effect, only through a grant: a typed object that reaches the field and the place the reader is entitled to and nothing else, so a wrong read is a type error or a compile refusal. The contract PR lands all of it dormant, and PR 12b turns it on.

Why this design. Two designs were made apart and agreed on the relation, the carriage, the temperatures, the skin arteriole's read and the sweat row. They differed on the reads. This one gives each reader a grant. The other gave every holder of a per-site port the same reads at any place, so a wrong place or ligand compiled. Its reason for a new kind of catalog entry, that two receivers of one field make two rows, is false: a coupling field's identity is its owner and local name (`Reads` in `crates/ng-contract/src/catalog.rs`), `FragmentBuilder::receive` declares no row, and the coupling test's nervous fragment already receives `CoreTemperature` (`crates/ng-contract/tests/coupling.rs`, `a_loop_of_two_couplings_is_one_block_of_coupled_residuals`).

## Considered options

- An explicit relation from each bed to the store it gives heat to, with containment left alone. Both designs rejected it. CV would carry heat once for each incoming vascular edge of a bed assigned to a non-core store, so an edge kernel would need a way to post to a store, and the relation would be a sealed anatomical declaration and a checked row outside the 38 arrows. It buys flexibility only for a heated territory with two feeds, and this map has none. Here it computes the same one paired column.
- One temperature field stated by MT's port, with the core as the containment root's entry, arrow 91's field renamed and the census at 17 wired, 32 held and 6 mixed. Rejected: it adds a census kind and a rename for the same rows, and it rests on the false premise above.
- Place reads generic over the owner and the site kind (the core's, a pool's, a store's) and a generic junction effect for MT. Rejected: any holder of a per-site port could read it at any place, and MT could read any ligand at any junction.
- A separate PR for the map's one changed parent. Rejected: a full proof for one line. Kept as PR 12b's first commit.
- A carriage coefficient per amount of water. Rejected: research 16's CV-082 is heat per volume per kelvin, so the contract converts the flow to volume.
- A temperature row as a solver block of its own, as a cost argument. Rejected: conduction reads the temperature and feeds heat back into the amount it reads.
- An amount read at a landmark, by fixing the landmark arm of `Source::Amount` in compile. Rejected: no caller needs it, and it would also let NS sample its own release route's amount at the pacemaker. NS reads the skin's temperature as a field at the landmark, which resolves at the landed site today (`at_landmark` in `crates/ng-contract/src/law.rs`).

## Consequences

The relation and the flow:

1. Blood's carriage at a bed runs from the core store to the nearest tissue heat store above the bed: from the bed's parent toward the root, the first tissue site with a thermal energy pool. A bed has a carriage reach exactly when that store differs from the core store. Today the `HeatRoute::CarriedByBlood` arm of `reaches_of` in `crates/ng-contract/src/compile.rs` pairs the core store with the heat of the bed's parent only.
2. So a site with no heat pool between a bed and its store is passed through, two beds in one store get two reaches, a bed in the core's own territory gets none, and a store with no bed, such as the airway store, gets conduction and a temperature with no carriage.
3. Each bed's own heat still goes from its bed heat to the core store, as `HeatRoute::FromBeds` does today.
4. Compile builds the index of stores from the schema's declared pools and the containment tree. There is no second anatomical declaration and no index in accepted state. A read of a placed store whose root or destination is missing refuses.
5. `bed_inflow` on a CV `CapillaryBed` kernel reads the signed flow of the bed's one incoming vascular edge. It matches the whole vascular network by destination, classifies the edge as `edge_flow` does, and binds the circulation edge's algebraic row or a valve's flow memory. It refuses zero or several feeders, a sinusoid the hepatic card owns, and an edge with no flow row. Today a flow read outside an edge context is `NoSuchRead`, and `inflow` is the urinary edge's (`crates/ng-contract/src/law.rs`).
6. The flow is signed in the network's direction, and the paired column debits and credits one signed extent. Any choice by direction belongs to CV's law.
7. The carriage law converts the water flow with `volume_flow`, the schema's partial-volume coefficient, so its coefficient is heat per volume per kelvin.
8. `heat_carried` returns a `BloodCarriage`, whose one method `post` takes a CV `CapillaryBed` kernel of `Rate<Energy>` and posts it through `transfer_at`. It gives no amount reads. A request still owes its law when the caller drops the token. Today `heat_carried` in `crates/ng-contract/src/scope.rs` returns a raw `SiteRoute<'b, Cardiovascular, CapillaryBed, Energy>`, and its one user, `blood_carries_heat_from_the_core_store_to_the_store_a_bed_sits_in_at_that_bed` in `crates/ng-contract/tests/routes.rs`, moves to `post`. Other site routes keep their amount reads.

The temperatures:

9. MT defines each tissue heat store's temperature once, `T_0 + H / C`, in the kernel where the store's amount and capacity are. The core store's is arrow 91's scalar `core_temperature` at the body, declared through the existing `CoreTemperature` coupling. Each other store's is `tissue_temperature` at its site, over the same non-core sites as conduction, declared by MT's own declaration and owing those rows alone.
10. Each store has its own capacity: one positive body binding for the core store and one positive site-keyed binding for each other store.
11. `OxygenDebtPerBed` and `CoreTemperature` keep their fields and stay issuable apart. Today `emit` owes a definition for every field it issues (`FragmentBuilder::emit` in `crates/ng-contract/src/law.rs`), so widening the debt coupling would put a temperature on MT's debt emitter in `crates/ng-metabolism/src/burn.rs`.
12. Asking for both of MT's temperature declarations in one fragment is `NamedTwice`.
13. MT declares the core scalar and the other stores' rows apart, and NS holds two handles on arrow 91. That leaves every row, field and pin of PR 12a where it is. It is looked at again when a third card needs a store's temperature.
14. Arrow 90's held `temperature`, held today by CV because "CV's laws run at a reference temperature" (`crates/ng-contract/src/catalog.rs`), becomes wired for ticket 12. CV reads it as two coordinates, the core and the tissue end of a bed's carriage reach, and only in the context of a bed with a non-core reach. `oxygen_debt_per_bed` stays wired for ticket 9.
15. Arrow 91 keeps `core_temperature`, its type, name and ticket, and gains `skin_temperature` for ticket 12: MT's field read by NS at the landmark `SkinThermoreceptor` in a body kernel, with no skin row of its own.
16. The landmark's sampled site must be a tissue heat store serving a skin bed. The landmark's height site is not where it reads.
17. Arrow 78's held `flow_per_bed` keeps its holder, and its reason becomes "MT's oxygen supply arrives by plasma transport. CV reads its own inflow for carriage." Only the wording moves.
18. Arrow 87 and every other arrow keep their entries. Shared substrate S10 gains one reader: MT reads RC's acetylcholine effect at the junction supplying the skin's beds. It sits beside the substrate readers, outside the 38 arrows.
19. The census keeps 38 arrows, and the whole body keeps 14 live, 6 relations and 18 wholly held. Its wired fields go from 16 to 18, its held fields from 33 to 32, and its mixed arrows from 7 to 6 (78, 79, 87, 89, 95, 114). The slice keeps 27 mapped and 11 listed unmapped, with 90 and 91 as inputs from outside it. Today's figures are the const checks in `crates/ng-contract/src/catalog.rs`.
20. The read grants and the places they allow are generated from the same arrow 90 and 91 declarations as the census rows.
21. MT reads its own temperature at a bed, as the store containing the bed, which is the core store in the core's territory, or at one of its heat exits, as the store at the exit's inside thermal end. PR 12a's evaporation residual reads it at the `SkinEvaporativeHeat` exit. A gas-space exit given as a tissue heat exit refuses at compile.
22. The skin reference, research 16's NS-046, is one typed body key bound once, with one descriptor for its unit, context and admissible set, shared by every deviation that reads it. The core's set point stays NS's memory alone.

The sweat and the skin arteriole:

23. MT's sweat grant gives only the acetylcholine effect, the one RC already issues to CV, sampled only in an MT kernel on the skin's beds through the strict `at_junction`, so `NoSupply`, `UnsuppliedRead` and `NoReceptor` stand. The catalog check requires the strict landing, even for a fixture that builds the port by hand.
24. The sweat rate is one site kernel at the skin bed, put on a body row with `only_at`, and MT's boundary kernels that do not drain read that row.
25. A sweat or loss law receives only the grants it uses, never raw heat ports or output handles.
26. The contract states `Blood: Includes<Beds<T>>`, which blood's including the beds already implies. The plain-edge law then reads a CV field on the skin's beds through `downstream_if_in`: the log of the factor research 12's dilator and local-temperature terms put on the skin arteriole's resistance. Off the skin it reads zero with zero slope, so the edge's multiplier is exactly one. No absolute temperature passes through `downstream_if_in`.
27. The plain-edge law covers every plain vascular edge but the hepatic card's sinusoids, as the glossary's Sinusoids entry says, so the skin's read and its count leave them out.
28. The `downstream_if_in` doctest in `crates/ng-contract/src/law.rs` that refuses a blood end reading `Beds<Muscle>` with E0277 compiles once the inclusion is stated. It is replaced by an enclosure witness, the tubule witness stays, and a positive skin-bed test is added. `tools/gate/must-fail.toml` quotes the old witness, so its row changes with it.

Compile, seals and types:

29. Each new read finds its row by owner, local name and context exactly, and a missing site row refuses even where a body scalar of that name exists. Today `Rows::slot` in `crates/ng-contract/src/compile.rs` tries the body first. A bound inflow also checks the hydraulic family, row kind, dimension, edge and owner.
30. Each field read, kept or discarded, records the dimension it expects. A missing definition is `UnemittedField`, a wrong dimension `FieldDimension`, and an unwired reader or landing `UnwiredRead`.
31. The catalog's check (`check_wired` in `crates/ng-contract/src/compile.rs`) covers the projected fields and MT's S10 grant: kept and discarded reads, guards, each site-term variant, the structural right where no site is realized, and each realized instance. A site with no allowed projection refuses. The contract's own tests forge a read to show it.
32. The new place-dependent reads stay out of explicit fields with no row, under the refusal that holds there today.
33. Seals gain appended tags for `BedInflow`, the store-heat and store-temperature sources, each projection end and each exit's identity. Changing an end, an exit or a landing changes the seal, and a body that uses none of them keeps its bytes.
34. Each type rule has a `compile_fail` doctest with its code beside the legal call. Asking the sweat grant for an optional junction fails with E0599, not E0308, since `at_junction_if_supplied` exists only on the CV kernel. `gate.py codes` settles every code on the built items.
35. §7's new refusals map to doctests and failing fixtures in `tools/gate/must-fail.toml`, and `tools/gate/mutants.toml` gains a row for each of: the first of two feeders, the outgoing flow, stopping at a parent with no heat, landing on the height site, the wrong reader or junction, the core row for a missing store, a column's ends reversed, and a selector left out of its seal.
36. The crate's catalog, the sketch's catalog, `crates/ng-contract/tests/census.rs` and `census-check.py` move with the census: its parser, field fixtures, counts, holders, wiring tickets and mixed list, and the projected landings as well as their counts.
37. The solver contract's §§1 and 2 gain the placements, the coupled fields and S6 and S10, and §7 the refusals. The atlas gains a short reader and landing table after the direct-couplings graph, so the coupling map's lines 77 to 114 keep their identities. Body 01's field and hold reasons and the glossary move with them.

Landing:

38. The contract PR lands dormant. Production anatomy, the packet and every law card stay at their parent revision, so every model, packet, seal and trace pin holds. Only the census's counts move.
39. Reparenting alone moves the whole body's model hash, and through the run identity (`crates/ng-contract/src/scheme.rs`), the admitted seal and every pin that seals the run identity. So PR 12b's first commit is the reparenting alone with the pins it moves.
40. PR 12b removes PR 12a's shared skin capacity, its body key `mt.skin-capacity` and the duplicated temperature expressions in the loss kernels, and binds the skin and airway capacities apart.
41. PR 12b replaces the three `mt.sweat-*` holds with laws and their bindings, and asserts that the three sweat exits draw on the skin bed's interstitium and that the heat exits and the skin thermoreceptor agree with the skin bed's store.
42. PR 12b's cost run takes blocks from the whole body's pin, since `tools/bench/run.py`'s cost line prints none.
43. PR 12e needs neither arrow 90 nor 91, and PRs 12c and 12d need no other thermal grant.

## Limits

- One well-mixed heat store a territory and one CV-owned feeder a carrying bed. Two hydraulic feeds into one heated bed, such as portal and arterial delivery, refuse carriage until the model gives them a mixing rule or carriage per edge.
- A bed that exchanges heat with two stores needs a declared partition.
- More than one skin bed needs a sweat aggregate in place of the one-site row (`NotOneSite`).
- Separate dilator and sudomotor transmitters need separate receptor grants and a new evidence decision.

None of these is needed by the 13-system map or by PRs 12b to 12d.

## What the runs show, and what none has shown

Every run was on PR 12a's tree, whose contract crate is main's, with synthetic values. Each is named by its log's SHA-256.

- Reparenting the skin bed alone (`895386cb7f90`). All 34 site sets, 1,465 rows, 1,463 columns, 358 programs, 1,756 instances with their bound inputs, 761 parameter slots, 415 blocks, 10,162 Jacobian nonzeros and the conserved basis compare equal. Carriage goes from no reach to one, at the skin bed from the body to the skin heat site, and the skin bed's incoming flow is CV's circulation row on the edge from the systemic arteries. The stated packet stays, the whole body's model hash moves, and the body region's order moves while the heart's and the kidney's stay. PR 12a's placement test passes.
- The signed inflow (`13c6d40b51e9`). With the outgoing flow held at 71, a volume coefficient of 2 and a synthetic scale of 3, inflows of -4, 0 and +7 give paired columns of -24, 0 and +42, at -1 on the core and +1 on the store. A second feeder refuses the read at compile.
- The skin's ratio on the plain edge (`2b35bf55c961`). The edge into the skin bed reads 2.5 times e to the 0.375, 3.6374785, and the other three edges read 2.5 to the bit, with slopes only on the skin edge.
- The nearest-store rule (`2b35bf55c961`) on five cases: two beds in one store, a site with no heat pool between, a store inside a store, a bed under a bed, and the core's territory, which gets no reach. In one MT fragment, a body row and a site row of one name refuse as `NamedTwice`.
- The dormant pieces (`0e966e7bfd01`): with the inclusion, the nearest-store rule, the inflow's seal tag and `bed_inflow` added, every `ng-contract` library and integration test, the whole body's model pin, the slice's model pin and PR 12a's placement test pass.
- The type witnesses with stand-in items (`5f3ed4c1fa0e`, `b1b4cfc348d5`): 23 of the 24 fail with their declared code, and the optional junction's fails with E0599. With the inclusion stated, every other `compile_fail` doctest of `ng-contract` keeps its code, and only the `Beds<Muscle>` witness compiles.

No run has shown, and the contract PR shows:

- Publication. No run varied a store's heat and capacity apart, nor lowered three readers to one row.
- That the catalog's changes leave the body dormant. No run touched the catalog: arrows 90 and 91's rows, `heat_carried`'s new return type, the new sources and `Over` variant, the grant checks and the temperature's publication are not built.
- Each witness's code on the built items.
- That each new seal tag is appended, and that changing an end, an exit or a landing changes the seal.
- The exact lookup refusing a missing site row where a body scalar of that name exists, across owners.

No run has shown, and PR 12b shows: the cost of five rows and one column, the 1,478 rows and 1,467 columns predicted for the whole of 12b, and at most 5% more instructions a simulated second, all predictions, since carriage couples the skin's pressure and flow to two thermal rows; admission; the skin's vessels changing before sweat starts in a warm room; and replay.
