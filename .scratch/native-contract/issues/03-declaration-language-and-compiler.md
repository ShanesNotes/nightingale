# 03: Declaration language and compiler for a two-reservoir body

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** An organ author writes whole-body and site-class kernels with instruction set v1, transfers and site transfers, memory rates, set points and modes, and `Assembly::compile` turns them into one hashed body with incidence, the conserved basis, sparsity and blocks.

Blocked by: 02

Status: ready-for-agent

- [ ] A two-reservoir manufactured body compiles; its incidence and conserved basis match the hand-derived ones
- [ ] A site-class Starling law over the two beds compiles to one program applied at both sites, with per-site parameters
- [ ] Every compile-fail doctest in the sketch has a compile-fail test here with the same error code, and each legal twin compiles
- [ ] Compilation refuses an omitted law, a second owner of a row, and a mode without law coverage, each naming the law

## Comments

From 02: `Schema::pools` lists one water pool per space of every site, in key order. Whether a pool no law touches is refused here, as the contract's orphan rule reads, or never becomes an amount row, as the arena's Codex runner and judge proposed, is this ticket's call; 02's answer gives both sides. Under the first reading, a two-bed Starling bench must also move or hold each bed's cell water. Compile-fail error codes are checked by the gate only for the sketch, and this ticket's third box needs the same check for `crates/ng-contract`.
