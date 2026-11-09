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

test('a device message that is not well formed is refused at the boundary', () => {
  const without = (message: object, key: string) => Object.fromEntries(Object.entries(message).filter(([k]) => k !== key))
  const cases: Record<string, unknown> = {
    'not an object': 'HR 61',
    'unknown kind': { ...reading, kind: 'alarm' },
    'a vital the table lacks': { ...reading, vital: 'EtCO2' },
    'filtration': { ...reading, vital: 'Filtration' },
    'no device': without(reading, 'device'),
    'empty boot': { ...reading, boot: '' },
    'no patient': without(reading, 'patient'),
    'sequence zero': { ...reading, sequence: 0 },
    'fractional sequence': { ...reading, sequence: 1.5 },
    'time as text': { ...reading, collected: '06:55:01' },
    'negative time': { ...reading, collected: -1 },
    'available before collected': { ...reading, available: reading.collected - 1 },
    'present value missing': { ...reading, value: { kind: 'present' } },
    'present value not a number': { ...reading, value: { kind: 'present', value: '61' } },
    'absent for no reason': { ...reading, value: { kind: 'absent', reason: 'tired' } },
    'statement missing a vital': { ...statement, channels: without(statement.channels, 'Temp') },
    'statement with an extra vital': { ...statement, channels: { ...statement.channels, EtCO2: 'sourced' } },
    'statement with an unknown source word': { ...statement, channels: { ...statement.channels, HR: 'maybe' } },
  }
  for (const [name, json] of Object.entries(cases)) assert.equal(parseDeviceMessage(json).ok, false, name)
  assert.equal(parseBatch({ messages: [reading] }).ok, false, 'a batch is a list')
  assert.equal(parseBatch([reading, { ...reading, vital: 'EtCO2' }]).ok, false, 'one bad message refuses the batch')
  assert.equal(parseAnswers({ answers: [{ boot: 'b', sequence: 1, answer: 'maybe' }] }).ok, false, 'an unknown answer')
  assert.equal(parseAnswers({ answers: [{ boot: 'b', sequence: 1, answer: 'refused' }] }).ok, false, 'a refusal names its kind')
})
