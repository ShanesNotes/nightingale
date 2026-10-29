# 01: Admit the ng-* crates under purity rules

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** Four empty crates (`ng-contract`, `ng-record`, `ng-observe`, `ng-validate`) join the workspace, and this repository's seam check knows them. Each may depend only on what the contract's module table allows. Nothing in the physiology closure may pull in an oracle, pulse-io, prost, a provider SDK, I/O, threads, a clock, unsafe or host libm.

**Blocked by:** None (can start immediately)

**Lands with:** `.scratch/foundation/issues/03-first-crate.md`

**Status:** resolved

**Claimed by:** claude

- [x] The seam check lists the four crates with their allowed dependencies
- [x] A probe crate that adds a forbidden dependency fails the check, and removing it passes
- [x] `tools/verify` exits 0 with the four crates present

## Answer

Landed with [First crate](../../foundation/issues/03-first-crate.md). The crate table in `docs/architecture/solver-contract.md` is the seam check's data: each row names a crate's allowed dependencies, and a new column marks the physiology closure, which `GLOSSARY.md` defines. `tools/verify.d/05-seam.sh` holds it on every merge run:

- every internal edge, of every kind, is one the table allows;
- a closure crate takes no crate from outside the workspace, so no Pulse, pulse-io, prost or provider SDK under any name, and a crate outside the closure takes only the registry dev-dependencies its `paths.toml` row admits, at an exact version;
- no crate's source or doctests name a host service (`std`, the environment macros, thread-locals, atomics), so I/O, threads and the clock are unreachable rather than listed, and `10-rust.sh` checks against rustdoc's own list that the scan read every doctest;
- `unsafe_code` and the host-libm bans are forbid-level in every crate.

Each rule carries the mutation it must catch. The ticket's probe adds a stub `prost` from outside the workspace to `ng-record` and must fail `seam.external`, and the tree without it passes. A real probe crate in a scratch clone failed `tools/verify` at `05-seam` with `seam.external`, and the tree passed once it was removed.
