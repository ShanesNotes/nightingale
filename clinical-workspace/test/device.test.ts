import assert from 'node:assert/strict'
import { test } from 'node:test'
import { parseAnswers, parseBatch, parseDeviceMessage } from '../src/device.ts'

const header = { device: 'test-monitor', boot: 'test-boot-1', patient: 'test-patient' }
const reading = {
  kind: 'reading',
  ...header,
  sequence: 2,
  vital: 'HR',
  collected: 24_901_000_000_000,
  available: 24_901_000_000_000,
  value: { kind: 'present', value: 61.5 },
}
const absent = { ...reading, sequence: 3, value: { kind: 'absent', reason: 'not collected' } }
const statement = {
  kind: 'statement',
  ...header,
  sequence: 1,
  at: 24_900_000_000_000,
  channels: {
    HR: 'sourced',
    'ABP sys': 'no source',
    'ABP dia': 'no source',
    'ABP mean': 'sourced',
    SpO2: 'no source',
    RR: 'no source',
    Temp: 'no source',
  },
}

test('a reading and a channel statement parse back from their JSON unchanged', () => {
  for (const message of [reading, absent, statement]) {
    const parsed = parseDeviceMessage(JSON.parse(JSON.stringify(message)))
    assert.deepEqual(parsed, { ok: true, value: message })
  }
  assert.deepEqual(parseBatch([statement, reading]), { ok: true, value: [statement, reading] })
})

test('the per-message answers parse back in the order of the batch', () => {
  const answers = [
    { boot: 'test-boot-1', sequence: 1, answer: 'taken' },
    { boot: 'test-boot-1', sequence: 2, answer: 'already held' },
    { boot: 'test-boot-1', sequence: 3, answer: 'refused', refusal: 'another patient' },
  ]
  assert.deepEqual(parseAnswers({ answers }), { ok: true, value: answers })
})
