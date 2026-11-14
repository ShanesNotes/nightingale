# 56: The admission identity leaves the drive frames out

Spec: the admission system in `crates/ng-contract/src/admission.rs`

Type: task

Status: open

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

- [ ] Should the admission identity hold the drive frames, or what they read, so `check` refuses a workspace solved under other frames as `ForeignWorkspace`?
- [ ] If it should: which callers solve on one port and admit on another today, and does any of them rely on that?
- [ ] If it should not: what other check, if any, refuses a workspace whose frames differ only under the tolerance?

**Non-goals:** designing the seal in this ticket. Changing the replay case's fence, which part 4c settled.
