# 98: Posture is owed at the anchor moment

Spec: [solver contract](../../../docs/architecture/solver-contract.md), section 6, [body ticket 05](../../body/issues/05-the-days-inputs.md)'s rule for lasting inputs, and `crates/ng-contract/src/boundary.rs` (the drive frame)

Type: task

Status: open

Blocked by: none

Blocks: none

**Why.** The glossary names posture a lasting input, one a law reads through each site's height. Body ticket 05's rule refuses a body that reads a lasting input with no value at the anchor moment. A body admitted with no posture stated runs with every height at zero, where the rule asks for a refusal. [Body ticket 24](../../body/issues/24-the-synthetic-day.md)'s day changes posture twice, so the body it admits reads heights that matter.

**What the second read of nightingale#215 found, by reading main.** The compiler binds a height read as `Bound::Height` and not as a lasting read, and `InputUnstated` names a `Lasting`, which posture is not. When nothing states posture at the anchor moment, the opening frame's trunk and legs are zero (`DriveFrame::default`), every height reads zero, and admission returns `Ok`. That holds for a law instance's height read and for a guard's: the valves' guards in `crates/ng-cardiovascular/src/valves.rs` take the head across the valve through `drive` in `crates/ng-cardiovascular/src/shared.rs`, which calls `Kernel::fall`. [Native-contract 96](96-an-input-only-a-guard-reads-is-owed-at-the-anchor-moment.md) owns the lasting inputs a guard reads as values.

- [ ] A body whose laws or guards read a height is refused at admission, by name, when nothing states posture at the anchor moment, shown by a test that is red before the change
- [ ] Each admission in the repo of a body that reads a height states its posture, each listed in the PR, and a body that reads no height keeps its bytes
- [ ] The PR says whether a posture stated as the frame's default and one never stated give one model hash and one admitted seal today, and which pins move
- [ ] A mutant row that takes the refusal out, CAUGHT by the test

**Non-goals:** how the body answers a change of posture ([body ticket 36](../../body/issues/36-the-whole-body-answers-posture.md)), and a lasting input read as a value ([native-contract 96](96-an-input-only-a-guard-reads-is-owed-at-the-anchor-moment.md)).
