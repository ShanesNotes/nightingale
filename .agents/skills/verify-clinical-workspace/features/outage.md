# A chart service outage

The monitor keeps taking readings with the workspace closed and the chart service stopped. It holds every message the service has not answered and says so in words. When it reaches the service again it delivers what it held, and those readings arrive late. Until they arrive, the chart shows their seconds as missing, and then as late.

## Sub-features

- `outage-closed` closes the chart screen, which stops it asking the chart service.
- `outage-held` shows the monitor holding its messages and since when the service has not answered.
- `outage-missing` shows the held seconds as missing in the reopened flowsheet.
- `outage-late` shows the same seconds late once the monitor delivers them.

## How to get to it (user POV)

- Choose **Close chart** in the workspace, stop the chart service, and later start it again and choose **Reopen chart**.

## Driving it with npm run drive

Preconditions:

- The monitor shows `Everything delivered to the chart.` and the workspace is open.

- **Close.** Run `npm run drive -- click workspace '#close'`. `#closed` reads `The chart is closed.`
- **Stop the service.** Send SIGTERM to the chart service's pid from `.dev/state.json`. The launcher and Vite keep running. After a few seconds `#link` on the monitor reads `Holding N messages. The chart service has not answered since HH:MM:SS.` and the tiles and `#clock` still move.
- **Cut the monitor off.** Run `npm run drive -- offline monitor 14` in the background, so the monitor cannot deliver for 14 s.
- **Start the service again.** Run `node scripts/service.ts` with `CHART_DATABASE_URL` from the state file and `PORT` set to the stopped service's port, in its own session, and record its pid.
- **Missing.** Run `npm run drive -- click workspace '#reopen'`. The HR monitor row reads `N missing`, and `GET /api/chart` lists the held seconds in `missing`.
- **Late.** Once the offline command ends, `#link` reads `Everything delivered to the chart.` again. Every second that was missing now has a reading whose arrival second is after its available second, and the HR monitor row reads `N late`.
- **Proof.** Shoot the monitor while it holds, the workspace while seconds are missing and after they turn late, and keep the chart's rows from both moments.

## Gotchas

- The restarted chart service must take the old port, since Vite's proxy points there.
- Without the offline step, the monitor delivers within a second of the service's return, and the missing seconds may never be seen.
- Stop the restarted service by its own pid at cleanup. The launcher only knows the first one.
