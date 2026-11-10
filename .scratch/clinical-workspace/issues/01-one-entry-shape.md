# 01: One shape for every kind of entry

Type: prototype
Status: open
Blocked by: none

**What to build:** One entry shape that holds every kind of chart entry, proven on two kinds and two views.

Today an entry is a vital and a number. The tables are `entry(patient, vital)` and `entry_version(value double precision, ...)` in `clinical-workspace/src/chart.ts`, and `NewEntry` in `clinical-workspace/src/chart-api.ts` carries `vital` and `value`. A note, an assessment or a medication administration fits none of that. Each would get its own table and its own screen, and the chart would stop being one. One kind is also only a guess at what entries share, and a second kind tests the guess.

The shape keeps what every entry has: its patient, its author, its care time, its charted time, its write id and its chain of versions. Its kind, and the body that kind defines, are data. A vital becomes one kind. The second kind is a note: free text with an author. It is not a number, and it needs no clinical content.

- [ ] Two or three shapes prototyped and one picked, with the reason (pstack `architect`: every later kind builds on this data shape). Each prototype answers where a kind's body is parsed, what the database can still refuse on its own, and how a view asks for one kind
- [ ] The chart takes, amends and returns both kinds through one write and one read model. Every rule the vitals entry has holds for both: the chart only appends, a stale version is refused at the database, a write sent again is charted once, and a care time is never after now
- [ ] A kind the chart does not know is refused at the boundary with a named refusal, and so is a body that does not fit its kind
- [ ] The flowsheet's grid for the recorded run is unchanged, cell for cell
- [ ] A second view lists notes by care time, with the author, the lateness and the earlier versions on request. It is a pure function of the read model
- [ ] Device readings stay device messages, and no reading becomes an entry (SRS rule 1)
- [ ] The `verify-clinical-workspace` walk charts and amends a note in the browser and reads it back from the chart service
- [ ] The pick says what a third kind costs: which files change, and whether the tables do

**Non-goals:** orders, medication administration, assessments and any clinical content; a migration of an old database, which `npm run reset` replaces; who may write which kind.
