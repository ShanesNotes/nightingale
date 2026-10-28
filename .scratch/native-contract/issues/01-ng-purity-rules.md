# 01: Admit the ng-* crates under purity rules

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** Four empty crates (`ng-contract`, `ng-record`, `ng-observe`, `ng-validate`) join the workspace, and the repository's boundary and workspace checks know them. Each may depend only on what the contract's module table allows. Nothing in the physiology closure may pull in an oracle, pulse-io, prost, a provider SDK, I/O, threads, a clock, unsafe or host libm.

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

- [ ] `tools/boundaries.json` and `tools/check_workspace.py` list the four crates with their allowed dependencies
- [ ] A probe crate that adds a forbidden dependency fails the check, and removing it passes
- [ ] `tools/verify` exits 0 with the four crates present
