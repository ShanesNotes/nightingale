# 69: The ledger walks every combination against every column

Spec: `crates/ng-record/src/history.rs` (`LedgerChunk::after`, `History::from_opening`, `History::digest`), [body 26](../../body/issues/26-cost-budget.md)'s cost line

Type: task

Status: resolved

Claimed by: nc69-build

Blocked by: none

The first milestone (SRS invariant I19) runs the whole body for 48 hours, and its cost a simulated second sets how long that takes. A perf profile by symbol of the body's cost run at main e8e8bc0 puts 28.32% of user instructions in one closure of `LedgerChunk::after`:

```
28.32%  [.] <...Enumerate<Iter<ng_contract::conservation::Combination>>, <ng_record::history::LedgerChunk>::after::{closure#2}> ...>::fold
```

For every conserved combination, each step walked every column against a dense row of net weights that is nearly all zero: 436 combinations times 1,356 columns. Body 26 gives the gas loop 12% of the body's cost, so this walk cost more than twice that loop.

**What to build:** each combination keeps its columns with a nonzero net weight, in column order, built once where the rows were built. The walk adds the same terms in the same order, so no bit moves.

- [x] `History` holds each combination's nonzero columns and the column count. `LedgerChunk::after` reads the step's gross exchange once and walks only those columns.
- [x] The opening seal and `History::digest` still write every column's weight, zeros included, from the sparse list. They were the dense rows' only other readers, and their bytes are unchanged.
- [x] A test on generated rows and steps holds the sparse walk to the dense one bit for bit, and the sealed bytes to the dense bytes.
- [x] Mutant row `ledger-walk-drops-the-last-nonzero-column`: CAUGHT by that test.

## Answer

The walk reads only a combination's nonzero columns. Every pin, seal and digest holds: the workspace's tests pass untouched, and the cost lines' seals match the base's.

Cost lines on the beelink, `python3 -I tools/bench/run.py cost`, user instructions a simulated second:

| Assembly | Base e8e8bc0 | Head | Counts |
|---|---|---|---|
| Slice | 907.4 M (load 6.5) | 890.4 M (load 17.7) | equal: 241 rows, 257 columns, 11,614 steps, 14,749 solves, seal a50fdfdb08184a02 |
| Body | 6,964.4 M (load 6.1) | 4,958.9 M (load 13.7) | equal: 1,419 rows, 1,356 columns, 11,794 steps, 15,263 solves, seal 848db8e180cf19c7 |

The body fell by 28.8%, about 34.0 M a step (196.6 steps a simulated second). The slice fell by 1.9%.

The profile at the head: the ledger's walk fell from 28.32% to 0.48% (the closure 0.15% and `flowed` 0.33%). The next three symbols under `ng_record` are `LedgerChunk::write` 0.92%, `exact_total` 0.73% and the columns' closure in `after` 0.66%. Nothing under `ng_record` is over 2%.

**What is left.** `ng_contract::identity::blake2b_block` is 10.19% at the head, the third symbol overall. Some of it hashes each step's chunk into its link. A symbol profile cannot say how much; a call-graph profile would. That is the next ledger cost to look at.
