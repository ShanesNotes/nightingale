# 01: Run the arena, then build the stage 1 milestone

Type: task
Status: ready-for-agent
Blocked by: `.scratch/native-contract/issues/11-arena-harness-ready.md`

The path after the arena harness:

1. Add the certificate check to `tools/verify`, proven on both solver cores before either is scored.
2. Run solver-A against solver-B on the slice in the contract's section 7, for 48 simulated hours with a synthetic fixture. Pick one core and do not graft the two. Vet the speed numbers with pstack `benchmark-checklist`.
3. Explain the measured result plainly to the developer reviewer, who confirms the pick (SRS rule 8).
4. Build the stage 1 milestone: all 13 systems at rung 0, through `/orchestrate`.

Rules for the arena seats:

- Seats read nightingale-pulse only at the import commit, never as a build edge: `mkdir -p reference/pulse && git -C <pulse clone> archive bdc01c567f6cadacbd96728c0600a2e052882a59 docs/council/2026-10-26-solver-pass-1 docs/council/2026-10-26-contract-pass-1 | tar -x -C reference/pulse` (`reference/` is gitignored).
- The cores go in `crates/ng-solver-a` and `crates/ng-solver-b`.
- Anchor attestation is a citation check, not a person's sign-off: each anchor cites clinically reviewed literature with a stated tier, plus its measurement method and averaging window (SRS rules 16 and 17).
- The milestone build reads [pulse 0006](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0006-first-milestone-stable-healthy-body.md)'s defaults and section 1e choices by permalink, once every anchor passes that check. They enter the repo only through a reviewed packet.

Source: the split plan's map ticket, from the consolidated vision's section B, steps 3 to 6.
