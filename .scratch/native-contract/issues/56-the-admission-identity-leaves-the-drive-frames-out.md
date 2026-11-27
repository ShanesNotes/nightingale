# 56: The admission identity leaves the drive frames out

Spec: the admission system in `crates/ng-contract/src/admission.rs`

Type: task

Status: resolved

Blocked by: none

**Why.** The first milestone (SRS invariant I19) needs the whole body judged by gates that cannot pass by luck. Body 06's replay gates admit every start from one admission solve made on a port with no input. Only a fence that the replay case reads from that solve stops an input inside the cycle from reaching a run started from a solve that read none. One layer down, the admission system could refuse that workspace itself.

**Where it comes from.** The independent check of nightingale#164 (body 06, part 4c), its section 10.2 and its defect 7. The admission system's identity seals the model, the packet, the anchor moment and the cycle, and leaves the drive frames out (`admission.rs`, where `initialization` builds `identity`). So `InitializationSystem::check` passes a workspace solved under other frames, and only the packet's tolerance judges it at the recheck. The identity is compared in `check` alone, so sealing more into it would move no pin.

What the check ran on the slice, with one input inside the one-second cycle at the anchor plus 0.5 s, comparing a start from the shared no-input solve (`admit_from`) with `admit_at`'s own solve:

| Input inside the cycle | Shared solve, `admit_from` | `admit_at` |
|---|---|---|
| The case's stand and 400 bolus | refused, `Inconsistent { row 8 }` | refused, `Inconsistent { row 1 }` |
| A lying posture, which changes no frame | admitted, equal | admitted |
| A posture of trunk 1e-12 | admitted with the no-input state and identity | admitted with another state and identity |
| A posture of trunk 1e-6 | refused, row 8 | admitted |
| A bolus of 1e-9 | admitted with the no-input state and identity | admitted with another state and identity |
| A bolus of 1e-3 | refused, row 140 | admitted |

A posture of 1e-12 and a bolus of 1e-9 pass the recheck on a workspace solved without them. Since part 4c's fix round the replay case refuses any input before the fence it reads from its solve (`admit_from`, `EventBeforeFence`), so no caller in the repo reaches this today. The admission system itself still takes such a workspace from any caller that builds one.

- [x] Should the admission identity hold the drive frames, or what they read, so `check` refuses a workspace solved under other frames as `ForeignWorkspace`?
- [x] If it should: which callers solve on one port and admit on another today, and does any of them rely on that?
- [x] If it should not: what other check, if any, refuses a workspace whose frames differ only under the tolerance?

**Non-goals:** designing the seal in this ticket. Changing the replay case's fence, which part 4c settled.

## Answer

### 1. Should the admission identity hold the drive frames?

**Yes: `InitializationSystem::identity` should seal `self.frames`, the `Vec<DriveFrame>` over the admission grid `[anchor_moment, fence)` (`crates/ng-contract/src/admission.rs:242-265, 274-279`).** It should seal `self.frames`, not the raw `BoundaryPort::digest()` (`crates/ng-contract/src/boundary.rs:407-412`).

**Why `self.frames` is the exact quantity to seal (read from `9a7469ff`):**

- `CompiledBody::initialization` (`admission.rs:242-265`) computes `instants` as `[start]` (`fence = start + 1 ns`, instant admission) or `[start, start + step_ns, ..., start + (intervals - 1) * step_ns]` (`fence = start + period_ns`, periodic admission), then builds `frames: Vec<DriveFrame>` via `pending.frame_at(SimTime::from_ns(*ns), self.lasting_reads())`. Every instant in `instants` is strictly before `fence`.
- `DriveFrame` (`boundary.rs:1124-1130, 1181-1202`) already implements `Seal` and already reflects what admission reads:
  - `drives`: every delivery in `boundary.deliveries` has already passed `CompiledBody::check_delivery` (`admission.rs:93-118`), which refuses unknown boundaries and any boundary no law reads (`DriveNotRead`).
  - `lasting`: `DriveFrame::set` (`boundary.rs:1156-1160`) stores only lasting inputs in `self.lasting_reads()` (extending `lasting_reads()` to lasting inputs read only by guards is tracked separately by [native-contract 96](96-an-input-only-a-guard-reads-is-owed-at-the-anchor-moment.md)).
  - `trunk` and `legs`: set by `Kind::Posture` (`boundary.rs:1141`), defaulting to `(0.0, 0.0)` when no posture is stated (`DriveFrame::default()`, tracked by [native-contract 98](98-posture-is-owed-at-the-anchor-moment.md)).
- **Refuses every sub-tolerance frame perturbation inside `[anchor_moment, fence)`:** Both `trunk = 1e-12` and `bolus = 1e-9` from the `nightingale#164` defect-7 table change `self.frames` bits, so `InitializationSystem::check` (`admission.rs:614-623`) refuses the foreign workspace immediately as `AdmissionError::ForeignWorkspace` before `recheck` runs, for every caller including callers that bypass `admit_from`.
- **Preserves shared solves for events at or after `fence`:** `PendingInputs::frame_at` (`boundary.rs:727-735`) includes a delivery only when `delivery.at <= at && at < delivery.ends`. Any event with `at >= fence` never enters `self.frames` and leaves `identity` byte-identical. Sealing raw `BoundaryPort::digest()` instead would hash events at `>= fence` too and break `admit_from`'s shared solve.
- **Moves zero pins:** `InitializationSystem.identity` is stored only in `InitializationSystem.identity` and `AdmissionWorkspace.system`, and compared only in `InitializationSystem::check` (`admission.rs:615`). It never enters `ModelHash`, `PacketHash`, `InitialRevision`, `RunIdentity`, or any pinned seal.

**Edge cases for the follow-up implementation ticket (read from `boundary.rs`):**

- *Lying posture (`trunk = 0.0, Legs::Horizontal`) or unread lasting input inside `[anchor_moment, fence)`:* Neither changes `self.frames`, so `identity` stays equal (matching row 2 of the table above: *"A lying posture, which changes no frame | admitted, equal"*). The admission equations over `[anchor_moment, fence)` are genuinely identical (same `start`, `fitted`, `witness`, and `PacketHash`), while `InitialRevision.parts.pending` keeps the event for the run; `admit_from`'s `EventBeforeFence` check still rejects pre-fence events on the shared replay path.
- *Zero-amount single-species delivery (`Delivered` with `amount == 0.0`):* Unlike `Meal` (`boundary.rs:500`), `Delivery::checked` (`boundary.rs:591-619`) permits `amount == 0.0`, and `DriveFrame::add` (`boundary.rs:1168-1173`) inserts `(boundary, 0.0)` into `frame.drives`, which seals differently from an empty `frame.drives` even though `frame.rate(boundary)` returns `0.0`. Sealing `self.frames` fails closed (`ForeignWorkspace`) on that case.
- *Posture on a body that reads no height:* `DriveFrame::take` (`boundary.rs:1141`) writes `trunk` and `legs` regardless of whether any law or guard reads a height, so a nonzero posture inside `[anchor_moment, fence)` fails closed (`ForeignWorkspace`), matching `StepTicket`'s own `DriveFrame` seal.
- *Error doc string:* `AdmissionError::ForeignWorkspace` (`crates/ng-contract/src/failure.rs:241`) currently reads *"A workspace from another compiled body, or of another shape."* The follow-up implementation PR should update that doc to cover another admission system's packet, anchor moment, cycle, or drive frames.

### 2. Caller audit: who solves on one port and admits on another today

Every call site of `CompiledBody::initialization`, `InitializationSystem::admit`, `solve_fixture`, `solve_admission`, `admit_from`, `admit_at`, `admit_solved`, `AssemblyCase`, `SliceCase`, and `BodyCase` across `crates/` and `tools/` was audited on `9a7469ff`.

**Exactly one caller in the repository solves on one `BoundaryPort` and admits on another:**

- **`ng_validate::initialize::admit_from` (`crates/ng-validate/src/initialize.rs:356-379`) paired with `solve_admission` (`initialize.rs:338-354`).**
  - *Direct callers:* `AssemblyCase<A>::start` (`crates/ng-validate/src/slice.rs:1229-1259`) and its unit test `a_start_from_the_shared_solve_refuses_an_input_a_nanosecond_before_its_fence_and_takes_one_at_it` (`slice.rs:1313-1341`).
  - *Transitive users of `AssemblyCase<A>`:*
    - `SliceCase` (`crates/ng-validate/src/slice.rs:1211`), used in `crates/ng-validate/src/arena.rs:173`, `crates/ng-validate/tests/arena_body.rs:1119,1179`, and `crates/ng-validate/tests/window.rs:254`.
    - `BodyCase` (`crates/ng-validate/src/body/mod.rs:36`), used in `crates/ng-validate/tests/body_run.rs:3399` (the whole-body replay gates).
    - `AssemblyCase<TwoSecondCycle>` in `crates/ng-validate/tests/assembly_fence.rs:53`.
  - *Does it rely on cross-port admission?* **Yes for events at or after `fence`, and no for events before `fence`.** `solve_admission` solves on `BoundaryPort::new(anchored(origin))` (empty for `Slice` and `TwoSecondCycle`; `[body_awake(origin), body_room(origin)]` for `WholeBody`) and records `fence = system.fence()`. `admit_from` (`initialize.rs:371-376`) rejects any event with `event.at() < *fence` (`AdmissionError::EventBeforeFence`), then builds the admitting port from `anchored(origin)` plus `events` (all stamped `>= fence`, or empty for `admit_withheld`). Because every added event has `at >= fence`, `admit_from`'s admitting system produces **bit-identical `self.frames`** over `[anchor_moment, fence)`. Sealing `self.frames` into `identity` therefore breaks **zero** callers while preserving the shared solve (avoiding repeated ~186 s whole-body admissions across the replay gates).

**All other callers solve and admit on the same `InitializationSystem` (same `BoundaryPort`), or intentionally test `ForeignWorkspace` across bodies, packets, or shapes:**

- `ng_validate::initialize::{admit_at, admit_solved}` (`crates/ng-validate/src/initialize.rs:393-414`): solves via `solve_fixture` and admits on the returned `system`.
- `ng_validate` fixtures and tests (`crates/ng-validate/src/fixture.rs:878-882`, `tests/replay.rs:292-296`, `tests/split.rs:601-606`, `tests/body_run.rs:3860`, `tests/admission.rs`, `tests/admission_radau.rs`, `tests/firing.rs`, `tests/transaction.rs`, `tests/arena_body.rs`, and other integration tests): each constructs one `system` and admits its own workspace, except the explicit `ForeignWorkspace` refusal tests (`tests/admission.rs:2172-2181`, `tests/admission_crossings.rs:613-649`, `tests/transaction.rs:1355-1402`).
- `crates/ng-contract` (`admission.rs:1992-1996`, `quadrature.rs:814-822, 1106-1114`, `ring.rs:624`, and `tests/*.rs`), `crates/ng-record/tests/session.rs:184-191`, and `crates/ng-solver/tests/core.rs:210-214`: all same-system.
- `tools/bench/src/main.rs:64,385,486`, `tools/bench/src/link_log.rs:221-224`, and `tools/generative-physiology/driver/src/teacher.rs`: `newton_update_us` does not admit; `cost`, `main`, and `teacher` call `admit_at` on one system, and `link_log.rs` admits `TwoReservoirs` via `fixture.rs:878-882`.

### 3. Fallback check if the identity did not hold the drive frames

**Not applicable because Box 1 is yes.** No tolerance-based recheck can close this hole: `InitializationSystem::recheck` (`admission.rs:1669`) checks residuals only against `self.tolerance` (`ErrorScales`), so any frame perturbation whose residual falls below `tolerance` (such as `trunk = 1e-12` or `bolus = 1e-9` above) passes `recheck` while producing the wrong admitted start state and `PacketHash`. Carrying `frames` (or their digest) separately on `AdmissionWorkspace` and comparing them in `check` is functionally equivalent to sealing `self.frames` into `InitializationSystem::identity` with extra fields. Implementing the seal change itself (one `frames.seal(&mut identity)` line in `admission.rs:274-279`, the `ForeignWorkspace` doc update, regression tests for `trunk = 1e-12` and `bolus = 1e-9`, and a mutant row) belongs in a follow-up code ticket per this ticket's non-goals.
