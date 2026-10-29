# 03: First crate: workspace, gate tiers and the code checks

Type: task
Status: ready-for-agent
Blocked by: none
Lands with: `.scratch/native-contract/issues/01-ng-purity-rules.md`

An acceptance checklist against the retained interface (the sketch's public surface and its signature walk):

- [ ] **Workspace.** `members = ["crates/*"]`, the sketch excluded, seams per the contract's crate table.
- [ ] **Tiers.** No-argument `tools/verify` stays the pre-merge gate. Add `--changed` and `--full`, affected crates from `cargo metadata`, `tools/gate/paths.toml` registration with a self-check, and the pre-push hook.
- [ ] **Checks.** `01-toolchain`; `10-rust` with `rustfmt.toml` and `clippy.toml` taken from nightingale-pulse at the import commit, pulse-only comments cut; the seam check (no provider SDK, oracle, I/O, threads, clock, unsafe or host libm in the physiology closure); the `f64` scan with its allow-list; the crate-name rule.
- [ ] **Nightly.** The developer reviewer adds the `jobs.d` job; its state lives at a fixed path, not in the policy env.
- [ ] **Docs-only paths.** Decide whether root `*.md` and `.scratch/` count as docs-only for the gate (old plan item 11). Today only `docs/` does.
- [ ] **Pin rule.** Carry or drop the pin rule from nightingale-pulse#173.

Background, not authority: the private brief `ng-tiered-gate.md` (its pin step, `--tier` flag, ADR-75 and `cargo check -D warnings` are obsolete).
