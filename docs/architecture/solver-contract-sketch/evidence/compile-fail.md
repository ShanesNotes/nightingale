# Compile-fail doctests: intended errors

Method. Every `compile_fail,EXXXX` marker in `src/lib.rs` was removed once (31 markers, one
`sed` pass on a saved copy), `cargo test --doc` was run, and `src/lib.rs` was restored and
re-verified (31 markers back, 31 compile-fail doctests passing in `cargo-doctest.txt`). With the
markers removed all 31 snippets failed to build; the 26 positive doctests still passed. Raw output:
`compile-fail-markers-removed.log`. Lines are the doctest fence lines rustdoc reports. Revision r1
(`../../solver-contract-review-r1.md`) added the last seven rows; the first sixteen kept their
lines and diagnostics.

| Line | Forbidden move | Marker | Actual primary diagnostic (markers removed) | Legal twin |
|---:|---|---|---|---|
| 19 | Name the private pool handle | E0603 | E0603 struct `PoolRef` is private | kept from B |
| 25 | Write a vital on the record | E0599 | E0599 no method `set_mean_arterial_pressure` on `&mut PatientRecord` | kept from B |
| 31 | Transfer from a donor alone | E0308 | E0308 expected `Pair<'_, Renal, Water>`, found `Endpoint<'_, Renal, Water>` | kept from B |
| 40 | Import CV internals | E0603 | E0603 module `internals` is private | kept from B |
| 46 | Anchor without citation | E0061 | E0061 `Specified::new` takes 4 arguments, 3 supplied | kept from B |
| 54 | Engine fault into demise | E0277 | E0277 `Demise: From<EngineFault>` not satisfied | kept from B |
| 60 | Water as private state | E0277 | E0277 `Water: NonMaterial` not satisfied | kept from B |
| 68 | Renal law on a CV route | E0308 | E0308 expected `Pair<'_, Renal, Water>`, found `Pair<'_, Cardiovascular, Water>` | kept from B |
| 78 | **New.** Expression smuggled from a finished kernel into the next (judge `b-cross-kernel-expression`) | E0521 | E0521 borrowed data escapes closure: `k` (`Kernel<'_, '1, Renal>`) escapes via `stolen`; plus an uncoded "lifetime may not live long enough" at the second kernel. Both are the per-kernel lifetime `'k` | line 93 |
| 111 | **New.** Two kernels open from one `&FragmentBuilder`, inner uses outer's expression (judge `b-double-kernel-from-builder`) | E0521 | E0521 ×2 at `second.add(own, foreign)`: `first` (`Kernel<'_, '1, Renal>`) escapes into the inner kernel | line 129 |
| 149 | **New.** Certify one record's inventories against another's ledger (judge `b-unrelated-certificate-inputs`, verbatim) | E0599 | E0599 no method `inventory_history` and E0599 no method `ledger` on `&PatientRecord`; also E0061 `certify` takes 2 arguments | line 157 |
| 166 | **New.** Step a faulted session | E0599 | E0599 no method `step` for `Session<'_, Faulted>` | line 171 |
| 177 | **New.** Publish through a lent record | E0596 | E0596 cannot borrow data in a `&` reference as mutable | `Session::step` (line 171) |
| 186 | **New.** RN receives the CV-to-NS baroreceptor edge | E0271 | E0271 type mismatch resolving `<BaroreceptorAfferents<Reads<'_, Renal>> as Coupling>::To == Renal` (expected `Renal`, found `Nervous`) | line 193 |
| 201 | **New.** Synthetic anchor in a reviewed packet | E0308 | E0308 expected `Specified<_>`, found `Synthetic<Pressure>` | line 209 |
| 218 | **New.** Bytes as split qualification | E0308 | E0308 expected `&SplitQualification`, found `&[{integer}; 32]` | line 223 |
| 232 | **r1.** Build a split trace from invented samples (judge probe P02) | E0599 | E0599 no associated function `new` for `SplitTrace` | line 241: `PatientRecord::split_trace` |
| 250 | **r1.** Give a refusal domain bounds (a reference range) | E0599 | E0599 no associated function `new` for `Admissible` | line 274: closed set on a slot, `ValidityRange` reports |
| 259 | **r1.** Put a sign constraint on a computed expression, `pressure − reference` (judge probe P13) | E0061 | E0061 this method takes 3 arguments but 4 arguments were supplied (`Kernel::finish`) | line 274 |
| 291 | **r1.** Hold a card without declaring what it supplies | E0061 | E0061 this method takes 1 argument but 0 arguments were supplied (`Assembly::hold`) | line 298 |
| 309 | **r1.** Book extents as one sampled rate times the step | E0451 | E0451 fields `_layout`, `_interval`, `_forward` and `_reverse` of `Extents` are private | line 317: `Quadrature::finish` |
| 327 | **r1.** Write a reaction from raw stoichiometry | E0451 | E0451 fields `_stoichiometry`, `_brand` and `_owner` of `BalancedReaction` are private | line 335: `ReceptorScope::parts` |
| 344 | **r1.** Call host math on an expression | E0599 | E0599 no method `value` on `Expr` | line 351: `Kernel::exp` |
| 360 | **#301.** A renal guard fires a CV mode transition | E0308 | E0308 expected `ModeTransition<'_, Renal>`, found `ModeTransition<'_, Cardiovascular>` | line 368: `Guard::new` with the owner's own transition |
| 378 | **Fold.** A capillary-bed law reads a junction's entry | E0308 | E0308 expected `CapillaryBed`, found `Junction` (`PureKernel<'b, Fluids, Dimensionless, CapillaryBed>`) | line 389: `Kernel::at_site` on the bed's own port |
| 401 | **Fold.** A set point used as the variable it guards | E0308 | E0308 expected `Expr<'_, '_, Nervous, Pressure>`, found `Expr<'_, '_, Nervous, Reference<Pressure>>` | line 410: `Kernel::deviation` |
| 421 | **Fold.** Volume times volume named a pressure | E0277 | E0277 `SameDimension<_>` is not implemented for `Product<Volume, Volume>` | line 430: `Product<Elastance, Volume>` as `Pressure` |
| 441 | **Fold.** A core reads the boundary timeline | E0599 | E0599 no method `boundary` on `&CoupledProblem<'_>` | line 448: `CoupledProblem::ticket(start, intrinsic_end)` |
| 457 | **Fold.** The solver draws chance (`threshold`) | E0599 | E0599 no method `threshold` on `&CoupledProblem<'_>` | line 462: `Kernel::draw` on the owner's stream |
| 471 | **Fold.** An observer draws error from the run seed | E0308 | E0308 expected `DomainSeed<ObservationDomain>`, found `DomainSeed<RunDomain>` | line 476: `ObservationSeed` |
| 482 | **Fold.** A guard fires on a set point | E0277 | E0277 `Reference<Pressure>: Sensed` is not satisfied | line 490: a guard on the sensed pressure |

## The judge's B probes against this crate

Compiled with `rustc --edition=2024 --emit=metadata --extern solver_contract_sketch=<target-final/debug/libsolver_contract_sketch.rlib>`,
probe files copied unchanged from `/home/ark/worktrees/arch-sc/judge-scratch/`.

| Probe | Result now | Reading |
|---|---|---|
| `b-cross-kernel-expression.rs` | exit 1, E0425 `KernelBuilder` not found | The free-standing builder is gone; the same move in the scoped API is doctest line 78 (E0521) |
| `b-double-kernel-from-builder.rs` | exit 1, E0599 no method `kernels` | `kernels()` is gone; the same move in the scoped API is doctest line 111 (E0521) |
| `b-unrelated-certificate-inputs.rs` | exit 1, E0599 ×2 + E0061 | Same as doctest line 149 |
| `b-no-tangent-constructor.rs` | exit 1, E0425/E0422 `Tangent` not found | The tangent is the workspace's y′. Legal path compiles (exit 0, no diagnostics): `workspace.derivative_mut()`, `workspace.point(t)`, `problem.evaluate(Scope::Whole, &point)` |
| `b-no-boundary-horizon.rs` | exit 1, E0615 `boundary` is a method | Problem fields are private now. Legal path compiled then (exit 0, no diagnostics): `problem.boundary().next_breakpoint(at)`; since the fold no core sees the timeline and the contract clips each ticket (line 441) |
