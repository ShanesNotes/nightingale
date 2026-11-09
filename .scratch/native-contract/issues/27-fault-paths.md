# 27: Every fault can be exported, and only recover continues a faulted branch

Spec: [solver contract](../../../docs/architecture/solver-contract.md), usage (b) and the **Checkpoint** section

Type: task

Status: resolved

Blocked by: none

The engine statechart (nightingale#77, `docs/statecharts/engine.tla`) found two paths where the code and the contract disagree. Its rules `AFaultCanBeExported` and `OnlyRecoverContinuesAFaultedBranch` break, and `regenerate.sh` asserts that they do, so the fix flips them to hold. A probe outside the repo replayed both on the crates at `48c3b93` (two-reservoir body, explicit core; `~/lanes/logs/ng-statechart/probe/tests/paths.rs`, log `probe.log`, 2 of 2 passed).

**1. A fault after the store refused a link can never be exported.** Usage (b) says a `Session<Faulted>` "can be inspected, exported for diagnosis, or continued only by `recover`". The Checkpoint section says `checkpoint` and `export` refuse while a link the store refused waits. `Session<Faulted>::export` calls `PatientRecord::checkpoint(true)`, which returns `StoreFailed` while `History::oldest_unkept` is set (`crates/ng-record/src/record.rs:171-174`). Only `History::push` hands waiting links to the store again, and a faulted session takes no step, so the wait never ends. Path: a step is accepted and the store refuses its link; the next step faults; `export` returns `StoreFailed`, and still does after the store recovers. With no earlier checkpoint, the run can be neither exported nor recovered.

**2. Resume continues a faulted branch without recover.** The Checkpoint section says "A fault's export carries the latch, so `resume` refuses it and only `recover` continues, on a new branch." The latch is on the session and its export, not on the branch: `Session::resume` takes any checkpoint without the latch and continues on its branch (`crates/ng-record/src/session.rs:121-142`). Path: a checkpoint is taken; the run faults; that earlier checkpoint is resumed in a fresh core; a step is accepted on the same branch, with no fork. With the same inputs this replays the faulted course bit for bit. A late input after the resume gives a second course under the same branch id.

**What to build:** decide each design call below, smallest first, write it into the contract's lines that state the old rule (keep the edit small), and make the code and the statechart agree.

- Gap 1, options: let `Session<Faulted>` hand its waiting links to the store again before it builds the export (the one path that already moves them, called once more); or let a fault export carry its waiting links so a host can keep them; or refuse to accept a step whose link the store refused. The first changes no type and keeps "a checkpoint is complete with that store".
- Gap 2, options: say plainly that resuming a checkpoint taken before a fault is a replay of the same branch, and that a course that departs from it (a late input) must take `rewind` with a new branch id, which the record enforces by refusing an input before the faulted revision on a resumed branch; or carry the latch into the store or the branch so `resume` refuses any checkpoint of a branch that faulted. The first keeps checkpoints stateless and matches what replay promises. Gap 2 adds or moves state, so run pstack `architect` first if the fix goes beyond wording and one refusal (team contract).

- [x] The probe's two paths become tests in `crates/ng-record/tests/` that fail on main, each through the public session API: a fault after a refused link exports once the store keeps the link, and the chosen gap 2 rule holds.
- [x] Usage (b) and the Checkpoint section state the chosen rules, and the sketch still compiles every doctest.
- [x] `docs/statecharts/engine.tla` models the fix, both rules hold, and `regenerate.sh` no longer expects them to break; the engine chart redraws.
- [x] A mutant row in `tools/gate/mutants.toml` for each fix is caught.

**Non-goals:** any other fault path; the step chart; checkpoint format changes beyond what the chosen rule needs.

## Answer

- **Gap 1: `export` hands the store the waiting links first** (the first option). `Session<Faulted>::export` now takes `&mut self` and calls `History::keep_waiting`, the step `History::push` already ran, before it builds the checkpoint, so a fault exports once the store keeps every link and still refuses with `StoreFailed` while one waits. No type changes beyond `&mut`, and a checkpoint stays complete with its store. Test: `a_fault_after_a_refused_link_exports_once_the_store_keeps_it` (`crates/ng-record/tests/session.rs`) fails on main at the second `export`; it also checks that `resume` refuses the export with `Latched` and that `recover` continues from it. Mutant row `export-leaves-links-waiting`.
- **Gap 2: wording, no new state** (the first option). Resuming a checkpoint taken before a fault is a replay of the same branch: a pure core on the same inputs reaches the same fault, so only `recover` goes past it. Usage (b) says so, and the Checkpoint section keeps "only `recover` continues" for the export. Test: `a_checkpoint_taken_before_a_fault_resumes_as_a_replay_to_the_same_fault` resumes the admission checkpoint, takes the same link (same seal), and faults again with the same fault and the same digests. It passes on main, because the rule is the contract wording and the code already keeps it; there is no code fix to mutate, so it has no mutant row. A course that departs from the replay, through a late input after the resume, is a property of every resumed checkpoint, faulted or not, and is outside this ticket.
- **The statechart.** `docs/statecharts/engine.tla`: `Export` hands the store the waiting links (`KeepSome`, shared with `Publish`); `faulted` records the head each branch faulted at, and a step on a branch that faulted is enabled only below that head, which is the replay. `AFaultCanBeExported` is now `ENABLED`: a fault not yet exported can always reach an export. `OnlyRecoverContinuesAFaultedBranch` is renamed `OnlyRecoverGoesPastAFault`. Both hold in the drawn config (229 states) and the larger check (2,865,915 states). With the export's store retry removed, `AFaultCanBeExported` breaks again. `regenerate.sh` expects every engine rule to hold, and the chart gains the arrow "the store keeps the waiting link, exported and latched".

## Comments
