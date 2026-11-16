# 39: A reaction grant names its space as well as its bed

Spec: `crates/ng-contract/src/law.rs` (`ReactsAt`) and `crates/ng-contract/src/compile.rs` (`check_reactions`), and [body ticket 04](../../body/issues/04-scopes-ports-and-census.md)'s Answer

Type: task

Status: resolved
Claimed by: nc39

Blocked by: none

A reaction grant names a site set and no space (`ReactsAt<C>`), so a granted card's reaction may sit in any space of its bed. Compile refuses one chemistry from two cards only where both stand in the same space of one site (`ReactionOwnedTwice`). So a second card's copy of a reaction in another space of the bed compiles. The check of nightingale#139, body ticket 04's part 4, showed it with its probe A5: the musculoskeletal card's glucose burn in the muscle bed's interstitium compiled beside metabolism's glucose burn in that bed's cells. Body 01 puts every reaction in cells (01:145). So the line in [body ticket 09](../../body/issues/09-tissues-respire.md) that a glucose burn the hepatic or musculoskeletal card declared at its bed would repeat metabolism's and be refused holds today only in the space of metabolism's burn.

No test shows that the one-owner check compares spaces. The check's row `v-owner-key-ignores-space` survives `cargo test -p ng-contract --test scopes`, the test its table names. The rows are in `~/lanes/logs/ng-body-wave-3/04-scopes-ports-census/verify-coordinator/pr139-scratch/verifier-rows-139.toml`, and the sweep beside them (`job-sweep.log`) marks it SURVIVED. The row, with its exact old and new text:

```toml
[[mutant]]
name = "v-owner-key-ignores-space"
path = "crates/ng-contract/src/compile.rs"
old = '''                        term.space,
                        term.change > 0,'''
new = '''                        Space::Cells,
                        term.change > 0,'''
```

**What to build:** a grant names the space as well as the bed, or the check refuses a reaction in a space its card was not given.

- [ ] A test: the musculoskeletal card declares metabolism's glucose burn in the muscle bed's interstitium, beside metabolism's burn in that bed's cells, and it is refused. If the fix makes it a type error, a compile-fail doctest names the error code and has a legal twin
- [ ] A mutant row that undoes the fix, CAUGHT by that test, on the PR's head (`tools/mutate.py --only`)
- [ ] The row `v-owner-key-ignores-space` above is CAUGHT by a test, or the Answer shows that no two cards can be given one chemistry in two spaces of one bed
- [ ] Metabolism's grant still reaches the space ticket 09 picks for its burn: the cells, or the interstitium beside the tissue store (ticket 09's second form)
- [ ] The sort the reverse rule rests on, `terms.sort_unstable()` in `check_reactions` (`crates/ng-contract/src/compile.rs`), is held by a test: a second card's reverse of a reaction written from its other species is refused. A mutant row that removes the sort is CAUGHT by that test. Every test today writes both cards' terms in one order, so the check of nightingale#139 found its row `r2-terms-not-sorted` SURVIVED
- [ ] The unit test `a_count_above_one_moves_the_seal_and_a_count_of_one_seals_only_the_other_fields` in `crates/ng-contract/src/body.rs` holds the seal's tag in bytes at counts two and three: each column seals to the four older fields, then `u64::MAX`, then the count. Today it holds the seal at count two only, so the check's rows `r2-tag-only-at-count-two` and `r2-count-sealed-as-two-at-most` survive

**Non-goals:** any reaction a body ticket declares, and any change to the one-owner rule within one space.

## Answer

The grant names no space. The one-owner check compares a reaction's terms without their spaces, so one chemistry from two cards at one site is `ReactionOwnedTwice` in whichever spaces of the site they stand. Record 0010 and the contract's must-fail row put one owner at one site, and a grant that named a space would close the hole only while every card's grant named the same one. Ticket 09's second form, metabolism's burn in the interstitium, stays open, and the musculoskeletal card's copy in the cells is refused beside it. Moving metabolism's grant to the cells would have moved the burning-beds pin.

- Probe A5 is a test in `crates/ng-contract/tests/scopes.rs`, and so is its mirror. Both are refused naming both laws and the muscle bed. The row `reaction-owners-compared-in-one-space` puts the space back in the key, and they catch it.
- The row `v-owner-key-ignores-space` is the rule now, so it stays out.
- The hepatic card's reverse of glycolysis, written from lactate, is refused. The row `reaction-terms-sorted` removes the sort, and that test catches it.
- The unit test on a column's seal holds the tag in bytes at counts two and three. The rows `column-count-tagged-at-every-count` and `column-count-sealed-whole` are the check's `r2-tag-only-at-count-two` and `r2-count-sealed-as-two-at-most`, and it catches both.
