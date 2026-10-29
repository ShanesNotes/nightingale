# 03: First crate: workspace, gate tiers and the code checks

Type: task
Status: resolved
Claimed by: claude
Blocked by: none
Lands with: `.scratch/native-contract/issues/01-ng-purity-rules.md`

An acceptance checklist against the retained interface (the sketch's public surface and its signature walk):

- [x] **Workspace.** `members = ["crates/*"]`, the sketch excluded, seams per the contract's crate table.
- [x] **Tiers.** No-argument `tools/verify` stays the pre-merge gate. Add `--changed` and `--full`, affected crates from `cargo metadata`, `tools/gate/paths.toml` registration with a self-check, and the pre-push hook.
- [x] **Checks.** `01-toolchain`; `10-rust` with `rustfmt.toml` and `clippy.toml` taken from nightingale-pulse at the import commit, pulse-only comments cut; the seam check (no provider SDK, oracle, I/O, threads, clock, unsafe or host libm in the physiology closure); the `f64` scan with its allow-list; the crate-name rule.
- [x] **Nightly.** The developer reviewer adds the `jobs.d` job; its state lives at a fixed path, not in the policy env.
- [x] **Docs-only paths.** Decide whether root `*.md` and `.scratch/` count as docs-only for the gate (old plan item 11). Today only `docs/` does.
- [x] **Pin rule.** Carry or drop the pin rule from nightingale-pulse#173.

Background, not authority: the private brief `ng-tiered-gate.md` (its pin step, `--tier` flag, ADR-75 and `cargo check -D warnings` are obsolete).

## Answer

Landed with [Admit the ng-* crates under purity rules](../../native-contract/issues/01-ng-purity-rules.md) in one PR. pstack `architect` shaped it: a Claude runner and a Codex runner, a Codex judge, and a synthesis that kept the first runner's planner and rule registry and the second's capability boundary.

- **Workspace.** `members = ["crates/*"]` with the sketch excluded, and four crates with exactly the edges the contract's crate table allows. The table is the gate's only list of crate names, edges and closure membership.
- **Runs.** Tier is the glossary's word for evidence strength, so the three runs are named merge, changed and full. No-argument `tools/verify` stays the pre-merge gate. It covers the paths changed since the merge base with `origin/main`, their crates and every crate that depends on them, read from `cargo metadata`. It covers everything when there is no base, when a toolchain, Cargo, gate or skill file changed, or when a path matches no rule. `--changed` covers only the changed crates; the pre-push hook runs it, and its last line is one `prove.sh` refuses. `--full` covers everything, ignored tests included. `tools/gate/paths.toml` registers steps and crates, and every run first checks its rows against the members and its step names against the files.
- **Checks.** `01-toolchain`. `05-seam`, whose 14 rules each carry the mutation it must catch, all applied on every merge run. `10-rust`: fmt, clippy with warnings denied, the tests, a check that every doctest rustdoc lists is one the host scan read, and a bans probe built from `clippy.toml` that also requires every stable `f64` and `f32` method in the toolchain's rust-src to be banned or named exact; pulse's list missed `sin_cos` at the import commit and the f32 twins before that, and this check reports both. `15-f64` reads the public API from rustdoc JSON, with an allow-list per `paths.toml` row. `rustfmt.toml` is pulse's at the import commit. `clippy.toml` carries pulse's bans plus `sin_cos`, and `min` and `max`, which may return either zero for +0.0 and -0.0; each entry states its reason, which clippy prints with the error. The crate-name rule: `05-seam` refuses a member the table does not name, and `docs/governance.md` points at the table.
- **Nightly.** The repo side is done: `tools/gate/nightly.sh` runs `--full` on a clean `origin/main` checkout and records each result at the fixed path `~/.local/state/nightingale/nightly.json`, and a changed run warns while it is red. The job is the developer reviewer's: [Add the nightly job](10-nightly-job.md).
- **Docs-only paths.** Yes: a regular, non-executable root Markdown file or Markdown file under `.scratch/` counts as docs-only, for the gate and for the merge rule. Under the old rule 2 of this branch's 9 commits were docs-only, and under the new one 7; the other 2 change gate code either way. Overwriting every root and ticket Markdown file with random bytes left every non-docs step's output unchanged. It stays sound because no crate can read a file (`seam.host`, `seam.include`, `seam.build`), and `registration.docs-sound` refuses a docs glob that covers code.
- **Pin rule.** Dropped as a check of its own. nightingale-pulse#173 was an open, unmerged pulse PR that guarded pin shape. Here the only pin outside a lockfile is `rust-toolchain.toml`, and `01-toolchain`'s exact match already fails `stable`, `1.98` and a `RUSTUP_TOOLCHAIN` override (measured). `Cargo.lock` under `--locked` pins crates, and an admitted dev-dependency names an exact `=x.y.z`. The rule returns with the first pin that has no lockfile.
