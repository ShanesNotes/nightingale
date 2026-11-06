# 12: The arena run reports throughput beside the record

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** The validation crate's `run` takes an `ArenaSpec`, which holds the slice's fixture packet, a boundary timeline and a seed. It runs the slice with the reference core for a short window, as the sketch's `run` does (`docs/architecture/solver-contract-sketch/src/ng_validate.rs:408`). It returns the record with `ThroughputEvidence` beside it: the wall time read from a `WallClock` that a host outside the workspace passes in, and the waveform samples the run produced. Compilation and admission are timed apart from the run, and every repeated run is kept. No throughput number enters the certificate.

Blocked by: 11

Status: ready-for-agent

- [ ] A short slice window's `run` returns throughput beside the record, timed by a clock that a host outside the workspace passes in, with compilation and admission timed apart

## Comments

From the engine bench (nightingale#31): the speed input. The bar is at least 100 times real time with full waveforms (`docs/srs.md:184`), at most 1,728 wall seconds for 48 h (contract §7). `tools/bench/run.py` gives the first baseline: 1.309x real time at 5 ms steps, a median of 5 fresh processes ranging from 1.300 to 1.316. That run took 3.31 ms per accepted step, 143.7 µs per Newton update and 16.73 s per admission on 240 rows, with no observer, over 5 simulated seconds. Its gap to the bar is 76x. One earlier pass on the same 1154 accepted steps read 1.14x, an 88x gap, so the difference between the two is the machine's state. Both baselines stand. At 100x a 5 ms step has 50 µs, and one Newton update costs about 140 µs. Linearizing takes 38.5 percent of wall time, and fat LTO gave 1.33x, so compiler settings are not the limit. This ticket reports throughput and tunes nothing. The reference core is not the arena's bar (ticket 10).

From 04: a whole step cost about 40 µs on the two-row bench. `chain` rehashes the whole image every step, so that cost grows with the state. Ticket 10's design put the hash at about 11 KB per step on the slice, and nobody has measured it.

From 05: `WallClock` has no implementation in the workspace, by rule, so a host crate outside it passes one in. Ticket 05's evidence ran one of about 30 lines. `tools/bench` already sits outside the workspace.

From 06: measure a landing's cost on the slice. On two vessels a landing cost 2.8 to 6.5 extra solves, each a whole `solve`, `verify` and ledger chunk, and bisection cost 10 to 30.

From 07: an observer keeps no log, so a waveform count comes from what `advance` returns.

From 08: `run` and `replay_gates` take no initializer, because one solve in `ng_validate` serves every core. Time admission apart from the run, as §7 asks, because it is a dense solve.

From 09: coupling fields add unknowns. On the slice's map a dense core took about 290 µs a step against 107 µs without the field rows, and a block core is not measured. `PendingInputs::after` clones the delivery list every step, and `BoundaryPort::new` finds a repeated id by a linear scan: harmless at bench size, and worth changing for a two-day timeline.

From the split of ticket 11: foundation 09 asks where the slowest steps' wall time in arena results goes, and §7's speed gate reports none. §7 times the bar "on the named machine", and no machine is named yet.

From the solver arena (in this PR): over 06:55 to 07:30, at a 10 ms cap and scales (0.1, 1e-3), the picked core ran 4.91 times real time and the other candidate 3.81, one run each on this machine under the shared lock. The reference ran about 1.05 times on the same scales to 07:00:30. Both candidates measured the record's own work at about 345 to 430 µs per accepted step, growing with the history, which holds any core near 11 to 15 times real time here at these scales. `python3 -I tools/bench/run.py ng-solver` times the pick.

From ticket 11: admission now takes a `HistoryStore` (`admit_at(.., origin, store)`), held by the session. A `run` over a long window needs a store whose links leave memory, passed in by the host outside the workspace as the clock is; `tools/bench/src/link_log.rs` is one. `MemoryStore` holds every link, with a sorted insert, so it suits tests and short windows only.
