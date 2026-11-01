# 09: Arena slice anatomy, held stand-ins and the census

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The arena slice's anatomy is declared as data (its beds, five junctions, the pleural and abdominal enclosures, positions), every card outside the slice is held with a declaration of what it supplies, and the coupling census is counted by the compiler.

Blocked by: 03

Status: resolved

- [x] The slice anatomy passes the graph checks; pleural and abdominal pressure reach the enclosed beds as held enclosure pressure
- [x] Every held stand-in declares its supplies; a held route its declaration omits is refused at compile
- [x] The census check passes: 38 = 27 mapped + 11 listed unmapped, rows matching the coupling map's lines 77 to 114

## Answer

The slice now has a map, every organ card outside the slice stands behind a written declaration, and the compiler counts the 38 arrows of the coupling map.

- **The map passes the graph checks, and pleural and abdominal pressure reach the beds inside.** `arena_anatomy` in `crates/ng-validate/src/slice.rs` holds 28 sites, each with a position: five tissue sites, ten vessel segments, five beds, the pleural and abdominal spaces, five junctions and one nerve segment. The lung bed sits in the pleural space and the gut bed in the abdominal space. A test changes each pressure and checks the exact water that crosses each bed's wall. A bed outside both reads zero on the gauge. The kidney's beds are among those, because the map's line 102 keeps abdominal pressure as a direct input to the kidney.
- **Every held stand-in declares what it supplies, and a route it leaves out is refused at compile.** `hold_outside_slice` holds all nine cards. Each declaration says which routes the card supplies, which spaces it holds and where it is valid. A law on a route the declaration leaves out is `UnclaimedLaw`, shown on a small bench and on the slice's own map. Nothing on the map is left out.
- **The census passes: 38 = 27 mapped + 11 listed unmapped.** The const checks in `crates/ng-contract/src/catalog.rs` count the rows, hold them in line order and tie each row's disposition to which of its ends run in the slice. `census-check.py` compares the crate's rows with the map's lines 77 to 114, and with the sketch's table byte for byte.

The stand-ins marked in `walk.rs` are restored, so NS reads line 77 and FL reads line 80 and its outside pressure. Sketch lines 186 (E0271) and 291 (E0061) land in the crate with their legal twins.

**Choices, each with its reason.**

- **A coupling value is a row.** `emit` owes one row per field, and `define` sets that row to a value. Two couplings that read each other are then solved together, as the contract asks, and a lens can read a coupling. The design with no rows copied formulas into their readers and had to refuse a bare loop. The cost is more unknowns: on the slice's map a dense core took about 290 against 107 microseconds a step. That was measured on the two arena candidates, and a block core is not measured.
- **A row owed a definition is its own kind of slot.** Only `define` serves it, and only a port reads it. A plain row of the same name stands in for neither. Names alone were not enough: the adversaries made a wrong-unit read compile and made compile panic.
- **A held card is an ordinary set of laws plus a small declaration.** What it prescribes shows in the rows it defines. There is no separate held type and nothing is pre-filled.
- **A held pool keeps its row.** It is an amount in the state and the ledger that no law moves. One card holds it, a space held must have a pool in this body, and a law that moves it is refused. A space a schema leaves out cannot also be held.
- **A held route belongs to its card.** The schema names where it goes, the declaration claims it, and its column records the card. A delivery needs the route's own law to read its drive, because any other read could pass admission and lose it. Another row of the same card may sense a drive, as ticket 08's anchors do, and a route's law cannot carry another route's delivery.
- **An enclosure's pressure is one row per enclosure.** A bed reads the nearest enclosure that names it, and what contains a bed cannot put it in an enclosure that does not, so an author lists the beds. A law that reads a bed in a rigid or elastic enclosure is refused until that wall has a law, so the walk's pleural space moved to Held by RS.
- **A census row keeps only what the checks read:** its line, its disposition and which ends run in the slice. Endpoints and labels stay in the table's text, where the script reads them.
- **A change to `catalog.rs` or `system.rs` runs every step,** because a crate test cannot read the map. That is one line in `tools/gate/paths.toml`.
- **The map has no renal tubule or bladder.** They need a network the map does not have, and ticket 10 adds them with the routes that move them.
- **Left out, because no check reads them:** a read of plasma for CV (ticket 10), and the sketch's `HeldDeclaration::reservoir` and its receipts and ligand ports (a second book). The statements a grouped region owes are foundation ticket 07's and are not built here.

**Hard-part check.** No law, cross-site coupling, stiff numerics or outside research was needed. One came close: an elastic wall's pressure needs the volume inside, a sum across sites, so a read of one is refused and left for later.

**Evidence**, all synthetic.

- **No number moved.** Main at `b55b31f` and this head, compiled from one probe over 52 runs and 24 seeds, give equal state, continuation, segment, mode, stream, applied and ledger digests and equal firings: 17,248 lines and 1,358 firings. The model hash moves on purpose, so run identities and seals differ. A control that moved one fixture number by one part in a million came back different, so the probe can fail.
- **Ticket 08 and this ticket meet cleanly.** This ticket's first drive rule forbade a row that senses a drive, which six of 08's tests need, so the rule was too broad and was rewritten. 08's solve then ran on this ticket's rows. A wrong guess across the whole map is solved to what defines each row, and a cycle solves field rows at every point and leaves held pools alone. An anchor on a held pool is refused, because a held pool is the packet's own start.
- **Mutation sweep.** 52 mutants on the merged code, one per rule or refusal, hand-picked. 49 are killed by a named test. Three survive, and each is equivalent. A declaration's supplies in the hash: a supply must have a law, whose column names its card. A row's site in the hash: it comes from the sealed anatomy and the catalog. A definition's flag in the hash: a definition's steps list its value first, and a written-out residual can only list the row first. The sweep also showed a check doing more than it needed, so a site now checks one step up.
- **A judge on another vendor.** Codex Astra at max, blind to which runner wrote which design, scored the design with rows 25 and the design without 13. Both runners were Opus, because the other vendors were out on limits when they ran, so the team's "one runner on another vendor" is not met.
- **Two adversaries, one per vendor, before the merge.** Each worked alone from the contract and the diff, 101 repros between them. They found the name-only field, the held claim with nothing behind it, the lost delivery and the census's gaps. All are fixed and each repro was rerun. What held: all seven replay gates on field rows, held pools and a held route, a held route's ledger closed over 50 h, and 6,000 random bodies with no panic.
- **Local reviews, Codex Astra at xhigh.** Six passes, and the last found nothing new on this branch. They found the bed with two enclosure chains, a fix too narrow for it, the missing markers on `Temperature` and `Phase`, and one finding declined: a flow may read a row that senses another route's drive. That is a law its author wrote, as a reflex is. The first route's delivery still arrives in full and the ledger closes over 48 h with both routes counted, so it stands. The last pass also raised a gap already on main, noted on ticket 10.
- **Comments.** Comment Sicko deleted 32 and trimmed 25, and found 11 false or unproven. No code moved, by a hash of each file with its comments stripped.

Limits: everything ran on the reference core. The adversaries read the code before ticket 08 merged, and the reviews read it after. The random bodies never reach a plain slot under a field's name.

## Comments

From 03: the orphan rule is opt-out, through `Schema::leave_out(space, reason)` (03's answer has the evidence). Held declarations extend it, so a pool a held declaration holds is no orphan either. The slice should leave out nothing a law or a held stand-in can cover. The coupling catalog (`emit`, `receive`, `define`) is not built, and in the sketch `define` now borrows its output like every other attachment. `IN_SLICE` and `OutsideSlice` are not built either. The fixture stand-ins in `crates/ng-contract/tests/walk.rs` mark what this ticket restores: the carotid sinus stretch port, and the capillary and extravascular pressures. Sketch probes this ticket owns: lines 186 (E0271) and 291 (E0061).

From 04: the boundary has one payload, `BoundaryInput::Water(WaterDelivery)`, for a plain amount through a named boundary. `Intake` adds composition and routes beside it, and the sketch shows both. `PendingInputs::after` clones the delivery list every step and `BoundaryPort::new` finds a repeated id by a linear scan: harmless at bench size, and worth changing for a two-day timeline. A ticket cut below the floor by a breakpoint faults at its first refusal, so a slice's closely spaced events need a floor below their gaps. A drive frame holds rates, so a ramp needs a frame with a slope.
