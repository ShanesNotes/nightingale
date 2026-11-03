# 11: A run's memory stays flat as it grows

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** A run's resident memory stops growing with its accepted steps, and the history that a 48 h slice run writes has a measured size. Today `History::push` (`crates/ng-record/src/history.rs:279`) keeps one `Link` per accepted step in memory: a ledger chunk, an image digest and a seal. Nothing drops a link. The validation crate's `MemoryStore` (`crates/ng-validate/src/replay.rs:435`) keeps every link that a checkpoint hands it. The slice's links go to a history store as each step is accepted, and the record keeps in memory only what the next step needs. The certificate and a resume read the rest through the store.

Blocked by: None (can start immediately)

Status: ready-for-agent

- [ ] `tools/bench` reads the same peak memory, within its stated margin, for two slice runs of different lengths, and reports the history's bytes per accepted step

## Comments

From the split of ticket 11: this ticket comes first because memory stops a long run before speed does.

From the engine bench (nightingale#31): peak memory read 26.3 MB at 1154 accepted steps and 101.2 MB at 5046. Each reading is one release run of 5 simulated seconds on the slice, at 5 ms and at 1 ms steps. That is about 19.7 KB per accepted step, or about 375 GiB per simulated day at 5 ms steps by arithmetic. Two readings of peak memory are a weak measurement, so measure again first. Reading the code puts one link at about 17.5 KB on the slice, which fits ticket 04's count of six compensated values per combination and four per column.

From 04 and 05: ticket 04's 19 GB for 48 h at a 2 ms step was for its two-row body. One link per accepted step adds 64 bytes to each chunk, about 5.5 GB more. Fixed-time chunks do not answer it, because grouping moves no bytes and merged chords do not bound the curve, so measure on the slice before choosing a store. A resume reads every link, so the resume gate's cost grows with the number of loads.

From the split of ticket 11: the certificate reads one sealed history (contract §5), and a resume walks the chain link by link, so both keep working through the store.

From the solver arena (in this PR): at the arena's scales (0.1, 1e-3) and a 10 ms cap, 06:55 to 07:30 took about 446,000 accepted steps, and a history link took about 15.6 KB on disk, so about 7 GB for that window. A meal check needs runs to 11:30 with and without the meal, about 4.6 simulated hours each and about 27 GB, so rows 9 and 10 wait on this ticket.
