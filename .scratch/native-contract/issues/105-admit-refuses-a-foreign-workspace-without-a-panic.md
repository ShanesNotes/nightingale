# 105: Admit refuses a foreign workspace without a panic

From: a break-it read of nightingale#226, which made `ng_validate::solve_fixture` public

Spec: `crates/ng-contract/src/admission.rs` (`InitializationSystem::check` and `admit`)

Type: task

Status: claimed

Claimed by: nc105

Blocked by: none

Blocks: none

**Why.** The first milestone (SRS invariant I19) starts every run from an admitted start. `admit` is the only constructor of that start, and its doc promises a refusal for a workspace from another system. A panic there takes the caller down instead.
