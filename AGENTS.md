# AGENTS: working in the Nightingale repo

The team contract is `~/memory/AGENTS.md`. This file adds what is specific here. Every session loads it, so it stays an index of rules and pointers.

## What this repo is

The native Rust engine for one simulated patient. Read `docs/srs.md` first: it binds, and every lower doc yields to it. `docs/adr/` holds the decision records, one series from 0001. `docs/architecture/solver-contract.md` is the design the crates build against, `docs/atlas/` maps the 13 systems, `GLOSSARY.md` defines the words, and `docs/governance.md` says what may enter the repo. Pulse-era work lives in nightingale-pulse: read it only at the commit [0001](docs/adr/0001-native-engine-repo.md) names, cite its records as pulse 000N with a permalink, and never depend on it to build or run.

## Rules

- **Load justification.** Every artifact must justify why an agent loads it, and a file no agent needs is removed. The caps on this file (60 lines) and the README (40) apply that rule.
- **Speak the glossary.** Use `GLOSSARY.md`'s words. Its _Avoid_ lines name the words not to use. A new or renamed term lands in the glossary in the same PR. State, chart entries and observations are different kinds of fact and never merge (SRS rule 1).
- **Nonclinical first.** Test values are obviously synthetic. Clinical values enter only in a reviewed packet, and cited, clinically reviewed literature with a stated tier attests each one. People review the software, in the role of developer reviewer, and never attest clinical content (SRS rules 12 and 16).
- **Human gates are exact-scope.** A release is a `ready-for-human` ticket, routed around, never faked. Anchor attestation and source admission are citation checks, not human gates.
- **Comments.** Only a non-obvious why or a cited physiological source or anchor. Run pstack `/no-comments` on code PRs.
- **Types.** Physical quantities are dimensioned types, never a bare `f64` across a crate boundary. Systems never own amounts, and illegal states are unrepresentable. Each type rule has a compile-fail test.
- **Behavior.** An expect-test trace pins each physiology slice, and the PR explains any changed trace.
- **Held by checks, not prose:** docs hygiene (`tools/verify.d/10-docs.sh`) and the sketch (`tools/verify.d/20-sketch.sh`).

## Verify and merge

Run the `verify-nightingale` skill on the PR's current head. Its proof block must show `full_proof: yes`. A docs-only PR (every changed path is under `docs/`) merges on that proof plus `MERGEABLE`, and reviews on it are informational. A code PR also needs a `/no-comments` report and no open review thread; a code-heavy one also needs a local `codex review` on Omarchy with its accepted findings fixed. Codex is never requested through GitHub. A push voids the proof. Machine-state reruns `tools/verify` on the candidate merge.

## Tracker and handoff

Tickets are local Markdown in `.scratch/` (`docs/agents/issue-tracker.md`). Write `nightingale-pulse#N` or `nightingale#N`, never a bare `#N`. A handoff goes in the PR and in `vault-log`: paths changed, commands run with results, synthetic or clinical status, evidence hashes and the next step.

## Agent skills

Issue tracker, triage labels and domain docs: `docs/agents/`. The verify skill: `.agents/skills/verify-nightingale/`.
