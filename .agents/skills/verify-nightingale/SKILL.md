---
name: verify-nightingale
description: >-
  Run tools/verify, Nightingale's merge gate, and record a proof block before calling a change
  done or asking for a merge. Use for a local verify, for reading a green or red log, and for the
  docs-only proof a docs PR merges on.
---

# Verify Nightingale

An agent loads this skill before it claims a change works or asks for a merge.

## What tools/verify runs

`tools/verify` takes no arguments and runs every `tools/verify.d/*.sh` in order:

- `10-docs.sh`: the docs hygiene rules in `docs/governance.md`.
- `20-sketch.sh`: the toolchain `rust-toolchain.toml` pins, the solver-contract sketch built with warnings denied, every doctest, the error code of each compile-fail doctest, and the coupling census.

It takes seconds, so it is both the docs-only proof and the full gate until the first crate adds its checks.

## How to run

    .agents/skills/verify-nightingale/ensure-prereqs.sh   # once per machine
    .agents/skills/verify-nightingale/prove.sh [log]      # from a clean checkout of the PR head

`prove.sh` runs `tools/verify`, hashes the whole log and prints a proof block. Paste the block into the PR. Its `commit` line must be the PR head.

## Reading the block

`full_proof: yes` needs all four: `verify_exit: 0`, the final line `verify: OK`, `worktree: clean` and `head_unchanged: yes`. Exit 2 means `tools/verify` passed but the run is not proof (a dirty tree, a moved head or a missing OK line); the `reason` line says which.

## Fail closed

If a prerequisite is missing, a step exits non-zero, the log cannot be hashed, or the commit differs from the PR head, the claim is not proven. Do not claim green, hide the failing step or rerun only that step and cite it. A later full run can replace the proof; a push voids it.
