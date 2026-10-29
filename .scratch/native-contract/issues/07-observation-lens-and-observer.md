# 07: Observation lens and observer, apart from physiology

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** A chart author compiles a lens (channels and an output grid) against the body, reads the last accepted revision through `Session::observation`, and runs an observer that applies windows, delays and seeded measurement error, one revision at a time.

Blocked by: 05

Status: ready-for-agent

- [ ] The lens digest is not in the run identity; applying a new lens to a finished replay gives identical physiology digests and new findings
- [ ] Physiology digests are identical with observers attached, detached and changed (the non-perturbation gate)
- [ ] An observer refuses a skipped or repeated revision; each grid instant appears in exactly one step
- [ ] An observer checkpoint restores its findings after a resume
- [ ] Measurement error draws from the observation namespace: adding a channel leaves physiology draws unchanged
