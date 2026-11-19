# 87: A firing sets a memory to a kernel's value

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 1, and `crates/ng-contract/src/law.rs` (`Guard::resetting`)

Type: prototype

Status: open

Blocked by: none

Blocks: none

**Why.** This ticket is not on the first milestone's route (SRS invariant I19): [body ticket 11](../../body/issues/11-breathing-is-controlled.md) controls the breath's rate, which stays in ticket 26's budget, and the milestone needs no more. It asks for the form that would let the controller move the breath's depth at rate's price. Body 11's plan measured depth as the stronger lever: on the body's first minute it removed twice what rate did of the open loop's rise in alveolar carbon dioxide. Depth on a continuous path costs 238.2 M instructions a simulated second against rate's 41.2 M, because the path from the gases to the pump's pressure joins the solver's largest dense block. `Guard::resetting` sets a memory only to zero today. If a firing at each breath edge could set the breath's depth from the drive scale, depth would cost what rate does (inferred, not built). That is a new interface and new state in a firing, so it gets a design round.

- [ ] The form: how a law states that a firing sets a memory to a kernel's value, and what compile refuses, such as a kernel that reads the memory it sets or a set into a held or non-memory slot
- [ ] What the run's identity covers: the set values are part of the record a replay compares, and a checkpoint taken at a firing resumes to the same bits
- [ ] Depth's cost on the whole body under this form, beside rate's, from the same bench in one job
- [ ] A compile-fail test for the misuse the form refuses, with the error code it declares

**Non-goals:** how the drive splits into rate and depth, which [research 23](../../research/issues/23-how-the-drive-splits-into-rate-and-depth.md) asks, and moving body 11 onto depth.
