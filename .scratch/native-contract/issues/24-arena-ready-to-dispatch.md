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
