# 01: One shape for every kind of entry

Type: prototype
Status: resolved
Blocked by: none

**What to build:** One entry shape that holds every kind of chart entry, proven on two kinds and two views.

Today an entry is a vital and a number. The tables are `entry(patient, vital)` and `entry_version(value double precision, ...)` in `clinical-workspace/src/chart.ts`, and `NewEntry` in `clinical-workspace/src/chart-api.ts` carries `vital` and `value`. A note, an assessment or a medication administration fits none of that. Each would get its own table and its own screen, and the chart would stop being one. One kind is also only a guess at what entries share, and a second kind tests the guess.

The shape keeps what every entry has: its patient, its author, its care time, its charted time, its write id and its chain of versions. Its kind, and the body that kind defines, are data. A vital becomes one kind. The second kind is a note: free text with an author. It is not a number, and it needs no clinical content.

- [x] Two or three shapes prototyped and one picked, with the reason (pstack `architect`: every later kind builds on this data shape). Each prototype answers where a kind's body is parsed, what the database can still refuse on its own, and how a view asks for one kind
- [x] The chart takes, amends and returns both kinds through one write and one read model. Every rule the vitals entry has holds for both: the chart only appends, a stale version is refused at the database, a write sent again is charted once, and a care time is never after now
- [x] A kind the chart does not know is refused at the boundary with a named refusal, and so is a body that does not fit its kind
- [x] The flowsheet's grid for the recorded run is unchanged, cell for cell
- [x] A second view lists notes by care time, with the author, the lateness and the earlier versions on request. It is a pure function of the read model
- [x] Device readings stay device messages, and no reading becomes an entry (SRS rule 1)
- [x] The `verify-clinical-workspace` walk charts and amends a note in the browser and reads it back from the chart service
- [x] The pick says what a third kind costs: which files change, and whether the tables do

**Non-goals:** orders, medication administration, assessments and any clinical content; a migration of an old database, which `npm run reset` replaces; who may write which kind.

## Answer

**The pick: a shared spine and a content table per kind.** Three shapes were prototyped. A kept each version's content as jsonb, checked by a PL/pgSQL interpreter of each kind's field checks. C kept it as text in a grammar per kind. B, the pick, keeps the spine every entry shares (`entry` and `entry_version`: patient, author, care time, charted time, write id and the chain of versions) and one typed table per kind, `content_<kind>`, with a column per field.

- **Where content is parsed.** At the chart API's boundary, by the catalog in `clinical-workspace/src/entry-kinds.ts`. Each field gives a reason a clinician can act on when it does not fit, and the refusal carries it to the screen. The same catalog writes the table's columns and checks and draws the forms.
- **What Postgres refuses on its own.** A kind it does not know, a version with no content or with another kind's content, a skipped version, a care time after the charted time, an author off the roster, a value of the wrong type or outside its check, a version that changes a fixed field (which vital a vital entry is about), and any update, delete or truncate of chart rows.
- **How a view asks for one kind.** `entriesOfKind(model, 'note')` returns typed notes from the one read model. The flowsheet asks for vitals and the notes view for notes, and neither knows the tables.
- **What a third kind costs.** Its catalog entry in `src/entry-kinds.ts` and its three samples in `test/chart.test.ts`, which the compiler demands, so every chart rule runs on it. A screen for it adds a draft in `src/workspace/program.ts` and a section in `src/workspace/view.ts`. The tables change: it adds `content_<kind>` and widens the check on `entry.kind`, so a developer's database needs `npm run reset`. The chart refuses to open one that lacks a known kind's content table and says so.
- **Why not jsonb or text.** Both keep a kind's checks twice, once in TypeScript and once in SQL, and both let SQL take a row the read then refuses: A's jsonb number check takes 1e400, and C's grammar checks spelling, not meaning. Typed columns make Postgres and the parser agree by construction.
- **What the shape leaves open.** Content is flat named fields. A kind with nested or repeated parts needs more relational work, faced by the ticket that brings it.

