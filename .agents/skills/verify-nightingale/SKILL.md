---
name: verify-nightingale
description: >-
  Run tools/verify, Nightingale's merge gate, and record a proof block before calling a change
  done or asking for a merge. Use for a local verify, for reading a green or red log, and for the
  docs-only proof a docs PR merges on.
---

# Verify Nightingale

An agent loads this skill before it claims a change works or asks for a merge.

## Three runs, one proof

`tools/gate/gate.py` plans each run from the changed paths and `tools/gate/paths.toml`. The runner then runs the steps the plan selects from `tools/verify.d/`, in order.

- `tools/verify` is the merge run and the only run that is proof. It diffs the working tree against `git merge-base HEAD origin/main` without fetching, and covers the crates the change touches, every crate that depends on them and each registered smoke test.
- `tools/verify --changed` is the changed run that the pre-push hook takes. It covers only the crates the change touches, skips the probes that compile, and ends with `verify: OK for changed paths only`, which is not proof.
- `tools/verify --full` is the full run that the nightly takes. It runs every step over every crate, ignored tests included.

`--base REV` sets the base of a merge or changed run.

## The steps

- `01-toolchain.sh`: rustc and cargo match the channel in `rust-toolchain.toml`, rustfmt and clippy answer, and python3 is 3.11 or newer.
- `05-seam.sh`: the workspace against the contract's crate table (names, allowed dependencies and the physiology closure), the rules that keep each crate's build and tests inside its own directory, a must-fail probe for every rule, and the plan and scan cases.
- `10-docs.sh`: the docs hygiene rules in `docs/governance.md`.
- `10-rust.sh`: `cargo fmt`, clippy with warnings denied and the tests over the planned crates, then the bans probe, which requires clippy to reject a use of every `clippy.toml` entry.
- `15-f64.sh`: no bare `f64` or `f32` in the public API of a planned crate unless its `paths.toml` row allows the item with a reason, then a probe crate the scan must flag exactly.
- `20-sketch.sh`: the solver-contract sketch built with warnings denied, every doctest, the error code of each compile-fail doctest, and the coupling census.

## Read the gate lines

Every log opens with the plan:

    gate: mode=merge scope=affected base=<sha> reason=crates/ng-record/src/lib.rs
    gate: crates=ng-observe ng-record ng-validate smoke=none
    gate: steps=01-toolchain 05-seam 10-rust 15-f64

The scope is `docs-only` when every changed path is a regular file under `docs/`, a root Markdown file or a Markdown file under `.scratch/`. It is `everything` when there is no base, nothing changed, or a path runs everything, such as a toolchain, Cargo, gate or skill file or a path no rule names. Otherwise it is `affected`. The `reason` field says what decided the scope. Each step prints `== <step>`, or `== <step> (not selected)` when the plan skips it, and a failing step ends the log with `verify: <step> failed`.

## How to run

    .agents/skills/verify-nightingale/ensure-prereqs.sh   # once per clone: the toolchain and the pre-push hook
    .agents/skills/verify-nightingale/prove.sh [log]      # from a clean checkout of the PR head

`ensure-prereqs.sh` sets `core.hooksPath` to the relative path `tools/gate/hooks`, so each worktree runs its own checkout's hook. It leaves an existing `core.hooksPath` or an active hook in `.git/hooks` alone and says so. `git config --unset core.hooksPath` turns the hook off, and `git push --no-verify` skips it for one push.

`prove.sh` runs `tools/verify`, hashes the whole log and prints a proof block. Paste the block into the PR. Its `commit` line must be the PR head.

## Read the proof block

`full_proof: yes` needs `verify_exit: 0`, the final line `verify: OK`, a `gate: mode=merge` plan line, `worktree: clean` and `head_unchanged: yes`. The `gate_scope` and `gate_base` lines copy the plan line, and `gate_scope: docs-only` marks the docs-only proof that `AGENTS.md` names. Exit 2 means `tools/verify` passed but the run is not proof; the `reason` line says why.

## Fail closed

If a prerequisite is missing, a step exits non-zero, the log cannot be hashed, or the commit differs from the PR head, the claim is not proven. Do not claim green, hide the failing step or rerun only that step and cite it. A later `prove.sh` run on the same head can replace the proof; a push voids it.
