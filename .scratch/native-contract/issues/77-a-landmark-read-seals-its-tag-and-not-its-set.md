# 77: A landmark read on a set seals its tag and not its set

Spec: `crates/ng-contract/src/law.rs` (`Seal for Where`)

Type: task

Status: open

Blocked by: none

**Why.** A seal names every field it holds: `AGENTS.md` states the rule and `tools/verify.d/05-seam.sh` holds it. `Where::MarkOn(_, landmark)` writes its tag and its landmark and drops its set, so two reads that differ in the set alone would seal the same. [Native-contract 75](75-an-edges-law-reads-a-beds-field.md) had the same fault in the read it added, and fixed it there after a review found it. This arm was on main before it, and the gate does not see a field dropped by `_` in a tuple pattern.

- [ ] Whether two programs can differ in that set alone: built if they can, and the reason if they cannot
- [ ] The arm seals its set, or `tools/gate/paths.toml` registers it as left out with its reason
- [ ] `seam.seal` sees a field dropped by `_` in a tuple pattern, with a probe that replays this arm
- [ ] Each pin that moves has its sentence
