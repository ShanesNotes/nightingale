# 10: Arena slice laws compile and initialize

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The slice's organ fragments compile on the contract. They are CV with valve modes and hydrostatic head per segment, FL Starling at every capillary bed with lymph, RN filtration and tubuloglomerular feedback, NS baroreflex with a set point and release and reuptake at every junction, the receptor bindings and the boundary routes. The body initializes at the 06:55 anchor moment.

Blocked by: 08, 09

Status: ready-for-agent

- [ ] The slice compiles with no law naming a site and no organ owning a pool
- [ ] Initialization meets every fixture anchor with recorded provenance
- [ ] Water, ions, moieties and energy close over the first accepted steps with the reference core

## Comments

From 03: owed here and not built: the renal scope (the `Renal` owner type exists), the pressure-gauge check, and per-edge modes and edge guards for valves. 03's modes are owner-wide, and a guard's zero kernel is whole-body. Solutes and their partial volumes are owed too, since 03 has water only. Reconcile two 03 shapes with the sketch. CV's circulation is an `EdgeRoute<Water>` with `transfer_along` in the crate, but a `FluidRoute` with `advect` in the sketch. Boundary water is named in the schema in the crate (`Schema::water_boundary`, `BoundaryPorts::water(name)`), but is fixed fields in the sketch (`intake_water`, `evaporative_loss`, `heat_exchange`). The BC/FL cell-water conflict is still open, and 03's benches leave cell water out with a stated reason. Sketch probes this ticket owns: lines 40 (E0603) and 327 (E0451).

From 04: `Impulse`, bolus delivery and nonlinear energy closure are not built, and `verify` is where energy closure goes. `ng_validate::ReferenceIntegrator` stops Newton on an absolute term of 1, which assumes rows near unit scale, so a row of order 1e-12 stops early; `ErrorScales` is uniform too. Give both row scales before the slice's mixed units run on it. The first accepted steps here are the first run of a body with mode variables.

Decided after the deep review of 02 to 04 (nightingale#13, findings 12 to 17): finding 14: `deviation` returns its own dimension, so a set point can never become the variable it guards. This lands before this ticket's laws are written. Finding 17: each law records its rate class now. Whether a step longer than a law's longest interval is refused is designed here, after measuring what enforcement costs in speed.
