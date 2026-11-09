# 29: The contract names the unit of each sensed dimension

Spec: [solver contract](../../../docs/architecture/solver-contract.md), and `crates/ng-contract/src/quantity.rs:21-30`, `:323` and `:386`

Type: task

Status: open

Blocked by: none

A `Measure<U>` is "a finite value of dimension `U` in its canonical unit" (`crates/ng-contract/src/quantity.rs:386`), and `Pressure` is a sensed dimension (`:323`), but no doc or code names pressure's canonical unit. Time names its unit ("Simulated time, in seconds in a packet", `:25`). Water's amount is "in the canonical amount unit" (`:29`), which is not named either. The arena fixture anchors arterial pressure at 100.0 (`crates/ng-validate/src/slice.rs:580-584`), and an observer passes that number on with no unit. A client has to guess. The clinical workspace prototype's bedside monitor shows mmHg, as monitors do, on that assumption alone.

**What to build:** name the canonical unit of every sensed dimension that a bedside observer reports (`Dimensionless`, `Pressure`, `Volume`, `Temperature`, `Length`, `Rate<Water>`), at least pressure's, and the amount unit behind water. Options for pressure:

- mmHg, the unit charting practice uses for vascular pressures. The glossary already names methods "as current charting practice names them" (Anchor). No client converts, and the fixture's 100.0 reads as 100 mmHg.
- An SI unit (kPa or Pa). The contract keeps one unit system, and every client converts for display.
- A unit carried by each lens channel, so each reading names its own. The most general, and it adds a field to every channel.

- [ ] The contract's lines that define `Measure` state the chosen units, and the edit stays small.
- [ ] A client can read a sensed reading's unit from the contract or its channel instead of assuming it.
- [ ] `GLOSSARY.md` changes in the same PR if a term does.

**Non-goals:** changing any fixture value; reviewed clinical values; how a client displays a unit.
