# 55: A term evaluated once for all the laws that add it, and the algebraic rows that are plain definitions

Spec: the evaluator in `crates/ng-contract` and the site term of [native-contract 42](42-veins-collapse-branch.md), part 2

Type: prototype

Status: resolved

Blocked by: 42

**Why.** The first milestone (SRS invariant I19) runs the healthy reference man for 48 hours, and how long that run takes rests on what a step costs. Native-contract 42 places a site term by writing it into each law that adds it, so a term three laws add is computed three times. Evaluating it once and feeding each law would cost less, but it changes every evaluation path in the crate that every run's bits depend on, so it was not built.

**Where it comes from.** The synthesis of native-contract 42's two design candidates (`~/lanes/logs/ng-wave-4/nc42-judge/SYNTHESIS.md`, the part on shared evaluation) rejected shared evaluation for now: it was not measured, and it changes every evaluation path. It made it this ticket, behind the same surface a law's author uses for a term, with the prototype that one candidate and the judge named. The written term measured +2.4% of seated step time with three veins. The synthesis keeps 2% of seated step time as the bar for this ticket.

The same synthesis asks for a count. In one candidate's measurement, three added algebraic rows cost 2% of a step and 37% of admission's instructions. An algebraic row that only defines a value from others may leave the solved system, and that may be worth far more than one term.

- [x] A prototype of shared evaluation: four private producer instances feeding the three host laws, with derivatives in the unknowns and the parameters, and the rows and sparsity unchanged. Its derivatives equal the written term's, bit for bit
- [x] Its cost against the written term, as seated step time on one named build worker under the same other load, held to the 2% bar
- [x] A count of how many of the slice's and the whole body's algebraic rows are plain definitions
- [x] The Answer says whether shared evaluation is built, and whether the definitions leave the solved system, each as its own ticket if so

**Non-goals:** building shared evaluation in this ticket. Changing a term's surface for a law's author. The three-halves power, which is [native-contract 54](54-three-halves-power-as-ratio-times-square-root.md).

## Answer

Neither shared site-term evaluation nor eliminating plain algebraic definitions from the solved system clears the 2% seated step-time bar, so neither is built in production and no follow-up build ticket is opened for either. Both prototypes live on branch `proto/nc55-explicit-rows` with exact measurements on `beelink` using load-invariant `perf stat -e instructions:u`.

**Part A — Shared site-term evaluation (do not build).** `Assembly::compile_shared` compiles private `TermInstance` producers for terms feeding at least `min_hosts` host laws while keeping rows, sparsity, parameter slots, and refusal sources unchanged (`crates/ng-contract/src/evaluate/shared_terms.rs`). Against an un-CSE'd written term, `TermDerivativeMode::Exact` matches every value, Jacobian entry, and parameter slope to 0 ULP (bit for bit); against a CSE-inlined written term (`Written::shared()`), values and parameter slopes match to 0 ULP and 5/129 Jacobian entries differ by 1 ULP because separating the producer disables cross-boundary CSE of `lumen.volumes`.

Measured on `beelink` (`perf stat -e instructions:u`, 60 s seated `ng_solver::Core::new`, 11,614 steps):
- Pre-NC57 3-host `vein collapse` slice (4 producers feeding `blood_pressure`, `elastic_energy`, `stroke_work`): inlined `833.09 M ins/sim-s`, `Shared(2, Exact)` `834.18 M ins/sim-s` (`+0.13%`), `Shared(2, Chain)` `833.53 M ins/sim-s` (`+0.05%`). Pure 2×N step-mix: `-0.06%` to `+0.03%`.
- Current `slice` (post-NC57 split into 3 distinct terms): 0 terms have fan-out > 1 (`0.00%`).
- Current `body` (1,756 instances; 6 producers = 4 `vein collapse stroke work` + 2 1-op `MT` terms): pure step-mix `41.898 M` -> `41.878 M` (`-0.05%`, `Exact`) / `41.937 M` (`+0.09%`, `Chain`).

Shared evaluation misses the 2% bar by roughly 30× because only 4/380 (`slice`) or 4/1,756 (`body`) site instances are marked veins, `stroke_work` is active only during systole, and producer separation forfeits cross-boundary CSE.

**Part B — Plain algebraic definition rows (count and decision).** Exact structural counts (`crates/ng-contract/src/explicit.rs`):
- `slice` (241 rows, 76 algebraic): **75 / 76** (98.7%) are pure acyclic definitions (`0 = x - f(y)`, no self-dependence: 74 single-instance `define`/`residual` rows + 1 `sum_at` `CV.elastic energy`), 0 coupled-block explicit, and 1 closed (`CV.wall power`).
- `body` (1,465 rows, 196 algebraic): **147 / 196** (75.0%) are pure acyclic definitions (145 single-instance + 2 `sum_at`: `CV.elastic energy`, `CV.contractile store`), plus **24 / 48** blood-chemistry explicit definitions inside 4-row coupled SCCs (`12 × BC.bicarbonate`, `12 × BC.oxygen saturation`, yielding **171 / 196** = 87.2% acyclic explicit), 24 truly self-implicit (`12 × BC.acidity`, `12 × BC.oxygen partial pressure`), and 1 closed (`CV.wall power`).

Measured on `beelink` across `{MainSymMD, BTF+MinFill (Cand C)} × {None, ProjectOnly, FillNonIncreasing, AllPure, AllWithBc}` (all passing conservation certificates to `<= 2.29e-20` on `body` and `<= 1.63e-25` on `slice`):
- Eliminating all 147 pure rows (or all 171 with BC) triggers **star-graph hub fill-in** from ~46 high-fanout hubs (`CV.circulation`, `FL.starling`, `HP.sinusoids`, `CV.blood pressure`), *increasing* `body` 1×N LU `own_nnz` (`14,966 -> 17,952`, `+20.0%` under True BTF) and `factor_macs` (`37,731 -> 40,993`, `+8.6%`), slowing 20 s seated `body` stepping by `+5.63%` (`AllPure`) to `+6.34%` (`AllWithBc`).
- Restricting condensation to greedy `FillNonIncreasing` (`Δnnz <= 0`: 53 rows on `slice`, 101 on `body`) avoids hub fill-in, but **True BTF + directed MinFill (`NC 89`) already eliminates 99% of that sparse-LU work automatically** (`body` `factor_macs` `37,731 -> 37,352`, `-1.0%`; `own_nnz` `14,966 -> 14,447`, `-3.5%`). Runtime Schur condensation therefore slows `body` stepping by `+1.73%` under True BTF (`+1.20%` under `MainSymMD`), though `ring::ring` condensation on the 101 `FillNonIncreasing` rows saves `-4.17%` (`36.91 B -> 35.37 B` ins/ring, `-61.6 B` over 40 admission rings) under True BTF (`-5.56%` under `MainSymMD`).
- Keeping all rows in `Layout` and sparse LU while projecting the 147 pure definitions in DAG order (`ProjectOnly`, `CoupledProblem::evaluate_projected`) saves `-1.05%` on `body` stepping (`4,094.5 -> 4,051.7 M ins/sim-s` under True BTF; `-0.93%` under `MainSymMD`) and `-0.86%` on `slice`, with unchanged `ModelHash` and `Layout`. Even combining that with the theoretical ceiling of compile-time elimination of the 101 `FillNonIncreasing` rows stays below the 2% bar (`~1.5–1.8%`) at high architectural cost across `Layout`, `StepProposal`, `ObservationLens`, and `ng-record`. Therefore explicit definitions stay in the solved system.
