# 79: What a vascular edge's transport columns cost

Spec: [solver contract](../../../docs/architecture/solver-contract.md) and `crates/ng-validate/src/body/packet.rs` (the shunt, the body's newest vascular edge)

Type: prototype

Status: resolved

Claimed by: edge-transport

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours, and what a step costs sets how long that takes. The check of nightingale#194 found that one new vascular edge costs the whole body more than its one flow (`~/lanes/logs/ng-wave-4/b10a/verify-coordinator/VERIFY-10a.md`, items 1, 10 and 12).

**What the check found.**

- The shunt, one new vascular edge, added one flow row and 33 columns to the whole body: 32 that carry blood's 31 solutes and its water, and one for gravity's work. It added 194 nonzeros.
- By the instructions a solve costs, it costs about 138.8 M a simulated second, 2.6% of the body's line. That is inferred from the solve count, not run at a fixed count.
- No species can be left off the edge without breaking the blood's composition across it.
- The body has 22 such edges, and the organs still to come add more.

- [x] A count, from the compiled body, of how many of its columns are edges' transport columns and what share of a solve's instructions they take
- [x] A prototype of a cheaper statement of an edge's transport that keeps every book, with solves by kind and the cost line on the whole body
- [x] Replay stays exact, or the Answer says what it would take
- [x] The Answer says whether the contract's layout would change, and if so asks for a design round

**Non-goals:** which species blood carries, and any change to a law.

## Answer

### 1. Compiled count and instruction share

From `whole_body()` (1,465 rows, 1,463 columns, 1,756 instances, 10,162 Jacobian nonzeros, 396 conserved combinations, 131,696 B per `LedgerChunk`, 131,760 B per link):

- **Vascular-edge transport (30 vascular edges):** 22 `CV.circulation` edges (704 columns: 22 water + 682 solutes), 5 `CV.valves` edges (160 columns: 5 water + 155 solutes), and 3 `HP.sinusoids` edges (96 columns: 3 water + 93 solutes) total **960 of 1,463 columns (65.62%)** (30 water + 930 solutes; plus 30 `CV.gravity work` columns = 990, or 67.67%), **960 of 1,756 instances (54.67%)**, **6,942 of 10,162 Jacobian nonzeros (68.31%)**, **61,440 of 131,696 B (46.65%)** of each `LedgerChunk` (59,520 B from the 930 solutes alone), and **0** nonzero incidence entries across all 396 conserved combinations.
- **All reflection-free `Context::Edge` fluid transport (44 edges):** adding 11 `FL.lymph` edges (55 columns: 11 water + 44 solutes), 2 `RN.tubular flow` edges (28 columns: 2 water + 26 solutes), and 1 `RN.filtration` edge (4 columns: 1 water + 3 solutes) brings fluid-transport edge columns to **1,047 of 1,463 (71.57%)** (44 water + **1,003 carried solutes**). On `slice` (241 rows, 257 columns, 380 instances, 1,580 Jacobian nonzeros, 21 conserved combinations, 18,480 B per `LedgerChunk`, 18,544 B per link), the 17 vascular edges contribute 102 of 257 columns (39.69%: 17 water + 85 solutes), and all 25 fluid-transport edges contribute 139 of 257 columns (25 water + 114 carried solutes).
- **Share of a solve's instructions (60 s, `ng-solver-10ms`, 14,411 whole-body solves):**
  - On the Wave 1 base (`nc79-wave1-base` `78a4f59f`, composing `opt/track-a-perf` with [native-contract 89](89-the-eliminations-order-inside-its-largest-block.md) and [native-contract 67](67-the-marchs-settle-test-reads-no-size.md): 2,645.0 M instructions/sim-s, 11.01 M instructions/solve), the 960 vascular-edge transport columns take **~871 M instructions/sim-s (~32.9% of whole-body instructions, ~3.63 M of 11.01 M instructions/solve)**, and all 1,047 fluid-transport edge columns take **~942 M instructions/sim-s (~35.6%)**. Of the ~871 M attributable to the 960 vascular columns, ~655 M are removed directly by the prototypes below (out of -730.4 M total across all 1,047 edge columns plus ~18 M from non-transport inlining), and ~216 M remain after Candidate 2 (`perf record` / `perf annotate` on `body` at `01b2e5de`, 66,677 samples: ~40 M in `evaluate_stage` where all 43 `TransportBatch`es take 43.9 M, ~10 M in `evaluate_published`, ~26 M in `jacobian_batch` where all 43 batches take 28.5 M, ~92 M in remaining active forward/flow entries of `eliminate_in` + `solve_in`, ~40 M in paired `Compensated` net-extent accumulation in `Quadrature` + `next_state`, and ~8 M for the 30 retained water-flow columns in `LedgerChunk` + `BLAKE2b`).
  - On pre-Wave-1 `main` (`9f5ba6ce`: 4,329.3 M instructions/sim-s, 18.03 M instructions/solve), the same 960 vascular-edge columns took **~2.05 to 2.20 B instructions/sim-s (~47 to 51% of a solve's instructions)**, consistent with the shunt check's inferred 138.8 M (2.6%) per edge prior to Wave 1.

### 2. Prototypes, solves by kind, and whole-body cost line

Two staged prototypes sit above `nc79-wave1-base` (`78a4f59f`), preserving every species, every law, and every conserved quantity and conserved combination (396 on `body`, 21 on `slice`), with zero external dependencies, `forbid(unsafe_code)`, strict IEEE-754 determinism, and zero edits to `ng-metabolism`, `ng-musculoskeletal`, `ng-respiratory`, or `crates/ng-validate/src/body/`:

1. **`proto/nc79-edge-transport` (zero layout change; commits `ddbbbb68`, `01b2e5de`, `090da9d2`):**
   - **Cand 1a (`ddbbbb68`):** `TransportBatch` in `crates/ng-contract/src/evaluate.rs` batches all carried solutes sharing a `(flow_row, from_vol, to_vol)` edge into one flow-sign and upwind pass in `evaluate_stage` and one active-only `jacobian_batch` scatter (skipping the inactive downwind endpoint), alongside a `Compensated::scaled` `+/-1` fast path, first-stage copy in `Quadrature::accumulate`, and zero-skip in `LedgerChunk::after`.
   - **Cand 1b (`01b2e5de`):** Compile-time specialized BTF+MinFill elimination masks (`Elimination::transport_masks` in `crates/ng-contract/src/compile.rs` and `elimination.rs`) derived structurally from `Context::Edge` forward downwind zeros and the top-4 cycle-breaking shuttable valves discovered by Tarjan SCC (no organ names hardcoded), lifted to `2xN` Radau blocks in `StagePattern::radau`, checked branchlessly with `(v.to_bits() << 1) == 0` (accepting Radau's `-0.0` while rejecting any nonzero, subnormal, `NaN`, or infinity) and falling back to full `closure` on non-finite pivots. On `whole_body()`, 1xN factoring multiply-adds drop from 37,731 to 9,591 in diastole (-74.6%) and 9,887 in systole (-73.8%), collapsing all seventeen 21x21 blood-solute SCCs to 1x1; `2xN` Radau factoring multiply-adds drop from 289,781 to 79,146 (-72.7%, 24.8x fewer than `main`) and solve multiply-adds from 46,328 to 34,811 (-24.9%).
   - **Cand 1b+ (`090da9d2`):** Fuses `evaluate_published` over `TransportBatch` (sharing `push_validity` and `push_judged` with `excursions`) and marks `Compensated` and `AdmissibleSet::holds` `#[inline]`.
2. **`proto/nc79-compact-layout` (compact ledger layout stacked on `090da9d2`; commit `cd2c7f44`):**
   - **Cand 2 (`cd2c7f44`):** `LedgerPlan::of(&layout)` in `crates/ng-contract/src/body.rs` partitions columns into `ledger_columns` (460 on `body`, 143 on `slice`: all boundary, reaction, wall, heat, water-flow, and nonzero-basis-incidence columns) and `carried_solutes` (1,003 on `body`, 114 on `slice`: reflection-free internal `Context::Edge` `Upwind5` unit solute transfers whose two endpoint pools hold identical weights in every conserved combination). Carried solutes accumulate as paired `Compensated` net extents in `Quadrature` and `next_state` (`amounts[from] += net.negated(); amounts[to] += net;`), keeping every conserved combination closed and every accepted state pool bit-identical while dropping 64,192 B/step (-48.72% of link bytes) from `LedgerChunk` and `BLAKE2b` hashing.

**Solves by kind (60 s, `ng-solver-10ms`, identical across every candidate below):**

- **`slice`:** 11,614 accepted steps, **14,749 solves** = **12,399 ordinary** (11,614 accepted, 785 refused) + **2,350 landing** (2,350 accepted, 0 refused).
- **`body`:** 11,661 accepted steps, **14,411 solves** = **12,351 ordinary** (11,661 accepted, 690 refused) + **2,060 landing** (2,060 accepted, 0 refused).

**Cost line on `beelink` (60 s, `ng-solver-10ms`, load-invariant hardware `instructions:u`):**

| Build | `slice` M ins/sim-s (vs Wave 1 / `main`) | `body` M ins/sim-s (vs Wave 1 / `main`) | `body` M ins/solve | `body` wall ms (`xRT`, box state) | `body` adm wall | `body` link B/step | Seals (`slice` / `body`) |
|---|---|---|---|---|---|---|---|
| `main` (`9f5ba6ce`) | 800.2 (+54.5% / -) | 4,329.3 (+63.7% / -) | 18.03 | 321.7 (3.11x, quiet) | 289.9 s | 131,760 | `29070aac...` / `b29d2d84...` |
| `nc79-wave1-base` (`78a4f59f`) | 517.9 (- / -35.3%) | 2,645.0 (- / -38.9%) | 11.01 | ~190 est (~5.3x, quiet est) | 222.0 s | 131,760 | `3e99e496...` / `3fbe2060...` |
| **Cand 1a** (`ddbbbb68`) | **484.6** (-6.4% / -39.4%) | **2,409.2** (-8.9% / -44.4%) | **10.03** | ~173 est (~5.8x, quiet est) | 182.6 s | 131,760 | **`3e99e496...` / `3fbe2060...`** |
| **Cand 1b** (`01b2e5de`) | **454.4** (-12.3% / -43.2%) | **2,221.9** (-16.0% / -48.7%) | **9.25** | 167.8 to 173.7 (5.8 to 6.0x, loaded) | 168.6 to 191.9 s | 131,760 | `ddb1d736...` / `1a63436b...` |
| **Cand 1b+** (`090da9d2`) | **442.4** (-14.6% / -44.7%) | **2,148.4** (-18.8% / **-50.4%**) | **8.95** | **154.3** (6.48x loaded; ~6.7x quiet est) | **173.1 s** (-40.3% vs `main`) | 131,760 | **`ddb1d736...` / `1a63436b...`** |
| **Cand 2** (`cd2c7f44`) | **420.4** (-18.8% / -47.5%) | **1,914.6** (-27.6% / **-55.8%**) | **7.97** | **141.8** (**7.05x** loaded; ~7.3x quiet est) | 188.9 s (loaded) | **67,568 (-48.7%)** | `210afb41...` / `bb72e080...` |

Quiet-box multi-repeat wall-time runs on `beelink` should confirm the wall ms line once `feat/nc89-nc67-lu-and-march` lands on `main` and both branches rebase onto it.

### 3. Replay

- **Cand 1a (`ddbbbb68`):** Replay stays **100% bit-identical** to `nc79-wave1-base` (`slice` seal `3e99e496a330e741`, `body` seal `3fbe20600e7fef9c`); zero digests or pins move.
- **Cand 1b / 1b+ (`01b2e5de`, `090da9d2`):** Within the build, replay, pace, resume, rewind, late admission, and regeneration stay **exact** (`replay` 84/84, `slice_certificate` 6/6, `admission` 85/85 pass). Specialized BTF elimination changes pivot order inside diagonal blocks, so admitted and run seals move at rounding scale (~1 ULP; 0 of 240 printed six-decimal values in `crates/ng-validate/tests/trace.rs` change; `model_hash` and `conserved_basis` do not move). Naming the engine's numerics revision in `RunIdentity` belongs to [native-contract 81](81-a-runs-identity-names-no-revision-of-the-engines-numerics.md); landing Cand 1b+ requires one digest re-pin of `trace.rs` (and `body_run.rs` if pinned).
- **Cand 2 (`cd2c7f44`):** Within the build, replay, checkpointing, rewind, late admission, and regeneration stay **exact** (`replay` 84/84, `transaction` 100/100, `split` 21/21, `slice_certificate` 6/6 pass), and every accepted state pool bit (`operators.rs` checks exact `hi` and `lo` bits of `Water` and carried `Albumin`) and admitted packet digest (`named` hash) stay identical to `090da9d2`. Because `LedgerChunk.columns` omits the 1,003 (`body`) / 114 (`slice`) folded carried-solute columns, link bytes and `sealed` run digests change; cross-version replay or reading history written before Cand 2 requires a `RecordFormat` version bump (`ngcheck3`) and format-keyed decoding.

### 4. Contract layout change and design-round request

- **Path A (`proto/nc79-edge-transport`, Cand 1a + 1b + 1b+): no contract layout change.** `Layout`, `ModelHash`, `conserved_basis`, `LedgerChunk`, `RecordFormat`, and every public crate API remain unchanged (-18.8% vs Wave 1, -50.4% vs `main`, 2,148.4 M instructions/sim-s). Ready to rebase onto `main` after [native-contract 89](89-the-eliminations-order-inside-its-largest-block.md) / [native-contract 67](67-the-marchs-settle-test-reads-no-size.md) merge, with no layout design round required.
- **Path B (`proto/nc79-compact-layout`, Cand 2): changes the ledger/record layout and asks for a design round.** `Layout` and `ModelHash` stay unchanged, while `LedgerChunk` in `crates/ng-record/src/history.rs` stores only `ledger_columns` (460 of 1,463 on `body`, 143 of 257 on `slice`), `HistorySegment::exchange(ColumnId)` narrows to ledger-tracked columns beside `HistorySegment::try_exchange(ColumnId) -> Option<...>`, and serialization needs a `RecordFormat` version bump. This delivers a further -233.8 M instructions/sim-s (-27.6% vs Wave 1, -55.8% vs `main`, 1,914.6 M instructions/sim-s, >7x RT) and cuts history link storage by 48.7% (131,760 to 67,568 B/step). A **design round is requested before landing Path B** to settle:
  1. Whether omitting gross forward/reverse extent curves for internal zero-incidence carried solutes from `LedgerChunk` (while keeping all 396 conserved combinations, all boundary/reaction/wall/heat/water columns, and bit-identical state pools) satisfies SRS invariant I10 and observer/diagnostic needs.
  2. The `RecordFormat` version bump and `HistorySegment::exchange` / `try_exchange` public surface in `ng-record`.
  3. Whether `LedgerPlan`'s partition should enter `ModelHash` / `RunIdentity`.

### Verification

Checked on both branches: `tools/verify.d/10-docs.sh`, `tools/verify.d/05-seam.sh` (all 18 rules/probes, 55 must-fail rows), `tools/verify.d/15-f64.sh` (18/18 leaks flagged), `cargo clippy --workspace --all-targets -- -D warnings`, `python3 -I tools/mutate.py tools/gate/mutants.toml --check` (707/707 mutants apply; 14/14 targeted modified-file mutants on `evaluate.rs`, `elimination.rs`, and `ring.rs` caught), `ng-contract` (231/231), `ng-record` (10/10), and `ng-validate` (`accuracy` 6/6, `admission` 85/85, `admission_radau` 8/8, `admission_crossings` 15/15, `arena_body` 10/10, `body` 18/18, `elimination_order` 1/1, `operators` 3/3, `prior_shape` 9/9, `reference_count` 7/7, `replay` 84/84, `slice_certificate` 6/6, `split` 21/21, `trace` 4/4, `transaction` 100/100).
