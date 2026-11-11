# 01: Run the arena, then build the stage 1 milestone

Type: task
Status: ready-for-agent
Blocked by: `.scratch/native-contract/issues/24-arena-ready-to-dispatch.md`, `.scratch/native-contract/issues/28-admission-energy-at-balance.md`

The path after the arena harness:

1. Add the certificate check to `tools/verify`, proven on the working core before it is scored.
2. Score the working core, `ng_solver::Core::new`, alone with `ArenaProtocol::pick()` on the slice in the contract's section 7, with a synthetic fixture. Once it passes, it runs the milestone protocol once. Vet the speed numbers with pstack `benchmark-checklist`. This departs from the earlier plan, which ran solver-A against solver-B: no rival core is built now (see the solver decision in the Comments).
3. Explain the measured result plainly to the developer reviewer, who confirms the pick (SRS rule 8).
4. Build the stage 1 milestone: all 13 systems at rung 0, through `/orchestrate`.

Rules for the arena seats:

- Seats read nightingale-pulse only at the import commit, never as a build edge: `mkdir -p reference/pulse && git -C <pulse clone> archive bdc01c567f6cadacbd96728c0600a2e052882a59 docs/council/2026-10-26-solver-pass-1 docs/council/2026-10-26-contract-pass-1 | tar -x -C reference/pulse` (`reference/` is gitignored).
- No rival core is built now, so `crates/ng-solver-a` and `crates/ng-solver-b` stay unbuilt.
- Anchor attestation is a citation check: each anchor cites clinically reviewed literature with a stated tier, plus its measurement method and averaging window (SRS rules 16 and 17).
- The milestone build reads [pulse 0006](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0006-first-milestone-stable-healthy-body.md)'s defaults and section 1e choices by permalink, once every anchor passes that check. They enter the repo only through a reviewed packet.

Source: the split plan's map ticket, from the consolidated vision's section B, steps 3 to 6.

## Comments

From native-contract 24: a seat calls `Arena::new(ArenaProtocol::full())?.score(cores, second_policy, clock, store)` and gets one `ArenaScore` whose run evidence (throughput and numerical health), certificate, accuracy, replay and day checks are each their own result. The seat contract is `~/lanes/briefs/ng-arena-seat-contract.md`. Three things stand before step 2. The crate table names only `ng-solver`, and 05-seam refuses any other crate or dependency, so `ng-solver-a` and `ng-solver-b` need their rows, and `ng-validate` may take both as dev-dependencies. The certificate fails every core on energy drift past a few seconds: ticket 14's synthetic 1e4 a unit against about 800 a second (124,246 over 155.5 s, measured), so 24 h windows need a synthetic energy limit of their own. And `full()` writes about 1,366 GB of link log on today's bench (683 GB a repeat) and takes about 40.5 h on the tower by arithmetic, where the tower had 322 GB free. nightingale#38 and nightingale#39 do not block: score every seat on one pinned commit, and rescore all of them if either lands mid-arena.

Where this stands after native-contract 24 to 27: the first pick landed as `crates/ng-solver` in nightingale#54, before the arena harness existed, so `crates/ng-solver-a` and `crates/ng-solver-b` do not exist. A candidate core is a new crate with a row in the contract's crate table and in `tools/gate/paths.toml`. `ArenaProtocol::full()` is now `milestone()`, and `pick()` scores a core on one 24 h window (native-contract 26). Step 2 waits on native-contract 28 (nightingale#91), on the drift check that nightingale#92 files as ticket 31, and on the synthetic energy drift limit. Step 4 also needs the reviewed packet (research 03 to 10) and the contract follow-ups (foundation 07), and no ticket scopes the nine systems that have no crate. The map's "Where the route stands" keeps this current.

The solver decision, made by the oversight seat after the developer reviewer delegated it. `ng-solver`'s `Core::new` is the working core for the milestone build, and it is scored alone: no rival core is built now, so `ng-solver-a` and `ng-solver-b` stay unbuilt. The pick becomes firm once that core passes `ArenaProtocol::pick()` on the slice, which waits on native-contract 28's energy drift limit and native-contract 31's check. It then runs the milestone protocol once. If it fails, the pick reopens as the milestone protocol says, and each part it fails becomes a ticket on this core rather than a reason to build a rival. Rival cores are considered again when [body 30](../../body/issues/30-step-plans-on-the-whole-body.md) has measured step plans on the whole body. The reasons: a four-system slice says little about thirteen systems, the contract's seam makes a later swap cheap, and [body 27](../../body/issues/27-step-plan.md) already contested four step plans for this core ([record 0009](../../../docs/adr/0009-a-step-reads-its-error-from-its-two-half-stages.md)). SRS rule 8 still holds: the developer reviewer confirms the pick after a plain explanation of the result (step 3). This replaces step 2's earlier plan of two cores.
