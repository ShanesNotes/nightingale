# 89: The elimination's order inside its largest block

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 4, and `crates/ng-contract/src/elimination.rs`

Type: prototype

Status: open

Blocked by: none

Blocks: none

**Why.** Cost bounds the first milestone (SRS invariant I19): [ticket 26](../../body/issues/26-cost-budget.md) holds each loop to a share of the body's instructions, and the elimination's factoring is about a tenth of them. Two rows that joined the body's largest block, 644 rows, added about 186 M instructions of factoring a simulated second, 42% of the factoring, where two rows of 644 would add about 1% to a dense factoring. The cause is inferred to be the order or its fill: neither was printed. A new order moves every pin and seal, so it gets a design round.

**Evidence.** [Body 10](../../body/issues/10-lungs-breathe.md)'s fourth PR measured four forms of the lung's water on the beelink, the base and each form back to back at one load, the user instructions inside one timed repeat of the first minute:

| Form | M a simulated second | Factoring | Largest block | Admission |
|---|---|---|---|---|
| Base, main at `b640ea3a2084` | 4,030.7 | 440.6 M | 644 | 185.0 s |
| A lining that reads the gas's water toward saturation | 4,244.6 (+213 at the base's solve count) | 626.5 M | 646 | 296.5 s |
| Saturation at a stated volume | 4,244.9 (+214) | 626.5 M | 646 | 230.9 s |
| A fixed rate | 4,048.3 (+15.4) | 442.1 M | 644 | 205.4 s |
| A flux in a memory | compile refuses it: a site law cannot read a body memory | | | |

The rows that joined are the two gas water pools, the alveolar gas's and the conducting airway's. They join because the lung interstitium's water row reads the lining's column, and the lining read the gas.

- [ ] The factor's nonzeros, and the two rows' places in the order, printed for the base and for the first build
- [ ] The candidate orders, each with its cost on the whole body from the same bench in one job
- [ ] What the run's identity covers, so a new order is a revision a replay can name
- [ ] How every pin and seal is re-pinned once
- [ ] A test that fails when rows join the largest block at more than a stated price

**Non-goals:** a new solver, a sparse direct solver from outside the workspace, and the lung's water, which body 10 builds as a feed-forward that reads no gas row.
