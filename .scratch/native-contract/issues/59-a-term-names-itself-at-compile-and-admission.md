# 59: The site term's loose ends: a refusal names the term, a forcing keeps its order, and two rows test what they claim

Spec: `crates/ng-contract/src/compile.rs`, `tools/gate/mutants.d/00-base.toml`, `tools/gate/mutants.d/nc59-terms.toml`, and [native-contract 42](42-veins-collapse-branch.md)'s part 2

Type: task

Status: resolved

Blocked by: none

**Why.** The first milestone (SRS invariant I19) is judged through refusals that name the law at fault and through mutant rows that each hold one rule. Native-contract 42's part 2 landed the site term with four small gaps, found by its check and its build. None changes a value; each changes what a refusal names or what a row proves. The plan of part 3 (`~/lanes/logs/ng-wave-4/nc42-pr3/PLAN.md`, item 9) sized each one.

- [x] A refusal at compile or admission names the term, not the law that adds it, where the read belongs to the term alone. For a read a variant uses, the op's origin is in `Code::origins`, so `UnemittedField` and `UnbackedConstant` can take `origin(op).unwrap_or(law)` where `addresses` resolves a read. An unused read has no op, so it stays named by the law, and the PR says so. Ticket 42's part 2 left this box open
- [x] With a term added and no site in its set, a body with an unread forcing and a second defect is refused by the forcing, as main refuses it. Today the second defect names the refusal. `check_chance` hands the forcing on only when some term's set has a site in the map, which needs the anatomy passed in. Or the contract says that the outcome, not the name, is main's
- [x] The row `unscaled-input-compiles` mutates a shared predicate, `fn unscaled(reads: &[bool]) -> bool`, called at both places that test it, so the row breaks both and not main's line alone
- [x] The row `variant-reads-addressed` fails on an assertion of the variant's address count, made before the variant is evaluated, and not by a panic in `Tape::bind`

**Non-goals:** the term's cost, which is [native-contract 55](55-a-term-evaluated-once-and-rows-that-are-definitions.md). Any law's form.

## Answer

All four gaps are closed in `crates/ng-contract` without changing any value, digest, or pin:

1. **Term naming for `UnemittedField` and `UnbackedConstant`:** `addresses` (`crates/ng-contract/src/compile.rs`) now names `reader` (`origin(op).unwrap_or(law)`) in `CompileDefect::UnemittedField`, and `CompiledBody::unbacked_constant` (`crates/ng-contract/src/packet.rs`) locates the offending op with `position` and returns `program.code.origin(op).unwrap_or(program.law)`. An unused read from a discarded term has no op in `Code::origins` (`origin: None` on `UnreachedRead`), so it stays named by the host law, as tested in `evaluate::terms::a_terms_unused_invalid_read_is_still_refused`. Shared ops cleared by `Written::shared` also stay named by the host law, as tested in `evaluate::terms::forms::a_terms_used_unbacked_constant_names_the_term`.
2. **Unread forcing order with an empty term set:** `KernelData::has_placed_term` (`crates/ng-contract/src/law.rs`) checks `Placing::any_inside`, and `check_chance` (`crates/ng-contract/src/compile.rs`) now takes `&Anatomy` and defers an unread forcing only when some law in the fragment has a placed term whose set holds at least one site in the map (`!anatomy.in_set(set).is_empty()`). When no site is in the term's set, `UnreadForcing` fires before `check_volume` and `place`, matching main (`evaluate::terms::forms::an_unread_forcing_beside_a_second_defect_names_the_forcing_when_no_site_is_in_the_terms_set`).
3. **Shared `unscaled` predicate:** Extracted `fn unscaled(reads: &[bool]) -> bool` in `crates/ng-contract/src/compile.rs` and called it for both the base program and each variant. Updated `unscaled-input-compiles` in `tools/gate/mutants.d/00-base.toml` to mutate the shared predicate and added `unscaled-input-compiles-in-variant` in `tools/gate/mutants.d/nc59-terms.toml`.
4. **Explicit address-count assertion before `Tape::bind`:** At the end of `place` in `crates/ng-contract/src/compile.rs` (before `CompiledBody::seal` / `Tape::of` / `Tape::bind`), every placed instance now asserts `placed.addresses.len() == expected` against its program's `Op::Input` count (and `evaluate::terms::a_read_main_refuses_by_hand_is_refused_through_a_term` asserts the same on `body.instances()`). Under `variant-reads-addressed`, the mutation fails on that explicit assertion rather than a panic inside `Tape::bind`.

Four new mutant rows (`unscaled-input-compiles-in-variant`, `unemitted-field-names-the-term`, `unbacked-constant-names-the-term`, `unread-forcing-defers-with-empty-term-set`) live in `tools/gate/mutants.d/nc59-terms.toml`.
