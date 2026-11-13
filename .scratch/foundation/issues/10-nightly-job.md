# 10: Add the nightly job

Type: task
Status: resolved
Claimed by: f10-nightly
Blocked by: none

[First crate](03-first-crate.md) built the repo side of the nightly full run. Its schedule is machine-state's, and a `policy/jobs.d` declaration with its predicate is the developer reviewer's to add.

- **Repo side (done).** `tools/gate/nightly.sh <log>` refuses a dirty tree or a HEAD other than `origin/main` (exit 2) and skips a commit already recorded green (exit 0). Otherwise it runs `tools/verify --full` and records the result through `tools/gate/gate.py nightly-record` at `~/.local/state/nightingale/nightly.json`, exiting 0 when green and with verify's status when red. `tools/verify --changed` warns while the last nightly failed.
- **Declaration.** `policy/jobs.d/nightingale-nightly.json` with its `agent-nightingale-nightly` timer and service: once a night, `timeout_seconds` 7200, `llm: false`, and the verify policy's env: the host's rustup, cargo and mise caches, and a `CARGO_TARGET_DIR` only this job uses.
- **Command.** A dedicated clone, never a live worktree, with `origin/main` checked out detached and clean, then `tools/gate/nightly.sh <log>`. WP11 runs no fetched instructions on a timer, so the clone's origin is a checkout already on this machine, such as `~/nightingale`, unless the reviewer records an exception.
- **Predicate.** A new `predicate_id`: `nightly.json` records `result: passed` for the clone's `origin/main` head. A red run exits non-zero, so `run-job` fails it and writes the alert line.

## Comments

Triage, 2026-10-29 (developer reviewer's call, delegated): ready for the developer reviewer. The repo side is done (ticket 03); the job and its predicate are machine-state's, which only the developer reviewer adds. Without it the full run, which ticket 05's proofs will join, never runs on a schedule.

Resolved by f10-nightly. The developer reviewer approved the job, and it is added in machine-state, on its branch `nightingale-nightly`:

- `policy/jobs.d/nightingale-nightly.json` declares the job: daily at 00:30, `timeout_seconds` 7200, a 36 h deadline, `llm: false`. Its units are `units/user/agent-nightingale-nightly.service` and `.timer`, with `Persistent=true`.
- `bin/nightingale-nightly` is the command. It refreshes the clone `~/.cache/nightingale/nightly-clone`, whose origin is `~/nightingale`, checks out that checkout's `origin/main` detached and clean, sets the verify policy's env with `CARGO_TARGET_DIR` at `~/.cache/nightingale/nightly-target`, and runs `tools/gate/nightly.sh` through the build workers' `job`. The run holds one job slot, in a scope of its own outside the sessions' slice, with a 10 G memory ceiling and a CPU weight below the other jobs.
- `checks/run-job` has the predicate `nightingale-nightly`: `nightly.json` records `result: passed` for the clone's `origin/main` head. `agentctl/wptests/wp11.py` fails it on a failed record, on a record for another head and on a missing record, and passes it on a matching one.
- One real run of the job on main at `28d0d78` passed the whole gate in 3,650 s beside other jobs, with memory at most 7.3 GiB and no process killed. That is under two thirds of 7200 s, so the timeout stands.

The job runs once the units are installed and the timer is enabled.
