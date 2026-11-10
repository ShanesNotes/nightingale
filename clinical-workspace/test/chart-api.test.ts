import assert from 'node:assert/strict'
import { test } from 'node:test'
import { parseAmendment, parseNewEntry, parseReadModel, parseVersions, parseWriteResult, ROSTER } from '../src/chart-api.ts'

const nurse = ROSTER.find((c) => c.discipline === 'nurse')!
const at = (seconds: number) => 24_900_000_000_000 + seconds * 1_000_000_000

const version = {
  entry: 'test-entry-1',
  patient: 'test-patient',
  kind: 'vital',
  number: 2,
  body: { vital: 'RR', value: 18 },
  care: at(60),
  charted: at(95),
  author: nurse,
}

const readModel = {
  run: { specId: 'test-spec', patient: 'test-patient', start: at(0), end: at(600), anchor: 1_000_000 },
  now: at(120),
  entries: [{ ...version, versions: 2 }],
  readings: [
    {
      device: 'test-monitor',
      boot: 'test-boot',
      sequence: 2,
      vital: 'HR',
      collected: at(1),
      available: at(1),
      arrived: at(3),
      value: { kind: 'present', value: 61.5 },
    },
  ],
  statements: [
    {
      device: 'test-monitor',
      boot: 'test-boot',
      sequence: 1,
      at: at(0),
      arrived: at(3),
      channels: { HR: 'sourced', 'ABP sys': 'no source', 'ABP dia': 'no source', 'ABP mean': 'sourced', SpO2: 'no source', RR: 'no source', Temp: 'no source' },
    },
  ],
  missing: [{ device: 'test-monitor', vital: 'ABP mean', from: at(1), through: at(2) }],
}

test('a new entry and an amendment parse from what the workspace sends', () => {
  assert.deepEqual(parseNewEntry({ writeId: 'test-write-1', patient: 'test-patient', author: nurse.id, kind: 'vital' as const, body: { vital: 'RR', value: '18' }, care: null }), {
    ok: true,
    value: { operation: 'new entry', writeId: 'test-write-1', patient: 'test-patient', author: nurse.id, kind: 'vital' as const, body: { vital: 'RR', value: 18 }, care: { kind: 'now' } },
  })
  const amendment = { writeId: 'test-write-2', patient: 'test-patient', author: nurse.id, entry: 'test-entry-1', replaces: 1, kind: 'vital', body: { vital: 'RR', value: '19.5' }, care: at(60) }
  assert.deepEqual(parseAmendment(amendment), {
    ok: true,
    value: {
      operation: 'amendment',
      kind: 'vital',
      writeId: 'test-write-2',
      patient: 'test-patient',
      author: nurse.id,
      entry: 'test-entry-1',
      replaces: 1,
      body: { vital: 'RR', value: 19.5 },
      care: { kind: 'at', at: at(60) },
    },
  })
})

test('empty or invalid value text is refused at the boundary, never parsed as zero', () => {
  for (const text of ['', '   ', 'twelve', '12 bpm']) {
    const parsed = parseNewEntry({ writeId: 'test-write-1', patient: 'test-patient', author: nurse.id, kind: 'vital' as const, body: { vital: 'HR', value: text }, care: null })
    assert.deepEqual(parsed.ok ? undefined : parsed.refusal, { kind: 'body does not fit kind' }, JSON.stringify(text))
  }
})

test('the read model, the versions of an entry and a write result parse back unchanged', () => {
  assert.deepEqual(parseReadModel(JSON.parse(JSON.stringify(readModel))), { ok: true, value: readModel })
  assert.deepEqual(parseVersions([version, { ...version, number: 1, body: { ...version.body, value: 17 } }]), { ok: true, value: [version, { ...version, number: 1, body: { ...version.body, value: 17 } }] })
  const taken = { kind: 'taken', entry: { ...version, versions: 2 } }
  assert.deepEqual(parseWriteResult(taken), { ok: true, value: taken })
  for (const kind of ['care time after now', 'write id reused with other content']) {
    const refused = { kind: 'refused', refusal: { kind } }
    assert.deepEqual(parseWriteResult(refused), { ok: true, value: refused }, kind)
  }
})

test('a body the workspace could not have sent is refused as malformed at the boundary', () => {
  const entry = { writeId: 'test-write-1', patient: 'test-patient', author: nurse.id, kind: 'vital' as const, body: { vital: 'RR', value: '18' }, care: null }
  const amendment = { ...entry, entry: 'test-entry-1', replaces: 1 }
  const newEntries: Record<string, unknown> = {
    'not an object': '18',
    'no write id': { ...entry, writeId: undefined },
    'an author off the roster': { ...entry, author: 'someone' },
    'no author': { ...entry, author: undefined },
    'a vital the table lacks': { ...entry, body: { ...entry.body, vital: 'EtCO2' } },
    'a value sent as a number': { ...entry, body: { ...entry.body, value: 18 } },
    'a care time as clock text': { ...entry, care: '06:56:00' },
    'no patient': { ...entry, patient: '' },
  }
  for (const [name, body] of Object.entries(newEntries)) assert.equal(parseNewEntry(body).ok, false, name)
  const amendments = { 'replaces version zero': { ...amendment, replaces: 0 }, 'no entry': { ...amendment, entry: '' }, 'no write id': { ...amendment, writeId: '' } }
  for (const [name, body] of Object.entries(amendments)) {
    assert.equal(parseAmendment(body).ok, false, name)
  }
  assert.equal(parseWriteResult({ kind: 'refused', refusal: { kind: 'too late' } }).ok, false, 'an unknown refusal kind')
  assert.equal(parseReadModel({ ...readModel, readings: [{ ...readModel.readings[0], value: { kind: 'present', value: null } }] }).ok, false, 'a reading without its value')
})
