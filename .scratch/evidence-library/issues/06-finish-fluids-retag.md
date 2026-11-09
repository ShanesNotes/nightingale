# 06: Finish the fluids retag

Type: task
Status: ready-for-human
Blocked by: none

**What to do:** Shane adds about $6 of TypeSafe credit, above what agentctl needs. The fluids retag from [ticket 04](04-library-fixes.md) stopped on 2026-11-09 at 199,971 of 360,430 chunks, when the organization's credits ran out (HTTP 402). The other 160,459 chunks take about 141 million input tokens. The same credits run agentctl's classifier, which stays blocked until the top-up.

A watcher probes the credits every 5 minutes until 2026-11-16. When credits return, it resumes the retag once, because the replay log asks nothing twice, and then rebuilds the layer-zero table. If the retag stops again, the watcher exits rather than drain a second top-up. Its log is `~/lanes/logs/ng-evidence-library/ticket-04/resume/watch.log`.

Done when that log reads "retag and table done" and the resumed run reports no errors. The tilt bars do not read the fluids facet, so they stand as ticket 04 scored them.
