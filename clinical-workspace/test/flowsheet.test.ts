import assert from 'node:assert/strict'
import { test } from 'node:test'
import type { ReadModel, ReadingView, StatementView } from '../src/chart-api.ts'
import { ROSTER } from '../src/chart-api.ts'
import { flowsheet } from '../src/flowsheet.ts'
import { formatClock, parseClock } from '../src/time.ts'
import { perVital } from '../src/vitals.ts'

const at = (clock: string) => {
  const time = parseClock(clock)
  assert.ok(time !== undefined, clock)
  return time
}

const run = { specId: 'test-spec', patient: 'test-patient', start: at('06:55:00'), end: at('07:05:00'), anchor: 1_000_000 }
const nurse = ROSTER[0]

const statement = (clock: string, sourced: readonly string[]): StatementView => ({
  device: 'test-monitor',
  boot: 'test-boot',
  sequence: 1,
  at: at(clock),
  arrived: at(clock),
  channels: perVital((v) => (sourced.includes(v) ? 'sourced' : 'no source')),
})

const reading = (vital: 'HR' | 'ABP mean', collected: string, value: number | 'not collected', arrived = collected): ReadingView => ({
  device: 'test-monitor',
  boot: 'test-boot',
  sequence: 2,
  vital,
  collected: at(collected),
  available: at(collected),
  arrived: at(arrived),
  value: value === 'not collected' ? { kind: 'absent', reason: value } : { kind: 'present', value },
})

const model = (overrides: Partial<ReadModel>): ReadModel => ({
  run,
  now: at('06:57:30'),
  entries: [],
  readings: [],
  statements: [],
  missing: [],
  ...overrides,
})

test('the flowsheet has one column per minute, from the earlier of the run start and the earliest care time, to now', () => {
  assert.deepEqual(flowsheet(model({})).minutes.map(formatClock), ['06:55:00', '06:56:00', '06:57:00'])
  const early = { entry: 'test-entry', patient: 'test-patient', vital: 'RR' as const, number: 1, value: 18, care: at('06:53:10'), charted: at('06:55:20'), author: nurse, versions: 1 }
  assert.deepEqual(flowsheet(model({ entries: [early] })).minutes.map(formatClock), ['06:53:00', '06:54:00', '06:55:00', '06:56:00', '06:57:00'])
})

test("a monitor cell shows the minute's latest present reading and counts its absent, late and missing readings", () => {
  const sheet = flowsheet(
    model({
      statements: [statement('06:55:00', ['HR', 'ABP mean'])],
      readings: [
        reading('HR', '06:55:00', 'not collected'),
        reading('HR', '06:55:01', 1.5),
        reading('HR', '06:55:02', 2.5, '06:55:40'),
        reading('HR', '06:56:00', 3.5),
      ],
      missing: [{ device: 'test-monitor', vital: 'HR', from: at('06:55:03'), through: at('06:55:05') }],
    }),
  )
  const hr = sheet.rows.find((row) => row.source === 'monitor' && row.vital.vital === 'HR')
  assert.ok(hr?.source === 'monitor')
  assert.deepEqual(hr.cells[0], { kind: 'readings', latest: { value: 2.5, collected: at('06:55:02') }, absent: 1, late: 1, missing: 3 })
  assert.deepEqual(hr.cells[1], { kind: 'readings', latest: { value: 3.5, collected: at('06:56:00') }, absent: 0, late: 0, missing: 0 })
  const spo2 = sheet.rows.find((row) => row.source === 'monitor' && row.vital.vital === 'SpO2')
  assert.deepEqual(spo2?.cells.map((c) => c.kind), ['no source', 'no source', 'no source'])
})

test('a monitor cell before any device statement says so, which is not no source', () => {
  const sheet = flowsheet(model({ statements: [statement('06:56:10', ['HR'])] }))
  for (const vital of ['HR', 'Temp']) {
    const row = sheet.rows.find((r) => r.source === 'monitor' && r.vital.vital === vital)
    assert.equal(row?.cells[0]?.kind, 'no statement', vital)
  }
  const temp = sheet.rows.find((r) => r.source === 'monitor' && r.vital.vital === 'Temp')
  assert.equal(temp?.cells[1]?.kind, 'no source')
})
