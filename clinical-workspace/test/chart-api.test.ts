import assert from 'node:assert/strict'
import { test } from 'node:test'
import { parseAmendment, parseNewEntry, parseReadModel, parseVersions, parseWriteResult, ROSTER } from '../src/chart-api.ts'

const nurse = ROSTER.find((c) => c.discipline === 'nurse')!
const at = (seconds: number) => 24_900_000_000_000 + seconds * 1_000_000_000

const version = {
  entry: 'test-entry-1',
  patient: 'test-patient',
  vital: 'RR',
  number: 2,
  value: 18,
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
  assert.deepEqual(parseNewEntry({ patient: 'test-patient', author: nurse.id, vital: 'RR', value: '18', care: null }), {
    ok: true,
    value: { kind: 'new entry', patient: 'test-patient', author: nurse.id, vital: 'RR', value: 18, care: { kind: 'now' } },
  })
  assert.deepEqual(parseAmendment({ patient: 'test-patient', author: nurse.id, entry: 'test-entry-1', replaces: 1, value: '19.5', care: at(60) }), {
    ok: true,
    value: {
      kind: 'amendment',
      patient: 'test-patient',
      author: nurse.id,
      entry: 'test-entry-1',
      replaces: 1,
      value: 19.5,
      care: { kind: 'at', at: at(60) },
    },
  })
})

test('empty value text is never zero: it parses to a value that is not a finite number', () => {
  for (const text of ['', '   ', 'twelve', '12 bpm']) {
    const parsed = parseNewEntry({ patient: 'test-patient', author: nurse.id, vital: 'HR', value: text, care: null })
    assert.ok(parsed.ok && Number.isNaN(parsed.value.value), JSON.stringify(text))
  }
})

test('the read model, the versions of an entry and a write result parse back unchanged', () => {
  assert.deepEqual(parseReadModel(JSON.parse(JSON.stringify(readModel))), { ok: true, value: readModel })
  assert.deepEqual(parseVersions([version, { ...version, number: 1, value: 17 }]), { ok: true, value: [version, { ...version, number: 1, value: 17 }] })
  const taken = { kind: 'taken', entry: { ...version, versions: 2 } }
  assert.deepEqual(parseWriteResult(taken), { ok: true, value: taken })
  const refused = { kind: 'refused', refusal: { kind: 'care time after now' } }
  assert.deepEqual(parseWriteResult(refused), { ok: true, value: refused })
})
