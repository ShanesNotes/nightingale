import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { parseAmendment, parseNewEntry, ROSTER } from '../src/chart-api.ts'
import { openChart, pgliteSql } from '../src/chart.ts'
import { isBootId, isPatientId } from '../src/device.ts'
import { flowsheet } from '../src/flowsheet.ts'
import { Message, monitor } from '../src/monitor/monitor.ts'
import { availableBetween, decodeRun } from '../src/run.ts'
import { formatClock, wallTime } from '../src/time.ts'
import { at } from './fixtures.ts'

const capture = new URL('../capture/307caf19/', import.meta.url)
const file = (name: string) => readFileSync(new URL(name, capture))
const [nurse, physician, therapist] = [ROSTER[0]!, ROSTER[1]!, ROSTER[2]!]

type Step =
  | { readonly at: string; readonly flush: true }
  | { readonly at: string; readonly kind: 'vital' | 'note'; readonly content: Readonly<Record<string, string>>; readonly author: string; readonly care: string | null; readonly amends?: number }

const STEPS: readonly Step[] = [
  { at: '06:56:00', flush: true },
  { at: '06:56:00', kind: 'vital', content: { vital: 'HR', value: '61' }, author: nurse.id, care: '06:53:30' },
  { at: '06:56:00', kind: 'note', content: { text: 'Synthetic note charted before the run began.' }, author: nurse.id, care: '06:50:00' },
  { at: '06:58:00', flush: true },
  { at: '06:58:00', kind: 'vital', content: { vital: 'RR', value: '18' }, author: therapist.id, care: '06:57:20' },
  { at: '06:58:30', kind: 'vital', content: { vital: 'RR', value: '20' }, author: physician.id, care: '06:57:20', amends: 4 },
  { at: '06:59:00', kind: 'vital', content: { vital: 'HR', value: '64' }, author: nurse.id, care: null },
  { at: '06:59:30', kind: 'note', content: { text: 'Synthetic note, amended below.' }, author: physician.id, care: null },
  { at: '07:00:10', kind: 'vital', content: { vital: 'Temp', value: '37.1' }, author: physician.id, care: null },
  { at: '07:00:20', kind: 'note', content: { text: 'Synthetic note, second version.' }, author: therapist.id, care: '06:59:30', amends: 7 },
  { at: '07:01:00', flush: true },
  { at: '07:05:00', flush: true },
]

test("the recorded run's flowsheet, with vitals charted, late and amended, and notes beside them, is main's grid cell for cell", async () => {
  const decoded = decodeRun({ lens: file('lens.npy'), catalogue: readFileSync(new URL('catalogue.txt', capture), 'utf8'), summary: readFileSync(new URL('summary.txt', capture), 'utf8') })
  assert.ok(decoded.kind === 'decoded')
  const { run } = decoded
  const boot = 'synthetic-recorded-flowsheet'
  assert.ok(isPatientId(run.packet) && isBootId(boot))
  const page = monitor({ patient: run.packet, boot, feed: { ...run, between: (after, through) => availableBetween(run, after, through) }, link: { send: async () => ({ kind: 'unanswered' }) } })
  const sql = await pgliteSql()
  try {
    const chart = await openChart(sql, { specId: run.specId, patient: run.packet, start: run.start, end: run.end }, wallTime(1_000_000))
    let device = page.init().model
    const entries: string[] = []
    for (const [index, step] of STEPS.entries()) {
      const now = at(step.at)
      if ('flush' in step) {
        device = page.update(device, Message.SecondPassed({ now })).model
        device = page.update(device, Message.BatchAnswered({ answers: await chart.writeDeviceBatch(device.outbox, now) })).model
        continue
      }
      const shared = { writeId: `synthetic-recorded-${index}`, patient: run.packet, author: step.author, kind: step.kind, content: step.content, care: step.care === null ? null : at(step.care) }
      const parsed = step.amends === undefined ? parseNewEntry(shared) : parseAmendment({ ...shared, entry: entries[step.amends], replaces: 1 })
      assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
      const result = await chart.write(parsed.value, now)
      assert.ok(result.kind === 'taken', JSON.stringify(result))
      entries[index] = result.entry.entry
    }
    const read = await chart.readModel(at('07:05:00'))
    assert.equal(read.entries.filter((entry) => entry.kind === 'note').length, 2)
    assert.ok(read.readings.length > 1000, 'the recorded run reached the chart as device readings')
    const grid = flowsheet(read)
    const minutes = grid.minutes.map((minute) => formatClock(minute).slice(0, 5))
    const cells: Record<string, unknown> = {}
    for (const row of grid.rows) {
      row.cells.forEach((cell, i) => {
        cells[`${row.vital.vital} ${row.source} ${minutes[i]}`] = cell.kind !== 'entries' ? cell : cell.entries.map(({ entry, lateBy, edited }) => ({
          value: entry.content.value,
          author: entry.author.id,
          care: formatClock(entry.care),
          charted: formatClock(entry.charted),
          version: `${entry.number} of ${entry.versions}`,
          lateBy,
          edited,
        }))
      })
    }
    assert.deepEqual(JSON.parse(JSON.stringify({ minutes, cells })), JSON.parse(readFileSync(new URL('recorded-flowsheet.json', import.meta.url), 'utf8')))
  } finally {
    await sql.close()
  }
})
