# 06: The whole body compiles on holds

Type: task
Status: open
Blocked by: 04, 05

**What to build:** A second assembly beside the slice: the whole body. It has every site, space, pool and route the 13 systems need at rung 0, and every law the slice lacks is a declared stand-in owned by its card, as the slice already holds the nine cards outside it. The slice stays as it is, so the arena's fixture does not move while cores are scored on it, and it stays the reference a deeper body is compared with.

- [ ] The body compiles, admits at 06:55 on its synthetic fixture and passes the certificate on a short window with its held support reported
- [ ] A table of every hold by card: what it fixes and which ticket releases it. This table is the build's to-do list, and the count of holds left is its progress. It takes [ticket 01](01-rung-0-specification.md)'s rows. The airway's exit for water and heat is a held route of the respiratory card beside the slice's skin and heat exits, and the liver bed's edges are held under the hepatic card until ticket 14
- [ ] A hold names one species in a space, so tickets 08, 15, 17 and 19 can each release their species in the cells
- [ ] Each fluid route states, for each new species, whether it carries it or keeps it back, since the compiler refuses a stranded cargo. Keeping back is the stand-in wherever a later ticket owns the crossing
- [ ] A test pins the hold table, as the slice's tests pin its nine holds, so a PR that adds or releases a hold shows it
- [ ] The replay gates pass on the body
- [ ] The body has its own expect trace and its own size pins, so a system ticket updates the body's and never the slice's
- [ ] Cost: unknowns, rows and milliseconds a step for the body on holds against the slice, measured on the heavy-run machine. [Ticket 26](26-cost-budget.md) sets the budgets from this

**Non-goals:** any new law. A stand-in is a constant or the simplest balanced flow that lets the body compile.

From [ticket 03](03-whole-body-lists.md)'s Answer: the cell anion is a species with a fixed charge of -1 whose pools sit in each bed's cells, and no route moves it. A pool that nothing moves or holds is refused as an orphan, so the hold table holds it, through the box above on a hold that names one species in a space: a held card holds a whole kind of space today. The conducting airway is a tissue site with a gas space on the airway network, not a vessel segment, so the airway's held exit for water and heat leaves from that site.

From [ticket 04](04-scopes-ports-and-census.md):

- The liver bed's edges are their own family, `Sinusoids`, which the hepatic card owns, and `Circulation` reaches every other vascular edge. This body holds HP through `hold`, lends HP's builder to the stand-ins with `HeldScope::stand_ins()`, and then hands it on with `hepatic(Some(hp))`. The stand-in for the liver bed's edges is written through `HepaticPorts::sinusoids` on that builder. An HP held whole through `HeldScope::parts()` has no such port, so compile refuses the liver bed's edges as a law omitted under HP (`OmittedLaw`, in `crates/ng-contract/tests/sinusoids.rs`).
- The renal card's reaction grant names the peritubular bed as `Beds<Tubule>`. So this body's map states `serves(peritubular bed, Tubule)`. The slice's map never states it, because that would write a tissue into the slice's anatomy digest.
- The pleural fluid cannot be left out while GI's abdominal enclosure is held. `leave_out` leaves out a kind of space at every site, and a held enclosure's fluid must exist (`crates/ng-contract/tests/held.rs:717`). Body 01 makes the pleural fluid a left-out space (01:362, 491, 713), so in this body both enclosures' fluids are held pools. A built RS that places the pleural fluid with no law to move it has an orphan pool (`crates/ng-contract/tests/scopes.rs:208`). Model default: once RS is built, it keeps an empty `HeldDeclaration` whose validity names the pleural fluid, so that fluid stays held (`crates/ng-contract/tests/scopes.rs:297`). The alternative is a leave-out for one site, which changes the contract. [Ticket 10](10-lungs-breathe.md) meets the same rule.
- MT's conduction feeds the conducting airway's heat store, and [ticket 10](10-lungs-breathe.md)'s airway heat exit leaves from that store. So MT's stand-in in this body serves conduction at the airway until [ticket 12](12-heat-balance.md) builds it.
- The slice and this body share the five system crates. A loop ticket that edits a law the slice runs therefore moves the slice. This body declares its laws through its own path, and a slice law that a later ticket must change forks for this body, so the slice keeps its law.
