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

## Comments

From 03: sketch probe this ticket owns: line 471 (seed domains never mix, E0308).

From 04: `Seed` is a plain struct, and this ticket brings `DomainSeed<D>` with the domains. `AcceptedStateView::last_segment` and `DenseSegment::sample` are what a lens reads. A segment is the degree 1 chord between the accepted ends, and the admission segment has no interval. `sample` returns `Compensated`, so a channel's value needs a rule for its one rounding.
