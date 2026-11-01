# 10: Arena slice laws compile and initialize

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The slice's organ fragments compile on the contract. They are CV with valve modes and hydrostatic head per segment, FL Starling at every capillary bed with lymph, RN filtration and tubuloglomerular feedback, NS baroreflex with a set point and release and reuptake at every junction, the receptor bindings and the boundary routes. The body initializes at the 06:55 anchor moment.

Blocked by: 08, 09

Status: resolved

- [x] The slice compiles with no law naming a site and no organ owning a pool
- [x] Initialization meets every fixture anchor with recorded provenance
- [x] Water, ions, moieties and energy close over the first accepted steps with the reference core

## Answer

The slice's organ laws compile on the contract. The body starts at the 06:55 anchor on three anchors, runs on the reference core, closes every book, and answers a stand through its own reflex. All values are synthetic.

- **The slice compiles with no law naming a site and no organ owning a pool.** Five law crates (`ng-cardiovascular`, `ng-fluids`, `ng-renal`, `ng-nervous`, `ng-receptor`) and the held cards give 240 rows and 251 columns. A law reads a port and the map says where. Pools belong to the schema. The E0603 probe lands in `ng-cardiovascular` and the E0451 probe in `ng-receptor`, each with a legal twin. There is no reaction type: a binding is a paired transfer.
- **Initialization meets every fixture anchor with recorded provenance.** Heart rate, mean arterial pressure and filtration each have a fit: the heart's own rate, the renal afferent resistance and the tubule's sodium capacity. Provenance names the anchor behind each fit, says which start values were solved and which came from the fixture, and gives the row nearest its limit. The solve takes about 26 s in a release build.
- **Water, ions, moieties and energy close over the first accepted steps with the reference core.** A 2 s run through at least eight valve and rhythm firings closes every book to one part in 1e12 of what crossed it, charge included. The elastic store stays on the sum of the walls. Held pools do not move.

`crates/ng-validate/tests/arena_body.rs` holds these checks in one test on one solve, each from a checkpoint of the start. It also checks that the body regulates itself:

- Every link of every loop shows a slope on the admitted start, 44 pairs in all, so a wrong map fails even where every count passes. The loops are the baroreflex from pressure to transmitter to rate, wall and arteriole, feedback at the kidney, Starling and lymph at every bed with what encloses it, and vasopressin from sodium to water back from the duct.
- A stand runs through the reflex. Blood falls into the legs, the sinus feels less stretch, traffic rises by more than a fifth and the rate follows. No law reads posture but the heads.
- In late diastole the heat shed matches what the flows lose to within a tenth, so the rate that closes the store is a real account of dissipation.
- A checkpoint in mid beat, with a posture input after it, runs on to the same bits.

**Choices, each with its reason.**

- **Two designs were built and judged.** The port kept the smaller one, which already admitted the whole slice. The judge preferred the other design's rules, so each of its counterexamples became a fix here and a test. Those are the next nine lines.
- **One firing rule.** The run and the ring fire guards through one function. Of two guards on one variable the first in stable order wins, and a guard a firing arms waits for the next step.
- **A crossing inside a step refuses the step.** A guard crossed and crossed back leaves no mark at the step's end, so every stage before the end is read, and a shorter step lands on it. At the floor the declared resolution is the limit. No check can prove nothing happened between the points a step looks at.
- **No solve at a landing.** No algebraic law may hold by a mode or read a row a firing resets, hazards' included, so no algebraic row jumps. A valve's flow is its memory, so a reset leaves nothing stale. The dissipated power is the one row that shows a left limit at a firing, because it is a rate.
- **A valve opens past a small head.** It shuts at or below zero head and flow, as research 02 says, and opens only past `cv.opening-head`, so a still body fires nothing.
- **A map states what is absent.** A law that reads a junction at a site needs a supply or `AnatomyBuilder::unsupplied`, and a junction needs the receptor. A lost line is `NoSupply` or `NoReceptor`, and never reads as no effect.
- **A route states what it stops.** A species one end of a fluid route lacks is `CargoStranded` until the route says `keeps_back`.
- **A total is solved only if the body regulates it.** A boundary law that reads the body makes a total regulated, and admission solves it. That test reads syntax, so `admit` also reads every book's net flow over the cycle and the rank of the whole ring. The ring solve moved into the contract so that `admit` can run it.
- **Energy has one store for the walls.** The heart's work and gravity's arrive as receipts, and a dissipated power moves the store into heat. That rate is what keeps the store on the sum of the walls, and it stays at or above zero, so heat never runs back. The moving blood's energy and the stretch of tissue and tubule are not in the sum, so they pass through it as heat.
- **Posture is a trunk and legs.** Heights run from the hip. A ramp is a train of events.
- **Point 0 is the first stage after the wrap,** so the phase there is one interval past zero. The period stays fixed and a fit on the heart's own rate lands the wrap on the ring's last point, so no period unknown is needed.
- **Small ions cross a capillary wall on their own,** `fl.ion-exchange`. Water alone left tissue sodium a nearly free level, three times the blood's, and Newton could not find it.
- **A vessel below its unstressed volume collapses and sucks nothing in, and the leg veins have a valve.** Without them a stand ran a chamber dry.
- **Sodium reclaim saturates.** A fixed share leaves the sodium the macula densa sees blind to filtration, so the feedback has nothing to read.
- **The solver's own work.** Admission marches 40 beats, then runs Newton on the ring with a line search, then sweeps the guards. The ring update walks the fixed elimination and solves one dense corner.

**Findings.** Finding 14: `deviation` returns `Deviation<U>`, which leaves only over a width of its own dimension or as a set point's rate. Finding 17: a ticket now stops at the body's tightest rate class and at a forcing's knot. On the slice that cost is less than nothing: asked for 20 ms steps, the capped run took 3.9 s for 5 simulated seconds and an uncapped copy took 4.9 s, because long steps are refused and solved again, and its pressure sat further from the anchor. The cap is 10 ms, the wave class of the heart's laws. A span past a law's interval is also how a core could see past a breakpoint, so the cap narrows that too.

**Speed.** A stage's residual, slopes, factor and solve take about 140 microseconds on 240 rows, and a ring update takes about 150 ms on 24,000 unknowns. The reference core takes 2.2 ms a step at 1 ms steps and runs at 1.3 times real time at 5 to 10 ms steps. The cost is the three stage solves of a doubled step, the landings at about ten firings a beat, and the record's own work. That is a first order reference, not the arena's bar, and it says what a faster core must beat. The reverse sweep and the fixed elimination are what bring it here: forward slopes and a dense solve cost about ten times as much.

**The comments below, each answered.**

- **03.** The renal scope, per-edge modes and edge guards are built. A pressure with no gauge shows as no rank. Solutes are carried, albumin takes up room, and a small ion takes up none. The crate's shapes stand: `EdgeRoute` with `transfer_along`, and boundaries named in the schema.
- **04.** Each row has a natural size, which the packet may state, and the reference's Newton stop reads it. Nonlinear energy closure sits in `verify` as a stage residual. These are the first steps of a body with modes.
- **05.** Sodium, potassium, chloride and posture have their own payload tags. Modes were already in the image, and a checkpoint in mid beat proves them.
- **06.** `ModesNotYetRun` now passes a valve. A plain guard's gap is its zero with the sign `Crossing` gives it. The late input fuzz ran 120 more rounds with a hazard an input can move, through a field, and every run that compared matched. A span now stops at a forcing's knot, and a hazard row can state a natural size.
- **07.** A pressure is a row with its site. A flow is a row too: a plain edge's flow row, or a valve's memory. A firing's instant reads the accepted state, which no algebraic row left stale.
- **08.** A mode stands per point. The period stays fixed. Boundary flows net to zero through regulated totals, with no route held. The ring has its own elimination. Newton stops when its update stops shrinking near rounding.
- **09.** The tubule, duct, bladder and their networks are on the map. CV reads an amount through its ports. GI, EN and the skin take their routes in `hold_outside_slice`. A delivery now needs its route's law to read the drive in every mode. A rigid or elastic enclosure is still refused. Rank is read on the whole ring. A held card's prescribed value can be no anchor, so I14 exempts nothing.
- **Research 02.** Every law form here is in use, and viscosity is the one left out, as below. Its constants stay synthetic.

**Left for later, each with its reason.**

- **Row 111 stays unresolved.** ICP ownership is not settled here, and nothing built reads it.
- **`Impulse` and bolus.** A delivery is a rate over a window, and a short window is a bolus. No check here needs an instant jump.
- **Viscosity and hematocrit.** The slice has no red cell amount, so a resistance is a parameter. Blood chemistry holds its cells.
- **`Guard::advances`.** The heart's rhythm draws from no stream yet.
- **An elastic enclosure's wall, sink fields and voiding as an event.** No law here needs them. An elastic or rigid enclosure is still refused by name.
- **The cell-water conflict between BC and FL** stays open: BC holds the cells.

**Law forms research 02 does not give, all model defaults:** chamber activation, the vessel pressure-volume law, stretch, the central sigmoid, the oncotic term, interstitial compliance, lymph flow, filtration, reabsorption by segment (saturable sodium, potassium with sodium, chloride beside them), vasopressin turnover, bladder drain, gut basal rates, ion exchange and collapse. They are labelled in the code. None needs research before ticket 11, and each needs a reviewed source before a clinical packet.

**Hard-part check.** No solver redesign was needed: the elimination that follows the ring is a plain one. No validation against Pulse or published physiology was done, and no disease was built.

**Evidence**, all synthetic. No number moved on the old benches: one probe built against main and this branch ran 62 cases at 1 s spans and printed every row's bits, and the two outputs are byte equal. A control that moved the span by 1 ms differs. The mutation sweep, verify and Codex evidence are in the PR.

## Comments

From 03: owed here and not built: the renal scope (the `Renal` owner type exists), the pressure-gauge check, and per-edge modes and edge guards for valves. 03's modes are owner-wide, and a guard's zero kernel is whole-body. Solutes and their partial volumes are owed too, since 03 has water only. Reconcile two 03 shapes with the sketch. CV's circulation is an `EdgeRoute<Water>` with `transfer_along` in the crate, but a `FluidRoute` with `advect` in the sketch. Boundary water is named in the schema in the crate (`Schema::water_boundary`, `BoundaryPorts::water(name)`), but is fixed fields in the sketch (`intake_water`, `evaporative_loss`, `heat_exchange`). The BC/FL cell-water conflict is still open, and 03's benches leave cell water out with a stated reason. Sketch probes this ticket owns: lines 40 (E0603) and 327 (E0451).

From 04: `Impulse`, bolus delivery and nonlinear energy closure are not built, and `verify` is where energy closure goes. `ng_validate::ReferenceIntegrator` stops Newton on an absolute term of 1, which assumes rows near unit scale, so a row of order 1e-12 stops early; `ErrorScales` is uniform too. Give both row scales before the slice's mixed units run on it. The first accepted steps here are the first run of a body with mode variables.

Decided after the deep review of 02 to 04 (nightingale#13, findings 12 to 17): finding 14: `deviation` returns its own dimension, so a set point can never become the variable it guards. This lands before this ticket's laws are written. Finding 17: each law records its rate class now. Whether a step longer than a law's longest interval is refused is designed here, after measuring what enforcement costs in speed.

From 05: a new boundary input takes a new tag beside `WATER_PAYLOAD` in `PendingInputs::write` and `read`. A mode variable joins `ImageParts` and the image digest in `crates/ng-record/src/record.rs`. On finding 17: a passing span is what shows a cut, and backward Euler doubled at a huge span near equilibrium passes its estimate, so a step longer than a law's longest interval is also how a core sees past a breakpoint. Enforcing the longest interval would narrow that as well.

From research 02: this ticket takes its law forms from the Answer in [Law forms the slice needs](../../research/issues/02-law-forms.md). Its constants stay synthetic, as the contract requires of the arena; the cited values in `data/` bind only in the reviewed packet. A form there that the contract contradicts is corrected in that ticket, and the contract stands.

From 06: lifting `ModesNotYetRun` for a valve needs two things this ticket does not build. A firing leaves algebraic values as they were, so a mode change that moves an algebraic law needs a solve at the landing. A plain guard needs a gap: `land` reads a guard's distance from its threshold and any function that crosses zero fits, with `Crossing` deciding the direction, and `Guard::advances` is unbuilt. Modes are one `Vec<usize>` in compiled order, so a per-edge mode is more entries, not another shape. A `memory_rate` kernel cannot read an amount or a drive, so no input can move a crossing yet, and the causal fuzz has not pressed that coupling: when a hazard rate can read an amount, run it again with an input that does. On finding 17: a rate a forcing varies keeps the class `Kernel::finish` gave it, in the model hash, and nothing enforces it. A span past a knot period aliases a forcing, and a stage's error over a forcing is not monotone in its length: a 64 s span passed where a 32 s trial was refused. Row scales matter here: a hazard row is held to the uniform scales, about 1 on a row near 1, so an event's accuracy rests on step length until a hazard row has a scale of its own.

From 07: a channel names a row by `RowKind`, so a solute's row or an algebraic pressure reaches a lens with no change, and a field added to `RowKind` reaches `ChannelSpec` by itself. A memory or algebraic row is whole-body and names no site today, so a pressure at a site must be a row that carries its site in `RowKind`, as `packet.start` needs too: an observation's site comes from its row. A flow is no row: reading one needs an algebraic row or a second `ChannelSpec` constructor. At the head a channel reads the accepted state, so a solve at a landing is what a channel reads at that instant, and the segment keeps the left limit.

From 08: a packet states its goal with `ParameterPacket::periodic(cycle)`, and `admit_solved` solves it. Five things wait for this ticket. A mode variable is refused under a cycle, so valves need a mode per point and rows for their guards. A cycle has a fixed period, so a heart that sets its own rate needs the period as an unknown and one phase row, and the fence then comes from the solved period at `admit`. `NetBoundaryFlow` refuses a combination whose boundary flows do not net to zero over the cycle, and the slice loses water to urine and skin, so either hold those routes at admission or let the total move by its known flow. A kidney's rate that reads an amount also stops a total being neutral, so decide whether that total is still specified. `solve_dense` is the plain solve and costs the cube of points times rows: one Newton update took 3.8 s at 2,001 unknowns and 31 s at 4,001 in a release build, so the slice needs an elimination that follows the block ring, behind `initialize`. An anchor can read an amount row, or a row that reads a drive, and per-site rows then pass `AnchorOnParameters` by themselves. A cycle mean's channel must read exactly the ring's points, with the run starting on the edge of one of its windows, so the ring's steps follow the lens that reads the anchors. Newton stops when no unknown moves by more than rounding on itself, so an unknown that rounding keeps moving costs all 32 updates, as one near zero does. That is time only, but it multiplies the cost. A plain sum in the cycle mean once did this to the fit at 4,001 unknowns, so a new elimination should measure its own noise on the fit. When `ErrorScales` gets row scales, `InitializationSystem::held` takes the row, and the packet's `within` states them.

From 09: the map has no renal tubule or bladder, and this ticket adds them with the network and routes that move them. A coupling field is a row, so a per-site pressure is already a row with its site, and a lens or `packet.start` reaches it by `RowKind::Algebraic(Slot { law, site })`. In 09's benches a capillary pressure is a parameter. CV has no read of an amount yet, so this ticket adds one to `CardiovascularPorts`. No held card declares a route yet: GI and the others take theirs here, in `hold_outside_slice`. A card reads only its own routes' drives, a route's law reads only its own, and a delivery needs that law to read it. That last check does not look at modes: a delivery passes when only an inactive mode's law reads the drive, and the amount waits for that mode. This is on main from before 09, and it matters here because routes gain modes here. A read of a rigid or elastic enclosure's pressure is refused until that wall has a law, and an elastic wall's law needs the volume inside, a sum across sites. A coupling loop with no rank passes structural matching and runs silently from a consistent start: `x = y`, `y = x` stays at whatever the packet gave it. The contract says initialization reports the offending rows, so a rank check belongs where the start state is solved. Two checks to run again: the causal fuzz, now that a hazard can reach an amount through a field, and whether I14's check exempts a held card's parameters. Fields add unknowns: about 290 against 107 microseconds a step on a dense core on the slice's map, measured on the arena candidates, and a block core is not measured.
