# 89: The elimination's order inside its largest block

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4, and `crates/ng-contract/src/elimination.rs`

Type: prototype

Status: resolved

Claimed by: lu-order

Lands with: `.scratch/native-contract/issues/67-the-marchs-settle-test-reads-no-size.md`

Blocked by: none

Blocks: none

**Why.** Cost bounds the first milestone (SRS invariant I19): [ticket 26](../../body/issues/26-cost-budget.md) holds each loop to a share of the body's instructions, and the elimination's factoring is about a tenth of them. Two rows that joined the body's largest block, 644 rows, added about 186 M instructions of factoring a simulated second, 42% of the factoring, where two rows of 644 would add about 1% to a dense factoring. The cause is inferred to be the order or its fill: neither was printed. A new order moves every pin and seal, so it gets a design round.

**Evidence.** [Body 10](../../body/issues/10-lungs-breathe.md)'s fourth PR measured four forms of the lung's water on the beelink, the user instructions inside one timed repeat of the first minute, each against a base run in the same job:

| Form | Job and its base | M a simulated second | Over the base at its solve count | Factoring | Largest block | Admission |
|---|---|---|---|---|---|---|
| A lining that reads the gas's water toward its vapour limit | a pair, base 4,029.0 M | 4,244.6 | +213.2 | 626.5 M (base 440.8) | 646 | 296.5 s (base 166.4 s) |
| Its water at a stated volume | a run of four, base 4,030.7 M | 4,244.9 | +211.8 (+214.2 raw) | 626.5 M (base 440.6) | 646 | 230.9 s (base 185.0 s) |
| A fixed rate | the same run, its base 4,030.7 M | 4,048.3 | +15.4 | 442.1 M | 644 | 205.4 s |
| A flux in a memory | compile refuses it: a site law cannot read a body memory | | | | | |
| The feed-forward built | a pair, base 4,029.0 M | 4,048.7 | +17.2 | | 644 | 219.4 s (base 152.0 s) |

The rows that joined are the two gas water pools, the alveolar gas's and the conducting airway's. They join because the lung interstitium's water row reads the lining's column, and the lining read the gas.

- [x] The factor's nonzeros, and the two rows' places in the order, printed for the base and for the first build
- [x] The candidate orders, each with its cost on the whole body from the same bench in one job
- [x] What the run's identity covers, so a new order is a revision a replay can name
- [x] How every pin and seal is re-pinned once
- [x] A test that fails when rows join the largest block at more than a stated price

**What the feed-forward leaves.** The built lining reads no gas row, so the gas's water share is set by the airway's wash-out and not held at its vapour limit: on the body it rises to 1.0574 of the saturated vapour share at five minutes. A lining that reads the gas's water holds the share at the limit, and a later ticket that needs the gas at its limit would want that form at the feed-forward's price.

**Non-goals:** a new solver, a sparse direct solver from outside the workspace, and the lung's water, which body 10 builds as a feed-forward that reads no gas row.

## Comments

From the solver explorations: the elimination refuses a pivot only when it is exactly zero or not finite (`crates/ng-contract/src/elimination.rs`, the test on `diagonal`), and a substitution that is not finite falls back to the closed pattern. A pivot of rounding size passes. One exploration staged a two-row algebraic block with a tiny diagonal (rows F2a, F2b and FIX-F2 of `~/lanes/logs/ng-explore-solver/report-2/claims.tsv`, not rerun). With a mild nonlinearity a diagonal of 1e-17 cost two refusals, 1e-18 eight, and 1e-20 ended the run in `NoAdmissibleStep` after `IterationLimit`, and on the linear block the solve by itself returned a wrong vector with no refusal. So the run is loud, through Newton's own test, and the solve is silent. Its relative test, a diagonal at most 1e-12 of the smaller of its row's and its column's largest entry, cleared the fixture and met no pivot in `admission_energy`. If this ticket reopens the order, the pivot's test is its neighbour: a relative test changes no bit of a run that never meets it.

## Answer

**Base diagnosis and order places.** At `9f5ba6ce`, the whole body has 1,465 rows (10,162 Jacobian nonzeros) partitioned into 415 strongly connected blocks: one 669-row block, seventeen 21-row blood-solute blocks, one 4-row block, fourteen 3-row blocks, thirteen 2-row blocks, and 370 1x1 blocks (including all 318 held pools). Admission's lifted Radau leg block has 2,930 rows in 438 strongly connected blocks (one 1,338-row block, seventeen 42-row blocks, and 348 2x2 blocks including the 318 held pools). Global symmetric minimum degree across `A + A^T` ignores block boundaries and had already grown from 104,103 factoring multiply-adds at native-contract 83 (644-row block) to 173,242 (+66.4%) at `9f5ba6ce` (669-row block), with 166,494 (96.1%) inside the 669-row block and 1,893,681 of 1,959,503 (96.6%) inside the 1,338-row Radau block. The two gas-water pools (row 214, conducting airway gas water, and row 986, pulmonary bed gas water) stood at order positions 1,078 and 1,079 of 1,465 on `main` (1,078 and 1,083 when pulmonary interstitium water row 945, at position 1,370, reads row 986), interleaved inside the 669-row block with `own` lower/upper counts of 1/9 and 2/8 despite forming a separate 2-row block.

**Candidate orders measured on `beelink` against `9f5ba6ce`.** Ordering blocks block-lower-triangular inside a flat LU exploded whole-body factoring multiply-adds 4.5x (173,242 to 781,820) because the 17 downstream 21x21 blood-solute blocks filled `L[21, 669]`. True block-triangular factorization and solve (`Digraph::strongly_connected_components` in Tarjan reverse-topological order, keeping cross-block reads raw in `starts..intra_starts` and eliminating only inside diagonal blocks) eliminates cross-block fill completely. Inside each block of size > 1, exact directed minimum fill (breaking ties by Markowitz product `row_deg * col_deg`, then least unknown) beats symmetric minimum degree by 3.1x on factoring multiply-adds:

| Candidate | Body `own` nnz | Body factor MACs (solve MACs) | Radau `own` nnz | Radau factor MACs | Slice M ins/sim-s | Body M ins/sim-s | Body adm wall |
|---|---|---|---|---|---|---|---|
| Baseline (`9f5ba6ce`, global sym-MD) | 20,559 | 173,242 (19,094) | 88,213 | 1,959,503 | 800.2 | 4,329.3 | 289.9 s (load 3.5) |
| Cand A (SCC-smart-upper + sym-MD, flat LU) | 20,073 | 115,828 | 81,651 | 1,083,453 | 789.3 | 4,174.0 | 295.7 s (load 4.6) |
| Cand B (aware SCC-smart-upper + dir MinFill, flat LU) | 16,529 | 43,108 | 66,435 | 407,899 | 767.1 | 3,913.0 | 338.6 s (load 7.4) |
| **Cand C (True BTF + dir MinFill, chosen)** | **14,966 (-27.2%)** | **37,731 (-78.2%)** (13,501, -29.3%) | **59,169 (-32.9%)** | **318,134 (-83.8%)** | **775.4 (-3.1%)** | **3,915.1 (-9.6%)** | **267.7 s (load 4.1)** |

Candidate C skips all 370 1x1 blocks without entering the elimination loop, sizes the scatter scratch to `max_block` (669 on body, 1,338 on Radau, both stack-allocated), restricts `closure` and `ghosts` to diagonal blocks, skips untouched blocks in `substitute_reached_with` with seed-only backup, and enforces the relative tiny-pivot guard from the Comments (`|diag| <= 1e-12 * min(row_max, col_max)`, checked lazily from per-row `row_max` so it costs ~0.5% and triggers zero times on slice and body).

**Identity and replay.** `RunIdentity` covers `ModelHash`, `PacketHash`, `Seed` and `DeclaredResolution`; `Elimination` sits outside `Layout::seal` and `RunIdentity`, so `model_hash`, `conserved_basis` and `stated_packet` do not move while floating-point elimination order changes admitted and run digests at rounding scale. Naming the engine's numerics revision in `RunIdentity` belongs to [native-contract 81](81-a-runs-identity-names-no-revision-of-the-engines-numerics.md).

**Re-pinning once.** This change lands with [native-contract 67](67-the-marchs-settle-test-reads-no-size.md) so every affected admitted/run pin (`crates/ng-validate/tests/trace.rs`, `crates/ng-validate/tests/body_run.rs` and `tools/generative-physiology/driver/src/smoke.rs`) moves once. On `trace.rs`, all 30 printed lines (240 six-decimal values) across all three cores stay identical; only the six cryptographic hashes move. On `body_run.rs`, all 10 first-second trace lines, valve closing times to 9 decimals, step/solve/refusal counts, breath summaries, and conservation figures stay identical; only `PACKET`, `ADMITTED_SEAL`, the 1-second `seal`, the controlled-vs-held admission rounding residual (`1.272e-2` of admission tolerance), and the three tissue heat-store cycle rounding residuals (`~2e-11` against `1.3e-1` / `2e-3` tolerances) move.

**Block-join regression gate.** `CompiledBody::elimination_metrics` and `CompiledBody::elimination_metrics_with_reads` expose stored nonzeros, factoring multiply-adds, solve multiply-adds, block count and largest block size. `crates/ng-validate/tests/elimination_order.rs` (`joining_rows_to_the_largest_block_stays_within_the_factoring_price`) bounds baseline whole-body nonzeros (<= 16,000) and factoring multiply-adds (<= 45,000), joins the two gas-water pools to the largest block (669 to 671: 37,731 to 38,993 MACs, +3.3%) and joins a 21-row blood-solute block (669 to 690: 37,731 to 39,037 MACs, +3.5%), refusing any growth over +10%.
