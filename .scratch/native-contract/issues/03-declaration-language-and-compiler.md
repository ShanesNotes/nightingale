# 03: Declaration language and compiler for a two-reservoir body

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** An organ author writes whole-body and site-class kernels with instruction set v1, transfers and site transfers, memory rates, set points and modes, and `Assembly::compile` turns them into one hashed body with incidence, the conserved basis, sparsity and blocks.

Blocked by: 02

Status: resolved

Claimed by: claude

- [x] A two-reservoir manufactured body compiles; its incidence and conserved basis match the hand-derived ones
- [x] A site-class Starling law over the two beds compiles to one program applied at both sites, with per-site parameters
- [ ] Every compile-fail doctest in the sketch has a compile-fail test here with the same error code, and each legal twin compiles. Answered, not ticked: the 14 whose types this ticket builds are here; the other 17 need types tickets 04 to 11 build, and each now lists its own (the table below)
- [x] Compilation refuses an omitted law, a second owner of a row, and a mode without law coverage, each naming the law

## Answer

`ng-contract` now holds the declaration language and the compiler. An organ author declares memory and algebraic slots, set points, parameters, modes and guards, requests route families through its scope's ports and attaches kernels by reference. `Assembly::compile` checks the schema and every fragment and returns one hashed `CompiledBody`: rows, columns with signed incidence and receipts, programs and their instances, the parameter layout with defaults, sparsity, blocks in dependency order and the conserved basis. Modes, guards, validity ranges and the instructions stay private for ticket 04, which evaluates them. All values in the tests are synthetic.

- **Two reservoirs.** `tests/reservoirs.rs`: two vessel segments under a tissue site, one vascular edge `a -> b` and a water boundary into `a`'s lumen. The rows are the two lumens. The circulation column is `[-1, +1]`, the inflow column `[+1, 0]` with its receipt at -1, and the basis is total water, `a + b`, each written out in the test. One edge law reads each end's own elastance through an end parameter, so it names no site, and both lumens form one block through the flow between them.
- **One Starling program at two beds.** `tests/starling.rs`: two beds in series on a vessel loop, with an enclosure, a junction and a nerve segment. The Starling law is declared once for capillary beds and compiles to one program with an instance per bed. Each instance reads its own `fl.e` and `fl.kf` slots, whose default binding is the capillary-bed class. The test writes out the 4 by 2 incidence and the two-combination basis, each bed's plasma plus interstitium.
- **Probes.** The sketch has 31 compile-fail doctests. The 14 whose types this ticket builds sit in `src/lib.rs` with the sketch's codes, each beside a legal twin, and four new type rules join them: an invented dimension, a per-site vector through the whole-body read, and `at_site` in a whole-body or an edge kernel. The other 17 need types that tickets 04 to 11 build, and each of those tickets now lists its probes in a comment. `gate.py codes` checks the crates' codes and the sketch's with one implementation: 26 in the crate and 31 in the sketch, each failing with its code.
- **Refusals.** Thirteen compile refusals: `OmittedLaw` (a memory slot, an algebraic slot, a route family, a boundary), `SecondOwner` (two every-mode laws, laws on two mode variables), `UncoveredMode`, `GuardOutsideMode`, `MissingFragment`, `NamedTwice`, `ParameterConflict`, `BadModeVariable`, `StructuralSingularity`, `OrphanPool`, `LeftOutTouched`, `MissingPartialVolume` and `BoundaryEndpoint`. Eight more refuse at the call: a scope issued twice, an unknown boundary name, `Mode::to` with its own value or another variable's, a zero rate interval, a reversed validity range, a partial volume that is not positive or differs from the first, and a boundary name declared twice. Each has a test that adds that one defect to a valid body, and the three this ticket names say which law. Disabling each check in a copy turns a test red: 40 of 40 mutations caught. The first pass caught 36; the other four checks were held only together with another, so each got its own test: moving a left-out pool, a boundary into a left-out pool, a left-out reason in the hash, and which end an end parameter binds at.
- **Order and the hash.** `tests/model_hash.rs` declares one body in two orders, with scopes, fragments, declarations and attachments all reordered and a dead read added. Every public array and the model hash are equal. A changed rate class, law, left-out space or reason, anatomy or validity range changes the hash; parameter values stay out of it.
- **The walk.** `tests/walk.rs` compiles the signature walk's steps 1, 1b and 1d, with each change marked by the ticket that restores it: the catalog's ports in 09, CV's chamber port in 10, step 1c's seeded chance in 06.

Choices, from a pstack `architect` arena: a Claude runner and a Codex runner, judged by Codex, 25 to 17 for the Codex candidate as base. The judge shares a vendor with that runner, and the deciding evidence was compiled probes, which I reproduced. The Codex runner and judge ran gpt-6-astra at xhigh, not the arena profile's max, and the Claude runner ran Opus 5.5 at the harness default effort.

- **Laws attach by reference.** `memory_rate(&slot, ..)`, `residual(&slot, ..)`, `transfer_at(&route, ..)`, `transfer_along(&route, ..)` and `transfer(Transfer::new(&pair, ..))` each append one law, and the kernel's own mode set says where it holds. The sketch consumed slots and routes by value, which made a second law unwritable, so `SecondOwner` could not exist. Borrowing makes all three of the ticket's refusals reachable, each naming the law by its owner's card and the slot, mode variable, route family or boundary it serves. Declaring a slot, requesting a route or naming a boundary records the debt at once, so dropping a handle drops nothing, and a scope issued but never added is `MissingFragment`. The sketch now borrows the same way, `define`, `advect`, `work` and `reaction` included.
- **Route families are requested.** `ports.starling(&mut laws)` and `ports.circulation(&mut laws)` register every instance the anatomy implies. A bench requests only what it models, no law names a site, and a family added later breaks no bench. Rejected: granting every route a scope could have, which breaks every bench of an owner whenever its scope grows.
- **The orphan pool: refused unless the schema leaves its space out.** Both runners and the judge chose to make a pool an amount only when a declared capability names it (the base candidate's activation). This answer chooses opt-out instead. Every registry pool is an amount unless `Schema::leave_out(space, reason)` states that the body does not model its space. A modeled pool no transfer moves is `OrphanPool`, a law that moves or reads a left-out pool is `LeftOutTouched`, and a read never counts as moving. The grounds are SRS section 2 ("the state holds their amounts") and rule 3, the contract's "an omitted law is not zero" and record 0002's explicitness. The same bodies, compiled under both rules (evidence `orphan-experiment/`, with the variant's diff):

| Body | Opt-out (chosen) | Activation |
|---|---|---|
| Two reservoirs | 2 amount rows of 2 pools | 2 of 2 |
| Two-bed Starling, cells, lumen and enclosure fluid left out | 4 of 8 | 4 of 8 |
| Two-bed Starling, nothing left out | `OrphanPool` (a, cells) | 4 of 8 |
| Two-bed Starling, route never requested | `OrphanPool` (a, plasma) | compiles, 0 of 8 |
| Two-bed Starling, law forgotten | `OmittedLaw` (FL starling) | `OmittedLaw` (FL starling) |
| Walk, cells and pleural fluid left out | 7 of 11 | 7 of 11 |
| Walk, nothing left out | `OrphanPool` (glomerular bed, cells) | 7 of 11 |
| Walk, fluids scope forgotten | `OrphanPool` (glomerular bed, interstitium) | compiles, 4 of 11 |

  Complete bodies compile the same under both rules. A forgotten scope or route compiles silently under activation, with its water gone from the state, and is refused under opt-out. The cost is one stated line per space a bench does not model: two in the walk, three in the Starling bench. The BC/FL cell-water conflict stays open, and ticket 09's held declarations extend "no transfer moves and no declaration holds".
- **Edge routes and end parameters.** The two-reservoir flow runs along a vascular edge, so `edge_kernel`, `EdgeRoute`, `transfer_along`, `at_edge` and the route's `from_amount` and `to_amount` ports are here. `edge_parameter` binds per edge with the vascular network as default. `end_parameter` is read by `upstream_parameter` and `downstream_parameter` and binds at that end's site. The arena's `NodeParameter` is named `EndParameter`, because the glossary avoids "node" for a site.
- **Composition data in the schema.** Water boundaries are named in the schema with a site, a space and a direction (`Schema::water_boundary`). Each owes a law whether or not a boundary scope is issued, and `BoundaryPorts::water(name)` returns its pair and drive. The partial volume of water is one fixed, hashed coefficient (`Schema::partial_volume`), and `Kernel::volume` is the one conversion from a water amount to a volume; a body that converts without it is `MissingPartialVolume`. Rejected: volume read ports with the coefficient as a parameter key, since it is not a property of the patient.
- **Modes and guards.** Modes are owner-wide here: a kernel holds in every mode or while one variable has one value. A guard's zero kernel is whole-body and must hold in the value it leaves (`GuardOutsideMode`). Per-edge modes and edge guards wait for ticket 10's valves.
- **Basis and blocks.** The basis is one indicator vector per connected part of the internal transfer graph. It is exact in integers because transfer incidence is totally unimodular; reactions bring rational elimination in ticket 10. Blocks are the strongly connected parts of the row graph after a structural matching of algebraic rows to algebraic unknowns (augmenting paths, refusing `StructuralSingularity`). Tarjan's numbering puts each block after those it depends on, and a block carries the fastest rate class of its laws.
- **Evaluation in 04.** The compiled body keeps, private to the crate, what the reference core needs: instructions with inputs by name, each instance's bound inputs, the parameter layout with defaults, modes, guards and validity ranges. Evaluation, the pinned transcendental operators, derivatives and `CompiledBody::refusal_sources` are ticket 04's.
- **Subtracted.** No renal scope, because no body here has a renal route (the `Renal` owner type stays for the probes). No pressure-gauge check, because no body here has a pressure loop; it lands with ticket 10. No `Pair::new`, and the sketch drops it too, so only a scope grants a pair. Only the dimensions this ticket uses, and no species but water, so `Schema::new` still has no refusal.
- **Departures from the synthesis.** `NamedTwice` also covers route families and the boundary's schema names, so a law's name names one law. `AdmissionError::PartialVolumeTwice` refuses a second, different coefficient instead of letting declaration order pick one. `Pair` carries only its boundary's name, because the schema is the one source of where a boundary is. A column's `receipt()` is the one way to tell an exterior column from an internal one. `Class` became public for `Binding::Class`.
- **Gate.** `tools/gate/gate.py codes` strips the compile-fail markers in a copy of the tree, runs the doctests and checks each failure's error code against the one declared, for the crates from `10-rust.sh` and for the sketch from `20-sketch.sh`. Its probe shows it flags a wrong code, a missing code and a block that compiles, and a mutation check of the gate caught 3 of 3.
- **Review.** REVIEW-PENDING

| Sketch line | Rule | Code | Where |
|---:|---|---|---|
| 19 | No law can name or hold a pool | E0603 | here |
| 25 | A vital has no setter on the record | E0599 | 04 |
| 31 | A transfer takes a pair, not its donor alone | E0308 | here |
| 40 | A neighbour's internal law state is not an input | E0603 | 10 |
| 46 | An anchor needs a citation | E0061 | 08 |
| 54 | An engine fault is not demise | E0277 | 04 |
| 60 | No material in a law's memory | E0277 | here |
| 68 | A renal law cannot take a CV route | E0308 | here |
| 78 | An expression cannot leave its kernel | E0521 | here |
| 111 | Two open kernels share no expression | E0521 | here |
| 149 | The certificate cannot pair two records' histories | E0599 | 04 |
| 166 | A faulted session is latched | E0599 | 04 |
| 177 | A lent record is read-only | E0596 | 04 |
| 186 | Only the map's receiver reads an edge | E0271 | 09 |
| 201 | A synthetic value cannot be an anchor | E0308 | 08 |
| 218 | A split qualification is not bytes | E0308 | 11 |
| 232 | No split trace from invented samples | E0599 | 11 |
| 250 | An admissible set takes no bounds | E0599 | here |
| 259 | A kernel's result takes no admissible set | E0061 | here |
| 291 | A held stand-in declares its supplies | E0061 | 09 |
| 309 | Extents come only from the quadrature | E0451 | 04 |
| 327 | No reaction from raw stoichiometry | E0451 | 10 |
| 344 | No host math on an expression | E0599 | here |
| 360 | A guard fires only its owner's transition | E0308 | here |
| 378 | A site-class law reads only its class's entries | E0308 | here |
| 401 | A set point is not the variable it guards | E0308 | here |
| 421 | A product is named only through the identity table | E0277 | here |
| 441 | A core cannot see the boundary timeline | E0599 | 05 |
| 457 | The solver holds no dice | E0599 | 06 |
| 471 | Seed domains never mix | E0308 | 07 |
| 482 | No guard on a set point | E0277 | here |

Evidence: `~/nightingale-evidence/nc03-declaration/` holds the arena (grounding, both candidates, the judge's report and the synthesis), the orphan experiment and the mutation logs.

## Comments

From 02: `Schema::pools` lists one water pool per space of every site, in key order. Whether a pool no law touches is refused here, as the contract's orphan rule reads, or never becomes an amount row, as the arena's Codex runner and judge proposed, is this ticket's call; 02's answer gives both sides. Under the first reading, a two-bed Starling bench must also move or hold each bed's cell water. Compile-fail error codes are checked by the gate only for the sketch, and this ticket's third box needs the same check for `crates/ng-contract`.
