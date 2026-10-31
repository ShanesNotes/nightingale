# Documentation governance

Governing principle: every artifact must justify why an agent loads it. The caps and checks below apply that principle and are not law of their own. An agent loads this doc before it adds, moves, renames or deletes a doc, a glossary term or a decision record.

## 1. Scope

This doc governs every prose file in the repo: what may enter, which words docs use, how a decision record is written, what an architecture pass reads, and how a doc is added or removed. Code rules live in `AGENTS.md`. nightingale-pulse is out of scope.

## 2. What belongs in the repo

A doc earns its load when it binds behavior, defines a word, records a hard-to-reverse choice, or scopes buildable work. Anything else stays out.

| Path | Role | Why an agent loads it | Authority |
|---|---|---|---|
| `docs/srs.md` | What the engine must do: scope, definitions, hard rules, invariants, structural model, fidelity aims | First, on every pass | Highest: every lower doc yields to it |
| `docs/adr/` | Decision records, one series from 0001 | When a pass touches that area, so it does not reopen a settled choice | Below the SRS |
| `GLOSSARY.md` | The vocabulary | Before writing prose, a ticket or a name | Words only. SRS section 2 binds the terms it defines |
| `docs/architecture/solver-contract.{md,svg,png}` and `solver-contract-sketch/` | The design the crates build against | When building or changing a crate or a seam | Below the records |
| `docs/statecharts/` | The engine lifecycle as a TLA+ model, its Mermaid chart and the README image, with the script that redraws them | When a change adds, removes or moves a lifecycle state or transition | Below the contract: the model follows it |
| `docs/atlas/` | The coupling map and one card per system, frozen | Only the cards a pass names, when that system is built | Lowest: the SRS and the contract win |
| `docs/governance.md` | This doc | Before adding, moving, renaming or deleting a doc, term or record | Rules for docs |
| `data/` | The source and inference records behind the reviewed packet and healthy-range set, as derived, cited numbers, with the identities they must close. Source text never enters | When binding a clinical value or writing a check that judges one | Below the records: a value is only as good as its source record |
| `.scratch/` | Tickets: buildable slices with blocking edges | When picking up the slice a ticket names | Work, not authority |
| `README.md`, `AGENTS.md`, `CLAUDE.md`, `docs/agents/`, `.agents/skills/` | Entry points, agent operating notes and the verify skill | At session start, to find the rest, and the skill before claiming a change works | Point, never restate |

Stays out: council drafts, model transcripts and session logs, review rounds, briefs, scratch notes, duplicated summaries, roadmaps that restate tickets, vision statements and personal framing, and nightingale-pulse files, which are read by permalink at the import commit and never copied.

A PR that adds a file names which agent loads it and when. Two sizes are worth questioning, as applications of the principle: about a dozen prose files outside `docs/atlas/` and `.scratch/`, and a doc past about 400 lines.

## 3. Words

- `GLOSSARY.md` is the vocabulary. Each term appears once, in one or two sentences on what it is, with an `_Avoid_` line of words not to use. Only project terms belong, never general programming words.
- SRS section 2 defines the terms its rules use. `GLOSSARY.md` repeats those definitions word for word, a change to one changes both in the same PR, and `10-docs` fails when they differ.
- Every other doc uses a term exactly and never redefines it.
- A pass that resolves or renames a term updates `GLOSSARY.md` in the same PR. A rename changes the entry, moves the old word to `_Avoid_` and replaces every use in docs and code names in one PR, expanding and contracting when the code change is wide. Prose gets no alias period.
- Architecture prose uses the Pocock `codebase-design` words exactly: module, interface, implementation, depth, seam, adapter, leverage, locality.
- Files are lowercase kebab-case. The crate table in `docs/architecture/solver-contract.md` is the list of crate names. A new name follows `ng-<noun>` with a glossary noun, the table wins where the two differ (as for `ng-observe` and `ng-validate`), and `tools/verify.d/05-seam.sh` refuses any other name.
- Prose goes through pstack `technical-writing`, then `unslop`: plain short words, no em dashes, no vague attributions, no filler.

## 4. Decision records

- One series in `docs/adr/`: `NNNN-slug.md` from 0001, each taking the next free number, with one row per record in `docs/adr/README.md`.
- The format is Pocock's: a title, then one to three sentences of context, choice and reason. Add Considered options or Consequences only when they carry weight. A record has no status or date field, because git history holds its lifecycle.
- Write one only when the choice is hard to reverse, surprising without context and a real trade-off.
- A choice the models made says "model default". A record never credits a person with a technical choice.
- A nightingale-pulse record is cited as pulse 000N or pulse ADR-NN with a permalink at the import commit. It is never copied. One that still matters is written fresh as a new record.
- A superseding record names the one it replaces, and the old record's index row says "superseded by NNNN". The old file stays, and its rule text is not copied forward.

## 5. What an architecture pass reads

Each pass declares its read set at the top and loads only that, in this order:

1. `docs/srs.md`
2. The records in the area it touches
3. `GLOSSARY.md`
4. `docs/architecture/solver-contract.md`, and the sketch if the pass edits types
5. Only the atlas cards the pass names

Rules for a pass:

- Load nightingale-pulse, council files, transcripts or history only when the pass names the exact file and why, and read pulse only at the import commit.
- State the inputs (paths and commit) and the outputs (files to change) before work, and list what changed after.
- Do not reopen a recorded decision. A pass that disagrees proposes a superseding record.
- When a lower doc contradicts a higher one, the higher wins. Fix the lower doc in the same PR or file a ticket, and never quote the stale text forward.
- Keep the main context lean: send bulk reading to subagents and keep their conclusions.
- Build on what the repo says now, not on memory or briefs. A brief may steer a pass but is never cited as authority.
- The engine is native Rust. No other build target is a context input or shapes design (SRS rule 5).

## 6. When sharpening stops

Sharpening ends, and a doc change then needs a ticket that build work exposed, when all of these hold (model defaults):

- every SRS invariant is phrased so one test can check it
- every question to decide has an answer or a written model default with a one-line alternative
- the last full pass added or renamed no glossary term
- the sketch passes its compile-fail and census checks
- tickets for the next build slice exist in `.scratch/`, each small enough for one fresh context window

## 7. Rules held by checks

`tools/verify.d/10-docs.sh` holds these over every Markdown file outside `.scratch/`, with permalink URLs exempt:

- a Markdown file outside the section 2 allowlist
- a dead relative link, including an index row that points at nothing
- a dead code-formatted repo path (a nightingale-pulse path must be a permalink)
- a crate name from pulse's `vitals` family
- `AGENTS.md` over 60 lines, because every session loads it whole, or `README.md` over 40, because it only points onward
- status text, because an agent would load stale state as fact: a heading that starts with Status, Current state, Progress, Next, Changelog, Session or Open, a `Status:` label, or a table row labelled status
- an ISO date, or `TODO`, `FIXME` or `TBD`
- an em dash
- the WASM target's name in `docs/srs.md`, `docs/architecture/`, `docs/atlas/`, `docs/adr/` or a root Markdown file
- a link to a nightingale-pulse record whose text does not start with "pulse"
- a `GLOSSARY.md` entry that differs from SRS section 2

Review holds the rest until a check can: no `_Avoid_` word in prose, and no term defined outside `GLOSSARY.md`.

## 8. Doc lifecycle

- Subtract before you add: check whether an existing doc should absorb the new content or be replaced by it.
- Add a doc in one PR, with a role from section 2 and the glossary updated for any new term.
- Delete a doc that has no unique content rather than leave a pointer stub, and fix its inbound links in the same PR.
- The atlas is frozen. Card changes arrive through tickets as each card's system is built.
