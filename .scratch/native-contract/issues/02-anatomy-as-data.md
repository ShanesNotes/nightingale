# 02: Anatomy as data, with its graph checks

Spec: [First build slice](../../../docs/architecture/solver-contract.md#first-build-slice-spec)

**What to build:** An author declares sites of the six site classes with positions, containment, vascular edges (beds in series and a loop), an enclosure with its compliance, a served-by edge and a region, and gets a checked `Anatomy` and a `Schema` whose pool registry is derived from it.

Blocked by: 01

Status: ready-for-agent

- [ ] The manufactured anatomy (two capillary beds in series on a vascular loop, one enclosure, one junction, one nerve segment) builds and hashes
- [ ] `finish` refuses, each with a named reason: an unreachable bed, an undrained bed, a site enclosed twice, a site with no position, a containment cycle, an orphan site
- [ ] A region's derived member list is the containment subtree; adding a site changes the model hash
- [ ] Every anatomy type and builder method in the sketch's signature walk compiles against the crate
