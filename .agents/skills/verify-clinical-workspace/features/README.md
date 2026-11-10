# Clinical workspace verification map

This folder is the maintained source for verifying what a person does with the clinical workspace prototype. Read this index before driving the pages, then use the matching feature file as the recipe. `walk.sh` in the skill folder runs every recipe in this order.

## Baseline preconditions

- Launch from an empty database with `npm run reset` then `npm run dev` in `clinical-workspace/`, under a container name of this run's own, as the skill's Launch section says.
- The doctor checks pass: the launcher, chart service and Vite pids are alive, and the chart service answers `GET /api/run` with the recorded run's spec id and patient.
- `npm run drive -- start` has launched Chromium, and the tabs `monitor` and `workspace` are open at the URLs the launcher printed.
- Never drive an instance this run did not start.

## Driving conventions

- Run every command from `clinical-workspace/`.
- Select by ids and data attributes, never by position.
- Wait for the expected text by polling `npm run drive -- text`. Foldkit paints in an animation frame, so a read in the click's own tick sees the old page.
- Read the chart back with `curl -s <service url>/api/chart` after each change, and keep the rows beside the screenshot.

## Proof and skip reporting

- A check names the action and the state it produced, with its screenshot and the chart's rows.
- A check that cannot run says which precondition failed, with the command it tried. A feature proven through another feature's path is not proven.

## Feature entry contract

Each feature file starts with an H1 title and one paragraph on what a person sees. Then four H2 sections, in order: `Sub-features`, `How to get to it (user POV)`, `Driving it with npm run drive`, and `Gotchas`.

## Features

- [The bedside monitor](monitor.md): the tiles, no source, the feed and the link's words (moment 1).
- [Chart a vital](chart-a-vital.md): any clinician charts any vital, care time and charted time, refusals in words (moment 3).
- [The flowsheet](flowsheet.md): the chart by minute, monitor and charted rows apart, no source, missing and late as words (moment 4).
- [Amend a chart entry](amend.md): details, a new version, earlier versions on request (moment 5).
- [Chart a note](chart-a-note.md): a note charted late, a refused blank note, an amendment in place, earlier versions inside the note, all read back from the chart service.
- [A chart service outage](outage.md): the monitor holds with the workspace closed and the service stopped, then its readings arrive late (moment 2).
- [Device messages](device-messages.md): a resend changes nothing, a sequence gap shows as missing, Postgres refuses changes (moment 6).
