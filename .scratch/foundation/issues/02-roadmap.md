# 02: Write the roadmap from SRS section 7

Type: task
Status: needs-triage
Blocked by: none

The approved SRS moved its section 7 out at import (ruling R5) into its own roadmap doc: a sequenced build plan holding the build order, the stage 1 milestone, its non-goals, D3 and D5. That doc is not written yet. The section is kept here word for word, so no approved text is lost and no agent has to read a brief for it.

Also settle when the roadmap is written:

- C11: does the SRS section 5 bullet "Pieces needed before the solver arena runs" move into the roadmap? It is build sequencing inside the structural model, and the speed gate lacks the slowest steps' wall time.
- The build order names "the agent", which is not an SRS term. Name it.
- The monitor stays an SRS requirement (section 6); the roadmap places it after the architecture.

## SRS section 7, word for word

Build order: architecture first (the solver, the contracts and the agent), display after. Until display work starts, nightingale-pulse's existing monitor covers display.

Milestone 1 ([pulse 0006](https://github.com/ShanesNotes/nightingale-pulse/blob/bdc01c567f6cadacbd96728c0600a2e052882a59/docs/decisions/0006-first-milestone-stable-healthy-body.md)): a healthy ICRP 89 reference male runs stable with all 13 systems at their simplest working version, conserving mass and volume, over a normal day (meals, drinks, thermoneutral sea-level air, seated awake, supine asleep, 24 h clock). Subject, intake, environment, posture and clock are data. The start state is solved once from anchors at 06:55. Stable means near-zero net change in every conserved quantity over sliding 24 h windows across at least two simulated days, judged on the ledger, not on vitals. The same trajectory at any pace.

Non-goals for milestone 1:

- No disease, hazard, intervention or debrief behavior. Only what they will attach to: an event type, a seed slot and the event log, checkpoint and branch, the patient as a packet, and a read-only view of state.
- No bedside scenarios as the definition of done. Done is the invariant list in section 4.
- No display work: display comes after the architecture (build order above). The chart is the first planned form of visibility (assessment, vitals, medication administration, labs, progress notes), not built now.
- No fine map detail beyond what the milestone needs.
- No build target other than native Rust.
- No demise behavior. No scripted outcomes. No arbitrary odds.

| # | Decision | Model default | Who decides |
|---|---|---|---|
| D3 | Fast-forward identity | Keep the same-trajectory rule strict; any reduced-resolution mode is a declared separate run | Developer reviewer |
| D5 | Map detail at milestone 1 | Coarse territories with a refinement rule | Models, revisited in build |
