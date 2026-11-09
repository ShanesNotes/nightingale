# The bedside monitor

The monitor replays the recorded run as hardware would: one tile per vital of the table, heart rate in /min and mean arterial pressure in mmHg as whole numbers each second, and "no source" for the five vitals the engine does not produce. It names the patient, says the feed is a recorded synthetic run and when it ended, and says in words how its link to the chart service is.

## Sub-features

- `monitor-tiles` shows each vital's latest reading, or why there is none.
- `monitor-no-source` shows "no source" for systolic, diastolic, SpO2, respiratory rate and temperature, and never shows filtration.
- `monitor-values` shows the observer's own value for the second on its clock.
- `monitor-link` says everything is delivered, or how many messages it holds and since when the chart service has not answered.

## How to get to it (user POV)

- Open the monitor URL the launcher printed, `<vite url>/monitor.html`.

## Driving it with npm run drive

Preconditions:

- The launch and doctor checks pass, and the `monitor` tab is open.

- **Tiles.** Wait for heart rate. Run `npm run drive -- wait monitor "/^\d+ \/min$/.test(document.querySelector('td[data-tile=\"HR\"]').innerText)" 30`. Then `npm run drive -- text monitor 'td[data-tile="ABP mean"]'` prints a whole number and `mmHg`.
- **No source.** Run `npm run drive -- text monitor 'td[data-tile="SpO2"]'`, and the same for `ABP sys`, `ABP dia`, `RR` and `Temp`. Each prints `no source`. `npm run drive -- eval monitor "/filtration/i.test(document.body.innerText)"` prints `false`.
- **Observer's value.** Read the clock and the tile in one frame: `npm run drive -- eval monitor "[document.querySelector('#clock').innerText, document.querySelector('td[data-tile=\"HR\"]').innerText]"`. Decode the capture with `src/run.ts` for that second's `algebraic/CV/rate/body` row: the tile equals the value times 60, rounded.
- **Link.** Run `npm run drive -- text monitor '#link'`. It prints `Everything delivered to the chart.` on a healthy link.
- **Proof.** Run `npm run drive -- shot monitor <evidence>/01-monitor.png`.

## Gotchas

- The tiles and the clock change every second, so read them in one `eval`.
- 06:55:00, before the first window fills, has passed before the page opens. The monitor's tests prove it says `not collected` then.
