# 09: Whether the arena's prerequisites leave the SRS (C11)

Type: grilling
Status: open
Blocked by: none

## Question

Does the SRS section 5 bullet "Pieces needed before the solver arena runs" leave the SRS, and where does its one unique requirement, the slowest steps' wall time in arena results, go?

The bullet orders the build inside the structural model. Its other pieces are SRS requirements already: the map and its enclosures as data, and site-indexed ports (section 5, "Schema data, not code"), laws per site class (rule 10), an event that arrives late (I5), seeded draws inside laws (section 1 and I8), and a retry that equals a loaded checkpoint (I3). The native-contract tickets that block [Run the arena, then build the stage 1 milestone](01-arena-and-milestone.md) hold the build order. The speed gate in `docs/architecture/solver-contract.md` (section 7, "Arena specification") reports no slowest-step wall time.
