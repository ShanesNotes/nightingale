# Domain docs

Single context. An agent loads this before it names a domain concept or touches a decision.

- `GLOSSARY.md` is the vocabulary. Use its words exactly. Its _Avoid_ lines name the words not to use. If a concept is missing, either you are inventing language (reconsider) or there is a real gap (add the term in the same PR).
- `docs/adr/` holds the decision records, one series from 0001. Read the records that touch your area before you start.
- `docs/srs.md` binds both. On a conflict it wins, and the lower doc is fixed in the same PR.
- `docs/governance.md` says when a new record is worth writing and what format it takes.

If your output contradicts a record, say so plainly rather than overriding it: "Contradicts 0001 (the engine never builds against pulse), but worth reopening because ...".
