import assert from 'node:assert/strict'
import { test } from 'node:test'
import { parseReadModel, ROSTER, type ReadModel } from '../src/chart-api.ts'
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

const statement = (clock: string, sourced: readonly string[]) => ({
  device: 'test-monitor',
  boot: 'test-boot',
  sequence: 1,
  at: at(clock),
  arrived: at(clock),
  channels: perVital((v) => (sourced.includes(v) ? 'sourced' : 'no source')),
})

const reading = (vital: 'HR' | 'ABP mean', collected: string, value: number | 'not collected', arrived = collected) => ({
  device: 'test-monitor',
  boot: 'test-boot',
  sequence: 2,
  vital,
  collected: at(collected),
  available: at(collected),
  arrived: at(arrived),
  value: value === 'not collected' ? { kind: 'absent', reason: value } : { kind: 'present', value },
})

function model(overrides: Readonly<Record<string, unknown>>): ReadModel {
  const parsed = parseReadModel({ run, now: at('06:57:30'), entries: [], readings: [], statements: [], missing: [], ...overrides })
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}

test('the flowsheet has one column per minute, from the earlier of the run start and the earliest care time, to now', () => {
  assert.deepEqual(flowsheet(model({})).minutes.map(formatClock), ['06:55:00', '06:56:00', '06:57:00'])
  const early = { entry: 'test-entry', patient: 'test-patient', kind: 'vital' as const, content: { vital: 'RR' as const, value: 18 }, number: 1,  care: at('06:53:10'), charted: at('06:55:20'), author: nurse, versions: 1 }
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

test('a charted cell shows the current version of each entry whose care time falls in its minute, how late it was charted and whether it was edited', () => {
  const entry = (id: string, vital: 'RR' | 'HR', care: string, charted: string, versions: number) => ({
    entry: id,
    patient: 'test-patient',
    kind: 'vital',
    number: versions,
    content: { vital, value: 18 },
    care: at(care),
    charted: at(charted),
    author: nurse,
    versions,
  })
  const onTime = entry('test-entry-1', 'RR', '06:56:10', '06:56:10', 1)
  const late = entry('test-entry-2', 'RR', '06:56:30', '06:57:15', 2)
  const other = entry('test-entry-3', 'HR', '06:55:05', '06:55:05', 1)
  const sheet = flowsheet(model({ entries: [onTime, late, other] }))
  const rr = sheet.rows.find((row) => row.source === 'charted' && row.vital.vital === 'RR')
  assert.ok(rr?.source === 'charted')
  assert.deepEqual(rr.cells.map((cell) => cell.entries.length), [0, 2, 0])
  assert.deepEqual(rr.cells[1]?.entries, [
    { entry: onTime, lateBy: 0, edited: false },
    { entry: late, lateBy: 45_000_000_000, edited: true },
  ])
  const hr = sheet.rows.find((row) => row.source === 'charted' && row.vital.vital === 'HR')
  assert.ok(hr?.source === 'charted')
  assert.deepEqual(hr.cells.map((cell) => cell.entries.length), [1, 0, 0])
})

test('the rows follow the vitals table, so EtCO2 is one more row: a monitor row with no statement and an empty charted row', () => {
  const withEtCO2 = [...flowsheet(model({})).rows.map((row) => row.vital).filter((v, i, all) => all.indexOf(v) === i), { vital: 'EtCO2', label: 'End-tidal CO2', unit: 'mmHg' }]
  const sheet = flowsheet(model({ statements: [statement('06:55:00', ['HR', 'ABP mean'])] }), withEtCO2)
  assert.deepEqual(sheet.rows.map((row) => `${row.vital.vital} ${row.source}`).slice(-4), ['Temp monitor', 'Temp charted', 'EtCO2 monitor', 'EtCO2 charted'])
  assert.equal(sheet.rows.length, 16)
  const etco2 = sheet.rows.filter((row) => row.vital.vital === 'EtCO2')
  assert.deepEqual(etco2.map((row) => row.cells.map((cell) => (cell.kind === 'entries' ? cell.entries.length : cell.kind))), [
    ['no statement', 'no statement', 'no statement'],
    [0, 0, 0],
  ])
})
