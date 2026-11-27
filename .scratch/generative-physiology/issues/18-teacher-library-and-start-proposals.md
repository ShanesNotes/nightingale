# 18: Teacher library, start proposals, and structural-mask pre-check

Type: research
Status: open
Blocked by: 14, `.scratch/native-contract/issues/86-admission-reaches-a-moved-rest.md`

**Question:** What buildable architecture governs the first two engine-facing builds beyond the slice posture experiment (`5.1` Teacher Library and `5.2` Start Proposals with multi-rest detection), and what empirical protocol settles `5.4`'s structural-mask pre-check on `whole_body()` without fabricating unmeasured density numbers?

This ticket is open again, and what follows is a draft. A review ran its claims against the engine at `9a7469ff` and changed sections 1.2, 1.3, 2 and 3 (Comments). A figure the review ran carries the label measured. The pass on [ticket 14](14-first-experiment-on-the-whole-body.md) takes the rest up, and [native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md) picks the route section 2 waits on. Nothing here is decided.

## Read set and scope

Checked at `origin/main` commit `9a7469ff` (PR `nightingale#242` merge over code commit `ffcd1a7a`, with `crates/`, `docs/`, `data/`, and `tools/` unchanged between `ffcd1a7a` and `9a7469ff`; `admission.rs` and `crossings.rs` line citations refreshed against `origin/main` `5b554ecf`) per [governance section 5](../../../docs/governance.md#5-what-an-architecture-pass-reads):

1. [SRS](../../../docs/srs.md): rules 2, 3, 4, 12, 13, 14, 15, 16; invariants I1 to I5, I7, I10, I11, I16 to I19; section 6 speed aim (`1x real time` live, `>= 10x real time` unbounded on the 13-system body).
2. Decision records: [0003](../../../docs/adr/0003-inferred-parameters.md), [0004](../../../docs/adr/0004-first-milestone.md), [0005](../../../docs/adr/0005-packet-and-healthy-range-set.md), [0006](../../../docs/adr/0006-waiting-input-shows-after-a-span-passes.md), [0007](../../../docs/adr/0007-a-guard-lands-before-anything-waiting-shows.md), and [0008](../../../docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md).
3. [GLOSSARY](../../../GLOSSARY.md).
4. [Solver contract](../../../docs/architecture/solver-contract.md): crate table (lines 241 to 250), validate/publish, causal step rule, waiting inputs on rewind (line 322), identity/replay, observation lens, and admission (lines 308 to 328, 686).
5. No atlas card (`docs/atlas/`).

Also verified directly on `9a7469ff` (including [ticket 16](16-architecture-foundations.md), landed on `origin/main` in `nightingale#242`):

- `tools/generative-physiology/`: `gp.py`, `rules.py`, `features.py`, `spec.py`, `teacher.py`, `capture.py`, `audit.py`, `selftest.py`, `spec/audit-v1.txt`, and `driver/` (`Cargo.toml`, `src/{lib.rs, main.rs, archive.rs, capture.rs, link_file.rs, npy.rs, smoke.rs, spec.rs, teacher.rs}`, `tests/{light.rs, acceptance.rs}`).
- `crates/ng-contract/src/{lib.rs, body.rs, packet.rs, admission.rs, boundary.rs, boundary/day.rs}`.
- `crates/ng-record/src/{lib.rs, record.rs, session.rs, history.rs, checkpoint.rs}`.
- `crates/ng-validate/src/{lib.rs, initialize.rs, replay.rs, certificate.rs, day.rs, arena.rs, body/mod.rs, body/packet.rs}` and `crates/ng-validate/tests/{body.rs, body_run.rs, certificate_fold.rs}`.
- Engine tickets: [native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md), [native-contract 95](../../native-contract/issues/95-the-record-trims-its-history.md), [body 28](../../body/issues/28-two-synthetic-days.md), [body 30](../../body/issues/30-step-plans-on-the-whole-body.md).

Note on current repo status at `ffcd1a7a`: `tools/generative-physiology/**` fast gate landed in `nightingale#238` (`tools/gate/paths.toml:14-16`), `CertificateFold` on `Session` landed in `nightingale#240`, and oscillation accuracy landed in `nightingale#241`. Code builds under `5.1`, `5.2`, and `5.4` remain parked until the line unparks ([the map](../map.md)).

## Draft answer

### 1. Build `5.1`: Teacher Library Blueprint

#### 1.1 Module Inventory (`tools/generative-physiology/`)

Each source file on `ffcd1a7a` keeps one clear owner across the Rust driver (engine lifecycle, capture, reduction, history store) and Python study (specs, scheduling, manifest checking, metrics, audit). The table lists 21 of the 22 tracked files. The other is `driver/Cargo.lock`, which carries unchanged while the driver adds no dependency. A row that names a prefix overlay, `branch-of` or parent-before-branch ordering belongs to section 1.2's alternative and is built only if that is taken.

| Path | Role on `ffcd1a7a` | Disposition | Planned change for `5.1` and `5.2` |
|---|---|---|---|
| `driver/Cargo.toml` | Standalone `[workspace]`, path deps on 5 workspace crates | `carries` | Keeps exact 5 crate path dependencies; adds no external dependencies |
| `driver/src/lib.rs` | Public driver module exports | `extends` | Exports `store` (`HistoryStore` policies) and `propose` (`5.2` bridge) |
| `driver/src/main.rs` | CLI (`smoke`, `run`, `interact`, `status`) | `extends` | Adds `--body slice\|whole-body`, `--store-mode`, `admit-only`, `mask-report` |
| `driver/src/npy.rs` | `NpyWriter<E>`, `Table<E>` aligned 128 B NPY 1.0 | `carries` | Unchanged binary table reader/writer |
| `driver/src/smoke.rs` | Slice `SMOKE_SPEC`, `SMOKE_PACKET`, `SMOKE_SEAL` pins | `carries` | Slice smoke stays intact; whole-body smoke pin added after the line unparks ([the map](../map.md)) |
| `driver/src/spec.rs` | `CAPTURE_VERSION = 2`, `RunSpec`, `PostureEvent`, `CoreChoice`, `Variant` | `extends` | Bumps `CAPTURE_VERSION` to `3`; adds `gp-run 2` (`body`, `packet`, `branch-of`, whole-body `Day` events) |
| `driver/src/archive.rs` | `RunSummary`, `IdentityDigests`, `PartialRun`, `Staged`, `RunState` | `extends` | Adds atomic `.refused/` archival (`spec.txt` + `refusal.txt`), `body`, `store_mode`, `links_kept_bytes`, `witnesses_bytes`, `parent_spec_id`, `branch_at_ns`, `packet_kind`; streaming SHA-256 replaces `fs::read` whole-file hashing (`tools/generative-physiology/driver/src/teacher.rs:863`) |
| `driver/src/capture.rs` | `Catalogue`, `Capture`, `BeatReading`, `edges.npy`, `beats.npy` | `extends` | Parameterizes `Catalogue` across assemblies, and keeps every row and column at every beat edge on the whole body too (section 1.5) |
| `driver/src/link_file.rs` | Single-file compacted `LinkFile` deleted at run end (`tools/generative-physiology/driver/src/teacher.rs:713`) | `extends` | Replaced or wrapped by `store.rs` for bounded retention and prefix/branch overlay |
| `driver/src/teacher.rs` | `Teacher`, `Feed`, `Original`, `follow`, `courses` | `extends` | Selects `whole_body`/`body_packet`/`body_lens` (`crates/ng-validate/src/lib.rs:411-416`), archives `InitializationFailure` to `.refused/`, where the error now leaves through a handled exit that archives nothing (`tools/generative-physiology/driver/src/teacher.rs:741-749`, `tools/generative-physiology/driver/src/main.rs:213-215`), wires `CertificateFold` (`crates/ng-validate/src/certificate.rs:1242`), exports solved starts |
| `driver/tests/{light.rs,acceptance.rs}` | Slice light and acceptance tests | `extends` | Adds whole-body spec grammar, `Day` event, overlay store, and `L1`/`L9` refusal tests |
| `gp.py` | CLI entry (`selftest`, `audit`) | `extends` | Adds `manifest`, `start-pool`, `propose`, `mask-report` subcommands (stdlib-only for library and manifest) |
| `rules.py` | Closed-loop `AuditPressure` rule | `carries` | Keeps slice rule; extends when whole-body closed-loop rules enter |
| `features.py` | Four posture response features (`Feature`, `Features`, `response_features`) | `extends` | Keeps posture features; adds `DayCheck`-aligned meal/drink windows |
| `spec.py` | `Plan` parser, `RunSpec` generator (`audit-v1.txt`) | `extends` | Extends `RunSpec` and `Plan` to `Day` events, `body`, `packet`, and `branch-of` |
| `teacher.py` | Pool scheduler, `_room()` disk check, `Settled` | `extends` | Adds parent-before-branch dependency ordering, bounded-store `_room()` accounting, manifest emission |
| `capture.py` | `RunData`, `Summary`, `Catalogue`, `Edges`, `Limbs`, `Ledger` | `extends` | Keeps v2 reader; adds v3 summary fields, `Day` event tables, and store-mode metadata |
| `audit.py` | Slice technical and informativeness gate | `extends` | Keeps slice audit; adds whole-body audit plan (`ticket 14`) |
| `selftest.py` | Synthetic fixture tests and `--real` driver test | `extends` | Adds `L9`/`L10`/`L12` manifest refusal tests, `Day` grammar tests, overlay store tests |
| `spec/audit-v1.txt` | Slice audit plan | `carries` | Frozen for slice; sibling `spec/audit-body-v1.txt` added after the line unparks ([the map](../map.md)) |
| `driver/src/store.rs` | Bounded `HistoryStore` (rolling eviction, witnesses, prefix overlay) | `new` | Implements disk-bounded retention and read-only prefix + writable branch overlay |
| `manifest.py` | `L9`/`L10`/`L12` manifest and solved-start table manager | `new` | Builds, checks, and digests `dataset_manifest.json`, `artifact_manifest.json`, and `starts-pool` keyed by `unread_packet_hash` |
| `driver/src/propose.rs` & `propose.py` | `5.2` start-proposal store, `L14` ladder, ingress function | `new` | Implements the `5.2` proposal store, the ladder and the multi-rest test, with one ingress function for the route native-contract 86 picks (section 2.1) |

#### 1.2 Fork 2: Paired-Branch Execution and Disk-Bounded Capture

Three verified engine mechanisms govern paired branches (such as a fed meal run vs. its counterfactual control) on whole-body multi-hour runs:

1. **Waiting inputs carry across rewind and resume (`docs/architecture/solver-contract.md:322`, `crates/ng-contract/src/admission.rs:234,263`, `crates/ng-contract/src/boundary.rs:900-943`, `crates/ng-record/src/checkpoint.rs:257`):**
   - `Checkpoint` embeds `ImageParts::pending` (`PendingInputs`), which holds every event admitted through `BoundaryPort`, `ParameterPacket::day` (`BoundaryPort::with_day` at `crates/ng-contract/src/admission.rs:234`), or `Session::enqueue` before the checkpoint.
   - On `Session::rewind` (and `Session::resume`), waiting inputs carry into the branch and enter the branch's event log (`docs/architecture/solver-contract.md:322`). Neither `rewind` nor `resume` can delete or withdraw an event already present in `PendingInputs` at the checkpoint instant `t_c`.
   - Furthermore, `ParameterPacket::day` enters `ParameterPacket::hash()` (`crates/ng-contract/src/packet.rs:535-537`), so two `Reviewed` packets whose `Day` timelines differ produce distinct `PacketHash` values and cannot share a checkpoint.
   - **Consequence:** Any shared prefix checkpoint at `t_c` MUST exclude post-`t_c` branching events from both `ParameterPacket::day` and `BoundaryPort`. Only shared events before `t_c` (plus baseline lasting inputs) live on the prefix; post-`t_c` branch-specific events (such as meal vs. omitted meal) enter each branch strictly after `t_c` via `Session::enqueue`, matching the pattern already used by `day_checks` in `crates/ng-validate/src/day.rs:448-470` (checkpoint before enqueueing plan events) and `Feed::AfterCheckpoint` in `tools/generative-physiology/driver/src/teacher.rs:414-432` (withholding events at or past `t_c + longest_step_ns`).

2. **`CertificateFold` on `Session` (`nightingale#240`, `crates/ng-validate/src/certificate.rs:1242-1359`, `crates/ng-record/src/history.rs:1026-1418`):**
   - At admission (`Chain::Opening`), `CertificateFold::open(&session, &policy)` starts with `Prefix::Empty` and zero `HistoryStore` reads (`crates/ng-record/src/history.rs:1064-1070`). As `fold.advance(&session)` runs, `ConservationFeed` pushes `PageMark` entries (one per `PAGE = 64` steps, `crates/ng-record/src/history.rs:1440`) and calls `store.resolve()` via `next_trailing_with_head` (`crates/ng-record/src/history.rs:1359-1417`) only once `windows.reaches(leading)` closes a trailing window (`crates/ng-validate/src/certificate.rs:1213-1236`). Once a window page passes behind the trailing cursor, it is never resolved again.
   - After `Session::resume` or `Session::rewind` (`Chain::Grown`), `ConservationFeed::from_record` calls `build_time_order(history, &backing, store)` (`crates/ng-record/src/history.rs:952-993, 1081`), which walks every link from `t_c` back to admission (`origin`) to populate `PageMark` entries; `take_prefix` (`crates/ng-record/src/history.rs:1218-1268`) then reads the prefix again, and not only the trailing window. Measured on the two-reservoir fixture, with 600 s windows and a checkpoint at 10,000 s: `restore` resolved 1,000 links, and opening the fold resolved 2,958 more, 999 of them newest-first to admission and then 1,959 by `take_prefix` over all 999 prefix revisions. Of those 1,959, 1,841 end at or before `t_c - window_ns`.
   - Also, `CertificateFold::certificate()` returns `Coverage` error whenever `span < policy.windows.coverage_ns` (`crates/ng-validate/src/certificate.rs:1326`), so 6-hour teacher runs cannot close a 24 h `CertificatePolicy`. For sub-24 h runs, `gp-driver` verifies step-by-step ledger closure from `Session::conservation().head()` and `edges.npy` directly (or uses a short `CertificatePolicy` window `<= run_length`), reserving 24 h `CertificateFold` for runs spanning at least 24 hours.

3. **`native-contract 95` history trimming (`nc95`, `docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md`):**
   - Before `nc95` lands, `LinkFile` writes every compacted link (~65,960 B/step on the 13-system body -> ~46 GB/sim-hour, `.scratch/native-contract/issues/95-the-record-trims-its-history.md:33`), and `ng_record::checkpoint::restore` (`crates/ng-record/src/checkpoint.rs:313` -> `History::walked` at `crates/ng-record/src/history.rs:738-791`) parses every link back to admission. `Session::rewind` additionally requires `parent.holds()` (`crates/ng-record/src/history.rs:717-737`), walking backward from the parent's head.
   - Once `nc95` lands, `HistoryStore` keeps a rolling day (up to 24 h) of complete links back to the checkpoint before the oldest open window (whole-body compressed size unmeasured before `nc95`; ~1.1 TB/day compacted alone at ~194 steps/s, `.scratch/native-contract/issues/95-the-record-trims-its-history.md:33`), plus 96-byte witnesses (~67 MB/sim-hour at ~194 steps/s -> ~402 MB for 6 h) and 1-minute checkpoints with head links (`~111 MB` for 48 h at slice scale, by record 0008's arithmetic from short windows and not from a measured course, `.scratch/native-contract/issues/95-the-record-trims-its-history.md:26`). Trimmed `restore` checks witnesses back to admission without reparsing discarded ledgers.

Two architectural shapes satisfy these constraints:

| Dimension | Shape A: Fresh prefix runs (`ticket 02` pattern) | Shape B: Prefix checkpoint via `Session::resume` + `OverlayStore` |
|---|---|---|
| Branch execution | Each branch admits independently from `t_0` with its own full `RunSpec` | Root prefix runs `t_0..t_c` once, writes `prefix.ckpt` before post-`t_c` events; each branch restores via `Session::resume` and enqueues post-`t_c` events |
| Waiting inputs / `Day` | `Day` or `BoundaryPort` may carry the entire run directly from `t_0`. Only a run that carries its events on the port can be compared with a branch | Prefix `Day` and `BoundaryPort` hold only `<= t_c` shared events; post-`t_c` events stay out of `PendingInputs` until after `prefix.ckpt` |
| `CertificateFold` & `store` | Opens at admission (`Prefix::Empty`); store holds only links inside active fold window (or zero links via `KeepsNothing`-like discard when no checkpoint/24 h fold runs) | Child branch `CertificateFold::open` walks prefix chain from `t_c` back to `t_0`; `OverlayStore` reads `(prefix_store read-only, branch_store read-write)` so `K` branches run in parallel without mutating prefix storage |
| Compute cost (`K` branches, prefix `t_c`) | `K * (T_admit + T_prefix + T_tail)` | `1 * (T_admit + T_prefix) + K * T_tail` (avoids `(K - 1) * (~240 s + t_c / speed)`), plus each branch's read of every prefix link |
| Transient disk (`K` parallel workers) | `K * store_bound` (before `nc95` with `KeepsNothing` when uncheckpointed: near `0`; with rolling 6 h links: `K * 276 GB`) | `1 * prefix_store + K * tail_store`; after `nc95` rolling eviction or sub-24 h step closure: prefix witnesses ~67 MB/h + active window |
| Replay & parallelism | Every `SpecId` replays from scratch in one file | Branch `SpecId` names `branch-of <parent_spec_id> at-ms <t_c>` and `realized` log carries prefix + branch events; `K` workers execute concurrently once `prefix.ckpt` lands |

Shape B would use `Session::resume` and not `Session::rewind`, because `Session::rewind` keeps the parent `Session` and its `CoupledSolver` live in one process and walks `parent.holds()` from the parent's head (`crates/ng-record/src/session.rs:174-181`, `crates/ng-record/src/history.rs:717`), and a `Session` is not `Send` (measured: it does not compile across threads). That choice has a cost in names. `rewind` and `recover` open a branch under its own `BranchId` (`session.rs:174, 219`), which is what the glossary means by a branch. `resume` continues the same run and gives a continuation no id (`session.rs:132`).

What the review measured at `9a7469ff`, on the two-reservoir fixture and on the slice with `ng-solver`:

- **Shape B works, for events carried on the port.** A branch resumed over a read-only prefix store and its own overlay gave seals and end digests equal to a fresh run's, bit for bit, for the fed branch and for the control, and no write reached the prefix store. A host can implement and wrap `HistoryStore`.
- **A fresh run that carries the event on the packet's day cannot be compared with a branch.** The day enters the packet hash, so the run identity and every seal differ.
- **A branch reads its whole prefix again.** `restore` walked every prefix link (3,877 of 3,877 on the slice), and a fold opened after the resume read the prefix about three times more (mechanism 2). A prefix store that has let one link go refuses the resume with `HistoryMissing`. On the whole body a link is about 66 kB, so an hour of prefix is 46 GB to read for each branch until native-contract 95 lands.
- **A branch in another process cannot name its run through public API.** `Session::resume` takes a `RunIdentity`. `RunIdentity::new` needs a `PacketHash` (`crates/ng-contract/src/scheme.rs:399`), and the one way to make that from a digest, `PacketHash::of_digest`, is `pub(crate)` (`crates/ng-contract/src/packet.rs:52`, error `E0624`). The probe had to parse the checkpoint at a private offset. This is a question for the engine's process, filed with [foundation 07](../../foundation/issues/07-contract.md).
- **The first experiment's prefix is short.** [Ticket 17](17-emulator-sensitivity-ranking.md)'s protocol admits at 06:55 and feeds at 07:30 in a six-hour run. The engine's own day check checkpoints at `t_0` after one admission and reruns the control from that start to the end of the meal's window (`crates/ng-validate/src/day.rs:433-434, 450-453, 471-473`), which is 38,100 simulated seconds for the pair. Sharing the prefix saves 2,100 of them, 5.5% of simulated time (or 6.7% of wall time at 2.5x and 9.7% at 10x against Shape A's two independent admissions, arithmetic).

- **Model default: Shape A** (fresh runs of one prefix), which is what the map holds ([the map](../map.md), "Paired branches are fresh runs of one prefix") and what [ticket 02](02-first-build-architecture.md)'s pass chose, so that each run replays from its spec alone. Ticket 02's reason against branches by rewind holds for `resume` as well: "every branch then depends on a stored checkpoint and on its links in a store".
- **One-line alternative: Shape B** (prefix checkpoint via `Session::resume` with a read-only prefix store and a writable overlay for each branch). It is worth reopening when a design shares a long prefix, once native-contract 95 bounds the links a branch must read and the engine's process has said how a branch in another process names its run. A pass that takes it supersedes the map's entry and says so.

#### 1.3 `L9`/`L10` Dataset Manifest Contract, Run-Spec Extensions, and Solved-Start Table

**Dataset Manifest (`dataset_manifest.json`, sorted-key compact JSON with SHA-256 `manifest_digest`):**

All datasets enforce `L9` (faults and admission refusals archived under `.fault/` and `.refused/` and counted, which `tools/generative-physiology/driver/src/teacher.rs:741-749` and `tools/generative-physiology/driver/src/main.rs:213-215` lack now: an error there exits `1` and leaves only a partial directory), `L10` (full provenance digests), and `L12` (teacher runs only on synthetic or reviewed packets, never real-person clinical data). Canonical formatting uses `json.dumps(..., sort_keys=True, separators=(",", ":"))`.

- **Top-level fields:**
  - `schema_version`: `1` (integer)
  - `partition`: `"train"` | `"validation_tuning"` | `"validation_call"` | `"heldout"` | `"replay"` | `"library"` | `"audit"` (each study partition has one reader, [spec.md](../spec.md) rule `L8`)
  - `origin_policy`: `"engine_teacher"` | `"authored_scenario_eval"`
  - `source_commit`: 40-char git commit SHA matching every entry's `summary.txt` or `refusal.txt`
  - `crates_tree_digest`: 40-char lowercase hex git tree hash of `crates/` (this repository's object format is SHA-1)
  - `model_hash`: 64-char hex (`CompiledBody::model_hash().digest()`)
  - `frozen_digest`: nullable 64-char hex (`FROZEN` tree digest; required non-null for frozen study partitions per `L8`)
  - `counts`: `{ "total_planned": integer, "complete": integer, "fault": integer, "refused": integer }`, requiring `total_planned == complete + fault + refused` (every planned run resolved and counted)
  - `packet_hashes_digest`: 64-char lowercase hex SHA-256 of the sorted unique non-null `packet_hash` hex strings across `entries`
  - `entries`: array sorted lexicographically by `spec_id`, covering every planned run including faults and admission refusals:
    - `spec_id`: 64-char hex `SpecId`
    - `origin`: `"engine_teacher"` | `"authored_scenario_eval"` (`"engine_teacher"` required for training and library partitions per `L12`)
    - `packet_kind`: nullable string, `"synthetic_fixture"` | `"reviewed_literature_packet"` for `"engine_teacher"` (`null` for `"authored_scenario_eval"`; `L12`: never real-person clinical data)
    - `packet_hash`: nullable 64-char hex pre-admission packet hash (present for `"engine_teacher"` even when `"refused"`; `null` for `"authored_scenario_eval"`). A host reads it through `packet.anchors().packet()` (`crates/ng-contract/src/packet.rs:254`, `crates/ng-contract/src/provenance.rs:93`), because `ParameterPacket::hash` is private (`packet.rs:424`, error `E0624`). The post-admission `InitialRevision::packet_hash()` is the one `summary.txt` records, as `admitted_packet_hash`
    - `packet_seed`: nullable string (for varied-packet studies, used by `L8` to refuse `PacketSeed` overlap with `heldout`; `null` for `"authored_scenario_eval"`)
    - `start_history_id`: nullable string (for study partitions, used by `L8` to refuse start-history overlap with `heldout` even on fresh runs; `null` for `"authored_scenario_eval"`)
    - `resolution_digest`: nullable 64-char hex (`DeclaredResolution::digest()` for `"engine_teacher"`, `null` for `"authored_scenario_eval"`)
    - `run_identity`: nullable 64-char hex (`RunIdentity::digest()`; non-null if and only if `origin == "engine_teacher"` and `outcome != "refused"`)
    - `outcome`: `"complete"` | `"fault"` | `"refused"`
    - `detail`: nullable string (`InitializationFailure` or `Fault` detail when `outcome != "complete"`, `null` when `"complete"`)
    - `summary_sha256`: 64-char hex SHA-256 of `<spec_id>/summary.txt`, `<spec_id>.fault/summary.txt`, `<spec_id>.refused/refusal.txt`, or (for `"authored_scenario_eval"`) the authored scenario file on disk
    - `parent_spec_id`: nullable 64-char hex (`null` for root runs and `"authored_scenario_eval"`; all branches sharing `parent_spec_id` remain in the same `partition`)
  - `manifest_digest`: 64-char lowercase hex SHA-256 of the canonical JSON text with `manifest_digest` omitted

Supplemental run-library metadata (`body_assembly`, `admitted_packet_hash`, `store_mode`, `branch_at_ns`, `first_seal`, `last_seal`, `revisions`, `beats`) lives inside each archived `summary.txt` (or `refusal.txt` for `.refused/`) so `summary_sha256` binds it without mutating `dataset_manifest.json` keys.

**Learned Artifact Manifest (`artifact_manifest.json`, `L10`, sorted-key compact JSON with `artifact_digest`):**
Every trained or fitted artifact (start proposer, emulator, generative physiology model, or observer) pairs its weights with `artifact_manifest.json` carrying:
- `schema_version`: `1` (integer)
- `artifact_kind`: `"start_proposer"` | `"emulator_stage_a"` | `"emulator_stage_b"` | `"gp_model"` | `"observer"` | `"chart_inference"`
- `family`: `"nearest"` | `"continuation"` | `"linear_map"` | `"gp_ard"` | `"neural_emulator"` | `"C1"` | `"C2"` | `"C4a"` | `"R"` | `"instrument"` | `"qualitative"` | `"npe"` | `"fmpe"`
- `claim_label`: `"proposal_only"` | `"validated"` | `"approximate"` | `"untested"`
- `claim_level`: integer `1` to `5` from ticket 01's section 6, `null` when `claim_label` is `"proposal_only"` or `artifact_kind` is `"emulator_stage_a"`
- `conserving`: boolean (`true` requires non-null `reduction_digest`)
- `continuous_rate`: boolean
- `engine_commit`: 40-char git commit SHA
- `crates_tree_digest`: 40-char hex git tree hash of `crates/`
- `model_hash`: 64-char hex (`CompiledBody::model_hash().digest()`)
- `dataset_manifest_digest`: 64-char hex `manifest_digest` of the training or library `dataset_manifest.json`
- `packet_hashes_digest`: 64-char hex matching `dataset_manifest.json`
- `reduction_digest`: nullable 64-char hex (required non-null when `conserving` is `true`)
- `frozen_digest`: nullable 64-char hex
- `train_script_sha256`: 64-char hex
- `training_seed`: integer
- `runtime_env_digest`: 64-char hex
- `weights_sha256`: 64-char hex
- `outside_checks`: array of `{ "citation": string, "source_set_tag": string, "claim_mode": string }` (empty allowed for `"proposal_only"` and `"emulator_stage_a"`; non-empty required for `"gp_model"`, `"observer"`, and `"chart_inference"`)
- `feature_accounting`: nullable object (`null` unless `artifact_kind == "observer"`)
- `artifact_digest`: 64-char lowercase hex SHA-256 of canonical JSON with `artifact_digest` omitted

`manifest.py` refuses any dataset manifest with missing or duplicate `spec_id`, `counts.total_planned != counts.complete + counts.fault + counts.refused`, any `.fault/` or `.refused/` directory omitted from `entries`, any digest mismatch (`summary_sha256`, `manifest_digest`, `artifact_digest`, `weights_sha256`), `origin != "engine_teacher"` when `origin_policy == "engine_teacher"` or on training or library partitions, a `validation_call` or `heldout` manifest passed to `fit()`, or a `heldout` manifest passed to `call` or to an inference-time lookup. Artifact loading against a compiled body with mismatched `model_hash`, `reduction_digest` (when `conserving`), `frozen_digest` (matching active `FROZEN` whenever `FROZEN` exists or `frozen_digest` is set), or empty `outside_checks` for a physiological-claim artifact (`gp_model`, `observer`, `chart_inference`) fails closed before inference.

**Whole-body `RunSpec` extensions (`gp-run 2`, `CAPTURE_VERSION = 3`):**
Extends `driver/src/spec.rs` in strict line order (`LINES_IN_ORDER`):
- `gp-run 2`
- `body <slice | whole-body>`
- `packet <preset | sha256-hex>` (selects `body_packet` / `ParameterPacket` builder and seed)
- `core <ng-solver ... | reference ...>`
- `end-ms <u64>`
- `branch-of <parent-spec-id> at-ms <u64>` (optional; only under section 1.2's alternative)
- Typed whole-body `Day` boundary event lines (`BoundaryInput`, strictly non-decreasing `at-ms >= fence`):
  - `event at-ms <MS> posture trunk <K> legs <horizontal|seated|standing>`
  - `event at-ms <MS> meal ends-ms <MS> water <F> na <F> k <F> cl <F> glc <F> tg <F> aa <F> base <F> ca <F> po4 <F> mg <F> fe <F>` (all 12 `Meal::SPECIES` from `crates/ng-contract/src/boundary.rs:65-78`)
  - `event at-ms <MS> drink ends-ms <MS> water <F>`
  - `event at-ms <MS> room air <K> radiant <K> humidity <H> air-speed <S> pressure <P> o2 <O> co2 <C> clo <CL>`
  - `event at-ms <MS> sleep <awake|nrem>`
  - `event at-ms <MS> clock since-midnight-ms <MS>`
  - `event at-ms <MS> void <open|shut>`
- `closed-loop <rule>` (optional)
- `variant <...>` (carries existing `none`, `late`, `checkpoint-resume`, `rewind`, `observer-off`, `replica`).
- `--store-mode` and `--memory-cap-gb` remain CLI runtime flags outside `SpecId`, because storage eviction never alters physiology bits (`I2`).

**Solved-Start Registry (`starts-pool`):**
At admission (`crates/ng-contract/src/admission.rs:1840-1841`), `InitialRevision::packet_hash()` folds solved `starts`, `modes`, `fitted` slots, witness, and `MethodHistory` over the pre-admission packet hash. Because `5.2` proposes starts for *unadmitted* packets that only possess that hash, `5.1` keys its solved-start table (`starts-pool.jsonl` / `starts/<unread_packet_hash>.json`) by **`(model_hash, unread_packet_hash)`**. The key fixes the start and not the admitted hash. Measured on the arena packet with the audit plan's three cores: one pre-admission hash gave one start state and three admitted hashes, because the admitted hash closes over each core's method history. So a row holds the start once, and it holds an admitted hash for each core resolution that reached it. A row stores: `unread_packet_hash`, `admitted_packet_hashes` (by `resolution_digest`), `anchor_moment_ns`, `cycle_spec`, `parameter_values` (in `CompiledBody::parameters()` canonical order), `solved_starts` (`Vec<f64>` in `rows()` order), `solved_modes`, `solved_fits`, `located_crossings`, `worst_residual`, `admission_s`, and `source_spec_id`.

#### 1.4 Synthetic and Audit Gates for `5.1`

1. **Whole-body teacher audit gate:** Passes both technical (whole-body first-second seal pin, checkpoint-resume, late admission, rewind, observer-off, replica, prefix equivalence, closed-loop replay, `64 * 2^-52` ledger closure, beat-identity check) and informativeness (meal/day-check responses above feature floor) gates once [ticket 14](14-first-experiment-on-the-whole-body.md) unparks ([the map](../map.md)).
2. **Paired-branch equivalence gate:** A Shape B checkpoint-branched run (`t_0 -> t_c -> branch`) produces bit-identical `seals`, `BranchSummary::end` digests (`RecordDigests`), and `edges.npy` rows to a Shape A fresh run from `t_0` over the same realized event list, carried on the port (`I2`, `I3`). This gate and the next apply only if section 1.2's alternative is built. The review measured the seals and end digests equal on the slice. It did not check `edges.npy`: a new capture starts each row's integral at zero and writes it into every edge row (`tools/generative-physiology/driver/src/capture.rs:445, 586`, implemented), and this draft does not say how a branch carries the prefix's.
3. **Waiting-input prefix refusal gate:** Constructing a Shape B shared prefix whose `ParameterPacket::day` or admission `BoundaryPort` includes any event past `t_c` is refused before execution (`SpecError` / `ManifestError`).
4. **Disk-bounded capture gate:** A 6-hour whole-body run (and a `K`-branch paired group) completes within a declared `links_kept_bytes + capture_bytes` ceiling rather than retaining 276 GB of compacted links per run.
5. **Planted-fault and manifest refusal gates (`L9`, `L10`, `L12`):**
   - Planted `.fault/` directory (`MemoryCap` or `EngineFault`) and planted `.refused/` directory (`refusal.txt`) increment `counts.fault` / `counts.refused` in `dataset_manifest.json` and fail manifest checks if omitted from `entries`.
   - `dataset_manifest.json` with `counts.total_planned != counts.complete + counts.fault + counts.refused`, duplicate `spec_id`, or `origin != "engine_teacher"` when `origin_policy == "engine_teacher"` or on a training/library partition is refused before export.
   - `artifact_manifest.json` load with mismatched `model_hash`, `manifest_digest`/`dataset_manifest_digest`, `reduction_digest` (when `conserving`), `frozen_digest` (against active `FROZEN` whenever `FROZEN` exists, or `frozen_digest` when set), or empty `outside_checks` when `artifact_kind` is in `{gp_model, observer, chart_inference}` is refused (`L10`).

#### 1.5 What a whole-body run captures

[Ticket 14](14-first-experiment-on-the-whole-body.md) planned a long run that keeps only the compiled reduction's totals and extents at each beat edge, and full state at sparse edges. It was written before the body's counts existed. With them, the full capture is small (arithmetic from the driver's formula and the body's pins):

- An edge costs `2 + M + 4R + 4C + 6Q` words of 8 bytes (`tools/generative-physiology/driver/src/capture.rs:230`), for `M` modes, `R` rows, `C` columns and `Q` conserved combinations.
- The whole body has 1,465 rows, 1,463 columns and 396 conserved combinations (`crates/ng-validate/tests/body.rs:928`), so an edge is about 113 kB. The slice's is about 17 kB.
- Six hours at 65 beats a minute is about 2.6 GB a run, 0.78 TB for the 296 runs of [ticket 17](17-emulator-sensitivity-ranking.md)'s design and 1.3 TB for 500.
- The links of the same run are 276 GB until native-contract 95 lands, so the capture is about 1% of what a run writes.

- **Model default:** keep every row and column at every beat edge on the whole body, as the slice does. One library then serves every cut that [ticket 16](16-architecture-foundations.md)'s fork 3 weighs, and ticket 07's sufficiency screen can compare cuts on the same runs.
- **One-line alternative:** capture only one reduction's totals and extents, if disk binds. Then a change of cut needs the teacher's runs again.

---

### 2. Build `5.2`: Start Proposals and Multi-Rest Detector Blueprint

#### 2.1 The three routes of fork 1, as the engine stands

[Native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md) leaves open three routes by which a start proposal can reach `InitializationSystem::admit` ([ticket 16](16-architecture-foundations.md), section 8, fork 1). The first draft of this section said one bridge would let `5.2` work unchanged whichever route is picked. The review ran each route at `9a7469ff` on the compiled whole body, and none works as that draft wrote it (measured):

1. **Route 1, packet start entries.** `ParameterPacket::<SyntheticFixture>::start`, `start_mode` and `calibrated` (`crates/ng-contract/src/packet.rs:462-474, 592-648`) are sealed into the pre-admission hash. But `body_packet` already starts every row and every mode (`crates/ng-validate/src/body/packet.rs:683-706`), a second start is refused as `DuplicateStart` (`packet.rs:599`), and the builder that could take other starts, `packet_of`, is private (`body/packet.rs:602`). So this route needs a packet builder from the engine's process. Varied packets need the same thing ([ticket 15](15-learned-emulator-for-parameters.md)).
2. **Route 2, a public `AdmissionWorkspace`.** A host builds `system = body.initialization(bound, port, request)?` (`crates/ng-contract/src/admission.rs:223`), takes `ws = system.workspace()` (`admission.rs:844`), writes stage slots through `ws.unknowns_mut()` (`admission.rs:437`) and calls `system.admit(ws, history)` (`admission.rs:1808`). The first draft missed two things. A fresh workspace holds no crossings (`admission.rs:844-857`), so `ws.shift_crossing()` (`admission.rs:487`) returns `CrossingOutside` until the public `sweep` has placed them (`crates/ng-contract/src/admission/crossings.rs:599`). And a workspace's length follows its crossings, so stage unknowns taken from a solve that has crossings do not fit a fresh one (`admission.rs:627-636, 719-727`). The whole body's workspace holds 293,003 unknowns over 100 points.
3. **Route 3, inside `ng-validate`'s solve.** The solve itself, `initialize`, is private (`crates/ng-validate/src/initialize.rs:243`), and taking a guess there is the engine's change to make. `solve_fixture` is public (`initialize.rs:416`, exported at `crates/ng-validate/src/lib.rs:432`) and returns the system with its solved workspace, so a host can already join it to route 2. The driver admits through this crate now (`tools/generative-physiology/driver/src/teacher.rs:741`).

So `5.2` is not decoupled from native-contract 86. It waits for that ticket's pick, and then it needs one ingress function for the picked route, not a dispatcher over three. A start proposal still has one shape whichever route carries it: `model_hash`, `unread_packet_hash`, `cycle_spec`, `start_state: Vec<f64>`, `start_modes: Vec<usize>`, `fitted_params: Vec<f64>`, and the proposer's own `rung`, `source_packet_hashes` and `proposer_digest`, which enter no identity. Stage unknowns and crossing offsets stay out until a route can take them.

#### 2.2 Two Architectural Shapes for Proposal Store and `L14` Ladder

| Dimension | Shape 1: Offline Python `propose.py` + `StartProposal` + a thin Rust ingress function | Shape 2: All-Rust `propose.rs` with binary `mmap` `start_pool` |
|---|---|---|
| Store owner | `propose.py` reads `starts-pool` (`5.1` manifest-verified, keyed by `(model_hash, unread_packet_hash)`) | `driver/src/propose.rs` reads packed fixed-width `start_pool.bin` directly in Rust |
| `L14` ladder execution | Python executes Rung 1 (passthrough), Rung 2 (standardized L2 nearest neighbor over `body.parameters()`), and Rung 4 (ridge/linear map fit on training fold only, `L8`); Rust executes `Rung 3` continuation sequence in-process over one compiled `whole_body()` | Rust executes all four rungs (including nearest-neighbor scan, ridge/linear solve, and continuation) inside `gp-driver` |
| Engine seam | `propose.py` emits a `StartProposal`; one ingress function in `driver/src/propose.rs` carries it over the route native-contract 86 picks | The same ingress function, coupled with Rust regression/store code |
| Alignment & dependencies | Honors `ticket 02`'s split (Python fits/selects models; Rust owns engine lifecycle/`admit`); adds zero dependencies or `clippy.toml`-sensitive math to `driver/Cargo.toml` | Duplicates linear-algebra/regression code in Rust under `clippy.toml` bans (no host libm, no hash maps) and mixes model fitting into driver |
| Latency | ~20 ms JSON/NPY handoff (negligible against 55 to 240 s whole-body admission) | Sub-ms lookup (wins only if admission itself drops below ~100 ms) |

- **Model default: Shape 1** (`propose.py` owns the `start_pool` lookup, Rung 2 nearest neighbor, Rung 4 linear map, and `L14` gate evaluation; `driver/src/propose.rs` owns the ingress function for the picked route plus the in-process `Rung 3` parameter-continuation loop reusing one compiled `whole_body()`). This keeps model fitting in Python per [ticket 02](02-first-build-architecture.md) and avoids linear-algebra duplication in `driver/`.
- **One-line alternative: Shape 2** (all-Rust `propose.rs`), chosen only if future admission speed falls below ~100 ms per packet where Python IPC dominates.

**How the `L14` ladder promotes rungs (`5.2`):**
1. **Rung 1 (Packet guess):** `body_start_guess` / packet's own `starts`. Baseline median admission wall time `T_1` and admission failure rate `F_1`.
2. **Rung 2 (Nearest admitted neighbor):** Select `P_nearest` minimizing standardized parameter distance `sum_k ((x_new[k] - x_i[k]) / width[k])^2` in `start_pool` (`model_hash` equal), propose its `(start_state, start_modes, fitted_params)` via `StartProposal`.
3. **Rung 3 (Continuation):** Invoked when Rung 2 fails to admit (or exceeds target wall/iteration cap). Interpolates `P(alpha) = (1 - alpha) P_nearest + alpha P_new` in-process from `alpha = 0` to `1` (adaptive step halving on `InitializationFailure`), carrying each admitted solve's unknowns into the next `alpha` as its guess. A workspace cannot pass between them. Each `alpha` is a new packet, the admission system's identity seals the packet's digest (`crates/ng-contract/src/admission.rs:275-279`), and `admit` refuses another system's workspace as `ForeignWorkspace` (`admission.rs:627-636`). So the unknowns are copied into the new system's own workspace, and they fit only once its crossings match (section 2.1, route 2).
4. **Rung 4 (Linear map):** Fits regularized linear map `W: x_param -> (start_state, fitted_params)` on the `train` partition of `starts-pool` (`L8`: `validation_call` and `heldout` strictly excluded). Promoted over Rungs 1 to 3 ONLY if it improves `validation_call` median admission time or admission success rate by a pre-registered margin without reading `heldout` (`L14`).

#### 2.3 Exact Multi-Rest Detection Test

To test whether a packet admits to one unique rest or multiple distinct rests ([ticket 16](16-architecture-foundations.md), section 5.2; [native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md) box 4):

1. **Independent guesses:** For each test packet `P`, construct `M >= 3` independent start guesses `{G_0, ..., G_{M-1}}`: `G_0` = Rung 1 packet guess; `G_1` = Rung 2/4 learned proposal; `G_2..G_{M-1}` = deterministic perturbations of initial `starts` and `calibrated` guesses inside `Row::admissible()` / `ParameterSlot::admissible()` bounds.
2. **Engine judgment:** Pass every `(P, G_j)` through the ingress function to `InitializationSystem::admit` with identical `BoundaryPort`, `ObservationLens`, `anchor_moment`, and `MethodHistory`. Log all refused guesses (`InitializationFailure`, `L9`); keep all admitted `InitialRevision_j` results.
3. **Pairwise tolerance comparison:** For every admitted pair `(a, b)`:
   - Compare discrete guard settings: `modes_a == modes_b`. Any mismatch is a **branch-distinct second rest**.
   - Compare every continuous start row `r in 0..body.rows().len()` and every fitted parameter `f in 0..fits.len()` against a bound the test states for itself, with row size `size[r]` and fit size `1.0` (`crates/ng-contract/src/admission.rs:808-814`):
     `d_r = |x_a[r] - x_b[r]| / (abs_tol * size[r] + rel_tol * max(|x_a[r]|, |x_b[r]|))`
   - That bound is not admission's tolerance. Admission bounds each stage's defect at its judged row (`admission.rs:1414-1418, 1586-1591`) and says nothing of the distance between two admitted states. A packet that states no tolerance is held exactly (`ErrorScales::EXACT`, `admission.rs:269`), which would make the denominator zero. The whole body states `1e-9` absolute and `1e-10` relative (`crates/ng-validate/src/body/packet.rs:621`), and `admission_tolerance()` (`1e-9` and `1e-12`, `crates/ng-validate/src/initialize.rs:17-20`) is that crate's own default. So `abs_tol` and `rel_tol` are a model default to set when the test is built: wider than the defect tolerance, and taken from the spread one packet's start shows across guesses that reach the same modes.
   - Compare `LocatedCrossing::at()` times (`crates/ng-contract/src/admission.rs:564-574`) within the admission `Cycle` step floor.
4. **Result and dynamical follow-up:**
   - If `modes_a == modes_b` and `max(d_r, d_f) <= 1.0` for all pairs: report **single rest** (`max_d`).
   - If `modes_a != modes_b` or `max(d_r, d_f) > 1.0`: report **second rest detected**, logging `(unread_packet_hash, admitted_hash_a, admitted_hash_b, max_d, offending_rows_or_modes)` and optionally stepping both `Session`s forward under identical empty post-fence inputs to check whether the two rests stay distinct or relax onto one orbit.

#### 2.4 Exact Synthetic Refusal Tests (`L1`, `L9`, `L14`)

1. **`L1` refusal (No unverified start-vector entry):** Construct a synthetic `StartProposal` whose coordinates satisfy every `AdmissibleSet` bound (`positive`/`finite`) yet violate a coupled algebraic/periodicity residual (perturb one pressure or pool row +10% off equilibrium). Submit through the ingress function to `InitializationSystem::admit`.
   - *Must fail with:* `InitializationFailure::Inconsistent { row }` (or `GuardSweep`/`NetBoundaryFlow`); `Session::admit` is never called. The first draft also asked that `gp-driver` never call `PatientRecord::restored`. That clause can never fail, because the function is `pub(crate)` (`crates/ng-record/src/record.rs:49`). The clause that can fail is the host half of [spec.md](../spec.md)'s rule `L1`: the driver resumes no checkpoint that its archive did not record.
2. **`L9` refusal (Refused admissions archived and counted, never dropped):** Run a batch of synthetic packets containing at least one deliberately un-admissible packet (e.g., parameter causing `NetBoundaryFlow` or `Inconsistent`), checking that `gp-driver` archives `<spec_id>.refused/refusal.txt` instead of leaving through an exit that archives nothing (`tools/generative-physiology/driver/src/teacher.rs:741-749`, `tools/generative-physiology/driver/src/main.rs:213-215`).
   - *Must pass/fail as:* `dataset_manifest.json` holds `counts.refused >= 1` with `outcome = "refused"`, `run_identity = null`, and exact `InitializationFailure` in `detail`; omitting `<spec_id>.refused/` from `entries` causes the manifest check (`counts.total_planned == counts.complete + counts.fault + counts.refused` plus directory completeness) to fail.
3. **`L14` refusal (Cheaper rung blocks unimproved richer rung):** On a synthetic validation-fold fixture where Rung 4 (linear map) is deliberately degraded or fails to outperform Rung 2 (nearest neighbor) in median admission wall time / success rate:
   - *Must fail with:* `L14` ladder selector refuses promotion of Rung 4 and retains the lower winning rung; forcing Rung 4 into the active proposer config triggers a gate refusal.

---

### 3. Build `5.4`: Structural-Mask Pre-Check Protocol

#### 3.1 Verified Public vs. Private Sparsity Surface (`origin/main` `9a7469ff`)

Verified directly in `crates/ng-contract/src/body.rs` and `crates/ng-validate/tests/body.rs:896-934`:

- **Whole-body compiled pins (`crates/ng-validate/tests/body.rs:928`):**
  - `rows`: `1,465` (`1,251` amount, `196` algebraic, `18` memory)
  - `columns`: `1,463`
  - `jacobian nonzeros` (`body.sparsity().unknowns().len()`): `10,162`
  - `blocks` (`body.blocks().len()`): `415`, with `largest block` = `669` rows
  - `parameter slots`: `761`; `conserved combinations`: `396`; `held pools`: `318`; `held route columns`: `84`
- **Public accessors on `CompiledBody` (`crates/ng-contract/src/body.rs:109-136, 780-819`):**
  - `body.rows()`, `body.columns()` (`Column::terms()`, `origin()`, `context()`, `receipt()`), `body.instances()` (`Instance::program()`, `context()`, `target()`, `parameters()`), `body.parameters()`, `body.sparsity()` (`Sparsity::row_offsets()`, `Sparsity::unknowns()`), `body.blocks()` (`Block::rows()`, `unknowns()`, `rate_class()`, in topological dependency order), `body.conserved_basis()`, `body.elimination_metrics()`.
- **Private (`pub(crate)`) internals (`crates/ng-contract/src/body.rs:502, 518, 599`):**
  - `Instance::inputs: Vec<Bound>` (`crates/ng-contract/src/body.rs:599`) and `Program::code: Code` (`crates/ng-contract/src/body.rs:502, 518`) are **`pub(crate)`**, NOT public.
- **Implication for mask extraction:** An external host (`gp-driver`) sees exact **per-row** `UnknownId` reads (`body.sparsity()`) and exact SCC **block ordering** (`body.blocks()`), NOT per-column (`Target::Column`) reads (`Instance::inputs`). For an `Amount` row touched by multiple transfers (such as a capillary plasma pool touched by vascular inflow/outflow, Starling filtration, gas diffusion, and metabolism), `body.sparsity()` returns the union of all unknowns read across every incident column on that row. Transfer-level masks derived strictly from public APIs are therefore **outer bounds** over incident row closures, and their actual density at each cut on `whole_body()` is unmeasured until the one-pass measurement below executes.

#### 3.2 The measurement at the finest cut, and the protocol for a coarser one

The first draft deferred this measurement to ticket 14's unparking. It needs only a compiled body, with no admission and no step, and it runs in about a second. The review ran it at `9a7469ff` on `whole_body()` at the finest cut, where each state row is its own total (measured):

| Quantity | Value |
|---|---|
| State rows | 1,269 (1,251 amount and 18 memory), beside 196 algebraic rows |
| Amount rows that no column touches, the held pools | 318 |
| States that one state row's rate reads directly | median 6, maximum 49 |
| The same once algebraic rows are closed | median 13, 95th percentile 37, maximum 70 |
| Density once closed, over all 1,269 state rows | 0.9% |
| Density once closed, over the 951 rows a law moves | 1.2% |
| Rows that read more than half the states | none |

The method uses public accessors only. It reads each row's unknowns from `body.sparsity()`. It closes each algebraic unknown onto the states it rests on, to a fixed point, because algebraic rows form loops inside a block: 296 reads go from one algebraic row to another in the same block, in 2 blocks, and the fixed point took 4 sweeps. Then each state row gets the states it reads and the closures of the algebraic unknowns it reads. Every algebraic row reads its own unknown, and a row's unknown index is its row index, so the closure is well defined. The count is structural. It says which states a rate can depend on and nothing of how strongly, it is an outer bound for a transfer (section 3.1), and the body is on its holds.

So the largest block's 669 rows do not make one row's rate dense. That overturns the inference in [ticket 16](16-architecture-foundations.md)'s section 5.4, which expected the circulation to stay dense, and that section now carries this figure.

A coarser cut unions its rows' reads and will be denser. Once a cut exists (ticket 05 on the whole body, after [ticket 14](14-first-experiment-on-the-whole-body.md)), the same pass runs for it from a driver command:

```text
gp-driver mask-report --body whole-body --cuts <coarse.txt>,<tissue.txt>,<richer.txt> --out <report.json>
```

**Algorithm (`driver/src/mask_report.rs` or subcommand):**

1. **Row-to-state closure:**
   - Number all `1,465` rows (`Amount`, `Memory`, `Algebraic`) and identify the `318` held pools from `body.conserved_basis().coverage().pools()`.
   - For each `Algebraic` row, accumulate the union of the `(Amount | Memory)` `UnknownId`s it reads via `body.sparsity()` and the closures of the `Algebraic` unknowns it reads, repeating until nothing changes. One ordered pass over `body.blocks()` (`crates/ng-contract/src/body.rs:130`) is not enough: 122 of the 296 reads inside a block go to a row listed later in it (measured). For every `Amount` and `Memory` row `r`, store its RHS dependency set `state_deps[r]` split into `(active_amount_reads, held_pool_reads, memory_reads)`.
2. **Project onto each candidate reduction cut `(P, S, B, A)`:**
   - Compute **(a) Total-to-total matrix `D_total`** (`M_totals x (M_totals + M_mem)`): `D_total[m, j] = 1` iff any non-held `Amount` row in total `T_m` has `(active_amount_or_memory)` `j` in `state_deps[r]`. Report `nnz(D_total)`, global density, per-species-group density (vascular/fluid, electrolytes, gases, metabolism/fuels, heat, transmitters), and per-total in-degree `(min, median, p95, max)`.
   - Compute **(b) Transfer-level outer-bound mask `D_transfer`** (`N_transfers x (M_totals + M_mem)`): for each derived transfer `t` linking totals `(from, to)`, compute the row-union bound across rows touched by `t`'s `Column::terms()` AND count **ambiguous dependency entries** where `t`'s incident `Amount` row is shared with other distinct transfer families (quantifying exact over-approximation caused by private `Instance::inputs`).
   - Compute **(c) Held-pool read audit** (`held_pool_reads` count per total) and **(d) SCC block composition** across all `415` blocks (size distribution and species breakdown of the `669`-row largest block).

**Decision rules settling Forks 3 and 4 from the measured numbers:**

- **Fork 3 (Where the cut sits):** Evaluate `{coarse, tissue-grouped, R (each-row)}` cuts against `D_transfer` parameter counts under [ticket 08](08-model-forms.md)'s `100,000`-parameter cap and [ticket 07](07-sufficiency-screen.md)'s sufficiency screen; pick the coarsest cut that passes sufficiency inside the parameter cap.
- **Fork 4 (Dense vs. masked transfers):**
  - If measured `D_transfer` density is `>= 0.70` across active totals at the chosen cut, keep dense transfer inputs (masking buys negligible pruning) OR, if ambiguous shared-row entries account for the high density while chemistry/renal/heat blocks remain uncoupled, file a single scoped question to the engine process requesting public `Instance::inputs(&self)` on `CompiledBody` (`crates/ng-contract/src/body.rs:599`) before re-measuring.
  - If measured `D_transfer` density is `< 0.70`, run `5.4`'s Step 2 head-to-head arm: train masked `C4a` vs. dense `C4a` under the same frozen study (`L8`, `L14`), keeping the structural mask only if it matches or outperforms dense `C4a` out-of-policy at equal or lower parameter count.
  - At the finest cut the measured row-level `D_total` density is 1.2% (over the 951 active state rows), and the column row-union `D_transfer` outer bound across the 1,463 columns is 2.8% over all 1,269 states (3.7% over the 951 active states; median 40, 95th percentile 70, maximum 82, measured at `5b554ecf`), far under that line, so the masked arm earns its run for `R`. A coarser cut's own figure decides for that cut.

## What is still owed

- Fork 1's route, from native-contract 86. Section 2 then gets one ingress function and its tests.
- The answers to the two questions filed with [foundation 07](../../foundation/issues/07-contract.md): how a branch in another process names its run, and what a host owes at `Session::resume`.
- The mask's density at each cut that ticket 05 defines on the whole body, by section 3.2's pass.
- Sections 1.1, 1.3 and 1.4 read again at the commit where [ticket 14](14-first-experiment-on-the-whole-body.md) unparks, because the driver and the engine both move before then.

## Comments

- Verified all Rust paths, symbol names, counts, and behaviors directly on `origin/main` (`9a7469ff`, with code tree unchanged from `ffcd1a7a`).
- Key engine interactions verified in code: `ParameterPacket::day` merges into `BoundaryPort` at `crates/ng-contract/src/admission.rs:234`, sealing future `Day` events into `PendingInputs` before any post-admission checkpoint (`docs/architecture/solver-contract.md:322`); `CertificateFold::open` on `Session::resume` walks links back to admission via `build_time_order` (`crates/ng-record/src/history.rs:952, 1081`); `InitialRevision::packet_hash()` seals solved `starts`, `modes`, and `fitted` values over the pre-admission `unread` `ParameterPacket::hash()` (`crates/ng-contract/src/admission.rs:1840-1841`), requiring `5.1`'s solved-start table to key on `unread_packet_hash` for `5.2`.

From a review of this ticket. Its probes ran at `9a7469ff` on the two-reservoir fixture, the slice and the compiled whole body, and nothing was admitted or stepped on the whole body:

- **Open again.** The answer is a draft. Every citation resolved, and what changed is what the code does behind them.
- **Section 1.2.** The default goes back to fresh runs, which the map holds. Checkpoint branches were shown to work and to match fresh runs bit for bit, and the costs the first draft left out are listed there. The claim that a fold reads only the trailing window after a resume was wrong.
- **Sections 1.3 and 1.5.** The partitions are split by reader, the tree hash is 40 characters, the pre-admission hash is read through a public accessor, a start table's row no longer assumes one admitted hash, and the capture keeps every row and column.
- **Section 2.** The claim that `5.2` is decoupled from native-contract 86 is withdrawn, with what each route needs. The multi-rest test states its own bound.
- **Section 3.2.** The measurement the first draft deferred is in, at the finest cut.
- **Not checked.** `edges.npy` across a branch. Native-contract 95's trimmed store, which is not built. Any admission refusal, since the driver admits one fixed packet.
