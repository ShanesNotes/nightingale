# AGENTS: working in the Nightingale repo

The team contract is `~/memory/AGENTS.md`. This file adds what is specific here. Every session loads it, so it stays an index of rules and pointers.

## What this repo is

The native Rust engine for one simulated patient. Read `docs/srs.md` first: it binds, and every lower doc yields to it. `docs/adr/` holds the decision records, one series from 0001. `docs/architecture/solver-contract.md` is the design the crates build against, `docs/atlas/` maps the 13 systems, `GLOSSARY.md` defines the words, and `docs/governance.md` says what may enter the repo. Pulse-era work lives in nightingale-pulse: read it only at the commit [0001](docs/adr/0001-native-engine-repo.md) names, cite its records as pulse 000N with a permalink, and never depend on it to build or run.

## Rules

- **Load justification.** Every artifact must justify why an agent loads it, and a file no agent needs is removed. The caps on this file (60 lines) and the README (40) apply that rule.
- **Speak the glossary.** Use `GLOSSARY.md`'s words. Its _Avoid_ lines name the words not to use. A new or renamed term lands in the glossary in the same PR. State, chart entries and observations are different kinds of fact and never merge (SRS rule 1).
- **Nonclinical first.** Test values are obviously synthetic. Clinical values enter only in a reviewed packet or healthy-range set, and cited, clinically reviewed literature with a stated tier attests each one. The one exception is a labeled inference record that names the search that found no source: it may fill a parameter or a number of the patient's day, adjust a sourced healthy range or within-subject variation by a sourced shift, or set the within-subject variation of an amount no method can measure from its sourced flows, and it never sets an anchor (records 0003 and 0004). People review the software, in the role of developer reviewer, and never attest clinical content (SRS rules 12 and 16).
- **Human gates are exact-scope.** A release is a `ready-for-human` ticket, routed around, never faked. Anchor attestation and source admission are citation checks, not human gates.
- **Comments.** Only a non-obvious why or a cited physiological source or anchor. Run pstack `/no-comments` on code PRs.
- **Types.** Physical quantities are dimensioned types, never a bare `f64` across a crate boundary. Systems never own amounts, and illegal states are unrepresentable. Each type rule has a compile-fail test.
- **Behavior.** Only checks drawn from anchors and requirements judge physiology, never a previous run's output. An expect-test trace on a synthetic fixture pins each physiology slice to show change, not correctness: it fails until the PR updates it and explains the change.
- **Held by checks, not prose.** Each rule in this table has a check that fails when the rule breaks.

| Rule | Held by |
|---|---|
| Docs hygiene and the line caps | `tools/verify.d/10-docs.sh` |
| The sketch and its compile-fail doctests | `tools/verify.d/20-sketch.sh` |
| The pinned toolchain | `tools/verify.d/01-toolchain.sh` |
| Crate names and allowed dependencies come from the contract's crate table. No crate from outside the workspace enters the physiology closure, no crate names a host service, and no crate has a build script, an include, a feature or its own clippy config | `tools/verify.d/05-seam.sh` |
| No unsafe code, host libm, hash order, wall clock or sockets, through the workspace lints and `clippy.toml` | `tools/verify.d/10-rust.sh`, whose bans probe proves each entry |
| No bare `f64` across a crate boundary | `tools/verify.d/15-f64.sh` |
| Comments | the `/no-comments` report |

## Verify and merge

Run the `verify-nightingale` skill on the PR's current head. Its proof block must show `full_proof: yes`. A docs-only PR is one whose proof block shows `gate_scope: docs-only`, because every changed path is under `docs/`, is a root Markdown file or is a Markdown file under `.scratch/`. It merges on that proof plus `MERGEABLE`, and reviews on it are informational. A code PR also needs a `/no-comments` report and no open review thread; a code-heavy one also needs a local `codex review` on Omarchy with its accepted findings fixed. Codex is never requested through GitHub. The skill's `ensure-prereqs.sh` turns on a pre-push hook that runs `tools/verify --changed`, and that run is never proof. A push voids the proof. Machine-state reruns `tools/verify` on the candidate merge. The developer reviewer reviews any PR as they see fit.

## Tracker and handoff

Tickets are local Markdown in `.scratch/` (`docs/agents/issue-tracker.md`). Write `nightingale-pulse#N` or `nightingale#N`, never a bare `#N`. A handoff goes in the PR and in `vault-log`: paths changed, commands run with results, synthetic or clinical status, evidence hashes and the next step.

## Agent skills

Issue tracker, triage labels and domain docs: `docs/agents/`. The verify skill: `.agents/skills/verify-nightingale/`.
