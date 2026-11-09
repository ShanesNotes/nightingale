# The flowsheet

The flowsheet is the chart by minute of patient time, built only from what the chart service sends. Each vital has a monitor row and a charted row, apart. A monitor cell shows the minute's latest reading and counts its absent, late and missing readings, or says no source, or that no device has made a statement. A charted cell shows each entry with its author, both times, how late it was charted and whether it was edited.

## Sub-features

- `flowsheet-monitor` shows the monitor's readings by minute.
- `flowsheet-charted` shows the clinicians' entries in their own row.
- `flowsheet-words` shows no source, missing and late as words, never as numbers.

## How to get to it (user POV)

- Open the workspace and read the table below the charting form.

## Driving it with npm run drive

Preconditions:

- The chart-a-vital recipe has charted HR 64 as Synthetic Nurse A.

- **Monitor row.** Run `npm run drive -- text workspace 'tr[data-vital="HR"][data-source="monitor"]'`. It prints whole numbers in /min by minute.
- **No source.** Run `npm run drive -- text workspace 'tr[data-vital="SpO2"][data-source="monitor"]'`. Each minute reads `no source`.
- **Charted row.** Run `npm run drive -- text workspace 'tr[data-vital="HR"][data-source="charted"]'`. It holds `64 /min by Synthetic Nurse A`, apart from the monitor row.
- **Proof.** Run `npm run drive -- shot workspace <evidence>/04-flowsheet.png`.

## Gotchas

- A minute's cell reads `td[data-cell="HR monitor 06:57"]`, with the vital, the source and the minute as `HH:MM`.
- Missing and late show here only after an outage or a gap. The outage and device-messages recipes prove them.
