import assert from 'node:assert/strict'
import { test } from 'node:test'
import { parseReadModel, parseRun, parseVersions, parseWriteResult, ROSTER } from '../src/chart-api.ts'
import { openChart, pgliteSql } from '../src/chart.ts'
import { isPatientId, OUTBOX_CAP, parseAnswers } from '../src/device.ts'
import { startService } from '../src/service.ts'
import { parseClock, type PatientTime, wallTime } from '../src/time.ts'

const at = (clock: string) => {
  const time = parseClock(clock)
  assert.ok(time !== undefined, clock)
  return time
}

const patient = 'test-patient'
assert.ok(isPatientId(patient))
const nurse = ROSTER[0]!

test('the chart service answers the chart API and the device protocol over HTTP', async () => {
  const sql = await pgliteSql()
  const chart = await openChart(sql, { specId: 'test-spec', patient, start: at('06:55:00'), end: at('07:05:00') }, wallTime(1_000_000))
  let now: PatientTime = at('06:56:00')
  const service = await startService({ chart, now: () => now, port: 0 })
  const call = async (method: string, path: string, body?: unknown) => {
    const response = await fetch(`${service.url}${path}`, {
      method,
      headers: { 'content-type': 'application/json' },
      ...(body === undefined ? {} : { body: typeof body === 'string' ? body : JSON.stringify(body) }),
    })
    return { status: response.status, json: await response.json() }
  }
  try {
    const run = await call('GET', '/api/run')
    assert.deepEqual(parseRun(run.json), { ok: true, value: chart.run })

    const entry = { writeId: 'test-write-1', patient, author: nurse.id, kind: 'vital' as const, body: { vital: 'RR', value: '18' }, care: null }
    const charted = await call('POST', '/api/entries', entry)
    const taken = parseWriteResult(charted.json)
    assert.equal(charted.status, 200)
    assert.ok(taken.ok && taken.value.kind === 'taken')
    assert.equal(taken.value.entry.charted, at('06:56:00'), 'the service stamps the charted time from the patient clock')

    const late = await call('POST', '/api/entries', { ...entry, writeId: 'test-write-2', care: at('06:56:01') })
    assert.deepEqual([late.status, late.json], [422, { kind: 'refused', refusal: { kind: 'care time after now' } }])
    const malformed = await call('POST', '/api/entries', { ...entry, writeId: 'test-write-3', author: 'someone' })
    assert.equal(malformed.status, 400)
    assert.equal(malformed.json.refusal.kind, 'malformed')
    assert.equal((await call('POST', '/api/entries', '{not json')).status, 400)

    now = at('06:57:00')
    const amended = await call('POST', '/api/amendments', { ...entry, writeId: 'test-write-4', entry: taken.value.entry.entry, replaces: 1, body: { ...entry.body, value: '19' } })
    assert.equal(amended.status, 200)
    const versions = await call('GET', `/api/entries/${taken.value.entry.entry}/versions`)
    const parsedVersions = parseVersions(versions.json)
    assert.ok(parsedVersions.ok)
    assert.deepEqual(parsedVersions.value.map((v) => v.kind === 'vital' ? v.body.value : undefined), [18, 19])
    assert.equal((await call('GET', '/api/entries/00000000-0000-4000-8000-000000000000/versions')).status, 404)

    const statement = {
      kind: 'statement',
      device: 'test-monitor',
      boot: 'test-boot',
      patient,
      sequence: 1,
      at: at('06:57:00'),
      channels: { HR: 'sourced', 'ABP sys': 'no source', 'ABP dia': 'no source', 'ABP mean': 'sourced', SpO2: 'no source', RR: 'no source', Temp: 'no source' },
    }
    const delivered = await call('POST', '/api/device/messages', [statement])
    assert.deepEqual(parseAnswers(delivered.json), { ok: true, value: [{ boot: 'test-boot', sequence: 1, answer: 'taken' }] })
    assert.equal((await call('POST', '/api/device/messages', [{ ...statement, vital: 'EtCO2', kind: 'reading' }])).status, 400)

    now = at('06:57:03')
    const model = parseReadModel((await call('GET', '/api/chart')).json)
    assert.ok(model.ok)
    assert.equal(model.value.now, at('06:57:03'))
    assert.deepEqual(model.value.entries.map((e) => [e.kind === 'vital' ? e.body.value : undefined, e.versions]), [[19, 2]])
    assert.deepEqual(model.value.statements.map((s) => s.arrived), [at('06:57:00')], 'the service stamps arrival from the patient clock')

    const note = { ...entry, writeId: 'synthetic-note-write', kind: 'note', body: { text: 'Synthetic note over HTTP.' }, care: at('06:55:30') }
    const noteAnswer = await call('POST', '/api/entries', note)
    const noteTaken = parseWriteResult(noteAnswer.json)
    assert.equal(noteAnswer.status, 200)
    assert.ok(noteTaken.ok && noteTaken.value.kind === 'taken')
    assert.equal(noteTaken.value.entry.kind, 'note')
    assert.deepEqual(await call('POST', '/api/entries', note), noteAnswer)
    const amendedNote = await call('POST', '/api/amendments', { ...note, writeId: 'synthetic-note-amendment', entry: noteTaken.value.entry.entry, replaces: 1, body: { text: 'Synthetic amended note over HTTP.' } })
    assert.equal(amendedNote.status, 200)
    const noteVersions = parseVersions((await call('GET', `/api/entries/${noteTaken.value.entry.entry}/versions`)).json)
    assert.ok(noteVersions.ok)
    assert.deepEqual(noteVersions.value.map((v) => v.body), [note.body, { text: 'Synthetic amended note over HTTP.' }])
    const unknownKind = await call('POST', '/api/entries', { ...note, kind: 'unknown' })
    assert.deepEqual([unknownKind.status, unknownKind.json], [400, { kind: 'refused', refusal: { kind: 'unknown entry kind' } }])
    const badBody = await call('POST', '/api/entries', { ...note, body: { value: 1.25 } })
    assert.deepEqual([badBody.status, badBody.json], [400, { kind: 'refused', refusal: { kind: 'body does not fit kind' } }])
    const withNote = parseReadModel((await call('GET', '/api/chart')).json)
    assert.ok(withNote.ok)
    assert.deepEqual(withNote.value.entries.map((entry) => entry.kind), ['note', 'vital'])
    assert.equal(withNote.value.readings.length, 0)
    assert.equal((await call('GET', '/api/nothing')).status, 404)
  } finally {
    await service.close()
    await sql.close()
  }
})

test('a write sent again over HTTP under its write id gets the same answer and charts nothing twice', async () => {
  const sql = await pgliteSql()
  const chart = await openChart(sql, { specId: 'test-spec', patient, start: at('06:55:00'), end: at('07:05:00') }, wallTime(1_000_000))
  const service = await startService({ chart, now: () => at('06:56:00'), port: 0 })
  const post = async (body: object) => {
    const response = await fetch(`${service.url}/api/entries`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) })
    return { status: response.status, json: await response.json() }
  }
  try {
    const entry = { writeId: 'test-write-1', patient, author: nurse.id, kind: 'vital' as const, body: { vital: 'RR', value: '18' }, care: null }
    const first = await post(entry)
    assert.equal(first.status, 200)
    assert.deepEqual(await post(entry), first)
    const reused = await post({ ...entry, body: { ...entry.body, value: '19' } })
    assert.deepEqual([reused.status, parseWriteResult(reused.json)], [422, { ok: true, value: { kind: 'refused', refusal: { kind: 'write id reused with other content' } } }])
    assert.equal((await post({ ...entry, writeId: undefined })).status, 400, 'a write without its write id is malformed')
    const model = parseReadModel(await (await fetch(`${service.url}/api/chart`)).json())
    assert.ok(model.ok)
    assert.equal(model.value.entries.length, 1)
  } finally {
    await service.close()
    await sql.close()
  }
})

test('the chart service answers each message of the largest batch a monitor can hold', async () => {
  const sql = await pgliteSql()
  const chart = await openChart(sql, { specId: 'test-spec', patient, start: at('06:55:00'), end: at('07:05:00') }, wallTime(1_000_000))
  const service = await startService({ chart, now: () => at('06:56:00'), port: 0 })
  const longest = (name: string) => `${name}-${'x'.repeat(199 - name.length)}`
  const header = { device: longest('test-monitor'), boot: longest('test-boot'), patient: longest('test-patient') }
  const channels = { HR: 'no source', 'ABP sys': 'no source', 'ABP dia': 'no source', 'ABP mean': 'no source', SpO2: 'no source', RR: 'no source', Temp: 'no source' }
  const batch = Array.from({ length: OUTBOX_CAP }, (_, i) => ({ kind: 'statement', ...header, sequence: i + 1, at: at('06:55:00'), channels }))
  try {
    const response = await fetch(`${service.url}/api/device/messages`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(batch),
    })
    const answers = parseAnswers(await response.json())
    assert.equal(response.status, 200)
    assert.ok(answers.ok)
    assert.equal(answers.value.length, OUTBOX_CAP)
  } finally {
    await service.close()
    await sql.close()
  }
})
