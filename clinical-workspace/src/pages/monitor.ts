import catalogue from '../../capture/307caf19/catalogue.txt?raw'
import lensUrl from '../../capture/307caf19/lens.npy?url'
import summary from '../../capture/307caf19/summary.txt?raw'
import { isBootId, isPatientId } from '../device.ts'
import { httpLink } from '../monitor/link.ts'
import { start } from '../monitor/monitor.ts'
import { availableBetween, decodeRun } from '../run.ts'
import { isWallTime } from '../time.ts'

// The monitor page: the recorded run as its feed, the chart service on this page's origin as its link, a fresh
// boot id at each load, and the run's anchor when the launcher gave one.
const container = document.getElementById('monitor')
if (container === null) throw new Error('monitor.html has no #monitor element')
const lens = new Uint8Array(await (await fetch(lensUrl)).arrayBuffer())
const decoded = decodeRun({ lens, catalogue, summary })
if (decoded.kind === 'refused') {
  container.textContent = `The recorded run does not decode: ${decoded.reason}`
  throw new Error(decoded.reason)
}
const { run } = decoded
const boot = crypto.randomUUID()
const anchorText: unknown = import.meta.env['VITE_RUN_ANCHOR']
const anchor = Number(anchorText)
if (!isPatientId(run.packet) || !isBootId(boot)) throw new Error('the run names no patient, or the boot id is empty')
start(container, {
  patient: run.packet,
  feed: { start: run.start, end: run.end, channels: run.channels, between: (after, through) => availableBetween(run, after, through) },
  link: httpLink('/api/device/messages'),
  boot,
  ...(typeof anchorText === 'string' && anchorText !== '' && isWallTime(anchor) ? { anchor } : {}),
})
