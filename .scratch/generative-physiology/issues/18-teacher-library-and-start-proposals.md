# 18: Teacher library, start proposals, and structural-mask pre-check

Type: research
Status: resolved
Blocked by: none

**Question:** What buildable architecture governs the first two engine-facing builds beyond the slice posture experiment (`5.1` Teacher Library and `5.2` Start Proposals with multi-rest detection), and what empirical protocol settles `5.4`'s structural-mask pre-check on `whole_body()` without fabricating unmeasured density numbers?

## Read set and scope

Checked at `origin/main` commit `9a7469ff` (PR `nightingale#242` merge over code commit `ffcd1a7a`, with `crates/`, `docs/`, `data/`, and `tools/` unchanged between `ffcd1a7a` and `9a7469ff`) per [governance section 5](../../../docs/governance.md#5-what-an-architecture-pass-reads):

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

Note on current repo status at `ffcd1a7a`: `tools/generative-physiology/**` fast gate landed in `nightingale#238` (`tools/gate/paths.toml:14-16`), `CertificateFold` on `Session` landed in `nightingale#240`, and oscillation accuracy landed in `nightingale#241`. Code builds under `5.1`, `5.2`, and `5.4` remain parked on [body 28](../../body/issues/28-two-synthetic-days.md) ([the map](../map.md)).

## Answer

### 1. Build `5.1`: Teacher Library Blueprint

#### 1.1 Module Inventory (`tools/generative-physiology/`)

Every file on `ffcd1a7a` keeps one clear owner across the Rust driver (engine lifecycle, capture, reduction, history store) and Python study (specs, scheduling, manifest checking, metrics, audit):

| Path | Role on `ffcd1a7a` | Disposition | Planned change for `5.1` and `5.2` |
|---|---|---|---|
| `driver/Cargo.toml` | Standalone `[workspace]`, path deps on 5 workspace crates | `carries` | Keeps exact 5 crate path dependencies; adds no external dependencies |
| `driver/src/lib.rs` | Public driver module exports | `extends` | Exports `store` (`HistoryStore` policies) and `propose` (`5.2` bridge) |
| `driver/src/main.rs` | CLI (`smoke`, `run`, `interact`, `status`) | `extends` | Adds `--body slice\|whole-body`, `--store-mode`, `admit-only`, `mask-report` |
| `driver/src/npy.rs` | `NpyWriter<E>`, `Table<E>` aligned 128 B NPY 1.0 | `carries` | Unchanged binary table reader/writer |
| `driver/src/smoke.rs` | Slice `SMOKE_SPEC`, `SMOKE_PACKET`, `SMOKE_SEAL` pins | `carries` | Slice smoke stays intact; whole-body smoke pin added after `body 28` |
| `driver/src/spec.rs` | `CAPTURE_VERSION = 2`, `RunSpec`, `PostureEvent`, `CoreChoice`, `Variant` | `extends` | Bumps `CAPTURE_VERSION` to `3`; adds `gp-run 2` (`body`, `packet`, `branch-of`, whole-body `Day` events) |
| `driver/src/archive.rs` | `RunSummary`, `IdentityDigests`, `PartialRun`, `Staged`, `RunState` | `extends` | Adds atomic `.refused/` archival (`spec.txt` + `refusal.txt`), `body`, `store_mode`, `links_kept_bytes`, `witnesses_bytes`, `parent_spec_id`, `branch_at_ns`, `packet_kind`; streaming SHA-256 replaces `fs::read` whole-file hashing (`tools/generative-physiology/driver/src/teacher.rs:863`) |
| `driver/src/capture.rs` | `Catalogue`, `Capture`, `BeatReading`, `edges.npy`, `beats.npy` | `extends` | Parameterizes `Catalogue` across assemblies; adds reduction-projected boundary capture and sparse whole-vector snapshots for long runs ([ticket 14](14-first-experiment-on-the-whole-body.md)) |
| `driver/src/link_file.rs` | Single-file compacted `LinkFile` deleted at run end (`tools/generative-physiology/driver/src/teacher.rs:713`) | `extends` | Replaced or wrapped by `store.rs` for bounded retention and prefix/branch overlay |
| `driver/src/teacher.rs` | `Teacher`, `Feed`, `Original`, `follow`, `courses` | `extends` | Selects `whole_body`/`body_packet`/`body_lens` (`crates/ng-validate/src/lib.rs:411-416`), archives `InitializationFailure` to `.refused/` instead of exiting uncaught (`tools/generative-physiology/driver/src/teacher.rs:741-749`, `tools/generative-physiology/driver/src/main.rs:213-215`), wires `CertificateFold` (`crates/ng-validate/src/certificate.rs:1242`), exports solved starts |
| `driver/tests/{light.rs,acceptance.rs}` | Slice light and acceptance tests | `extends` | Adds whole-body spec grammar, `Day` event, overlay store, and `L1`/`L9` refusal tests |
| `gp.py` | CLI entry (`selftest`, `audit`) | `extends` | Adds `manifest`, `start-pool`, `propose`, `mask-report` subcommands (stdlib-only for library and manifest) |
| `rules.py` | Closed-loop `AuditPressure` rule | `carries` | Keeps slice rule; extends when whole-body closed-loop rules enter |
| `features.py` | Four posture `ResponseFeature` windows | `extends` | Keeps posture features; adds `DayCheck`-aligned meal/drink windows |
| `spec.py` | `Plan` parser, `RunSpec` generator (`audit-v1.txt`) | `extends` | Extends `RunSpec` and `Plan` to `Day` events, `body`, `packet`, and `branch-of` |
| `teacher.py` | Pool scheduler, `_room()` disk check, `Settled` | `extends` | Adds parent-before-branch dependency ordering, bounded-store `_room()` accounting, manifest emission |
| `capture.py` | `RunData`, `Summary`, `Catalogue`, `Edges`, `Limbs`, `Ledger` | `extends` | Keeps v2 reader; adds v3 summary fields, `Day` event tables, and store-mode metadata |
| `audit.py` | Slice technical and informativeness gate | `extends` | Keeps slice audit; adds whole-body audit plan (`ticket 14`) |
| `selftest.py` | Synthetic fixture tests and `--real` driver test | `extends` | Adds `L9`/`L10`/`L12` manifest refusal tests, `Day` grammar tests, overlay store tests |
| `spec/audit-v1.txt` | Slice audit plan | `carries` | Frozen for slice; sibling `spec/audit-body-v1.txt` added after `body 28` |
| `driver/src/store.rs` | Bounded `HistoryStore` (rolling eviction, witnesses, prefix overlay) | `new` | Implements disk-bounded retention and read-only prefix + writable branch overlay |
| `manifest.py` | `L9`/`L10`/`L12` manifest and solved-start table manager | `new` | Builds, checks, and digests `dataset_manifest.json`, `artifact_manifest.json`, and `starts-pool` keyed by `unread_packet_hash` |
| `driver/src/propose.rs` & `propose.py` | `5.2` start-proposal store, `L14` ladder, ingress bridge | `new` | Implements `5.2` proposal store, ladder, `Fork1Route` dispatcher, and multi-rest detector |

#### 1.2 Fork 2: Paired-Branch Execution and Disk-Bounded Capture

Three verified engine mechanisms govern paired branches (such as a fed meal run vs. its counterfactual control) on whole-body multi-hour runs:

1. **Waiting inputs carry across rewind and resume (`docs/architecture/solver-contract.md:322`, `crates/ng-contract/src/admission.rs:233,262`, `crates/ng-contract/src/boundary.rs:900-943`, `crates/ng-record/src/checkpoint.rs:257`):**
   - `Checkpoint` embeds `ImageParts::pending` (`PendingInputs`), which holds every event admitted through `BoundaryPort`, `ParameterPacket::day` (`BoundaryPort::with_day` at `crates/ng-contract/src/admission.rs:233`), or `Session::enqueue` before the checkpoint.
   - On `Session::rewind` (and `Session::resume`), waiting inputs carry into the branch and enter the branch's event log (`docs/architecture/solver-contract.md:322`). Neither `rewind` nor `resume` can delete or withdraw an event already present in `PendingInputs` at the checkpoint instant `t_c`.
   - Furthermore, `ParameterPacket::day` enters `ParameterPacket::hash()` (`crates/ng-contract/src/packet.rs:535-537`), so two `Reviewed` packets whose `Day` timelines differ produce distinct `PacketHash` values and cannot share a checkpoint.
   - **Consequence:** Any shared prefix checkpoint at `t_c` MUST exclude post-`t_c` branching events from both `ParameterPacket::day` and `BoundaryPort`. Only shared events before `t_c` (plus baseline lasting inputs) live on the prefix; post-`t_c` branch-specific events (such as meal vs. omitted meal) enter each branch strictly after `t_c` via `Session::enqueue`, matching the pattern already used by `day_checks` in `crates/ng-validate/src/day.rs:448-470` (checkpoint before enqueueing plan events) and `Feed::AfterCheckpoint` in `tools/generative-physiology/driver/src/teacher.rs:414-432` (withholding events at or past `t_c + longest_step_ns`).

2. **`CertificateFold` on `Session` (`nightingale#240`, `crates/ng-validate/src/certificate.rs:1242-1359`, `crates/ng-record/src/history.rs:1026-1418`):**
   - At admission (`Chain::Opening`), `CertificateFold::open(&session, &policy)` starts with `Prefix::Empty` and zero `HistoryStore` reads (`crates/ng-record/src/history.rs:1064-1070`). As `fold.advance(&session)` runs, `ConservationFeed` pushes `PageMark` entries (one per `PAGE = 64` steps, `crates/ng-record/src/history.rs:1440`) and calls `store.resolve()` via `next_trailing_with_head` (`crates/ng-record/src/history.rs:1359-1417`) only once `windows.reaches(leading)` closes a trailing window (`crates/ng-validate/src/certificate.rs:1213-1236`). Once a window page passes behind the trailing cursor, it is never resolved again.
   - After `Session::resume` or `Session::rewind` (`Chain::Grown`), `ConservationFeed::from_record` calls `build_time_order(history, &backing, store)` (`crates/ng-record/src/history.rs:952-993, 1081`), which walks every link from `t_c` back to admission (`origin`) to populate `PageMark` entries; `take_prefix` (`crates/ng-record/src/history.rs:1218-1268`) then reads trailing segments across `[t_c - window_ns, t_c]` when `t_c >= admission + window_ns` (or when `EdgeWindows` are active).
   - Also, `CertificateFold::certificate()` returns `Coverage` error whenever `span < policy.windows.coverage_ns` (`crates/ng-validate/src/certificate.rs:1326`), so 6-hour teacher runs cannot close a 24 h `CertificatePolicy`. For sub-24 h runs, `gp-driver` verifies step-by-step ledger closure from `Session::conservation().head()` and `edges.npy` directly (or uses a short `CertificatePolicy` window `<= run_length`), reserving 24 h `CertificateFold` for runs spanning at least 24 hours.

3. **`native-contract 95` history trimming (`nc95`, `docs/adr/0008-the-certificate-checks-each-window-as-the-run-goes.md`):**
   - Before `nc95` lands, `LinkFile` writes every compacted link (~65,960 B/step on the 13-system body -> ~46 GB/sim-hour, `.scratch/native-contract/issues/95-the-record-trims-its-history.md:33`), and `ng_record::checkpoint::restore` (`crates/ng-record/src/checkpoint.rs:313` -> `History::walked` at `crates/ng-record/src/history.rs:738-791`) parses every link back to admission. `Session::rewind` additionally requires `parent.holds()` (`crates/ng-record/src/history.rs:717-737`), walking backward from the parent's head.
   - Once `nc95` lands, `HistoryStore` keeps a rolling day (up to 24 h) of complete links back to the checkpoint before the oldest open window (whole-body compressed size unmeasured before `nc95`; ~1.1 TB/day compacted alone at ~194 steps/s, `.scratch/native-contract/issues/95-the-record-trims-its-history.md:33`), plus 96-byte witnesses (~67 MB/sim-hour at ~194 steps/s -> ~402 MB for 6 h) and 1-minute checkpoints with head links (`~111 MB` measured at slice scale for 48 h, `.scratch/native-contract/issues/95-the-record-trims-its-history.md:26`). Trimmed `restore` checks witnesses back to admission without reparsing discarded ledgers.

Two architectural shapes satisfy these constraints:

| Dimension | Shape A: Fresh prefix runs (`ticket 02` pattern) | Shape B: Prefix checkpoint via `Session::resume` + `OverlayStore` |
|---|---|---|
| Branch execution | Each branch admits independently from `t_0` with its own full `RunSpec` | Root prefix runs `t_0..t_c` once, writes `prefix.ckpt` before post-`t_c` events; each branch restores via `Session::resume` and enqueues post-`t_c` events |
| Waiting inputs / `Day` | `Day` or `BoundaryPort` may carry the entire run directly from `t_0` | Prefix `Day` and `BoundaryPort` hold only `<= t_c` shared events; post-`t_c` events stay out of `PendingInputs` until after `prefix.ckpt` |
| `CertificateFold` & `store` | Opens at admission (`Prefix::Empty`); store holds only links inside active fold window (or zero links via `KeepsNothing`-like discard when no checkpoint/24 h fold runs) | Child branch `CertificateFold::open` walks prefix chain from `t_c` back to `t_0`; `OverlayStore` reads `(prefix_store read-only, branch_store read-write)` so `K` branches run in parallel without mutating prefix storage |
| Compute cost (`K` branches, prefix `t_c`) | `K * (T_admit + T_prefix + T_tail)` | `1 * (T_admit + T_prefix) + K * T_tail` (avoids `(K - 1) * (~240 s + t_c / speed)`) |
| Transient disk (`K` parallel workers) | `K * store_bound` (before `nc95` with `KeepsNothing` when uncheckpointed: near `0`; with rolling 6 h links: `K * 276 GB`) | `1 * prefix_store + K * tail_store`; after `nc95` rolling eviction or sub-24 h step closure: prefix witnesses ~67 MB/h + active window |
| Replay & parallelism | Every `SpecId` replays from scratch in one file | Branch `SpecId` names `branch-of <parent_spec_id> at-ms <t_c>` and `realized` log carries prefix + branch events; `K` workers execute concurrently once `prefix.ckpt` lands |

Note why Shape B uses `Session::resume` rather than `Session::rewind` for production branches: `Session::rewind` keeps the parent `Session` and its `CoupledSolver` live in one process and walks `parent.holds()` from the parent's head (`crates/ng-record/src/session.rs:174-181`, `crates/ng-record/src/history.rs:717`), blocking multi-worker parallelism across branches. `Session::rewind` stays covered as an explicit `teacher` audit variant (`A5`).

- **Model default: Shape B** (prefix checkpoint via `Session::resume` with read-only prefix / writable per-branch `OverlayStore` and post-`t_c` event enqueue). For paired branches sharing a multi-hour day-so-far prefix at ~2.5x real time (~1.2 h per 3 h prefix + ~240 s admission), Shape B cuts redundant compute and admission cost by `(K - 1) / K` on the prefix while enabling parallel execution across branches.
- **One-line alternative: Shape A** (fresh prefix runs from `t_0`), preferred only if `body 30` speed rises past ~10x real time and pre-existing `Reviewed` packets carry immutable post-`t_c` differences inside `Day`.

#### 1.3 `L9`/`L10` Dataset Manifest Contract, Run-Spec Extensions, and Solved-Start Table

**Dataset Manifest (`dataset_manifest.json`, sorted-key compact JSON with SHA-256 `manifest_digest`):**

All datasets enforce `L9` (faults and admission refusals archived under `.fault/` and `.refused/` and counted, fixing `tools/generative-physiology/driver/src/teacher.rs:741-749` and `tools/generative-physiology/driver/src/main.rs:213-215` which currently exit uncaught without archiving), `L10` (full provenance digests), and `L12` (teacher runs only on synthetic or reviewed packets, never real-person clinical data). Canonical formatting uses `json.dumps(..., sort_keys=True, separators=(",", ":"))`.

- **Top-level fields:**
  - `schema_version`: `1` (integer)
  - `partition`: `"train"` | `"validation"` | `"heldout"` | `"library"` | `"audit"`
  - `origin_policy`: `"engine_teacher"` | `"authored_scenario_eval"`
  - `source_commit`: 40-char git commit SHA matching every entry's `summary.txt` or `refusal.txt`
  - `crates_tree_digest`: 64-char lowercase hex git tree SHA of `crates/`
  - `model_hash`: 64-char hex (`CompiledBody::model_hash().digest()`)
  - `frozen_digest`: nullable 64-char hex (`FROZEN` tree digest; required non-null for frozen study partitions per `L8`)
  - `counts`: `{ "total_planned": integer, "complete": integer, "fault": integer, "refused": integer }`, requiring `total_planned == complete + fault + refused` (every planned run resolved and counted)
  - `packet_hashes_digest`: 64-char lowercase hex SHA-256 of the sorted unique `packet_hash` hex strings across `entries`
  - `entries`: array sorted lexicographically by `spec_id`, covering every planned run including faults and admission refusals:
    - `spec_id`: 64-char hex `SpecId`
    - `origin`: `"engine_teacher"` | `"authored_scenario_eval"` (`"engine_teacher"` required for training and library partitions per `L12`)
    - `packet_kind`: `"synthetic_fixture"` | `"reviewed_literature_packet"` (`L12`: never real-person clinical data)
    - `packet_hash`: 64-char hex pre-admission `ParameterPacket::hash()` (`unread` digest; post-admission `InitialRevision::packet_hash()` is written to `summary.txt` and `starts-pool` as `admitted_packet_hash`)
    - `resolution_digest`: 64-char hex (`DeclaredResolution::digest()`)
    - `run_identity`: nullable 64-char hex (`RunIdentity::digest()`; `null` if and only if `outcome == "refused"`)
    - `outcome`: `"complete"` | `"fault"` | `"refused"`
    - `detail`: nullable string (`InitializationFailure` or `Fault` detail when `outcome != "complete"`, `null` when `"complete"`)
    - `summary_sha256`: 64-char hex SHA-256 of `<spec_id>/summary.txt`, `<spec_id>.fault/summary.txt`, or `<spec_id>.refused/refusal.txt`
    - `parent_spec_id`: nullable 64-char hex (`null` for root runs; all branches sharing `parent_spec_id` remain in the same `partition`)
  - `manifest_digest`: 64-char lowercase hex SHA-256 of the canonical JSON text with `manifest_digest` omitted

Supplemental run-library metadata (`body_assembly`, `admitted_packet_hash`, `store_mode`, `branch_at_ns`, `first_seal`, `last_seal`, `revisions`, `beats`) lives inside each archived `summary.txt` (or `refusal.txt` for `.refused/`) so `summary_sha256` binds it without mutating `dataset_manifest.json` keys.

**Learned Artifact Manifest (`artifact_manifest.json`, `L10`, sorted-key compact JSON with `artifact_digest`):**
Every trained or fitted artifact (start proposer, emulator, generative physiology model, or observer) pairs its weights with `artifact_manifest.json` carrying:
- `schema_version`: `1` (integer)
- `artifact_kind`: `"start_proposer"` | `"emulator_stage_a"` | `"emulator_stage_b"` | `"gp_model"` | `"observer"` | `"chart_inference"`
- `family`: `"nearest"` | `"continuation"` | `"linear_map"` | `"gp_ard"` | `"neural_emulator"` | `"C1"` | `"C2"` | `"C4a"` | `"R"` | `"instrument"` | `"qualitative"` | `"npe"` | `"fmpe"`
- `claim_label`: `"proposal_only"` | `"validated"` | `"approximate"` | `"untested"`
- `conserving`: boolean (`true` requires non-null `reduction_digest`)
- `continuous_rate`: boolean
- `engine_commit`: 40-char git commit SHA
- `crates_tree_digest`: 64-char hex git tree SHA of `crates/`
- `model_hash`: 64-char hex (`CompiledBody::model_hash().digest()`)
- `dataset_manifest_digest`: 64-char hex `manifest_digest` of the training or library `dataset_manifest.json`
- `packet_hashes_digest`: 64-char hex matching `dataset_manifest.json`
- `reduction_digest`: nullable 64-char hex (required non-null when `conserving` is `true`)
- `frozen_digest`: nullable 64-char hex
- `train_script_sha256`: 64-char hex
- `training_seed`: integer
- `runtime_env_digest`: 64-char hex
- `weights_sha256`: 64-char hex
- `outside_checks`: array of `{ "citation": string, "source_set_tag": string, "claim_mode": string }` (empty allowed only when `claim_label == "proposal_only"`)
- `feature_accounting`: nullable object (`null` unless `artifact_kind == "observer"`)
- `artifact_digest`: 64-char lowercase hex SHA-256 of canonical JSON with `artifact_digest` omitted

`manifest.py` refuses any dataset manifest with missing or duplicate `spec_id`, `counts.total_planned != counts.complete + counts.fault + counts.refused`, any `.fault/` or `.refused/` directory omitted from `entries`, any digest mismatch (`summary_sha256`, `manifest_digest`, `artifact_digest`, `weights_sha256`), `origin != "engine_teacher"` on training or library partitions, or `partition == "heldout"` passed to `fit()` or inference-time lookup. Artifact loading against a compiled body with mismatched `model_hash`, `reduction_digest` (when `conserving`), `frozen_digest` (matching active `FROZEN` whenever `FROZEN` exists or `frozen_digest` is set), or empty `outside_checks` when `claim_label != "proposal_only"` fails closed before inference.

**Whole-body `RunSpec` extensions (`gp-run 2`, `CAPTURE_VERSION = 3`):**
Extends `driver/src/spec.rs` in strict line order (`LINES_IN_ORDER`):
- `gp-run 2`
- `body <slice | whole-body>`
- `packet <preset | sha256-hex>` (selects `body_packet` / `ParameterPacket` builder and seed)
- `core <ng-solver ... | reference ...>`
- `end-ms <u64>`
- `branch-of <parent-spec-id> at-ms <u64>` (optional; Shape B branch)
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
At admission (`crates/ng-contract/src/admission.rs:1695-1696`), `InitialRevision::packet_hash()` folds solved `starts`, `modes`, `fitted` slots, witness, and `MethodHistory` over the pre-admission `unread` `ParameterPacket::hash()`. Because `5.2` proposes starts for *unadmitted* packets that only possess `unread`, `5.1` keys its solved-start table (`starts-pool.jsonl` / `starts/<unread_packet_hash>.json`) by **`(model_hash, unread_packet_hash)`** and stores: `unread_packet_hash`, `admitted_packet_hash`, `anchor_moment_ns`, `cycle_spec`, `parameter_values` (in `CompiledBody::parameters()` canonical order), `solved_starts` (`Vec<f64>` in `rows()` order), `solved_modes`, `solved_fits`, `located_crossings`, `worst_residual`, `admission_s`, and `source_spec_id`.

#### 1.4 Synthetic and Audit Gates for `5.1`

1. **Whole-body teacher audit gate:** Passes both technical (whole-body first-second seal pin, checkpoint-resume, late admission, rewind, observer-off, replica, prefix equivalence, closed-loop replay, `64 * 2^-52` ledger closure, beat-identity check) and informativeness (meal/day-check responses above feature floor) gates once [body 28](../../body/issues/28-two-synthetic-days.md) unparks ([ticket 14](14-first-experiment-on-the-whole-body.md)).
2. **Paired-branch equivalence gate:** A Shape B checkpoint-branched run (`t_0 -> t_c -> branch`) produces bit-identical `seals`, `BranchSummary::end` digests (`RecordDigests`), and `edges.npy` rows to a Shape A fresh run from `t_0` over the same realized event list (`I2`, `I3`).
3. **Waiting-input prefix refusal gate:** Constructing a Shape B shared prefix whose `ParameterPacket::day` or admission `BoundaryPort` includes any event past `t_c` is refused before execution (`SpecError` / `ManifestError`).
4. **Disk-bounded capture gate:** A 6-hour whole-body run (and a `K`-branch paired group) completes within a declared `links_kept_bytes + capture_bytes` ceiling rather than retaining 276 GB of compacted links per run.
5. **Planted-fault and manifest refusal gates (`L9`, `L10`, `L12`):**
   - Planted `.fault/` directory (`MemoryCap` or `EngineFault`) and planted `.refused/` directory (`refusal.txt`) increment `counts.fault` / `counts.refused` in `dataset_manifest.json` and fail manifest checks if omitted from `entries`.
   - `dataset_manifest.json` with `counts.total_planned != counts.complete + counts.fault + counts.refused`, duplicate `spec_id`, or `origin != "engine_teacher"` on a training/library partition is refused before export.
   - `artifact_manifest.json` load with mismatched `model_hash`, `manifest_digest`/`dataset_manifest_digest`, `reduction_digest` (when `conserving`), `frozen_digest` (against active `FROZEN` whenever `FROZEN` exists, or `frozen_digest` when set), or empty `outside_checks` when `claim_label != "proposal_only"` is refused (`L10`).

---

### 2. Build `5.2`: Start Proposals and Multi-Rest Detector Blueprint

#### 2.1 Decoupling from `native-contract 86`'s Three Fork 1 Routes

[Native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md) leaves open three Fork 1 routes by which a start proposal can reach `InitializationSystem::admit` ([ticket 16](16-architecture-foundations.md), section 8 Fork 1):

1. **Route 1 (Packet start entries):** `ParameterPacket::<SyntheticFixture>::start` / `start_mode` / `calibrated` guesses (`crates/ng-contract/src/packet.rs:462-474, 592-648`), sealed into `unread` `ParameterPacket::hash()`.
2. **Route 2 (Public `AdmissionWorkspace`):** Host builds `system = body.initialization(bound, port, request)?` (`crates/ng-contract/src/admission.rs:222`), allocates `ws = system.workspace()` (`crates/ng-contract/src/admission.rs:831`), writes stage slots via `ws.unknowns_mut()` (`crates/ng-contract/src/admission.rs:424`) and guard-crossing shifts via `ws.shift_crossing()` (`crates/ng-contract/src/admission.rs:474`), optionally polishes locally, and calls `system.admit(ws, history)` (`crates/ng-contract/src/admission.rs:1663`).
3. **Route 3 (Private `ng-validate` solve):** `nc86` extends `ng-validate`'s internal `initialize` / `solve_fixture` (`crates/ng-validate/src/initialize.rs:243, 416`) to accept external guesses or continuation paths.

To ensure `5.2` works unchanged regardless of which route `nc86` selects, define a **route-neutral `StartProposal` struct** (`model_hash`, `unread_packet_hash`, `cycle_spec`, `start_state: Vec<f64>`, `start_modes: Vec<usize>`, `fitted_params: Vec<f64>`, optional `stage_unknowns: Option<Vec<f64>>`, optional `crossing_offsets_s: Option<Vec<CrossingHint>>`, plus non-identity proposer metadata `rung`, `source_packet_hashes`, `proposer_digest`) and isolate the engine seam inside one small `Fork1Bridge` dispatcher in `gp-driver`.

#### 2.2 Two Architectural Shapes for Proposal Store and `L14` Ladder

| Dimension | Shape 1: Offline Python `propose.py` + `StartProposal` + thin Rust `Fork1Bridge` | Shape 2: All-Rust `propose.rs` with binary `mmap` `start_pool` |
|---|---|---|
| Store owner | `propose.py` reads `starts-pool` (`5.1` manifest-verified, keyed by `(model_hash, unread_packet_hash)`) | `driver/src/propose.rs` reads packed fixed-width `start_pool.bin` directly in Rust |
| `L14` ladder execution | Python executes Rung 1 (passthrough), Rung 2 (standardized L2 nearest neighbor over `body.parameters()`), and Rung 4 (ridge/linear map fit on training fold only, `L8`); Rust executes `Rung 3` continuation sequence in-process over one compiled `whole_body()` | Rust executes all four rungs (including nearest-neighbor scan, ridge/linear solve, and continuation) inside `gp-driver` |
| Fork 1 decoupling | `propose.py` emits canonical `StartProposal`; only `Fork1Bridge::apply()` in `driver/src/propose.rs` translates to Route 1, 2, or 3 | Same `Fork1Bridge::apply()` isolation, coupled with Rust regression/store code |
| Alignment & dependencies | Honors `ticket 02`'s split (Python fits/selects models; Rust owns engine lifecycle/`admit`); adds zero dependencies or `clippy.toml`-sensitive math to `driver/Cargo.toml` | Duplicates linear-algebra/regression code in Rust under `clippy.toml` bans (no host libm, no hash maps) and mixes model fitting into driver |
| Latency | ~20 ms JSON/NPY handoff (negligible against 55 to 240 s whole-body admission) | Sub-ms lookup (wins only if admission itself drops below ~100 ms) |

- **Model default: Shape 1** (`propose.py` owns the `start_pool` lookup, Rung 2 nearest neighbor, Rung 4 linear map, and `L14` gate evaluation; `driver/src/propose.rs` owns `Fork1Bridge` across Routes 1/2/3 plus the in-process `Rung 3` parameter-continuation loop reusing one compiled `whole_body()`). This keeps model fitting in Python per [ticket 02](02-first-build-architecture.md), avoids linear-algebra duplication in `driver/`, and cleanly insulates `5.2` behind one `Fork1Bridge` function regardless of how `nc86` resolves Fork 1.
- **One-line alternative: Shape 2** (all-Rust `propose.rs`), chosen only if future admission speed falls below ~100 ms per packet where Python IPC dominates.

**How the `L14` ladder promotes rungs (`5.2`):**
1. **Rung 1 (Packet guess):** `body_start_guess` / packet's own `starts`. Baseline median admission wall time `T_1` and admission failure rate `F_1`.
2. **Rung 2 (Nearest admitted neighbor):** Select `P_nearest` minimizing standardized parameter distance `sum_k ((x_new[k] - x_i[k]) / width[k])^2` in `start_pool` (`model_hash` equal), propose its `(start_state, start_modes, fitted_params)` via `StartProposal`.
3. **Rung 3 (Continuation):** Invoked when Rung 2 fails to admit (or exceeds target wall/iteration cap). Interpolates `P(alpha) = (1 - alpha) P_nearest + alpha P_new` in-process from `alpha = 0` to `1` (adaptive step halving on `InitializationFailure`), advancing each admitted `workspace` as the guess for the next `alpha`.
4. **Rung 4 (Linear map):** Fits regularized linear map `W: x_param -> (start_state, fitted_params)` on the `train` partition of `starts-pool` (`L8`: `val`/`heldout` strictly excluded). Promoted over Rungs 1 to 3 ONLY if it improves held-out `val` median admission time or admission success rate by a pre-registered margin (`L14`).

#### 2.3 Exact Multi-Rest Detection Test

To test whether a packet admits to one unique rest or multiple distinct rests ([ticket 16](16-architecture-foundations.md), section 5.2; [native-contract 86](../../native-contract/issues/86-admission-reaches-a-moved-rest.md) box 4):

1. **Independent guesses:** For each test packet `P`, construct `M >= 3` independent start guesses `{G_0, ..., G_{M-1}}`: `G_0` = Rung 1 packet guess; `G_1` = Rung 2/4 learned proposal; `G_2..G_{M-1}` = deterministic perturbations of initial `starts` and `calibrated` guesses inside `Row::admissible()` / `ParameterSlot::admissible()` bounds.
2. **Engine judgment:** Pass every `(P, G_j)` through `Fork1Bridge` -> `InitializationSystem::admit` with identical `BoundaryPort`, `ObservationLens`, `anchor_moment`, and `MethodHistory`. Log all refused guesses (`InitializationFailure`, `L9`); keep all admitted `InitialRevision_j` results.
3. **Pairwise tolerance comparison:** For every admitted pair `(a, b)`:
   - Compare discrete guard settings: `modes_a == modes_b`. Any mismatch is a **branch-distinct second rest**.
   - Compare every continuous start row `r in 0..body.rows().len()` and every fitted parameter `f in 0..fits.len()` against the packet's declared admission `ErrorScales` (`packet.within`, defaulting to `admission_tolerance()` at `1e-9` abs / `1e-12` rel in `crates/ng-validate/src/initialize.rs:17-20`, with `row_scale` `size[r]` and fit size `1.0` per `crates/ng-contract/src/admission.rs:799`):
     `d_r = |x_a[r] - x_b[r]| / (abs_tol * size[r] + rel_tol * max(|x_a[r]|, |x_b[r]|))`
   - Compare `LocatedCrossing::at()` times (`crates/ng-contract/src/admission.rs:550-561`) within the admission `Cycle` step floor.
4. **Result and dynamical follow-up:**
   - If `modes_a == modes_b` and `max(d_r, d_f) <= 1.0` for all pairs: report **single rest** (`max_d`).
   - If `modes_a != modes_b` or `max(d_r, d_f) > 1.0`: report **second rest detected**, logging `(unread_packet_hash, admitted_hash_a, admitted_hash_b, max_d, offending_rows_or_modes)` and optionally stepping both `Session`s forward under identical empty post-fence inputs to check whether the two rests stay distinct or relax onto one orbit.

#### 2.4 Exact Synthetic Refusal Tests (`L1`, `L9`, `L14`)

1. **`L1` refusal (No unverified start-vector entry):** Construct a synthetic `StartProposal` whose coordinates satisfy every `AdmissibleSet` bound (`positive`/`finite`) yet violate a coupled algebraic/periodicity residual (perturb one pressure or pool row +10% off equilibrium). Submit through `Fork1Bridge` -> `InitializationSystem::admit`.
   - *Must fail with:* `InitializationFailure::Inconsistent { row }` (or `GuardSweep`/`NetBoundaryFlow`); `Session::admit` is never called; `gp-driver` contains zero bypass calls to `PatientRecord::restored` outside `Session::resume`/`rewind`/`recover`.
2. **`L9` refusal (Refused admissions archived and counted, never dropped):** Run a batch of synthetic packets containing at least one deliberately un-admissible packet (e.g., parameter causing `NetBoundaryFlow` or `Inconsistent`), checking that `gp-driver` archives `<spec_id>.refused/refusal.txt` instead of exiting uncaught (`tools/generative-physiology/driver/src/teacher.rs:741-749`, `tools/generative-physiology/driver/src/main.rs:213-215`).
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

#### 3.2 One-Pass Empirical Measurement Protocol (at [ticket 14](14-first-experiment-on-the-whole-body.md) Unparking)

When [body 28](../../body/issues/28-two-synthetic-days.md) resolves and [ticket 14](14-first-experiment-on-the-whole-body.md) unparks, execute ONE read-only inspection pass over `whole_body()` (requiring only `CompiledBody` construction, zero admission solve or time-stepping) via a new driver command:

```text
gp-driver mask-report --body whole-body --cuts <coarse.txt>,<tissue.txt>,<richer.txt> --out <report.json>
```

**Algorithm executed in one deterministic pass (`driver/src/mask_report.rs` or subcommand):**

1. **Row-to-state transitive closure over `body.blocks()`:**
   - Number all `1,465` rows (`Amount`, `Memory`, `Algebraic`) and identify the `318` held pools from `body.conserved_basis().coverage().pools()`.
   - Walk `body.blocks()` in topological dependency order (`crates/ng-contract/src/body.rs:130`). For each `Algebraic` row, accumulate the transitive union of direct `(Amount | Memory)` `UnknownId`s read via `body.sparsity()`, plus upstream `Algebraic` closures. For every `Amount` and `Memory` row `r`, store its RHS dependency set `state_deps[r]` split into `(active_amount_reads, held_pool_reads, memory_reads)`.
2. **Project onto each candidate reduction cut `(P, S, B, A)`:**
   - Compute **(a) Total-to-total matrix `D_total`** (`M_totals x (M_totals + M_mem)`): `D_total[m, j] = 1` iff any non-held `Amount` row in total `T_m` has `(active_amount_or_memory)` `j` in `state_deps[r]`. Report `nnz(D_total)`, global density, per-species-group density (vascular/fluid, electrolytes, gases, metabolism/fuels, heat, transmitters), and per-total in-degree `(min, median, p95, max)`.
   - Compute **(b) Transfer-level outer-bound mask `D_transfer`** (`N_transfers x (M_totals + M_mem)`): for each derived transfer `t` linking totals `(from, to)`, compute the row-union bound across rows touched by `t`'s `Column::terms()` AND count **ambiguous dependency entries** where `t`'s incident `Amount` row is shared with other distinct transfer families (quantifying exact over-approximation caused by private `Instance::inputs`).
   - Compute **(c) Held-pool read audit** (`held_pool_reads` count per total) and **(d) SCC block composition** across all `415` blocks (size distribution and species breakdown of the `669`-row largest block).

**Decision rules settling Forks 3 and 4 from the measured numbers:**

- **Fork 3 (Where the cut sits):** Evaluate `{coarse, tissue-grouped, R (each-row)}` cuts against `D_transfer` parameter counts under [ticket 08](08-model-forms.md)'s `100,000`-parameter cap and [ticket 07](07-sufficiency-screen.md)'s sufficiency screen; pick the coarsest cut that passes sufficiency inside the parameter cap.
- **Fork 4 (Dense vs. masked transfers):**
  - If measured `D_transfer` density is `>= 0.70` across active totals at the chosen cut, keep dense transfer inputs (masking buys negligible pruning) OR, if ambiguous shared-row entries account for the high density while chemistry/renal/heat blocks remain uncoupled, file a single scoped question to the engine process requesting public `Instance::inputs(&self)` on `CompiledBody` (`crates/ng-contract/src/body.rs:599`) before re-measuring.
  - If measured `D_transfer` density is `< 0.70`, run `5.4`'s Step 2 head-to-head arm: train masked `C4a` vs. dense `C4a` under the same frozen study (`L8`, `L14`), keeping the structural mask only if it matches or outperforms dense `C4a` out-of-policy at equal or lower parameter count.

## Comments

- Verified all Rust paths, symbol names, counts, and behaviors directly on `origin/main` (`9a7469ff`, with code tree unchanged from `ffcd1a7a`).
- Key engine interactions verified in code: `ParameterPacket::day` merges into `BoundaryPort` at `crates/ng-contract/src/admission.rs:233`, sealing future `Day` events into `PendingInputs` before any post-admission checkpoint (`docs/architecture/solver-contract.md:322`); `CertificateFold::open` on `Session::resume` walks links back to admission via `build_time_order` (`crates/ng-record/src/history.rs:952, 1081`); `InitialRevision::packet_hash()` seals solved `starts`, `modes`, and `fitted` values over the pre-admission `unread` `ParameterPacket::hash()` (`crates/ng-contract/src/admission.rs:1695-1696`), requiring `5.1`'s solved-start table to key on `unread_packet_hash` for `5.2`.
