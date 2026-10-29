# Solver and core contract

## Problem

Nightingale needs one body model that either numerical core can solve without knowing an organ exists, and one record that accepts a step only after the whole step is checked. Four constraints make the shape non-obvious. Organ authors must state laws with no dt, solver or record in reach. The record must stay the only owner of accepted state (pulse 0006:70; pulse 0003:14). Every pace must give the same trajectory (pulse 0006:29). Conservation must close at the incidence step, not in an audit afterwards. This document is the synthesized design: laws are data compiled by the contract, with three ideas from the other candidate and the repairs the cross-vendor review required. It is a type sketch and a rationale, not an implementation.

Completion here means the standalone sketch (`solver-contract-sketch/`) passes `cargo check --all-targets` with warnings denied and every doctest: the three call sites below, the external signature walk (`src/signature_walk.rs`), and the compile-fail doctests with legal twins (`src/lib.rs`). The coupling census is counted by the compiler. It does not mean either core already meets the 48 h certificate, the accuracy gate or the speed bar; the arena measures those. Revision r1 adopted findings from an outside review of the diagram; [solver-contract-review-r1.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/architecture/solver-contract-review-r1.md) has the triage. The slice is pulsatile CV, FL, RN, the **minimal NS baroreflex**, the receptor layer and the boundary, 48 simulated hours, synthetic numbers.

Binding constraints: `docs/srs.md`, whose hard rules carry pulse 0002 to pulse 0004 and pulse 0006's contract picks, and [pulse 0006:20–35](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0006-first-milestone-stable-healthy-body.md#L20-L35). The record owns accepted state; observation never writes physiology; there is no output setter, private material store, numerical-fault death, pace-dependent trajectory or runtime dependency on the Pulse path. Contract-A means `ng-contract`, contract-B `ng-core-contract`; solver-A is the compiled conservation graph, solver-B the beat-synchronous ledger stepper. The pairs never mix.

Sources: **CA** = [pulse contract pass 1, candidate A](https://github.com/ShanesNotes/nightingale-pulse/tree/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/a-unknown-model/ng-contract); **CB** = [pulse contract pass 1, candidate B](https://github.com/ShanesNotes/nightingale-pulse/tree/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-contract-pass-1/raw/b-unknown-model/ng-core-contract-draft1); **SA**, **SB** = pulse solver pass 1, [candidate A](https://github.com/ShanesNotes/nightingale-pulse/tree/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-solver-pass-1/raw/candidate-a) and [candidate B](https://github.com/ShanesNotes/nightingale-pulse/tree/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/council/2026-10-26-solver-pass-1/raw/candidate-b); **FU** = `~/lanes/logs/ng-q1-frame-followup.md` (outside the repo, sha256 `c79e0e58b35e7f6be618cd23c2a760173201be02416633b930b89dae3faaeb10`); **MAP** = `docs/atlas/coupling-map.md`.

## Usage (caller's view)

These three call sites are the specification the sketch was derived from; each compiles as a doctest. No fragment or kernel receives a record, solver, interval, dt, wall clock or RNG cursor. The sketch bodies are `todo!()`.

### (a) The RN author declares the kidney

```rust
use solver_contract_sketch::ng_contract::*;
use solver_contract_sketch::ng_renal::{filtration_law, RenalParameters};

fn kidney<'b>(scope: RenalScope<'b>) -> Fragment<'b, Renal> {
    let (mut laws, ports) = scope.parts();
    let parameters = RenalParameters::declare(&mut laws);
    let inputs = ports.filtration_inputs;
    let rate = laws.kernel(|graph| filtration_law(graph, inputs, &parameters));
    laws.transfer(Transfer::new(ports.filtration, rate));
    laws.finish()
}
```

The scope grants renal capabilities once; RN never sees a pool name. `ports.filtration` names both registry endpoints of the glomerular-water route and cannot be split, used by a CV law, or built from a donor alone. `laws.kernel` opens one closed expression graph with a fresh lifetime, so nothing read inside it can leak into another kernel. `RenalParameters::declare` declares parameter slots (Kf, feedback gain) by key; their values arrive later from a bound packet. Coupling-map arrows come from the generated catalog: RN emits tubuloglomerular feedback with `laws.emit::<TubuloglomerularFeedback<_>>()` and defines each field once, and it can `receive` only arrows the map draws to RN. There is no renal `post(rate * dt)` or `evolve`; a memory slot's rate is declared with `memory_rate`, and the contract integrates it. The compiler refuses this partial fragment in a full renal schema because other laws are missing; it is a valid filtration-bench declaration.

### (b) The runtime steps a session: accept, refuse, fault, checkpoint, resume, rewind

```rust
use solver_contract_sketch::ng_contract::*;
use solver_contract_sketch::ng_record::*;

fn advance<'m>(session: Session<'m, Running>)
    -> Result<(Session<'m, Running>, Receipt), Session<'m, Faulted>> {
    let mut attempt = session.step();
    loop {
        attempt = match attempt {
            Step::Accepted(session, receipt) => return Ok((session, receipt)),
            Step::Refused(session, refused) => session.retry(refused),
            Step::Faulted(stopped) => return Err(stopped),
        };
    }
}

fn save_resume_rewind<'m>(session: &Session<'m, Running>, codec: &dyn CheckpointCodec,
    body: &'m CompiledBody, solver: &'m dyn CoupledSolver, history: &dyn HistoryStore,
    fork: BranchId) -> Result<(Session<'m, Running>, Session<'m, Running>), CheckpointError> {
    let saved = session.checkpoint();
    let bytes = codec.encode(&saved)?;
    let resumed = codec.decode(&bytes, session.record().identity(), body, solver, history)?;
    let forked = session.rewind(&saved, fork)?;
    Ok((resumed, forked))
}

fn after_fault<'m>(stopped: Session<'m, Faulted>, earlier: &Checkpoint, fork: BranchId)
    -> Result<Session<'m, Running>, CheckpointError> {
    let _diagnosis = stopped.export();
    stopped.recover(earlier, fork)
}
```

`step` runs `next_step`, `solve`, `PatientRecord::validate` and `publish` inside the session. A refusal means nothing changed: amounts, stream positions, commands, pending inputs and history are untouched, and `retry` solves the same base over a strictly smaller interval, never below the resolution's declared minimum. An `EngineFault` from any stage, including an exhausted retry budget, latches the session as `Session<Faulted>`, which has no `step`. It can be inspected, exported for diagnosis, or continued only by `recover`, which opens a new branch with ancestry from an earlier checkpoint. How the product presents a stopped session stays open (nightingale-pulse#293). A checkpoint is the whole accepted image, and an observer checkpoints its outputs beside it; `decode` resumes the branch head, and `rewind` forks without resetting the parent.

### (c) The arena scores a 48 h run

```rust
use solver_contract_sketch::ng_contract::*;
use solver_contract_sketch::ng_validate::*;

fn measure(body: &CompiledBody, solver: &dyn CoupledSolver, cores: &dyn CoreFactory,
    initializer: &dyn Initializer, spec: ArenaSpec, policy: &CertificatePolicy, cases: &AccuracyPolicy)
    -> Result<ArenaScore, ArenaFailure> {
    let checked = accuracy(body, solver, initializer, cases).map_err(ArenaFailure::Accuracy)?;
    let replay = replay_gates(body, cores, initializer, &spec)?;
    let run = run(body, solver, initializer, spec, Pace::Unbounded)?;
    let certificate = certify(&run.record().conservation(), policy)
        .map_err(ArenaFailure::Certificate)?;
    score(certificate, checked, run.into_evidence(), replay)
}
```

`ArenaSpec` accepts only a `ParameterPacket<SyntheticFixture>`, so the arena is nonclinical by type. `accuracy` runs the same core on manufactured cases with known answers and against the contract's reference integrator, because a step can close every ledger and still be wrong; a score needs its evidence. `certify` takes one `ConservationView` lent by one record and policy data; it cannot take a vital, a waveform or two histories. Wall time and replay evidence are collected beside the record, never inside the certificate. Both cores receive the same compiled body, fixture packet, boundary timeline and protocol.

## Shape

### Data first

The dominant structures, in the order data flows:

1. **`Anatomy`**: sites, each of one site class, with containment, flow networks, enclosures, served-by edges and positions. `Schema::new` derives the pool registry from it. Organs are regions of it, not owners of it.
2. **Fragments** from organ crates: capabilities, closed kernels (whole-body or per site class) and parameter slots, branded by assembly and owner.
3. **`CompiledBody`**: dense immutable arrays (pool registry, paired transfer columns, balanced reactions, work claims, one expression program, equation rows, guards, dependency blocks, CSR pattern, conserved basis, parameter layout) and a model hash.
4. **`BoundParameters`**: packet values in compiled order, with provenance and the packet's evidence manifest.
5. **`PatientRecord`**: the accepted image (state, solver continuation, last dense segment, stream positions, pending inputs, command cursor) and one sealed history of ledger chunks. No observer state.
6. **`CoupledProblem`**: body, values, identity, base seal and boundary view, lent by a running session for one step.
7. **`StepProposal` → `NextImage` → `Receipt`**: one transaction.
8. **`ConservationView` → `Certificate`**.
9. **`ObservationLens` → `ObservationView` → `Observer`**: a lens compiled against the body but hashed apart from it, a read-only view of one accepted revision, and an observer that holds its own state.

The paths are direct: anatomy → schema; organ → fragment → compiler; solver → problem → proposal; record → validate → publish; record → view → certificate; record → lens view → observer. No later ad hoc map is needed in the hot loop (per foundational-thinking and model-the-domain).

**Interface depth.** A solver sees one problem of about a dozen methods, a workspace, a quadrature and a dense-output builder. Behind them sit dependency analysis, sparsity, block partitioning, incidence, extent reconstruction, the conserved basis, identity hashing and history sealing; no core re-implements any of them. Organ authors see scopes, kernels and the catalog, never layouts, rows or pools. Validation lives at the boundaries (per boundary-discipline): `bind` parses packets, `BoundaryPort::new` parses timelines, `decode` checks identity before any body byte, and `validate` decides acceptance once; inside, typed values are trusted. The contract deliberately has no organ callback, no record access from a law, no dt in a law, no amount or output setter, and no conversion between failure kinds.

### 1. Organ declarations are consumable, typed data

`ng-renal`, `ng-cardiovascular`, `ng-fluids`, `ng-nervous` and `ng-receptor` return `Fragment<'b, Owner>`; there is no runtime `Organ` trait and no organ object survives compilation. This keeps CA's owned handles and deterministic registry but replaces string lookups and runtime bind errors with assembly-scoped capabilities (CA/src/id.rs:26, CA/src/organ.rs:88–155), and it adopts CB's complete-law coverage, including an explicit zero kernel for an inactive mode (CB/src/numerics.rs:87–100). An omitted law is not zero.

**No organ owns a pool**, so no organ can name one: the `(site, species)` registry belongs to the body and its amounts to the record (MAP:7–20). An endpoint capability refers to the registry under one law's authority; it carries no amount and no public pool id (`PoolRef` is private). `Assembly` issues each scope once under an invariant lifetime brand. `Pair<Owner, Q>` needs both ends with one species and unit. `StateSlot<Owner, U: NonMaterial>` excludes water, ions, mass and energy.

**Kernels are closed expression graphs**, not callbacks. `FragmentBuilder::kernel` hands its closure a `Kernel<'b, 'k, Owner>` whose `'k` is fresh and invariant (`src/ng_contract/law.rs`), so an `Expr` from one kernel, finished or still open, does not type-check in another. The closure runs once, at declaration, and only builds the graph; no solve ever calls it. Reading a port adds the dependency; the dependency set is the structural sparsity, so there is no hand-written sparsity list and no way for dependencies and output to describe different programs. Operators carry unit signatures from a pinned, versioned instruction set. An `Expr` carries no value, so a kernel cannot call host math; transcendental operators such as `exp` are the contract's own routines (§3, replay). A future primitive needs a contract version, not an escape hatch.

**Admissible is not normal.** A trial can be refused for leaving a set in only three places: a pool's amount below zero, a memory or algebraic slot outside its `Admissible` set (one of `finite`, `nonnegative`, `positive` or `unit_interval`; there is no constructor with bounds), and an operator's own precondition, such as a logarithm's positive argument. A kernel's output takes no set, so a sign constraint cannot be put on a computed expression such as `pressure − reference`. Refusal is followed by a smaller retry, as SUNDIALS does with sign constraints (IDA §5.2.2.4, ARKODE §2.2.17). Where a law's form is trusted is a separate `ValidityRange` with its basis, declared with `FragmentBuilder::validity`. An accepted state outside it publishes and the receipt records a `ValidityExcursion`; it never refuses or faults. Types close the direct routes. A law can still encode a threshold through an operator's precondition or a slot's dynamics, so `CompiledBody::refusal_sources` lists every source with its law and `NumericalHealth` counts refusals per source. A deteriorating patient keeps publishing until physiology, not the solver, ends the run (pulse 0004).

**Modes are declared values.** `FragmentBuilder::modes` declares one discrete mode variable of the owner and returns its values. `Mode::only` gives the `ModeSet` a kernel holds in, and `Mode::to` gives the `ModeTransition` a guard fires; it refuses the same value or a value of another variable. `Guard::new(zero, crossing, transition)` keeps the zero kernel's own dimension (a valve's guard is a pressure head), and `event` declares it. Compilation requires each guard's kernel to hold in the mode it leaves, and initialization picks starting values consistent with the guards. The signature walk declares a valve that opens on a rising head and shuts on a falling one.

**Anatomy is data, and laws are written per site class.** The schema is an `Anatomy` (`src/ng_contract/anatomy.rs`): sites, each of exactly one class from a closed set (`CapillaryBed`, `VesselSegment`, `Junction`, `EnclosedSpace`, `TissueSite`, `NerveSegment`), and five relations over them. Containment is a tree whose named nodes are regions, so an organ or a territory is a set of sites and its totals are exact sums. Flow networks (vascular first, then lymph, airway, ducts and CSF) are directed graphs in which beds in series (glomerular then peritubular) and loops (the circle of Willis) are ordinary edges. An enclosure surrounds beds and answers the volume inside it with one `Compliance` law: rigid, elastic, or `Held` by a named stand-in, which compilation links to that card's `HeldDeclaration` and coverage entry. A served-by edge joins a site to the nerve segment it reports to, with its `Afferent` kind and block and lesion parameters keyed by the edge. Every site has a `Position` in the body frame, so hydrostatic head follows from posture in one place. `AnatomyBuilder::finish` refuses an unreachable or undrained bed, a site with two enclosure chains or no position, a containment cycle and an orphan site. A finer map is a new anatomy and a new model hash; no law changes. A law is declared once for a class. `FragmentBuilder::site_kernel::<C>` builds a `PureKernel<.., C>` whose graph reads its own site's entry of a `PerSite<C, U>` port (`Kernel::at_site`) and site-keyed parameters. `transfer_at` attaches it to a `SiteRoute<C, Q>`, one column per site. A capillary-bed law cannot read a junction's entry, and a whole-body kernel cannot read a site entry at all. FL's Starling exchange is one declaration for every bed, NS releases and clears transmitter at every junction, and CV reads each vessel segment's hydrostatic head. Each bed's extravascular pressure comes from its enclosure, which replaces the separate pleural and abdominal wires. `Representation::Reduced { region }` marks a grouped region's law with the region it stands for. `CompiledBody::aggregation(region)` gives a region's exact book totals over its sites, in compensated arithmetic and with no new state.

**Refinement.** A grouped region may be replaced by its subtree only when all of these hold:

- the port sets are equal and the subtree's aggregates obey the same books;
- port behavior matches within a declared envelope on manufactured cases, and loop margins hold;
- sensors on the region are remapped;
- only the anatomy and the packet change, and the whole-body regression passes.

 A refined anatomy states this as a `RefinementRule` (`AnatomyBuilder::refines`): the grouped and replacement regions, `SplitRule`s whose children each carry their own weight and derive their values as `Derived` identities of the parent, the sensor remapping, and a `PortEquivalence` naming the ports, envelope, manufactured histories, tolerance, loop margin and regression. The rule is a request for that evidence, never a certificate that it ran. With no new data, a refined body starts equivalent to the grouped one. A refined body is a new model; there is no mid-run zoom. The port-equivalence gate is built at the first deepening after the milestone; the data shape affords it now.

**Instruction set v1.** `multiply` and `divide` return `Product<A, B>` and `Quotient<A, B>`, which become a named dimension only through the sealed `SameDimension` table (`Product<Elastance, Volume>` is a `Pressure`); volume times volume cannot be cast to a pressure. `scale` multiplies by a dimensionless factor. `smooth_max` and `smooth_min`, `0.5 (a + b ± sqrt((a − b)² + w²))` with `w` a declared parameter, give limits and waterfalls a continuous Jacobian. `logistic` is built from the pinned `exp`, and `ln` lists its positive argument as a refusal source, like `divide`'s divisor. Each operator uses only correctly rounded IEEE operations and the contract's own routines, so it replays bit for bit.

**Set points are their own dimension.** `FragmentBuilder::set_point` declares slow memory of dimension `Reference<U>`, in the unit of the variable it guards (a pressure, a temperature, a stretch). It is `NonMaterial`, so it is memory like any other and moves by `memory_rate`, but it is not `U`: no law can add, subtract or relax across the two. Only a `Sensed` dimension (dimensionless, pressure, volume, temperature, length) has a set point, and a `Reference` is never sensed, so no guard fires on a set point itself. `Kernel::deviation(measured, reference)` is the one operator that compares them. History and disease shift set points, gains and states; nothing writes the guarded variable.

**Chance lives in the laws.** A scope mints a `Stream` under a stable key (owner and name), which enters the model hash, so adding a stream renumbers no other. `Kernel::draw(&stream)` reads the stream's value for its current physiological event ordinal, uniform on (0, 1) and constant until the next event; a beat or breath guard declared with `Guard::advances(&stream)` moves it to the next ordinal when the image publishes. A hazard is memory whose rate physiology and treatment set. Its `HazardGuard` fires when the integral reaches the stream's exponential draw, and on publication the contract resets the integral, advances the stream and applies the mode change together; a refusal discards all three. Better care lowers the rate, so the same dice give a later event or none. Smooth variability tied to the clock is a `Forcing`: knots every declared period with seeded values, interpolated by a cubic B-spline, a pure function of seed, key and knot index. Knots are not breakpoints, so a forcing never cuts a step and replay holds at any pace or step policy. Variability modulates rates and never injects an amount. The solver holds no dice: `CoupledProblem` has no `threshold`, and stream positions live once, in the accepted image.

The forbidden moves fail to compile (`src/lib.rs`; each probe added since the first eight has a passing legal twin beside it, except the publish one, whose legal path is `Session::step`):

| Move | Error |
|---|---|
| Name or hold a pool (`PoolRef`) | E0603 |
| Write a vital (`set_mean_arterial_pressure`) | E0599 |
| Transfer from a donor alone | E0308 |
| Read another crate's internals | E0603 |
| Anchor without a citation | E0061 |
| Convert an engine fault to demise | E0277 |
| Hide material in private state | E0277 |
| Use another owner's route | E0308 |
| Carry an expression into a later kernel / into a kernel open inside another | E0521 / E0521 |
| Read a coupling the map draws to another card | E0271 |
| Certify one record's inventories against another's ledger | E0599 |
| Step a faulted session | E0599 |
| Publish outside the session | E0596 |
| Put a synthetic anchor in a reviewed packet | E0308 |
| Fire another owner's mode transition | E0308 |
| Pass bytes as split qualification | E0308 |
| Give a refusal domain bounds (a reference range) | E0599 |
| Put a sign constraint on a computed expression | E0061 |
| Build a split trace from invented samples | E0599 |
| Hold a card without declaring what it supplies | E0061 |
| Book extents as one sampled rate times the step | E0451 |
| Write a reaction from raw stoichiometry | E0451 |
| Call host math on an expression | E0599 |
| Read a junction's entry in a capillary-bed law | E0308 |
| Use a set point as the variable it guards | E0308 |
| Name a product by cast (volume times volume as a pressure) | E0277 |
| See the boundary timeline from a core | E0599 |
| Fire a guard on a set point | E0277 |
| Draw chance on the solver seam | E0599 |
| Draw measurement error from the run seed | E0308 |

`Assembly::compile` performs SA's §3.3 checks (SA/solver-design.md:549–578): resolve scopes and endpoints in canonical key order; check units and reference conditions; require exactly one owner per state, process and algebraic row in every reachable mode (algebraic loops become coupled residuals, replacing CA's DAG-only refusal); generate signed incidence for every transfer, exact element and charge stoichiometry for reactions and one debit/credit route per work claim; check domains, scales, modes and rate classes; check structural matching and pressure gauges without fake epsilon storage; derive sparsity, blocks and deterministic order; derive the conserved basis; compile CB's PK uniqueness rule, one owner per `(site, ligand, role, pathway)` regardless of representation (CB/src/pharmacokinetics.rs:47–101); check every held stand-in's declared supplies against its fragment's exterior columns and its intensive fields against the stores in the slice; refuse an orphan pool that no law touches and no declaration holds; and hash the normalized model, held declarations included. Parameter values are not in the model hash; they are bound later by key. Structural matching is not numerical rank; initialization and evaluation report the offending rows.

| Module / proposed crate | Knowledge owned | Allowed body dependencies |
|---|---|---|
| `ng-contract` | Anatomy, declaration language, compiler, catalog, units, PK claims, problem and seam, admission system, split measurement, observation lenses | None |
| `ng-cardiovascular`, `ng-fluids`, `ng-renal`, `ng-nervous`, `ng-receptor` | Constitutive and binding laws over granted capabilities | `ng-contract` only |
| `ng-solver` | Newton or beat algorithm, scratch, event location, error control | `ng-contract` only |
| `ng-record` | Accepted image, validate and publish, session latch, history, checkpoint, observation view | `ng-contract` only |
| `ng-observe` | Observers: windows, delays, measurement error and findings through a lens, one revision at a time | Contract and record, read only |
| `ng-validate` | Synthetic composition root and stand-ins, certificate, arena | Contract and record |

The sketch is a standalone crate outside the workspace. No oracle, pulse-io, prost, provider SDK, I/O, threads, clock, unsafe or host-libm in the physiology closure.

### 2. Coupling and conservation at incidence

For every accepted amount, `x_next = x_base + B·J + S·ξ`; exterior columns also have two ends, the exterior side being a receipt rather than a negative pool (CA/src/ledger.rs:148–168; SA/prototype/src/model.rs:300–308). A backend cannot supply amount deltas: `CoupledProblem::proposal` rebuilds amounts from the start plus the paired extents in deterministic column order. There is no clipping, clamping or balancing injection. The record keeps each amount, and the history's cumulative gross exchange, as a compensated pair of f64 (`Compensated`, two-sum); solver workspaces stay plain f64. Measured as end-of-run closure over 48 h with synthetic magnitudes (`evidence/ledger-rounding.txt`), plain f64 stayed within 3.2e-10 of inventory, but against the exterior exchange its error grew as that exchange shrank relative to the pools' turnover, up to 29 times the exchange when it was 1e-21 of a pool per step. The compensated ledger stayed within 1.1e-13 of the exchange in every case. An i64 ledger with a fractional carry per column, as the review proposed, closes exactly and drifted at most 2.8e-4 of the exchange from the compensated trajectory, at that smallest ratio. It is the measured alternative; compensated pairs are preferred because they need no quantum or range policy per species and no carry state (review r1, F25). Vascular volume is derived from the same water, solute and carrier amounts that advection moves, through fixed declared partial-volume coefficients; `FluidRoute` expands every carried species and heat once from the schema. Pressure is an algebraic constitutive output. Energy is a book: chemical, thermal, elastic and exterior-work accounts are paired by `WorkPair` and heat routes, and elastic energy's potential closure sits in the DAE, so a scalar work cost cannot debit two stores. Pairing proves bookkeeping, not the adequacy of a constitutive law; CB's `LinearInventoryAudit` rejects algebraic states (CB/src/ledger.rs:190–217) and cannot validate this slice.

**The coupling catalog is generated.** `src/ng_contract/catalog.rs` declares each of the 38 arrows of MAP's direct-couplings graph (MAP:77–114) once. One macro emits the census row and, for a mapped arrow, a payload struct carrying every field its label names: line 78 carries myocardial work and flow per bed, line 90 O2 debt per bed and temperature, line 114 bleeding competence, viscosity and thrombosis. A payload is generic over its side: the emitter gets one `Output` per field to define once, the receiver one `ReadPort` per field, and `receive` requires the map's `To` owner. The compiler checks `38 = 27 mapped + 11 listed unmapped`, `28 slice-incident = 27 + 1`, and that rows are lines 77–114 in order.

| Disposition | Map lines | Count |
|---|---|---:|
| Live: both ends in the slice | 77 CV→NS, 80 CV→FL, 97 RN→CV | 3 |
| Input: held stand-in emits into the slice | 83, 90, 91, 99, 100, 102, 105, 106, 108, 114 | 10 |
| Anatomy relation: an enclosing space's pressure, or site positions under posture, not a wire | 82, 96, 101 | 3 |
| Sink: slice emits to a held receiver that consumes no physiology | 78, 79, 81, 86, 87, 88, 89, 98, 110, 112, 113 | 11 |
| Outside: neither end in the slice | 84, 85, 92, 93, 94, 95, 103, 104, 107, 109 | 10 |
| Unresolved: FL→NS "brain water and ICP" | 111 | 1 |

"Mapped" proves the route has a declared home, not that the outside system is simulated. `Assembly::hold::<S>(declaration)` gives each card outside the slice a scope pre-filled from its catalog rows; the composition root adds receipt pools and, for EN, surrogate ligand routes. The `HeldDeclaration` says what the stand-in prescribes, every route on which it supplies or removes material or energy, the catalog rows it does not answer and where it is meant to hold. Material cannot arrive silently, because every route is a paired column; the declaration makes the support visible, enters the model hash, and the certificate reports each supply route's flow per window. Support can also be intensive: a held concentration or temperature can stand in for a store while the inventory behind it changes. A `HeldField` that stands for a store is therefore either derived from that store inside the slice (`compile` refuses a held one) or declared an external reservoir of its book. No minimal NS respiratory, motor or hypothalamic law is invented to fill a sink. `tools/verify.d/20-sketch.sh` compares the rows with MAP line by line (`solver-contract-sketch/evidence/census-check.py`).

Shared substrates are a separate denominator, not extra arrows: S1 one vascular registry with RN bed routes and FL Starling and lymph; S4 registered Na, K, Cl, Ca, Mg, P and weak acids with RN reclamation; S5 renal glucose and nitrogen cargo; S6 paired heat advection and mechanical work with the thermal network held external; S7 ligand pools, one PK register and the receptor layer; S8 water and osmoles per space with one ledger; S9 fixed carriers; S10 NE and ACh release → junction pools → receptors → CV and RN laws (MAP:11–20). `ng-receptor` declares per site what is dynamic and what is approximated: binding is a dynamic balanced reaction, transduction from occupancy to effect is algebraic (a declared approximation), and desensitization is declared absent at rung 0. Six boundary route groups are new relative to MAP and not counted among the 38: intake delivery, urine exit, evaporative loss, posture geometry, environment and clock-and-sleep, all on one timeline.

Conflicts stay visible, not settled: MAP:11 gives each card its bed's edges while the CV card claims every vascular edge and RN the arterioles (provisional: RN owns renal bed laws, CV the assembled hemodynamic closure; TGF is a typed contribution, not a second writer). CV and EN both claim natriuretic secretion (the slice exports wall stress to the held EN). MAP:111 puts ICP on FL→NS while NS owns ICP (whole arrow unresolved). FL owns an I&O ledger while pulse 0006:31–33 requires one body ledger. BC and FL overlap cell-water shifts.

### 3. One numerical seam, two cores, no adapter

Contract-B's `CoupledSolver` and `Initializer` remain the seam (CB/src/numerics.rs:31, :292), revised: `CoupledProblem` holds the compiled body and **bound parameter values**, not `&[&dyn Organ]`; every trait method is required, with no default bodies to hide a gap; both cores implement them in `ng_solver.rs`, importing only the contract. Every value a core produces or consumes has a public producer and consumer; `src/signature_walk.rs` proves it from outside the crate. It walks the anatomy, a set point, a guarded mode with seeded chance, one law for every capillary bed, compile, a boundary timeline and initialize. It then walks a Newton stage, a block stage, quadrature, proposal, validation, step and latch, checkpoint and resume, split measurement, certificate queries, the accuracy gate and observation through a lens.

**Gross extents only from evaluated rates.** `Quadrature`, made by `problem.quadrature(&ticket, plan)` and bound to that ticket and evaluation plan, accumulates weighted `StageEvaluation`s from one compiled layout and is the only producer of `Extents` and `MemoryIncrement`. Backward Euler is one evaluation at weight 1; a beat core's bins are block evaluations whose weights sum to one per column; SDIRK uses its stage weights. A stage time carries the digest of the ticket that minted it, so an evaluation from an earlier attempt is refused. Amounts and memory are the start plus the increment, never the solver's own coordinates, and the core's end candidate supplies only algebraic values and modes, which validation rechecks. Forward and reverse parts of every column stay apart, so the ledger keeps gross in and out. A mismatched layout or a column not covered over the whole interval is an `EngineFault::BindingMismatch`, a solver bug. The seam therefore admits exactly the methods whose accepted increment is a nonnegative weighted sum of rate evaluations at stages inside the ticket: backward Euler, step doubling, implicit Runge–Kutta tableaus with nonnegative weights, and backward-Euler substeps. A linear multistep (BDF) update carries a history term from earlier intervals, and a Rosenbrock update, though one-step, adds Jacobian terms to its stages, so neither is such a sum; either needs a contract version. Both prototypes fit (SA engine.rs:440–460 step-doubles backward Euler; SB network.rs:447–456 bins backward-Euler steps). Runge–Kutta methods preserve linear invariants (Ketcheson 2019, arXiv:1905.09847 §5), which is why a weighted stage sum can carry the ledger.

**The record checks the method, not only the books.** Pairing makes the ledger equal the published update, but on its own it would let a core book one sample at the start while its endpoint and error estimate came from other stages, or evaluate blocks at stale coordinates under the coupled plan. So the quadrature also builds a `StageWitness`: every stage's time, point (y, y'), recorded rates, weight and block, and the plan's digest. Each `EquationBlock` carries the plan that issued it, and the quadrature refuses a block from another plan, so a witness cannot claim a partition its stages did not use. Each `StageEvaluation` carries a copy of the point it was evaluated at, so the witness outlives the solver's workspace. The resolution declares its `StageScheme` (stage coefficients, nonnegative weights, embedded error weights, or substeps) and its `ErrorScales`. `validate` re-evaluates each witnessed stage on the compiled body in the scope it names: the whole, or that block of the witnessed plan. A recorded rate that differs, or an increment that is not the scheme's weighted sum, is the fault `MethodMismatch`. Unmet stage equations are refused as `StageResidual`. The scheme's error estimate is recomputed against the scales, refused above one and a fault at the floor. Lag, single-sample booking and excess error at the floor are thus caught at publication, where they happen. The cost of the extra evaluation per stage counts in the 100× measurement.

| Seam method or accessor | Solver-A source and use | Solver-B source and use | Change |
|---|---|---|---|
| `CoupledSolver::resolution` | engine.rs:8 `Config`; checkpoint.rs:143 saves it | stepper.rs:42 `Policy`; network.rs:155 `FastPolicy` | Hash method, config and continuation schema into `DeclaredResolution`; never pace |
| `CoupledSolver::next_step` | engine.rs:359–374 horizon, :412–419 next step | stepper.rs:207–224 beat remainder, next order | Drop target clipping (SA:363) and stop markers (SB:568–579; run.rs:108–123) |
| Breakpoint clipping (inside `CoupledProblem::ticket`) | engine.rs:363–369 queue and CPR boundary | stepper.rs:209–211 next order | Physical inputs and delivery ends only; the contract clips, no core sees the timeline |
| `CoupledProblem::ticket`, `StepTicket::at` | engine.rs:370–374 `h` from next step and horizon | stepper.rs:212–219 remaining beat | The core asks for its intrinsic end; exact integer start and end; stage times only inside |
| `CoupledSolver::solve` | engine.rs:440 doubling, :462 Newton | stepper.rs:261 attempt, :274 sweeps; network.rs:309, :369 | Replace fixed arrays and Network/Slice/Organ calls with the rows below |
| `CoupledProblem::workspace`; `SolveWorkspace::{unknowns_mut, derivative_mut, point, candidate}` | engine.rs:464 `let mut y = old` | network.rs:317–318 explicit predictor | The method owns y and y′ in one fixed layout |
| `CoupledProblem::evaluate` | engine.rs:469; model.rs:250 | network.rs:325–326 flows and rates | Reads bound parameter values; scope whole or one block |
| `CoupledProblem::linearize`; `Jacobian::view` | engine.rs:493–502 dense `1 − dt·J`; model.rs:288 Jet | network.rs:344–355 finite differences, `solve_dense` | Sparse CSR over the dependency envelope; `UnknownId::index` is a column's workspace slot and view row `r` is residual `r`; `Shift` is 1/h |
| `CoupledProblem::blocks`; `EquationBlock::{rows, unknowns, rate_class}` | none: one coupled system | stepper.rs:276, :301, :325 hand-built lanes | Dependency-derived; a split needs a measured qualification |
| `Quadrature::{add, finish}`, `StageWeight` | engine.rs:475 `dt·flux`; :443–445, :455–457 doubled halves summed | network.rs:453–456 `bins += h·q`; slice.rs:187 `rate·dt` | Only producer of extents; gross directions kept |
| memory increment (inside `Quadrature`) | engine.rs:477 `old + dt·rhs` | slice.rs:239–249 reflex backward Euler in `evolve` | Quadrature of memory rates; no organ integration |
| `CoupledProblem::dense_output`; `DenseOutputBuilder::{node, finish}` | none | network.rs:466–468 waveform push; stepper.rs:419 | Bernstein segment; observer samples the grid |
| `CoupledProblem::transitions` | engine.rs:385–404 guard bisection | stepper.rs:358–375 transitions at the boundary | Stable guard ids, integer-ns bracket, event ordinals |
| `CoupledProblem::proposal` | engine.rs:476 `inventories_from_flux`; :482–485 recheck | stepper.rs:394–397 commit and `closure_check` | Amounts from extents; acceptance only in the record |
| `PatientRecord::validate`, `publish` | engine.rs:406–409 writes state and ledger in place | stepper.rs:228 orders before attempt; :237 `*patient = candidate` | Immutable validation of the whole image; one infallible publish |
| `Initializer::initialize`; `InitializationSystem::{workspace, evaluate, linearize, admit}`; `InitializationRequest::new` | engine.rs:216–233 checks a supplied state | slice.rs:385 hardcoded start | Admission system with calibrated unknowns; contract mints provenance |
| `Session::{step, retry}`, `Session<Faulted>` | engine.rs:310, :328 return `Err`, stay callable | run.rs:108 returns `Fault`, `Run` usable | Typestate latch; recovery only by branch |
| `ConservationView::segments`; `HistorySegment::{basis, inventory, inflow, outflow, held_inflow, held_outflow, impulses}`; `Impulse::{at, quantity, amount}`; `CoverageManifest::held_routes` | engine.rs:271, :279 balances and totals | run.rs:79–106 trace samples; :175 windowed mean | One sealed view; dense segments, never samples |
| `PatientRecord::split_trace`; `SplitMeasurement::qualify` | none | stepper.rs:376–394 coupling change between sweeps | Four-family metric plus reference error (CB/src/numerics.rs:122–260) |
| `CheckpointCodec::{codec_digest, encode, decode}`; `CanonicalCheckpoint::{as_bytes, from_bytes}` | checkpoint.rs:143–200, :201–358 own image | run.rs:75–77 clone; :138 fork | Record-owned postcard envelope; the host stores its bytes; identity before decode |
| `Rhs::rates`, `Integrator::substep` (bench) | CA/integrate.rs:18, :37, :94; model.rs:250 | network.rs:289; slice.rs:180 not dt-free | Heun on an explicit body only; no DAE relabelled |

File names in the table shorten `SA/prototype/src/…` and `SB/prototype/src/…`. These are migration mappings, not proof of behavioral compatibility. SB currently applies orders before its disposable attempt (stepper.rs:228), accepts at maximum sweeps without convergence (:387–388) and accepts fast minimum steps with excess error (network.rs:439); those paths must become refusals or faults before the arena. SA clips to caller output times and lacks a pulsatile heart. Preserving either behind an adapter would invalidate the experiment.

**Validate, then publish.** `PatientRecord::validate(&self, StepProposal) -> Result<NextImage, TrialFailure>` is immutable; a `Refused` is retried smaller, and an `EngineFault` from the method checks latches the session. The proposal still borrows the `CoupledProblem` it was built against, so validation rechecks closures with the same compiled body and parameter values. It checks identity, branch and base seal, the ticket's exact start **and** end (closing CB/src/patient.rs:449–453), due input ids, complete state, mode and closure coverage, admissible sets, amounts equal to base plus paired extents, the stage witness against the declared scheme, nonlinear energy closure, the method history's restart after a breakpoint, continuation schema and dense-segment endpoints. It returns the **entire** next image: amounts, memory, algebraic values and modes; fired events; solver continuation; the ledger chunk; the command-cursor advance and remaining pending inputs; the last dense segment; stream positions, advanced by the hazard guards that fired; and the new seal. `publish(&mut self, NextImage) -> Receipt` installs it and cannot fail. It is the only `&mut` path, and `&mut PatientRecord` is never lent outside `ng-record`; even input admission is a staged image. The brief's transaction typestate is these types: `StepProposal` (proposed), then `NextImage` (accepted, not yet visible), `Refused` or `EngineFault`, then `Receipt`. CB/src/patient.rs:431–565 is the base. A refusal therefore cannot follow a partial write, the failure nightingale-pulse#291 and `pulse-port/run.md:714` recorded.

**Failure kinds.** `Refused` is a retryable attempt with the record untouched and a strictly smaller ticket. `EngineFault` means the solver or model cannot produce a possible body under the declared policy. Retry-budget exhaustion, or a retry that would go below the resolution's declared minimum interval, is the diagnosed fault `NoAdmissibleStep`, not a `From<Refused>`; a core may not accept excess error at its floor (SB network.rs:439 does today). Because `validate` recomputes the error estimate from the witness, a core cannot accept excess error at its floor either; SB network.rs:439–449 does today and must not. A breakpoint may end an interval shorter than the floor. A fault carries the attempted interval and, per failing row, the law it came from (`CoupledProblem::diagnose`, `CompiledBody::origin`) and its residual, so a structural defect is named instead of retried; each refusal carries its diagnosis too, and `FaultReport` keeps the fault and the refusals before it. A law undefined at a state the trajectory approaches (division by a flow that reaches zero) shows as repeated precondition refusals ending in a floor fault that names the operator; a law that must hold in a supported regime has to be defined there, which the zero-flow case tests (§7). `Demise`, `IrreversibleInjury` and arrest rhythms exist as types (a hazard is law memory with a stream, §1), with no death-producing method and no `From<EngineFault>`. The session typestate makes "never continue after a fault" structural (nightingale-pulse#293 body; nightingale-pulse#291): `Session<Running>` becomes `Session<Faulted>`, which can be read, exported or recovered on a new branch and has no `step`. Accepted physiology stays intact; the stopped status is never shown as a live patient.

**Commands.** Durable receipts of authorized commands (`AuthorizedInput::from_command_receipt`) are the one command authority; they authorize ingress and are not a physiology store. `admit_input` is idempotent on `(id, payload digest)`: the same request returns its receipt, a changed payload is refused, and applied ids advance only with a published image. An input stamped before the accepted head is refused; the host stamps a click at the head or later. A crash before publication restores the previous envelope; after it, the new one.

**The causal step rule.** An interval is the core's own step, cut short only at the next breakpoint, and no step depends on an input beyond its own end. The types enforce it: a core asks `CoupledProblem::ticket(start, intrinsic_end)` for its own step, and the contract ends the ticket at the earlier of that end and the next physical breakpoint. No core can see the timeline, so a step controller cannot spread its steps toward a known later input. An input admitted mid-run through `Session::enqueue` therefore gives the same bits as the same input known from the start, at any pace. The arena checks it (§7).

**Identity, time and replay.** Run identity is model hash + parameter packet hash + seed + declared resolution; resolution covers algorithm, version, arithmetic profile, event rules, tolerances and partition policy, so two cores on byte-identical inputs have distinct identities. Pace lives only in the runtime and `ng-validate`; a reduced-detail run declares its own resolution. All boundaries are integer ns; kernels see an exact origin plus a bounded stage offset. Rate classes span wave, beat, reflex, minutes and hours as scheduling constraints, not organ lanes. A physical input or guard can end an interval; a UI refresh, pace change, checkpoint request or wall-clock delay cannot. A ticket that starts at such a breakpoint, or after a guard changed mode, says so (`StepTicket::after_breakpoint`). The core restarts its numerical caches there (method history, Jacobian, predictor, step controller), as CVODE does after a discontinuity (CVODE §3.4.3.13), and `validate` checks that the proposal's `MethodHistory` starts at that instant. Physiological state and the cumulative ledger never restart (pulse 0006), and under the quadrature rule no ledger term crosses the breakpoint. Guards localize to an integer-ns bracket with deterministic simultaneous-event order; chattering or unresolvable mode consistency is a fault. Chance is drawn in laws from keyed streams (§1), a pure function of the seed, the stream key and the physiological event ordinal, never of substep, attempt or evaluation counts; a refused proposal discards its ordinals. Three kinds of chance keep separate seeds, and `DomainSeed<D>` keeps them apart in the types. `PacketSeed` decides which patient; it is used when the packet is built, so its draws are inside the packet hash. `Seed`, the run domain, drives streams, forcings and hazards. `ObservationSeed` drives measurement error, and only an observer takes it. The domain enters every stream's key derivation, so equal seed words in two domains never alias, and stream positions are keyed by stream, not by number. A retry from a save keeps both seeds, a fresh course needs a new run seed, and a new patient needs a new packet.

**Observation is a lens, apart from the run.** `CompiledBody::lens` compiles an `ObservationCatalog` (channels and the output grid) into an `ObservationLens` with its own digest. The lens is not part of the run identity and the output grid is not part of the resolution, so a new lens applies to any replay and gives the same physiology with new findings. A `ChannelSpec` names the quantity and site it reads, its transformation (window and delay), an `ErrorModel` drawn from the observer's `ObservationSeed`, its `Access` class (device, examination, laboratory, imaging, teaching) and, optionally, a `ReferenceInterval` for flags. Reference intervals are observation annotations with evidence; physiology never reads them, and they are a fourth kind of range beside `Admissible`, `ValidityRange` and a population record. `Session::observation(&lens)` lends an `ObservationView` of the last accepted image, also after a fault. It gives the lens's channels, each channel's value at the accepted instant, and samples on the last segment. Its grid instants exclude the segment's start and include its end, so each instant falls in exactly one step. A lens from another body, or an instant outside the segment, is refused. An `ng_observe::Observer` applies windows, delays and error, holds its own continuation and refuses a skipped revision. Its findings carry collected and available times and a typed absence (not collected, pending, invalid signal), never a zero. The session never sees an observer, so physiology bits cannot depend on one. The view has no write path and exposes no layout: `Candidate` stays opaque. A chart entry that documents a drug links to the command receipt that delivered it; only that command moved the body, and notes are never parsed into physiology. Measuring acts that move matter or energy (a blood draw, a cuff occlusion) are boundary actions with removal routes, never observations.

**What replay promises.** Bitwise identity within one declared `ArithmeticProfile` at any pace and chunking; exact continuation from a checkpoint. Nothing is promised across profiles or other targets. The profile admits only correctly rounded IEEE 754 add, subtract, multiply, divide and `sqrt`, plus the contract's versioned transcendental routines; `mul_add` stays out because the determinism list in nightingale-pulse's [clippy.toml](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/clippy.toml) bans it with the transcendental functions. Rust documents `exp`, `ln`, `sin`, `powf` and the like as having precision that "varies by platform, Rust version, and can even differ within the same execution" (Rust 1.98.1 `std` f64 docs, "Unspecified precision"), which is why they are banned. The profile binds the whole closure, cores and initialization included, not only kernels: reductions run in a fixed order, and arena cores are single-threaded, so worker count cannot matter. A later parallel evaluation must keep that order. Rust cannot forbid a core from keeping a hidden counter; the resume gate, which decodes into a freshly built core, catches one.

**Split evidence.** A held or split block needs a `SplitQualification`, and the only producer is `SplitMeasurement::qualify` over CB's four-family metric (state, exchange integral, observable, event time; CB/src/numerics.rs:122–260). Each `SplitTrace` requires all four families on a shared grid, is bound to its run identity and comes only from `PatientRecord::split_trace`; the coupled reference's own error comes from `ReferenceError::self_convergence` against a refined reference, never assumed zero, and keeps the binding of the reference trace it was measured on, so `SplitMeasurement::new` refuses an error from another run. Neither convergence of a tone vector nor a small ledger residual qualifies a split.

**Checkpoint.** A checkpoint carries identity, branch ancestry, revision and seal, state, bound parameters (so resume never refits), solver continuation, stream positions, pending and active boundary deliveries (nightingale-pulse#238), command cursor and fence, history roots, and the last dense segment. An observer checkpoints its own continuation beside it (`ObserverCheckpoint`, bound to the seal it last read), so restore still brings back outputs as well as state (nightingale-pulse#273). Postcard body bytes are deterministic per writer and sealed with `sha256-raw-v1` (pulse ADR-68:12,27); only the header is RFC 8785 JCS (pulse ADR-70). The host persists `CanonicalCheckpoint::as_bytes` and reloads them with `from_bytes`, which checks nothing. Verify the stored bytes, never re-encode; check identity before decoding the body; reject trailing content. The envelope holds history roots, not chunks, so `decode` takes a `HistoryStore` that resolves each root to sealed chunk bytes, verified against the root before the record is rebuilt; a checkpoint is complete only with such a store. There is no compatibility claim for SA's `NGPROBE1` or SB's clone.

### 4. The admission solve, values as ranges, and evidence

`Specified<U>{anchor, value, unit, Citation}`, `Derived<U>{identity}` and `Calibrated<U>{guess, target anchors}` are distinct types. `Citation` needs a source, a page or table, and `FullText` or `SecondaryReproduction`; there is no from-memory variant, and a type cannot prove someone read the source. **The packet's kind decides its anchors:** `ParameterPacket<Reviewed>` holds cited `Specified` anchors; `ParameterPacket<SyntheticFixture>` holds `Synthetic` anchors and fixture design values, which have no citation and no conversion to `Specified`. Calibration targets refer to the same packet's anchors, so the synthetic arena is constructible without manufacturing citations. The packet keeps real values with provenance; `CompiledBody::bind` parses it against the compiled parameter layout, so evaluation reads values, never a digest.

`Initializer::initialize` runs once; this is the consistent-initialization problem of a DAE (IDA §5.2.1, after Brown, Hindmarsh and Petzold 1998). `CompiledBody::initialization` builds a square admission system: state unknowns plus calibrated parameters; consistency residuals plus one row per calibration target (plus periodicity rows for a periodic goal). The request carries goal, budget, guess and the 06:55 anchor moment. `InitializationSystem::admit` is the only constructor of `InitialRevision`; it rechecks every row and anchor, records Specified, Derived, Calibrated or Fixture provenance per value from the packet, and cannot move a specified conserved total. Run-in happens in disposable pre-admission scratch. The first accepted revision is the continuation point (SA/solver-design.md:566–570); there is no daily reset or post-meal re-anchoring. Admission solves the skeleton: every forcing is held at its mean and every stream at its first ordinal, and variability starts at the anchor moment.

**Three kinds of number.** A constitutive parameter (an elastance, a conductance, an unstressed volume) is bound once per run. An output (cardiac output, mean arterial pressure) is computed only by laws; no type makes it a parameter and none can set it. An anchor is a target an output must meet at admission, and it states its `AnchorStatistic` with `over`: an instant (the default), a cycle mean or a window mean. `bind` refuses an instant anchor on an output whose kernel has a wave or beat rate class, so an anchor on a pulsatile output is a cycle mean, and its admission row matches it under the periodic goal. Cardiac output is therefore beat-to-beat physics that emerges from stiffness, resistance and filling, and its range comes from the ranges of those parameters.

**Values come with their population and evidence.** A `PopulationRecord` gives a parameter's distribution (mean and SD, quantiles or an interval with coverage), the `EvidenceId`s behind it and the correlation group it varies with, so body-size scaling moves several values together (P18). An `EvidenceRecord` is one located claim in an admitted source. It holds a `SourceSpan` in the repository's source registry (SourceWork, Snapshot, Span; pulse ADR-61), the existing `Reading`, the `Claim` it supports (magnitude, range, shape or validity) and the statistic. It also holds the `Population` measured (description, count, covariates), the method, a `Certainty` and the rights grant. A textbook reproduction may support a shape; a magnitude or a range needs a primary record, or a reproduction that names its primary and locator, as P1 does. Records join to parameter keys in a manifest hashed into the packet, so laws stay pure and a display can join by key. A value drawn for a patient comes from `ParameterPacket::sampled`, which reads the population record from the packet's own manifest and the `PacketSeed`, and records `ValueProvenance::Sampled`; an authored value comes from `authored`, chosen inside its population record's range with its reason, so it carries that range and its evidence (I16). There is no way to pin a parameter to an anchor: an anchor is a target an output meets, never a parameter's value. `ValidityRange` rests on a `Basis`: an evidence id or a named fixture. Arena packets keep synthetic values with notes; evidence records land before the first reviewed packet, and rebuilding a packet from its records must give the same packet hash. A population-band check runs in validation over a virtual population, never inside the engine.

The brief says P1–P26 are sourced, but the supplied source column is incomplete; the design keeps that gap visible (FU:74–99). No physiological number was copied into the sketch.

| Anchors | Supplied source status | Consequence |
|---|---|---|
| P1 | Wang 2010 quoting Elia 1992 | Secondary reproduction naming both and the locator |
| P2–P6 | B/A anchor or identity references; MRI indirect for P2 | Resolve publication and locator before any `Specified` |
| P7–P13 | B/C section and identity references | Resolve underlying publications, not "B §2" |
| P14 | E identity/A note; Levitt 2016 named | Page or table, and which quantity each supports |
| P15–P17 | B/C identities and D | Unresolved |
| P18 | Poggio 2009 median and BSA scaling | Measurement specified; absolute scaling is `Derived` |
| P19 | Sund-Levander 2002, H rhythm sources | Distinct mean and rhythm sources; admission-hour temperature `Derived` |
| P20–P21 | D/H and §4 source families | Separate cortisol, AVP, GFR and autonomic records; several gains are model choices |
| P22 | Levine 2000, H[52] | Needs an inspected locator and reading status |
| P23–P24 | Q1 E/A identities | Underlying primary sources unresolved |
| P25 | Explicitly UNSOURCED | Cannot be a physiological anchor |
| P26 | Reference equations plus Lumb & Nunn 1991 | Seated reference, posture decrement and reading provenance kept apart |

FU:66 labels several organ masses from memory; a table does not give them a citation. FU §1d's derived values (FU:102–126) are checks, not repeated anchors.

### 5. The certificate reads one sealed history

`certify(&ConservationView, &CertificatePolicy)` is pure. The view is lent by one record: bound to its run, branch and base seal, it carries the compiled `ConservedBasis` and exposes the segment queries the certificate needs. Each `HistorySegment` gives its interval, its dense basis (Bernstein, with its degree), the inventory curve of every combination, cumulative gross inflow and outflow in the same basis, and right-continuous impulses for boluses, each with its time, combination and amount. Two separately supplied histories are no longer possible. The policy is built against the basis (for each combination, a closure tolerance, absolute and relative, and the drift bound record 0004 gives it) and carries its digest; a mismatch is a coverage failure.

For every combination and window the certificate checks two different things: closure, `inventory change − net boundary exchange`, and stability drift, `inventory change − expected change`. Matching an output to an input closes a ledger without proving a stable body, and neither check proves the trajectory accurate; that is the accuracy gate's job (§7). Each window also reports the gross flow on every held route (`CoverageManifest::held_routes`), read from that route's own exchange curves, which the history keeps apart from the combination totals. Ordinary intake and a held supply into the same pool stay distinguishable, so stability that rests on held support is visible. Expected drift is `Zero` for meals and drinks, or a stated, provenance-bearing curve for a later fasting protocol; there is no fitted baseline, detrending or daily recentering. Closure holds to the policy's floor, and so does the drift of charge, which physics holds at zero. Every other drift holds to 1.96 times the combination's within-subject variation, because a healthy man's amounts swing from day to day even on a fixed diet; for a combination no method can measure, that variation rests on an inference record that reasons from its sourced flows (record 0004). The result names the set it read. Every sliding 24 h window over at least 48 h passes: window starts include every accepted segment boundary and its 24 h shift, and inside a segment the Bernstein coefficients' hull bounds the interior extrema of both differences, so no sampling is involved. A core that cannot supply segments cannot claim every window; sample-only scoring is a failure.

**The basis is conserved combinations, not free species.** The compiler derives it from incidence and balanced reactions: every combination no internal transfer or reaction changes. Receptor binding consumes free NE into bound complexes, so free NE need not close; total catecholamine (vesicular + junctional free + receptor-bound + metabolite pools) must, and likewise the choline moiety (free and bound ACh + choline).

| Book | Why present and what the certificate claims |
|---|---|
| Water | CV advection, FL distribution and lymph, renal filtration and urine, drinks, evaporation; volume derived, never double counted |
| Na, K, Cl, Ca, Mg, P (elements) and charge | RN reclamation and excretion and meal cargo; charge derived from composition |
| Glucose (`Moiety::Glucose`) and nitrogen | RN filtration and reclamation and meal cargo; hepatic release, tissue uptake and nitrogen metabolism held, with receipts |
| Albumin and carrier mass | FL oncotic gradients and carrier volume; synthesis and turnover outside the slice |
| Catecholamine and choline moieties; held ligand surrogates | Release, binding, reuptake and clearance into named product pools |
| Energy | Paired thermal, mechanical, elastic and exterior-work accounts; the thermal network is held external (MT) |

Every registered intake component reaches an active pool or a named held receipt pool; nothing disappears because GI, MT or HP is absent. The `CoverageManifest` lists which combinations close inside the slice and which depend on held receipts; it is declared coverage, not proof of whole-body metabolism. History is stored as fixed-time sealed chunks with prefix extents; seals bind chunk roots and never rehash all history each beat (contrast CB/src/patient.rs:252–256, 367–369). Compensated values survive into the certificate: each curve is a compensated start plus f64 coefficients within its segment, and conserved-basis sums and window differences are formed in compensated arithmetic before any rounding. Only solver coordinates are rounded once, under the arithmetic profile. Storage policy is part of the measured performance. A region's total of any book is the exact sum over its sites, derived from the registry with no new state, so an organ-level view and the refinement check read the same aggregation.

### 6. Boundary timeline and stress cases

One `BoundaryPort` carries timestamped, idempotent intake, posture, environment, clock and sleep payloads ordered by `(time, id)`. Posture is a continuous trunk angle plus a leg descriptor and subject bed offsets. Environment covers air and radiant temperature, humidity, air speed, clothing and bedding, pressure and inspired fractions. Oral, tube and IV deliveries share one intake envelope with distinct routes. Every payload has a public producer that refuses non-finite values; units, ranges and the intake schema are checked against the compiled body when the port is bound. Pending events, active deliveries, phase origin and cursor are record state and checkpoint. The held EN stand-in owns synthetic AVP, angiotensin II, aldosterone and ANP surrogate turnover with declared clock and osmotic gains, acting only through receptors; meal absorption is an exterior staging law. Na, Cl and water stores integrate freely: the fixture may not schedule excretion equal to intake. **Force the drivers, check the outputs** (FU §4, FU:255–279); a published rhythm that drivers produce is a check, never a forcing, and fixture magnitudes are synthetic.

| Case (both days, exact ns from 06:55) | Input and first route (seconds) | Baroreflex (seconds) | Slower routes (minutes to hours) | Checks, never forcings |
|---|---|---|---|---|
| **07:00 supine → seated** (trunk 0→90°, legs seated-dependent, ramped) | Posture event → site positions in the anatomy (line 96, an anatomy relation) → CV hydrostatic head per vessel segment. CV advection moves water, Na, K, Cl and albumin from central and pulmonary pools to dependent leg and splanchnic venous pools | Carotid transmural pressure falls → `BaroreceptorAfferents` (line 77) fall → NS afferent filter → NE release transfers from the vesicular store to sinoatrial, myocardial, arteriolar, venous and renal junction pools rise; ACh release to the sinoatrial junction falls → binding raises β1-bound (sinoatrial node, myocardium) and α1-bound (arterioles, veins, renal arterioles), lowers M2-bound → CV laws read occupancy: heart rate and elastance up, arteriolar conductance down, venous unstressed volume down | Dependent capillary pressure (line 80) rises → FL Starling filtration into the dependent interstitium over 10–35 min, partly returned by lymph; plasma albumin concentration rises. Renal α1 occupancy and TGF (line 97) lower filtration and Na excretion slightly. Atrial wall stress (line 79, sink) falls | MAP at heart level recovers within seconds and settles above supine; HR up; plasma volume, stroke volume and output down; creatinine clearance slightly down (magnitudes: FU §1d, reviewed packet only) |
| **07:30, 12:30, 18:30 meals and drinks** | Intake event → staging → absorption into splanchnic venous pools over 1–3 h | Small: splanchnic pooling and volume expansion move stretch | CV advects water and ions; plasma Na and osmolality rise → held AVP surrogate rises → V2-bound rises → RN collecting-duct water reabsorption rises. FL filtration expands interstitial water. RN raises Na, K and Cl excretion over hours; the bladder fills. Each meal starts from the inventories it finds, and pending absorption survives checkpoint and sleep. Fuel and energy the slice does not use go to named held receipts | Day-over-day water, Na, K, Cl closure, and drift within each one's within-subject variation (the certificate); postprandial K direction (FU §3a). Insulin-driven K shift and diet-induced thermogenesis are declared external |
| **23:00 seated → supine with sleep onset** (same instant, separate input ids, applied in id order as one boundary transaction) | Reverse shift: central and pulmonary volume rise, leg and splanchnic fall. NREM state enters NS release laws as a declared gain | Stretch rises → NE release falls, ACh rises, occupancy settles back; the NREM gain lowers sympathetic release further at every junction | Lymph refill and less filtration raise plasma volume (10–35 min). Clock gain on renal hemodynamics (forced) lowers filtration; the AVP night modifier (forced) raises V2-bound; wall stress rises for the held ANP surrogate. Bedding changes evaporative loss | BP and HR dip, posture and sleep calibrated together rather than added; night urine flow and Na excretion fall as checks; no output is set |
| **06:30 sleep offset** (still supine until 07:00) | Sleep = Awake removes the NREM gain | Sympathetic release rises at sinoatrial, arteriolar and venous junctions | AVP night modifier ends on its forced window; the renal clock gain follows its phase, not sleep. Day 2's 06:55 falls here with no reinitialization | The dip reverses before 07:00 |

Thermal set point, metabolism and DIT, cortisol and hepatic glucose, and respiratory chemoreflex stay outside slice coverage, not held successes; the pulse 0006 milestone must activate them first.

**Interventions enter the same way.** A bolus, a drip or a position change is an authorized input admitted mid-run through `Session::enqueue`; under the causal step rule it gives the same bits whenever it was committed. Later additions need only a contract version: stop and titrate by delivery id, scripted insults, and new payloads such as CPR as exterior chest work, airway and oxygen devices, leg raise, lateral and prone positions, and removals such as a blood draw. The port checks each against the compiled body when it binds. They wait until their systems leave the parking lot.

### 7. Arena specification

This document specifies the numerical arena; it does not dispatch it. Shared work comes first, once: `ng-contract`, the slice organs, stand-ins and the certificate shard; otherwise the arena compares organ code, not solvers.

Gates for each core:

- **Slice and certificate.** The full slice and declared boundary for 48 continuous hours from 06:55 with a synthetic packet and full waveform on a declared output grid; every covered combination passes closure, and drift within its bound from the synthetic healthy-range set, over every sliding 24 h window, with the 11 unmapped arrows and all held physiology listed in the coverage manifest and held support reported per window.
- **Speed.** At least **100× real time** with full waveform and the same checkpoint cadence (at most 1,728 wall seconds for 48 h on the named machine), stage-witness validation included, with initialization and compilation timed separately and all repeated runs retained.
- **Pace and resume.** Identical accepted-state, ledger, event, continuation and observer bits under unbounded, live wall-clock and irregular chunked pace; checkpoint and resume at existing boundaries around posture, sleep, absorption, same-time inputs and refusals, decoded into a freshly built core with a history store, matching uninterrupted runs on full decoded state, outputs included.
- **Refusal and inputs.** A refusal forced at a delivery instant leaves every accepted field, pending input and stream position byte-identical when snapshotted just before and after it, and the retried run applies the delivery exactly once at its time; bitwise continuation is compared against a run with the same accepted schedule, otherwise within declared tolerances. Same-id replay applies an intake once. A forced extra evaluation before a stochastic event leaves its draw and ordinal unchanged. The slice's stochastic fixture is a declared hazard law with its own stream, and one forcing fixture runs through the pace and resume gates, so replay with variability is proven before a core is chosen.
- **Late admission (the causal step rule).** Every timeline input admitted mid-run through `Session::enqueue`, at the accepted head, gives the same accepted state, ledger, event and observer bits as the same input known at admission.
- **Rewind.** A branch rewound in the same process after the parent ran ahead matches the same branch decoded into a freshly built core, so an in-game retry equals a loaded save.
- **Evidence binding.** `replay_gates` runs all five replay gates, building each fresh core from a `CoreFactory`, and returns evidence bound to the run's identity. `score` needs the certificate, the accuracy evidence, the run evidence and that replay evidence; evidence from another run, or a gate that did not run, is `ArenaFailure::MissingEvidence`, never a pass.
- **Non-perturbation.** Physiology bits are identical with observers attached, detached or changed, and a new lens applied to a finished replay gives the same physiology with new findings.
- **Accuracy.** Every `AccuracyCase` within the `AccuracyPolicy`: the manufactured two-reservoir case with positive inventories, a nonzero initial difference and a stepped exterior inflow, against its closed form; drain and refill to the admissible bound; zero flow and restore; controlled removal with every removed amount in its receipt; evaluation independence (identical residual and Jacobian bits for one stage before and after refused evaluations); and windows of the slice (admission with the first posture change and meal, sleep onset, the second day's waking) against the contract's `ReferenceIntegrator`, fully coupled backward Euler at three refinements, which give the reference's own error and the core's observed order. Metrics cover states, exchanges, outputs and event times. A core is never compared with itself.
- **Splits.** Solver-B passes the full split metric with traces drawn from accepted history, against the reference integrator. Runs under measurement are never scored.
- **Health.** A `NumericalHealth` report beside the certificate: refusals by reason and by refusal source, floor faults and validity excursions.

Identical bits between different methods are not required. The judge picks one core; **do not graft the two algorithms**. The developer reviewer confirms. SUNDIALS IDA is excluded ([pulse 0007](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0007-solver-picked-by-arena-ida-dropped.md)). If neither passes, report neither. The accuracy tolerances, the error scales, each core's minimum interval and the certificate's closure floor are policy values left open here.

A gate counts only if it rejects a core built to fail it. Each one below is part of the arena:

| Must-fail core or model | Rejected by |
|---|---|
| Forward Euler on the two-reservoir case with h > RC and a nonzero initial difference: every ledger closes, the difference grows | Accuracy gate, before positivity intervenes; the certificate alone passes it |
| Accepts a step whose normalized error exceeds one at its minimum interval (SB network.rs:439–449) | `validate` recomputes the error from the witness: no publication, fault `NoAdmissibleStep` |
| Accepts after the sweep limit on a strongly coupled fixture whose change is still above tolerance (SB stepper.rs:387–388) | `validate`'s whole-scope re-evaluation (`MethodMismatch` or `StageResidual`); accuracy gate |
| Evaluates blocks at stale coordinates under the coupled plan | Same whole-scope re-evaluation; accuracy gate |
| Books one sample at the start while its endpoint came from other stages (rate 1 + t over h = 1: one sample books 1, the trapezoid 1.5) | `MethodMismatch`: the increment is not the declared scheme's weighted sum. Building `Extents` directly does not compile |
| Adds an evaluation from an earlier attempt | `Quadrature::add`: the stage time belongs to another ticket |
| Qualifies a split from invented samples | Does not compile: a `SplitTrace` comes only from accepted history |
| Applies a due input before the attempt and keeps it after a refusal (SB stepper.rs:228) | Snapshot before and after the forced refusal |
| Keeps a counter or cache outside its continuation | Resume into a freshly built core must match the uninterrupted run; an in-process rewind after the parent ran ahead must match it too |
| Spreads its steps toward a known later input | Does not compile (no core sees the timeline); the late-admission gate |
| Draws chance per solver step | Does not compile (no `threshold` on the problem); the extra-evaluation and two-step-policy checks |
| Keeps its predictor or Jacobian across a posture breakpoint | `validate`: the method history must start at the breakpoint |
| Refuses on a reference range | Does not compile (`Admissible` has no bounds; a kernel output takes no set). A precondition that encodes one shows in `refusal_sources` and the health counts |
| A pressure-volume update whose volumes pair but whose elastic energy does not close | `validate`'s nonlinear energy closure |
| A held stand-in using a route its declaration omits, or holding a concentration whose store is in the slice | `Assembly::compile` |
| An orphan pool | `Assembly::compile` |
| A reaction that creates an element, written outside a scope or slipped into the schema | Does not compile outside a scope; `Assembly::compile` refuses it and names the process |
| A kernel calling host math on an expression | Does not compile (`Expr` has no value). Host math on a constant during declaration is outside this test; constants come from packets |
| An initializer returning a packet guess that violates a row or anchor | `InitializationSystem::admit` refuses and names the row. A guess that already solves the system is admitted |

### 8. Diagram source

The Mermaid source below is the single source. [solver-contract.svg](solver-contract.svg) and [solver-contract.png](solver-contract.png) are rendered from it with `@mermaid-js/mermaid-cli` 11.17.0 (`-w 3200`, and `-b white` for the PNG) under `~/lanes/heavy.sh`; re-render both whenever it changes.

```mermaid
flowchart LR
  subgraph DAY["Boundary port: one timeline, saved with the record"]
    MEALS["Meals and drinks"]
    POSTURE["Posture"]
    ROOM["Room air and bedding"]
    CLOCK["Body clock and sleep"]
  end
  ANATOMY["Anatomy as data: sites by class, containment, vessels in series and loops, enclosures, nerves, heights"]
  subgraph ORGANS["Organ crates: laws as data, each written once per site class; none stores an amount or a state"]
    CV["Heart and vessels"]
    FL["Fluids"]
    RN["Kidney"]
    NS["Nerves (baroreflex)"]
    RC["Receptors"]
    HELD["Held stand-ins for the other systems: each declares what it supplies and where it holds"]
    DICE["Chance: hazards draw from keyed seeded streams inside the laws; the solver holds no dice"]
  end
  CV -- "stretch" --> NS
  NS == "transmitter into junction pools" ==> RC
  RC -- "bound receptors set rate, force and tone" --> CV
  CV -- "capillary and venous pressure" --> FL
  RN -- "tubuloglomerular feedback" --> CV
  HELD -- "held chest and belly pressure, felt by every bed they enclose" --> CV
  POSTURE -. "gravity, from each site's height" .-> CV
  POSTURE -. "gravity, from each site's height" .-> FL
  CLOCK -. "sleep and time of day" .-> NS
  CLOCK -. "time of day" .-> RN
  MEALS == "intake" ==> HELD
  ROOM == "heat and skin water exchange" ==> HELD
  ANATOMY --> COMPILE
  ORGANS -- "laws as data" --> COMPILE["Contract checks and compiles one body"]
  COMPILE --> INIT["Start state solved once from anchors"]
  INIT --> REC
  subgraph STEP["One step"]
    SOLVER["Solver core, chosen by the A vs B benchmark: solves trial states in its own workspace, changes no accepted state"]
    TX["One proposed step: amounts = start + integrated transfers, with the stages that produced them"]
    REFUSED["Refused: state, inputs, cursors and history untouched; retry shorter. Below the floor it is an engine fault"]
    FAULT["Engine fault: stop and show it, never death"]
  end
  subgraph READ["Read from the record, never written back"]
    CERT["Stability certificate: conservation and drift over every 24 h window, not accuracy"]
    ACC["Arena accuracy gate: accepted output against known answers and a reference integrator"]
    VIEW["Monitor and chart: a lens kept apart from the body; observers read each step in order and keep their own state"]
    SAVE["Checkpoint: state, solver history, pending inputs and the last output segment; resumes bit for bit"]
  end
  COMPILE --> SOLVER
  DAY -- "pending inputs, applied once at step boundaries; a late order gives the same result as an early one" --> REC[("Patient record: one home for every amount and state, in shared vascular, fluid and ligand pools")]
  REC -- "accepted state" --> SOLVER
  SOLVER --> TX
  TX == "stages re-evaluated, conservation and admissibility checked, then published; abnormal physiology still publishes" ==> REC
  TX -- "refused" --> REFUSED
  REFUSED --> SOLVER
  SOLVER --> FAULT
  REC --> CERT
  REC --> ACC
  REC --> VIEW
  REC --> SAVE
  KEY["Arrows: thick = amounts move, solid = signals, dotted = forcing"]
```

## Synthesis decision

This design came out of pstack `architect`.

**How it was made.**
- **Ground.** Three `how` explorers (the contract drafts, the solver prototypes, and the coupling map with the repo gates), merged by one explainer. Two `why` investigators (git and PR history, and issues), merged by one synthesizer.
- **Arena.** Two structurally different candidates from the same brief:
  - **A**: laws as trait implementations over capability handles.
  - **B**: laws as data, compiled by the contract.
- **Judge.** A cross-vendor judge on Codex CLI scored both against a six-part rubric. It re-ran both builds and compiled adversarial probes against each sketch.

**Base: B.** Organ crates describe their laws as typed data. The contract checks the data and compiles it into one hashed body, and both solver cores consume only that body. The deciding question was which design a future maintainer can extend without breaking invariants, given that the maintainer is an agent that copies the nearest example and takes the shortest path that compiles. In A, every organ stays an executable object inside each solve. The judge compiled an organ that keeps a hidden counter (`AtomicU64`) and feeds it into its rates; a forged system brand also compiles. I reproduced both. Hidden state outside the record and checkpoint breaks "the next step depends on the record alone", and that is what makes the same seed give the same bits at any pace. Rust cannot forbid such state in arbitrary organ code. B's closed instruction set can.

**Cross-judge verdict.** B scored 19/30 and A 14/30. My own first read, before the judge returned, favoured A, because A had more compile-fail checks and a richer document. I had not probed for escape hatches. I reversed the pick once the counterexample reproduced.

**Grafts from A**, folded into B's model:
1. The coupling catalog and the census come from one declaration, and the compiler checks the count (A `anatomy.rs:253-287`).
2. Extents can be made only by summing evaluated rates, with forward and reverse kept apart (A `problem.rs:307-339`).
3. A parameter packet is either reviewed (cited anchors) or a synthetic fixture (A `provenance.rs:175-237`).
4. A's stress-case table, which traces each baroreflex route.

**Repairs to B**, from the judge's findings:
1. Every value a solver core needs now has a public producer: tangents, equation blocks, the boundary horizon and bound parameter values. A compiling signature walk proves it.
2. Each kernel graph has its own identity, so an expression cannot leak from one law to another.
3. The certificate takes one view bound to a single run, branch and seal, and its basis counts bound and product forms.

**Fixes both candidates missed:**
- Validation returns the whole next image, then one publish step that cannot fail.
- A faulted session cannot step again.
- The checkpoint carries observer and output state.
- A split qualification can only come from a measurement.

**Rejected:**
- **A's runtime organ objects**: hidden state, forgeable brands, and a `NonMaterial` bound any crate could implement.
- **A's hourly certificate windows and its 60 s ledger grid as a solver breakpoint**: they replace "every sliding window", and they let reporting cadence change the physics.
- **B's handwritten port markers.**
- **Any adapter between the two solver cores, or any graft of one numerical algorithm into the other.** The later arena picks one core.

**Dropouts and caveats.**
- The judge is the same vendor as the winning candidate. Two things offset that: the deciding evidence is compiled code that I reproduced, and my own pre-judge scores went the other way.
- Neither candidate, and not this synthesis, proves conservation, replay, 48 h stability or speed. Those are the arena's job.

**Revision r1.** Three outside models reviewed the diagram alone. [solver-contract-review-r1.md](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/architecture/solver-contract-review-r1.md) triages every finding against this document and the sketch. A Codex judge then probed the triage with compiled code, and the diagram was given cold reads before and after. Adopted:
- the record re-evaluates a stage witness against the declared scheme, so method consistency, lag and excess error at the floor are checked at publication;
- the accuracy gate, with manufactured cases and a contract reference integrator, and its must-fail cores;
- refusal sets on slots only, separate from reporting-only `ValidityRange`s;
- held-stand-in declarations, including intensive fields;
- a readable resolution (scheme, scales, floor) and diagnosed refusals and faults;
- restart at breakpoints and mode changes;
- the replay ladder and pinned transcendental operators;
- compensated amounts through to the certificate;
- split traces only from accepted history;
- history stores for checkpoints;
- a diagram that shows trial-state solving, edge kinds, initialization and what the certificate does not prove.

The split-file audit (a private working note; rows 39–41, 45, 48, 52, 61 and 65) added five more:
- an `Elastance` dimension, so `cv.elastance` is no longer typed as a pressure;
- a glucose book (`Moiety::Glucose`);
- `FragmentBuilder::memory_rate` in place of `evolve`;
- `AuthorizedInput::from_command_receipt` in place of a Pulse-era command name;
- no history-conditioned initialization goal.

Rejected: rejecting algebraic loops, and a quasi-steady reflex. An i64 ledger with fractional carry measured viable and is kept as the alternative to compensated pairs. Moves that a type can forbid are compile-fail doctests.

**Revision r2: the fold.** A second pstack `architect` run folded the pre-arena items of an architecture delta and of a game-readiness audit into this contract. The items were the audit's anatomy fix, values as ranges with evidence, observation as a lens, pain as a nociceptive field, seeded chance in the laws, the causal admission rule and the rewind gate. Grounding was the delta's own `file:line` pass over this tree and the audit's. Two runners sketched it independently from one brief: candidate A and candidate B. Both converged on the same whole shape: anatomy as data with laws expanded per class, reference-typed set points, draws in laws with positions only in the record, observation identity apart from the body, and replay gates for late admission, rewind and non-perturbation. They differed in how a law meets its sites. A types the site class into the kernel (`Kernel<.., C>`, `PerSite<C, U>`), so a crossed read fails in the Rust compiler. B lends symbolic site-relative ports from a runtime class grant (`Assembly::class`), with about 90 new public types against A's smaller set. The judge (a Codex seat; see the dropouts) chose **B as base** for breadth: its anatomy carried aggregation and refinement data, its catalog replaced the pressure and posture wires, its packet separated inputs from targets, and its replay gates came with a protocol. It also found that B's class capabilities carry no class identity, so a law from one class scope could read another's port. That defect is the deepest in either candidate, and its fix is A's site context threaded through every capability. **The synthesis keeps A's skeleton and takes B's breadth**, which the judge's own graft list makes the same end state with less rework: A's typed site classes and step clipping, plus every B strength the judge named.

**Grafts from B**, each named by the judge or its table:
- typed seed domains (`DomainSeed<D>`: `PacketSeed`, `Seed`, `ObservationSeed`), so measurement error cannot draw from the run seed;
- a `HazardGuard` whose reset, stream advance and mode change the contract applies together, in place of a hand-written `−ln(draw)` comparison, and stream positions keyed by stream;
- the catalog's pressure and posture wires (lines 82, 96 and 101) replaced by an `AnatomyRelation` disposition, with census counts unchanged, and held enclosure pressure linked to its card's `HeldDeclaration`;
- `AggregationMap`, `RefinementRule`, `SplitRule` (one record per child, not parallel lists) and `PortEquivalence` as a request for evidence;
- set points only for `Sensed` dimensions, and no guard on a set point;
- `ParameterPacket::pinned` removed, with `sampled` and `authored` reading population records from the packet's own manifest;
- typed absence on findings and an observer `resume`;
- `CoreFactory`, `replay_gates`, replay evidence bound to the run's identity and required by `score`, and `ArenaFailure::MissingEvidence`.

**Repairs the judge found in A, made:** no core sees the timeline at all, because `CoupledProblem::ticket` takes the core's intrinsic end and clips it itself (the judge showed that A's start-only breakpoint query still let a core aim at the next input); `AnchorStatistic` now reaches the anchors through `over`; the observer has a restore path.

**Rejected from B:** runtime class grants for site laws, whose class identity the judge found erased; observer processing inside the record transaction, because A's observer outside the session makes non-perturbation structural; chart entries in the record ahead of the chart's own design; a condition owner ahead of its rule; `ComputedOutput` and `AnchorTarget` types, since removing `pinned` and stating each anchor's statistic close the same door with less surface.

**Subtracted:** `PerBed` (now `PerSite`), `KineticRepresentation` (now `Representation`), `CoupledProblem::threshold` and `CoupledProblem::boundary`, the stream positions in `SolverContinuation`, observer state in the record and the old `observe` and `waveform` functions, the unused `Hazard` record type, `ParameterPacket::pinned`, three wire payloads (`PleuralPressure`, `PositionAndGravity`, `AbdominalPressureOnVessels`), and a second way to scale by a dimensionless factor.

**Dropouts.** The judge ran on Codex, the same vendor as candidate B, and it chose B. Its report is kept with the run (`judge.last.md`, sha256 recorded in nightingale-pulse#302); every graft and repair above traces to a line of it.

## Tradeoffs accepted

- We accept a small typed expression language and a compiler as real contract work in exchange for one analyzable, hashed model with no runtime organ coupling and no undeclared callback reads.
- We accept the kernel closure's extra lifetime in every kernel signature in exchange for a compile-time guarantee that a kernel's dependencies and output come from one graph.
- We accept rewriting the prototypes' model access in exchange for keeping their distinct numerical cores on one problem. "No adapter" does not mean "no migration."
- We accept dense generic arrays inside the compiled boundary in exchange for a small solver interface; typed ownership stays on the authoring side and reconstruction on the acceptance side.
- We accept a runtime binding check (layout digests) on quadrature, dense output and tickets in exchange for not branding every numerical value with the body's lifetime; a mismatch is a diagnosed solver-bug fault.
- We accept that a host which drives the solver itself can only dry-validate, in exchange for the fault latch living in the only path that publishes.
- We accept an explicitly open, synthetic slice with held cards and an unresolved arrow in exchange for a bounded experiment; its stability cannot stand in for the whole-body milestone.
- We accept physiological input and guard boundaries with intrinsic numerical steps in exchange for exact replay at every pace. A checkpoint may report an earlier accepted instant than the UI asked for.
- We accept explicit nonlinear energy-closure work in exchange for an honest conservation claim.
- We accept a seam that admits only methods whose increment is a nonnegative weighted sum of in-interval stage rates, in exchange for a ledger that equals the update by construction; a multistep or Rosenbrock core needs a contract version.
- We accept a closed set of admissible domains, in exchange for refusal that can never encode a physiological range.
- We accept two f64 per stored amount and per cumulative exchange, in exchange for closure that rounding does not limit.
- We accept one more evaluation per stage at publication, counted in the speed bar, in exchange for a record that checks the method instead of trusting it.
- We accept a closed set of site classes, where a new class is a contract version, in exchange for laws written once per class and checked by the compiler, so a finer anatomy is a data change.
- We accept a site parameter on `Kernel` and `PureKernel`, defaulted to the whole body, in exchange for one kernel language at both levels.
- We accept observers that live outside the session and must read every revision in order, in exchange for physiology that cannot depend on what is watched.
- We accept a causal step rule that forbids a core from planning steps around a known later input, in exchange for identical bits whenever an order arrives.
- We accept keyed streams and forcings in the model hash, in exchange for chance that no solver, step policy or added stream can move.

## Alternatives considered

1. **Live organ objects with gated contexts.** A declare/bind/evaluate lifecycle (CA/src/organ.rs:28; CB/src/manifest.rs:202) keeps each organ an executable object inside every solve, including derivative sweeps, and makes manifests a second list to keep in sync with callbacks. The surface is wider and its guarantees are conventions: an organ can hold mutable state outside the checkpoint that changes the next evaluation after a refused attempt. Rejected for laws as data.
2. **Monolithic generated Rust body per solver.** Fast, but it duplicates equations, ownership and hashes across Newton and beat code, so one organ change needs coordinated codegen and solver edits. Rejected because it undermines the controlled arena seam.
3. **Actor or message organs with local stores.** Small local interfaces, but consistency, rollback, lagged coupling and conservation leak to the coordinator, and simultaneous physical loops are not message queues. Rejected.
4. **Networks as owners** (CV owns the vascular graph, FL owns Starling everywhere, NS the neural graph) with organ cards only as views. It is the right ownership rule once the conflicts in §2 resolve by mechanism, but as types now it doubles the owner set before the milestone; site-class laws give the same "one rule everywhere" without it. Kept as the rule to write before the milestone build.
5. **Untyped site sets** checked at assembly instead of site classes in the type. A smaller type surface, but a bed law that reads a junction entry would fail at compile of the body instead of in the Rust compiler, and an agent copying a law would not see the mistake.
6. **One assembly-wide expression arena with global node ids** instead of a per-kernel brand. Signatures get simpler and subexpression sharing becomes legal, but a shared node then belongs to kernels with different modes, domains and rate classes, so block partitioning, per-kernel hashing and differentiation order stop being local to one law. Rejected for self-contained kernels with a fresh brand.

## First build slice: spec

Tickets: `.scratch/native-contract/issues/` in this repository, one file per ticket, numbered in dependency order.

### Problem statement

The contract exists only as a type sketch. Neither solver core can be compared until a real `ng-contract` compiles a real body, a real record accepts and refuses steps, and the replay gates can reject a core built to fail them. Every later choice (the core, the ledger, the first 13 systems) rests on that harness, and the arena would otherwise benchmark the organ code instead of the solvers.

### Solution

Build the contract, record, observer and validation crates against one manufactured body small enough to know exactly. It has two capillary beds in series on a vascular loop, one enclosure, one junction, a baroreflex filter with a set point, one hazard stream and one forcing, all with synthetic numbers. The body runs through validate and publish with a backward Euler reference core, and every gate in §7 that does not need the full slice runs on it. Then the arena slice's anatomy and fragments (pulsatile CV with valve modes, FL Starling at every bed, RN filtration and feedback, NS baroreflex release at five junctions, the receptor layer and the boundary) compile on the same crates.

### User stories

1. As an organ author, I want to declare a law once for a site class, so that adding a bed or a junction never means editing my crate.
2. As an organ author, I want the compiler to refuse a law that reads another site class's entry, so that a copied example cannot read the wrong place.
3. As an organ author, I want limits, saturations and products with checked dimensions, so that a baroreflex sigmoid or a waterfall is one expression, not an opcode.
4. As an organ author, I want a set point that cannot be mistaken for the variable it guards, so that a resetting reflex cannot write a pressure.
5. As an organ author, I want to declare a hazard and its stream in my law, so that chance follows physiology and the solver never holds dice.
6. As an anatomy author, I want sites, containment, networks, enclosures, nerve edges and heights as data, so that a finer map changes the data, not the laws.
7. As an anatomy author, I want the anatomy refused when a bed is unreachable, undrained or enclosed twice, so that the topology errors surface before any solve.
8. As a solver core author, I want the problem to show me only the next breakpoint, so that my step controller cannot depend on a later order.
9. As a solver core author, I want a refused step to leave the record untouched and hand me a smaller ticket, so that I can retry without cleanup.
10. As a solver core author, I want a fault that names the law and row that failed, so that a structural defect is fixed instead of retried.
11. As the record, I want to rebuild amounts from paired extents and re-evaluate the stage witness, so that a step is accepted only when its method and books both check.
12. As the runtime host, I want a mid-run order to give the same bits as the same order known at admission, so that pace and click timing never change the patient.
13. As the runtime host, I want an in-process rewind to equal a loaded save, so that "try again from here" replays the same course.
14. As the runtime host, I want checkpoint bytes I can store and reload, checked before any body byte is read, so that a corrupt save is refused, not decoded.
15. As a chart or monitor author, I want a lens I can compile and apply to any replay, so that a new reading never invalidates old runs.
16. As a chart or monitor author, I want an observer that reads each revision once and refuses a gap, so that no sample repeats or drops.
17. As a chart or monitor author, I want measurement error from its own seed namespace, so that adding a monitor never changes the physiology.
18. As a packet author, I want every value to carry its population, evidence or fixture note, so that a reviewer can trace each number to its source.
19. As a packet author, I want anchors to state their statistic, so that a pulsatile output is fitted as a cycle mean, not an instant.
20. As the arena orchestrator, I want each gate to reject a core built to fail it, so that a pass means something.
21. As the arena orchestrator, I want the certificate, the accuracy evidence and the replay evidence as separate results, so that a closed ledger is never mistaken for an accurate one.
22. As a reviewer, I want region totals that are exact sums of site totals, so that a split can be checked to keep every regional total exact (I11).
23. As a reviewer, I want held stand-ins to declare what they supply, so that stability bought with held support shows in the report.

### Implementation decisions

- Four crates enter the workspace under `ng-*` purity rules first: the contract (anatomy, declaration language, compiler, seam, admission, evidence records, lenses), the record (accepted image, session latch, history, checkpoint, observation view), the observer, and validation (manufactured bodies, stand-ins, certificate, accuracy and replay gates). Organ crates and solver cores come after, each depending on the contract only.
- The public surface is the sketch's: the signature walk is the acceptance test for the API, and its compile-fail doctests carry over as compile-fail tests.
- The anatomy builder runs its graph checks at `finish`; the compiler derives the pool registry, incidence, conserved basis, sparsity and blocks from the anatomy and the fragments, and hashes the normalized model with the anatomy in it.
- Site-class kernels compile to one program instantiated per site; parameters bind per site by key, with a class default.
- The instruction set is v1: add, subtract, multiply, divide, the dimension-identity table, scale, smooth max and min, logistic, exp, ln and deviation, each from correctly rounded operations and the contract's own routines.
- Amounts and cumulative exchanges are compensated pairs; solver workspaces are f64.
- The reference core is fully coupled backward Euler at three refinements; it doubles as the accuracy reference.
- Streams, forcings and hazard guards are compiled law data; stream positions live in the accepted image.
- Lenses are compiled against the body and hashed apart from it; observers live outside the session.
- Policy values (tolerances, minimum intervals, the certificate's closure floor) are model defaults set in the validation crate and reported with every result. The certificate's drift bounds are not policy values: they come from the healthy-range set, except charge's, which is the closure floor (record 0004).

### Testing decisions

- Test external behavior through the highest seam: a manufactured body declared through the public API, run through the session, scored by the gates. No test reaches into compiler or record internals.
- Every gate ships with its must-fail core or model from the §7 table, and a gate counts only when it rejects that core.
- The manufactured two-reservoir case and drain-and-refill are checked against closed forms; the slice windows against the reference integrator.
- Replay gates compare digests of accepted state, ledger, events and findings: pace, resume, late admission, rewind and non-perturbation.
- Prior art: the sketch's signature walk and compile-fail doctests, and the evidence scripts beside the sketch (`evidence/census-check.py`, the ledger rounding experiment).

### Out of scope

- Running either 48 h core or picking one: the arena's job, after this slice.
- The 13 systems beyond the slice, the condition owner and disease layer, new intervention payloads, demise behavior.
- A reviewed packet, source admission and evidence attestation (`ready-for-human`).
- The chart and monitor UI, pain laws and the port-equivalence gate for refinement.

### Further notes

- Purity rules come first.
- The numbers are synthetic by type: the validation crate accepts only a fixture packet.
