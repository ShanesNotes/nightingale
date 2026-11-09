# Nightingale

Nightingale is a Rust simulation of one patient's body, for teaching. It does not inform the care of a real person. Pressures, flows, lab values and other readings are meant to be computed from conserved amounts (water, salts, gases, heat) and from physics, so illness, chance and treatment would later act on the same body. Numbers in the crates are synthetic. Cited clinical records live in `data/`, and no crate reads them yet.

[docs/statecharts](docs/statecharts) holds five statecharts, each a TLA+ model that TLC checks: the system, how its modules hand off from the map to admission, the record crate, the boundary port, observers and the arena; the engine, each step's accept, refuse and fault with the history store, checkpoints and the certificate; one step of a run; the chart, which is a placeholder; and the bedside monitor's link to the chart service in the clinical workspace prototype. Everything the first four draw but the chart runs today on synthetic bodies, the admission solve of a start state included.

## Why it matters

A monitor and a chart can both describe one patient and still disagree. The design keeps those facts apart. Body state is computed. A chart entry is what a clinician documented. An observation is a reading taken from state, and it never writes back. No code is allowed to assign a vital sign. The same patient and the same inputs are meant to give the same result after a pause or a replay. The first milestone, in [record 0004](docs/adr/0004-first-milestone.md), is not yet run. It is a healthy reference adult man through two ordinary days of sleep, meals, posture and room air. It would pass only if amounts balance, readings stay inside healthy ranges for what he is doing, and later mornings meet his own morning anchors.

## How it is built

Rust 1.98.1, edition 2024, one Cargo workspace, `unsafe` code forbidden. Laws are data. One time step is meant to run in this order:

1. `ng-contract` compiles anatomy and laws into one body.
2. A solver proposes the next state and sees only that body. `ng-solver` holds the core the solver arena picked first, and `ng-validate` holds the reference core it is measured against.
3. `ng-record` accepts or refuses the whole step and keeps the ledger.
4. `ng-observe` would read the accepted step and would not write.
5. `ng-validate` checks conservation and accuracy, and would compare solvers. It sits outside that path.

The ten crates under `crates/` compile. `ng-contract` holds the anatomy, the language laws are declared in, the compiler and the solver interface. `ng-cardiovascular`, `ng-fluids`, `ng-renal`, `ng-nervous` and `ng-receptor` declare the laws of heart and vessels, fluids, kidney, nerves and receptors. `ng-solver` holds the solver core. `ng-record` holds the accepted state, the step check and the ledger, and `ng-observe` reads accepted steps through a lens. `ng-validate` holds the synthetic bodies and the slice, a backward Euler reference core, the certificate, the accuracy gate and the replay gates. The type sketch the crates grew from is [solver-contract-sketch](docs/architecture/solver-contract-sketch), and its function bodies are `todo!()`. Read [the requirements](docs/srs.md), [decision records](docs/adr), [the 13-system atlas](docs/atlas), [the glossary](GLOSSARY.md) and [the solver contract](docs/architecture/solver-contract.md). nightingale-pulse is a private reference implementation. This tree does not build or run against it.

## Build

Python 3.11 or newer, and the toolchain pinned in `rust-toolchain.toml`. From a clone:

```sh
.agents/skills/verify-nightingale/ensure-prereqs.sh
cargo check --workspace --locked
cargo test --workspace --locked
cargo check --locked --manifest-path docs/architecture/solver-contract-sketch/Cargo.toml
cargo test --doc --locked --manifest-path docs/architecture/solver-contract-sketch/Cargo.toml
tools/verify
```

There is no program to run. Workspace tests run synthetic bodies through the compiler, the step check and the gates. The sketch's doctests check the API shape, including forms that must fail to compile. The first command installs the pinned toolchain and, when git has no hook path yet, turns on this repo's pre-push check. `tools/verify` is the merge check.

## What is in place

Requirements, decision records, the atlas and the solver-contract sketch are written, and the sketch type-checks. The workspace runs a manufactured two-reservoir body, and it does not simulate a patient. Disease, treatment and learner scenarios are specified and not implemented. `clinical-workspace/` holds a prototype bedside monitor and chart that replay one recorded synthetic run, and they never run the engine.
