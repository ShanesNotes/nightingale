# Issue tracker: Local Markdown

Issues and specs for this repo live as markdown files in `.scratch/`.

## Conventions

- One effort per directory: `.scratch/<effort-slug>/`
- A spec is `.scratch/<effort-slug>/spec.md`
- Implementation issues are one file per ticket at `.scratch/<effort-slug>/issues/<NN>-<slug>.md`, numbered from `01`, never a single combined tickets file
- Triage state is a `Status:` line near the top of each issue file (see `triage-labels.md` for the role strings)
- Tickets that ship in one PR each carry a `Lands with:` line near the top that names the others' paths. Claiming one claims them all, and none resolves until all do.
- Comments and conversation history append to the bottom of the file under a `## Comments` heading
- Orchestrate runs live in `.scratch/runs/<run-id>/` (gitignored except `run.md` and contracts); slice worktrees go under `~/worktrees`, because a sandboxed seat dies when its working directory is under the repo's `.scratch/`

## When a skill says "publish to the issue tracker"

Create a new file under `.scratch/<effort-slug>/` (creating the directory if needed).

## When a skill says "fetch the relevant ticket"

Read the file at the referenced path.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a file with one **child** file per ticket.

- **Map**: `.scratch/<effort>/map.md` (Destination / Notes / Decisions-so-far / Not yet specified / Out of scope).
- **Child ticket**: `.scratch/<effort>/issues/NN-<slug>.md`, numbered from `01`, with the question in the body. A `Type:` line records the ticket type (`research`/`prototype`/`grilling`/`task`); a `Status:` line records `open`/`claimed`/`resolved`/`ready-for-human`/`out-of-scope`.
- **Blocking**: a `Blocked by: NN, NN` line near the top. A ticket is unblocked when every file it lists is `resolved`.
- **Frontier**: scan `.scratch/<effort>/issues/` for files that are open, unblocked, and unclaimed; first by number wins.
- **Claim**: set `Status: claimed` and `Claimed by: <seat>` and save before any work.
- **Resolve**: append the answer under an `## Answer` heading, set `Status: resolved`, then append a context pointer (gist + link) to the map's Decisions-so-far in `map.md`.
