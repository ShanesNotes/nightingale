# 83: The elimination factors the pattern the body has

Spec: `crates/ng-contract/src/elimination.rs` (`Elimination::plan`, `Elimination::factor`, `Factors`) and [body 26](../../body/issues/26-cost-budget.md)'s cost line

Type: task

Status: claimed

Claimed by: nc83-build

Blocked by: none

**Why.** The first milestone (SRS invariant I19) runs the whole body for 48 hours, and the linear solve is a large share of each simulated second. Today the plan stores and factors the symmetric closure of the body's pattern: an entry is kept wherever its mirror is. The extra entries hold zeros, and the factorization adds and multiplies them on every solve.

**What is asked.** Keep today's order: exact minimum degree on the symmetric pattern, ties to the least unknown, and today's pivots. Store the lower and the upper patterns apart, each with only the entries the body has and the entries that fill. Keep every bit of every solve and of every refusal's source.

- [ ] The plan stores and factors the two patterns apart, at today's order and pivots
- [ ] Every solve and every refusal keeps today's bits, on random systems and on the compiled body and slice, shown while both plans exist in the branch
- [ ] Mutant rows for a dropped upper entry, a dropped lower entry and a missed fill entry, each CAUGHT
- [ ] No pin moves
- [ ] The body's first minute measured at base and head in one job, with admission's time, and a profile at the head
