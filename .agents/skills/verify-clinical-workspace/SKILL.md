---
name: verify-clinical-workspace
description: >-
  Drive the clinical workspace prototype, the bedside monitor and the chart screen, in headless Chromium against
  a fresh local Postgres, and keep screenshots and the chart's rows as proof. Use before calling a change under
  clinical-workspace/ done, to reproduce a bug in it, and for the gate walk before review.
---

# Verify the clinical workspace

The clinical workspace is two pages and a chart service in `clinical-workspace/`: the bedside monitor, which replays the recorded run, and the chart screen. Proof means driving both pages in a real browser against a real Postgres and reading the chart back from the chart service. The unit tests run on PGlite and never stand in for this.

## Walk

    bash .agents/skills/verify-clinical-workspace/walk.sh <evidence folder> [feature ...]

`walk.sh` is the whole recipe: launch, doctor, every mapped feature, evidence and cleanup. With no feature named it walks all of them in order: `monitor chart-a-vital flowsheet amend outage device-messages`. It prints one `PASS` or `FAIL` line per check into `walk.txt` in the evidence folder and exits 0 only when every check passed. A failed launch ends the walk at once.

The walk starts a Postgres container, the chart service, Vite and Chromium together, so run it as one heavy step where the machine has a heavy-step lock. It takes about three minutes.

## Launch

The walk runs the launcher from an empty database: `npm run reset` with `CW_DB_CONTAINER=cw-verify-<pid>`, then `npm run dev` in its own session, with the inherited file descriptors closed so the dev servers hold no lock. The launcher prints `dev: workspace <url>` and `dev: monitor <url>` once Vite answers, and writes every pid to `.dev/state.json` in the package. Ready means the chart service answers `GET /api/run` with the run row for the spec id in `clinical-workspace/capture/307caf19/summary.txt`, and both pages answer.

To launch by hand for a look, run `npm run dev` in `clinical-workspace/`. It reuses the container `clinical-workspace-pg` and its volume, so the chart and its patient clock carry over between runs. `npm run reset` removes both for a fresh run.

## Doctor

Before driving anything, read `.dev/state.json` in `clinical-workspace/` and check:

- the launcher, chart service and Vite pids are alive (`kill -0 <pid>`);
- `curl -s <service url>/api/run` names the spec id and the packet hash from `summary.txt` as the patient;
- `docker ps` shows the container named in the state file, bound to `127.0.0.1` only.

Drive only an instance this run started. The developer's own `npm run dev` and the `nightingale-pg17` container on port 5432 belong to other work.

## Drive

`npm run drive -- <command>` drives Chromium (`/usr/local/bin/chromium`) over the DevTools protocol, one command per call, with named tabs. `npm run drive -- help` lists the commands: `start`, `open`, `wait`, `text`, `click`, `fill`, `choose`, `eval`, `offline`, `shot`, `close` and `stop`. The walk opens the monitor in a tab named `monitor` and the chart screen in one named `workspace`. Select by the ids and data attributes the pages render: `td[data-tile="HR"]`, `#link`, `#clock`, `#feed` on the monitor; `#author`, `#vital`, `#value`, `#care`, `#chart`, `#notice`, `#service`, `#close`, `#reopen`, `#details`, `#edit`, `#versions-button` and `td[data-cell="<vital> <monitor|charted> <HH:MM>"]` on the chart screen.

Foldkit paints in an animation frame, so read the page by polling for the expected text, never in the tick of the click that changes it. The walk's `expect` does that.

[features/README.md](features/README.md) indexes one file per feature: what it is, how a person reaches it, the exact drive commands and what proves it.

## Evidence

The evidence folder is the one you pass, outside the repo. Use `~/lanes/logs/<lane>/walk/` for a gate walk. It keeps:

- `walk.txt`, every check with its result, the commit walked and the cleanup lines;
- `NN-<feature>.png`, full-page screenshots after each step that changes what a person sees;
- `chart-<phase>.json`, the chart's rows as `GET /api/chart` gave them at that moment;
- `device-first.json`, `device-again.json` and `postgres-refusals.txt`, the chart service's answers and Postgres's own refusals;
- `dev.log` and `service-again.log`, the launcher's and the restarted chart service's output.

A proof drives the real user path and reads the chart back. A screenshot alone proves nothing about the chart: pair it with the rows.

## Cleanup

The walk's exit trap stops Chromium by the pid in `.dev/drive.json`, the restarted chart service by its pid file, and the launcher's session (the chart service and Vite stop with it), then removes the container `cw-verify-<pid>` and its volume. It never removes the evidence folder. After a run, `docker ps -a --filter name=cw-verify` should list nothing.

## Gotchas

- Chromium runs no animation frames for a tab that is not in front, and Foldkit paints in one, so a background page keeps working but its DOM goes stale. `npm run drive -- open` gives each page a window of its own, and each command brings its tab to the front before it reads or acts. A screenshot forces a frame, so a stale read beside a correct screenshot is this.
- 06:55:00, when heart rate and mean pressure show no number, passes before a page can open. `clinical-workspace/test/monitor.test.ts` proves that moment.
- The chart service anchors the patient clock at its first start on an empty database, and the run ends at 07:05:00. A walk that starts later than ten minutes after its own launch sees a stopped clock.
- `offline` cuts one tab off the network only while its command runs, because DevTools drops the setting when its connection closes.
