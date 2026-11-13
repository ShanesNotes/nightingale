# 06: Finish the fluids retag

Type: task
Status: resolved
Claimed by: claude
Blocked by: none

**What to do:** The developer reviewer adds about $6 of TypeSafe credit, above what agentctl needs. The fluids retag from [ticket 04](04-library-fixes.md) stopped on 2026-11-09 at 199,971 of 360,430 chunks, when the organization's credits ran out (HTTP 402). The other 160,459 chunks take about 141 million input tokens. The same credits run agentctl's classifier, which stays blocked until the top-up.

A watcher probes the credits every 5 minutes until 2026-11-16. When credits return, it resumes the retag once, because the replay log asks nothing twice, and then rebuilds the layer-zero table. If the retag stops again, the watcher exits rather than drain a second top-up. Its log is `~/lanes/logs/ng-evidence-library/ticket-04/resume/watch.log`.

Done when that log reads "retag and table done" and the resumed run reports no errors. The tilt bars do not read the fluids facet, so they stand as ticket 04 scored them.

## Answer

Done on 2026-11-09. The credits returned at 03:55 UTC and the watcher resumed the retag once. It finished at 05:02 UTC with no errors: 158,711 new requests, 139.8 million input tokens, $5.87. Every chunk the retag asked about, 360,430 in all, now has a fluids answer on the new wording.

The layer-zero table carries the new answer on 355,800 of its 356,910 chunks. None of the other 1,110 lacks an answer:

- 1,105 chunks share their title, section and text with another chunk. The replay log answered each pair once, under the other chunk's tag, and the table step matches answers by tag, so these keep the old answer.
- 5 are rows for chunks that ticket 04's split replaced.

[Ticket 05](05-two-step-hub-and-work-families.md) fixes both before its rerun. The audit after the retag finds no leaks and puts the slice's Jev cost at $57.29, inside the $60 bar. Credits remained after the run, so agentctl's classifier still works.
