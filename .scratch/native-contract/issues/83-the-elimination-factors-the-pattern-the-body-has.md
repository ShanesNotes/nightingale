# 83: The elimination factors the pattern the body has

Spec: `crates/ng-contract/src/elimination.rs` (`Elimination::plan`, `Elimination::factor`, `Factors`) and [body 26](../../body/issues/26-cost-budget.md)'s cost line

Type: task

Status: resolved

Claimed by: nc83-build

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours, and the linear solve is a large share of each simulated second. Today the plan stores and factors the symmetric closure of the body's pattern: an entry is kept wherever its mirror is. The extra entries hold zeros, and the factorization adds and multiplies them on every solve.

**What is asked.** Keep today's order: exact minimum degree on the symmetric pattern, ties to the least unknown, and today's pivots. Store the lower and the upper patterns apart, each with only the entries the body has and the entries that fill. Keep every bit of every solve and of every refusal's source.

- [x] The plan stores and factors the two patterns apart, at today's order and pivots
- [x] Every solve and every refusal keeps today's bits, on random systems and on the compiled body and slice, shown while both plans exist in the branch
- [x] Mutant rows for a dropped upper entry, a dropped lower entry and a missed fill entry, each CAUGHT
- [x] No pin moves
- [x] The body's first minute measured at base and head, with admission's time, and a profile at the head

## Answer

**What the plan holds.** `Elimination` keeps the symmetric pattern it orders by, `closure`, unchanged. Beside it, `own` stores each pivot's line with the entries its row holds and the entries the elimination fills, lower and upper apart. `ghosts` lists the entries of `closure` that `own` leaves out. The order, the pivots and the places of the Jacobian's entries are as before. A factorization and a solve run over `own`.

**Why the bits hold.** Every entry `closure` stores and `own` leaves out holds a zero. Above the diagonal it is a positive zero, and below it it is a zero divided by its pivot, so it has the pivot's sign. While every value is finite:

- In the factorization, a running value is never a negative zero. The scatter starts from a positive zero, and a subtraction gives a negative zero only from one. So subtracting a ghost's zero product changes nothing.
- In a solve, a ghost's product with a finite read is a zero whose sign is the two signs' difference. Subtracting it changes only a sum that is a negative zero, which it turns positive for good. A sum can end as a negative zero only where its side was one. So a group whose sides hold a negative zero settles each row that ends on one, and any other group walks without the ghosts.

Where a value is not finite, a zero times it is not zero. A factorization that meets such a value redoes its work over `closure`. A solve over `own` whose answer is not finite redoes its walk over `closure`, with the ghosts' zeros built from `own`'s factors. A finite answer means every value on the way was finite.

**Which cases a run reaches.** Over admission and the body's first minute, 33,700 factorizations ran. None met a value that was not finite, no solve went back to `closure`, and no ghost turned a sign (counters in a probe copy, one body run). The tests reach each case: sides with negative zeros turn signs on the compiled body thousands of times, and a value that is not finite in the Jacobian or a side takes `closure`. Each guard is kept, because a run could reach it.

**What the tests hold.** Each test runs 18 random systems of up to 64 unknowns. The systems' reads run one way, the rows are shuffled against the unknowns, entries hold zeros of both signs, and pivots have both signs. The first test compares `own`'s pattern with a dense symbolic elimination. The second compares `own`'s answer with a dense solve. The third compares every entry and ghost, and every solve alone, reached and full, with the symmetric plan's bits, on sides with zeros of both signs, values at the ends of the range and values that are not finite. The fourth puts an infinity, a NaN or an overflow into the Jacobian and holds the factors, the refusal's row and a solve to the symmetric plan's. A probe on the compiled body and slice gave the same: four seeds each, every one of the 31,392 and 5,197 entries, 24 sides alone and 6 groups reached and full, none apart.

**The figures.** All on the beelink.

| | Main | This plan |
|---|---|---|
| Body: stored entries | 31,392 | 18,894 |
| Body: multiply-adds a factorization | 471,724 | 104,103 (22.1%) |
| Slice: stored entries | 5,197 | 3,197 |
| Slice: multiply-adds a factorization | 42,322 | 14,031 (33.2%) |
| Instructions a factorization, body, synthetic values | 6.34 M | 1.84 M (29.0%) |
| Instructions a solve, body, synthetic values | 364.8 k | 263.0 k (72.1%) |
| Cost line, user instructions a simulated second | 5,175.7 M | 4,016.5 M (77.6%) |
| Admission's wall time | 192.8 s at load 2.0 | 146.8 s at load 2.2, 150.4 s at load 8.5 |

The counts come from the plan. The per-operation instructions count 2,000 repeats on the compiled body with no admission. The two scans a solve's guards add take 13.2 k of the 263.0 k. Both cost lines take 11,739 steps and 14,452 solves and end on seal `818f1809a3938706`. Main's line ran in one job with the first head, back to back, and the first head read 4,170.0 M; the second head's line adds the cheaper solve. The profile at the head, a frame-pointer build sampled on user instructions, gives the factorization 10.8% of the line and the triangular solves 7.6%. Six body admissions were made: main's line, two head lines, the probe's minute and two profiles.

The line fell 22.4%, by 1,159.2 M a simulated second (measured). A factorization keeps its scatter and its row setup, so its instructions fall to 29.0% of main's and not to the count's 22.1%.

**No pin moved.** The model hash, the packet digest, the admission seal, every run seal and every trace pin, in `ng-contract`'s and `ng-validate`'s tests. The run's seal and counts on the cost line are unchanged. The elimination is outside every seal and hash.

**Left for later.** Ordering by strongly connected block would cut the multiply-adds further, and it moves a run's seal by rounding, so it is a later ticket. This change leaves room for it: `own` takes any order the plan gives it.
