# Nightingale

The native Rust engine for one simulated patient: a whole-body physiology computed from conserved amounts, deterministic and bit-reproducible. Values are synthetic until a reviewed packet says otherwise.

## Build and verify

    .agents/skills/verify-nightingale/ensure-prereqs.sh   # the pinned toolchain and the pre-push hook
    tools/verify                                           # the merge run, which prove.sh records

## Where docs live

- `docs/srs.md`: the software requirements specification (SRS), which binds every other doc
- `docs/adr/`: decision records, one series from 0001
- `docs/architecture/solver-contract.md`: the design the crates build against, with its sketch
- `docs/atlas/`: the coupling map of the body's 13 systems
- `GLOSSARY.md`: the domain words
- `docs/governance.md`: what may enter the repo, and how docs and records are written
- `AGENTS.md`: rules for agents working here
