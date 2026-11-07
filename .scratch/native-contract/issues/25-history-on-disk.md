# 25: History on disk fits a full arena run

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec), section 5 and **Checkpoint**

Type: task

Status: claimed

Claimed by: owner-nc25

Scope (Shane, 2026-11-07): build moves 1 and 2's first half (write once, end values only). Measure the delta with zstd on a 30-minute window and report it; it does not ship here. No full 48 h run.

Blocked by: none

**What to build:** `ArenaProtocol::full()` writes about 1.36 TB of link log for `ng_solver::Core` at 10 ms, and the tower has 322 GB free. Cut it below 200 GB with no change to the seal, the record format, the contract or any pinned digest. Do it in two lossless moves inside the host's history store (`tools/bench/src/link_log.rs`), plus one pure layout helper in `ng-record`.

1. **Keep a seal once.** `HistoryStore::keep` says keeping a seal the store already holds changes nothing. `LinkLog` breaks that: it appends every link, so two repeats write two chains. `keep` should skip a seal it holds. Repeats resume one admitted start and hand over the same links in order, so a check against the next expected record is enough. This takes 1,362 GB down to 681 GB.
2. **Write each link's values once.** A chunk stores `[start, end]` for every curve, and each start is bit for bit the end of the link before (`LedgerChunk::after`, `crates/ng-record/src/history.rs:31-80`). The interval's start is the previous end, and the previous seal is the previous record's seal. The store writes the end values only. Then it writes them as an XOR delta against the previous link's ends, in frames of 4,096 links, each frame opening on a full keyframe and compressed with zstd. On `resolve` it rebuilds the exact bytes `keep` was given. `Walk::read` already checks every resolved link against its seal, so a lossy codec fails closed as `HistoryAltered`. Dropping the start values alone takes 681 GB down to 341 GB. The delta with zstd takes it to an estimated 70 to 170 GB, which this ticket measures.

The codec needs the chunk's layout, which is 21 combinations and 257 columns on the slice. `ng-record` gains a pure pair: `compact(previous, link) -> Vec<u8>` and `expand(previous, compact) -> Vec<u8>`. Neither touches `std`. zstd stays in `tools/bench`, which is outside the workspace and the physiology closure. Check `05-seam.sh` before adding the crate; if it refuses, ship the delta with bit packing and no zstd.

- [ ] The delta with zstd is measured on a 30-minute window (06:55 to 07:25): bytes per link and the throughput cost, with the count of held-route columns and the bytes a link of only those would take. Reported, not shipped.
- [ ] On ticket 21's window (06:55 to 07:30, `ng_solver::Core` at 10 ms, about 446,000 links), `tools/bench score` reports `history_bytes` for the old and new log. It ends on the same seal and gives the same part verdicts as main, and a resume from the run's final checkpoint lands on that seal (tower, measured).
- [ ] Two repeats write one chain: `history_bytes` after two repeats equals one repeat's.
- [ ] Every resolved link equals the bytes `keep` was given, over the whole window. A mutant row that flips one bit in `expand` is caught by `HistoryAltered` (`tools/gate/mutants.toml`).
- [ ] The PR states the measured bytes per link, the throughput with and without the codec (the store's cost counts in the timed run, per section 5), and the projected `full()` total by arithmetic.

**Non-goals:** heartbeat-cadence or merged chunks; f32 values; dropping columns from a link; checkpoint-and-regenerate stores; any change to `RecordFormat`, the seal or the certificate. Each one conflicts with section 5, the **Checkpoint** section or SRS invariant I1, or needs its own record. See the proposal in the 2026-11-07 vault log.

## Comments
