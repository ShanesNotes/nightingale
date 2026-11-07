# 01: Run the arena, then build the stage 1 milestone

Type: task
Status: ready-for-agent
Blocked by: `.scratch/native-contract/issues/24-arena-ready-to-dispatch.md`

The path after the arena harness:

1. Add the certificate check to `tools/verify`, proven on both solver cores before either is scored.
2. Run solver-A against solver-B on the slice in the contract's section 7, for 48 simulated hours with a synthetic fixture. Pick one core and do not graft the two. Vet the speed numbers with pstack `benchmark-checklist`.
3. Explain the measured result plainly to the developer reviewer, who confirms the pick (SRS rule 8).
4. Build the stage 1 milestone: all 13 systems at rung 0, through `/orchestrate`.

Rules for the arena seats:

- Seats read nightingale-pulse only at the import commit, never as a build edge: `mkdir -p reference/pulse && git -C <pulse clone> archive bdc01c567f6cadacbd96728c0600a2e052882a59 docs/council/2026-10-26-solver-pass-1 docs/council/2026-10-26-contract-pass-1 | tar -x -C reference/pulse` (`reference/` is gitignored).
- The cores go in `crates/ng-solver-a` and `crates/ng-solver-b`.
- Anchor attestation is a citation check: each anchor cites clinically reviewed literature with a stated tier, plus its measurement method and averaging window (SRS rules 16 and 17).
- The milestone build reads [pulse 0006](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0006-first-milestone-stable-healthy-body.md)'s defaults and section 1e choices by permalink, once every anchor passes that check. They enter the repo only through a reviewed packet.

Source: the split plan's map ticket, from the consolidated vision's section B, steps 3 to 6.

## Comments

From native-contract 24: a seat calls `Arena::new(ArenaProtocol::full())?.score(cores, second_policy, clock, store)` and gets one `ArenaScore` whose run evidence (throughput and numerical health), certificate, accuracy, replay and day checks are each their own result. The seat contract is `~/lanes/briefs/ng-arena-seat-contract.md`. Three things stand before step 2. The crate table names only `ng-solver`, and 05-seam refuses any other crate or dependency, so `ng-solver-a` and `ng-solver-b` need their rows, and `ng-validate` may take both as dev-dependencies. The certificate fails every core on energy drift past a few seconds: ticket 14's synthetic 1e4 a unit against about 800 a second (124,246 over 155.5 s, measured), so 24 h windows need a synthetic energy limit of their own. And `full()` writes about 1,366 GB of link log on today's bench (683 GB a repeat) and takes about 40.5 h on the tower by arithmetic, where the tower had 322 GB free. nightingale#38 and nightingale#39 do not block: score every seat on one pinned commit, and rescore all of them if either lands mid-arena.
