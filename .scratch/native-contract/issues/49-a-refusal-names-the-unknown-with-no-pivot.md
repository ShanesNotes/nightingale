# 49: An admission refusal names the unknown that has no pivot

Spec: `crates/ng-contract/src/admission.rs` (`InitializationSystem::solve` and `admit`), `crates/ng-contract/src/ring.rs`, and the solver contract's paragraph on admission

Type: task

Status: resolved

Claimed by: lu-order

Blocked by: none

When admission's ring solve stops, the refusal names the wrong row. On the whole body as it was before part 4 of [body 06](../../body/issues/06-whole-body-on-holds.md), `admit_at` returned `Inconsistent { row: Body(1) }`, the norepinephrine in the arteriolar junction's vesicle (`~/lanes/logs/ng-wave-4/body-06-p4/02-smoke.log`). That row was only the first one `recheck` failed on a workspace the solve had left unsolved. An instrumented copy of the solve found the cause: the first Newton update on the whole ring had no pivot at the stomach's lumen water, stage 0, which `InitializationSystem::solve` returned as slot 1189 (`03b-diagnose.log`, line 25). The cause was a gut transit that read no state, so nothing placed each species between the stomach and the small intestine.

Finding it took a copy of the admission solve with prints. A refusal that named the unknown with no pivot, its row and its stage would have shown it at first look, and every later body ticket that adds rows can meet the same kind of free direction.

What changes:

- When the ring's linear solve finds no pivot, admission refuses with a failure that names the unknown's row and stage, not the first row a recheck fails.
- A test builds a small body with two moved pools joined only by a transfer that reads no state, and asserts the refusal names one of those pools.

- [x] A ring solve with no pivot refuses with the unknown's row and stage, and a test shows it on a body built to have a free direction
- [x] A mutant row that returns the recheck's row instead is caught by that test
- [x] The slice's model hash, basis and the pins of `crates/ng-validate/tests/trace.rs` stay

**Non-goals:** pinning a free direction to the packet's guess, which body 06 refused; any change to how the ring solves.

## Answer

- `InitializationFailure::NoRank` in `crates/ng-contract/src/admission.rs` now carries `stage: Option<usize>` alongside `row: RowId` (`Some(0)` end or instant stage, `Some(1)` interior stage, `None` for a fitted parameter whose anchor row is named).
- `crates/ng-contract/src/ring.rs` returns the unknown column `pivot` with no pivot left (rather than the row permutation index after partial pivoting), maps per-leg block failures to `point * size + at`, and maps corner failures to `last * size + at` or `border + at - size`.
- `InitializationSystem::recheck` checks unread fits, unpaced cycles and full-ring rank before blaming the first failed stage, crossing-root or anchor residual on an unsolved workspace, leaving admissible-set, net-boundary-flow and conserved-total refusals intact and adding no extra work to valid workspaces.
- `a_free_direction_on_the_ring_names_the_unknown_and_stage_not_the_recheck_row` in `crates/ng-validate/tests/admission.rs` verifies the refusal names one of the two free pools and the exact stage `InitializationSystem::solve` had no pivot on rather than an unrelated regulated pool whose residual failed first, and `admission-no-pivot-names-recheck-row` in `tools/gate/mutants.d/nc49-no-pivot.toml` pins that check.
