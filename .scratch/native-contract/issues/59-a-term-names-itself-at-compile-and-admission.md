# 59: The site term's loose ends: a refusal names the term, a forcing keeps its order, and two rows test what they claim

Spec: `crates/ng-contract/src/compile.rs`, `tools/gate/mutants.toml`, and [native-contract 42](42-veins-collapse-branch.md)'s part 2

Type: task

Status: open

Blocked by: none

**Why.** The first milestone (SRS invariant I19) is judged through refusals that name the law at fault and through mutant rows that each hold one rule. Native-contract 42's part 2 landed the site term with four small gaps, found by its check and its build. None changes a value; each changes what a refusal names or what a row proves. The plan of part 3 (`~/lanes/logs/ng-wave-4/nc42-pr3/PLAN.md`, item 9) sized each one.

- [ ] A refusal at compile or admission names the term, not the law that adds it, where the read belongs to the term alone. For a read a variant uses, the op's origin is in `Code::origins`, so `UnemittedField` and `UnbackedConstant` can take `origin(op).unwrap_or(law)` where `addresses` resolves a read. An unused read has no op, so it stays named by the law, and the PR says so. Ticket 42's part 2 left this box open
- [ ] With a term added and no site in its set, a body with an unread forcing and a second defect is refused by the forcing, as main refuses it. Today the second defect names the refusal. `check_chance` hands the forcing on only when some term's set has a site in the map, which needs the anatomy passed in. Or the contract says that the outcome, not the name, is main's
- [ ] The row `unscaled-input-compiles` mutates a shared predicate, `fn unscaled(reads: &[bool]) -> bool`, called at both places that test it, so the row breaks both and not main's line alone
- [ ] The row `variant-reads-addressed` fails on an assertion of the variant's address count, made before the variant is evaluated, and not by a panic in `Tape::bind`

**Non-goals:** the term's cost, which is [native-contract 55](55-a-term-evaluated-once-and-rows-that-are-definitions.md). Any law's form.
