# 02: Chart the foundation map from SRS section 7

Type: task
Status: ready-for-agent
Blocked by: none

The approved SRS moved its section 7 out at import (ruling R5) for a roadmap doc, but governance keeps out roadmaps that restate tickets. The section becomes this effort's wayfinder map instead: `.scratch/foundation/map.md`, written by `/wayfinder` in the layout `docs/agents/issue-tracker.md` gives. The map indexes each decision and links where it lives; it never restates a ticket or a record. Sort the section into it:

- **Destination:** invariant I19 passing on the reference man ([record 0004](../../../docs/adr/0004-first-milestone.md)). It replaces the section's milestone paragraph, which judged stability on the ledger, not on vitals.
- **Notes:** the build order, display after the architecture, with nightingale-pulse's existing monitor covering display until then; and the wayfinder override that carries execution into the map, since this effort builds as well as decides.
- **Decisions so far:** D5 links to [record 0002](../../../docs/adr/0002-map-starts-grouped.md), and D3 to SRS rule 4.
- **Out of scope:** the non-goals for milestone 1.
- **Not yet specified:** C11, whether the SRS section 5 bullet "Pieces needed before the solver arena runs" moves out of the SRS (it is build sequencing inside the structural model, and the speed gate lacks the slowest steps' wall time); a name for "the agent" in the build order, which is not an SRS term; and where the monitor goes, which stays an SRS requirement (section 6).

Writing the map resolves this ticket: append an `## Answer` that links the map, and add this ticket's line to the map's Decisions so far. The section stays below word for word, so no approved text is lost. Triage then moves the foundation tickets from the triage labels onto the wayfinder statuses.

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
