# 89: The elimination's order inside its largest block

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4, and `crates/ng-contract/src/elimination.rs`

Type: prototype

Status: open

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

- [ ] The factor's nonzeros, and the two rows' places in the order, printed for the base and for the first build
- [ ] The candidate orders, each with its cost on the whole body from the same bench in one job
- [ ] What the run's identity covers, so a new order is a revision a replay can name
- [ ] How every pin and seal is re-pinned once
- [ ] A test that fails when rows join the largest block at more than a stated price

**What the feed-forward leaves.** The built lining reads no gas row, so the gas's water share is set by the airway's wash-out and not held at its vapour limit: on the body it rises to 1.0574 of the saturated vapour share at five minutes. A lining that reads the gas's water holds the share at the limit, and a later ticket that needs the gas at its limit would want that form at the feed-forward's price.

**Non-goals:** a new solver, a sparse direct solver from outside the workspace, and the lung's water, which body 10 builds as a feed-forward that reads no gas row.

## Comments

From the solver explorations: the elimination refuses a pivot only when it is exactly zero or not finite (`crates/ng-contract/src/elimination.rs`, the test on `diagonal`), and a substitution that is not finite falls back to the closed pattern. A pivot of rounding size passes. One exploration staged a two-row algebraic block with a tiny diagonal (rows F2a, F2b and FIX-F2 of `~/lanes/logs/ng-explore-solver/report-2/claims.tsv`, not rerun). With a mild nonlinearity a diagonal of 1e-17 cost two refusals, 1e-18 eight, and 1e-20 ended the run in `NoAdmissibleStep` after `IterationLimit`, and on the linear block the solve by itself returned a wrong vector with no refusal. So the run is loud, through Newton's own test, and the solve is silent. Its relative test, a diagonal at most 1e-12 of the smaller of its row's and its column's largest entry, cleared the fixture and met no pivot in `admission_energy`. If this ticket reopens the order, the pivot's test is its neighbour: a relative test changes no bit of a run that never meets it.
