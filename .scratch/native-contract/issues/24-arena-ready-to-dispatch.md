# 24: Arena ready to dispatch

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** An arena seat contract can name the validation crate's entry point and the report it returns, so solver-A and solver-B are built and scored against one known interface. The report keeps the certificate, accuracy, replay, numerical health, throughput and day-check results apart.

Blocked by: 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23

Status: ready-for-agent

- [ ] An arena seat contract names the validation crate's entry point and report schema, and both exist on main

## Comments

From the split of ticket 11: this ticket keeps the last check of the old ticket 11, "Arena harness ready to dispatch". Tickets 11 to 23 hold its other claims, one each, so each lands in a small PR. Foundation 01 waits on this ticket.

From foundation 04: each arena seat carries its own trace, and the pick PR keeps the winner's.

From the split of ticket 11: two open issues may move the slice's bits before dispatch, nightingale#38 (fire admission guards at their exact zero) and nightingale#39 (fluid route request order reaches the model hash).

From the solver arena (in this PR): the first pick, the core in `crates/ng-solver`, was made before this harness existed. A throwaway harness outside the repo scored two candidate cores with main's own checks, a 06:55 to 07:30 slice run and a script for the body-response checks. Rescore that pick here once tickets 21 and 24 land, and record whether it stands. The PR links the evidence.

From ticket 12: `ArenaSpec::repeats` sets how many times the window runs, and the evidence keeps each repeat's wall time. `tools/bench`'s `arena <interval ms> <core> <simulated s> <repeats>` mode prints that evidence under the host clock. Three speed-gate items are still open. No machine is named. §7's checkpoint cadence is not part of the timed run. The slowest steps' wall time that SRS section 5 asks for is not reported (foundation 09).
