# 68: The ledger's sum of a combination's rows reads one last place off when the rows move among themselves

Spec: [solver contract](../../../docs/architecture/solver-contract.md) §5 (compensated values survive into the certificate) and its tradeoffs line on two f64 per stored amount, `crates/ng-record/src/history.rs` (`LedgerChunk::after`, `History::opening`) and `crates/ng-validate/src/certificate.rs` (the closure check)

Type: task

Status: claimed

Claimed by: nc68-build

Blocked by: none

The milestone protocol's 48 h run on `ng_solver::Core::new` at 10 ms refused at [native-contract 52](52-energy-fails-the-sliding-windows.md)'s head: albumin's closure on the window from 06:55:00.002930551 was 2^-96 (1.262177448353619e-29), where the slice allows it exactly zero, because no route carries albumin. `certify` keeps only the earliest failing window, so the run said nothing of how many failed or of energy.

**The cause.** Albumin is 20 rows that 31 columns inside the body move among themselves: circulation, valves, Starling flow, lymph and the albumin exchange. Each transfer is exact in both rows it moves: over the first two hours of the arena's plan, 1,488,980 steps, the exact sum of albumin's rows never changed. The ledger does not read that exact sum. It sums the rows in compensated arithmetic, one row at a time in row order, and a compensated pair holds about 106 bits. From 08:03:38 a small row holds a bit finer than the total's last place, and the sum must round that bit. Which way it rounds depends on how the rows split the total, not on the total, so the sum moved by one last place, and back, in 43,672 of those steps. A window whose two ends read the sum on different sides of that rounding fails closure. Rows (1000, 3.8173233833909553e-17), (599.9999999999999, -2.158212844549645e-14) and (85, 2.096380695060251e-20), with 7.506156663584488e-24 moved from the second to the third, show it in three rows: each row holds the transfer exactly and the sum moves by -2^-96. The evaluation of a window's end inside a step is not the cause: a curve whose two ends are equal reads that value at every instant.

The short protocols pass because they end before the first fine bit. The scored pick passed because its one window reads the sum at two instants and both fell on the same side.

**A second rounding.** A row cannot hold a transfer finer than its own last place, and two rows of different size round one transfer to different places, so the stored total itself moves. Two reservoirs at 600 and 424 relaxing toward 512 each, with no route, refuse closure this way after 3.6 h at 100 ms steps. The slice has not shown it yet: over four hours the exact sum never moved, but the rows' least room above their last place fell from 15 bits to 4 at 08:03 and to 2 at 10:41, so a 48 h run may reach it.

**What to build:** the sum a combination holds reads its rows' exact total, rounded once to a compensated pair, so it depends on the total alone; and the closure floor admits a stated count of the total's last places, for the row rounding a compensated amount cannot avoid.

- [ ] `crates/ng-record/src/history.rs` sums each combination's rows exactly (an integer sum of each part times its whole weight) and rounds once to the nearest pair, for each step's chunk and for the opening.
- [ ] A test that fails before the change: the three rows above read one sum before and after the transfer.
- [ ] Mutant rows in `tools/gate/mutants.toml`: the sum folded in compensated arithmetic again, and the rounding's tie broken away from even.
- [ ] `crates/ng-validate/src/certificate.rs`: `CertificatePolicy::admitting_last_places`, in the policy's digest and reported in each `QuantityResult`; `slice.rs` sets 1,024 for `arena_certificate`. A test: the two reservoirs at 600 and 424 refuse at zero last places and pass at 1,024. A mutant row drops the last places from the allowance.
- [ ] The pins that move are updated with their reason: `trace.rs`'s three first-second seals and `body_run.rs`'s `ADMITTED_SEAL`.
- [ ] The contract's §5 sentence on conserved-basis sums says they are exact and rounded once; the tradeoffs line on two f64 per amount says closure is limited to a stated count of last places; §7 and the first slice's policy values name that count; the glossary's policy value entry names it.
- [ ] The milestone driver reports, for every combination, its worst closure against what its window allowed, its worst drift against its bound, each with its window, and how many window starts failed each check, whether the certificate passes or refuses.
- [ ] The 48 h milestone run passes, together with native-contract 52's first box, and its report shows each combination no route moves at a closure of zero or within its last places, which then names the row rounding above.

## Answer
