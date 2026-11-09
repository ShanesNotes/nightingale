import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { availableBetween, decodeRun } from '../src/run.ts'
import { formatClock, isPatientTime, type PatientTime } from '../src/time.ts'

const capture = new URL('../capture/307caf19/', import.meta.url)
const bytes = (name: string) => readFileSync(new URL(name, capture))
const text = (name: string) => readFileSync(new URL(name, capture), 'utf8')
const summaryValue = (key: string) => {
  const line = text('summary.txt').split('\n').find((l) => l.startsWith(`${key} `))
  assert.ok(line, key)
  return line.slice(key.length + 1)
}

const committed = () => ({ lens: bytes('lens.npy'), catalogue: text('catalogue.txt'), summary: text('summary.txt') })

test('the committed capture matches the hashes its run summary names', () => {
  for (const name of ['spec.txt', 'catalogue.txt', 'lens.npy']) {
    assert.equal(createHash('sha256').update(bytes(name)).digest('hex'), summaryValue(`file ${name}`), name)
  }
})

test("the recorded run decodes into its lens channels and the observer's observations", () => {
  const decoded = decodeRun(committed())
  assert.equal(decoded.kind, 'decoded', decoded.kind === 'refused' ? decoded.reason : '')
  const { run } = decoded
  assert.equal(run.specId, summaryValue('spec_id'))
  assert.equal(run.packet, summaryValue('packet'))
  assert.deepEqual(run.channels, [
    'algebraic/CV/rate/body',
    'algebraic/CV/blood pressure/systemic arteries',
    'algebraic/RN/filtration/glomerular bed>renal tubule',
  ])
  assert.equal(run.observations.length, 1803)
  for (const channel of [0, 1, 2]) assert.equal(run.observations.filter((o) => o.channel === channel).length, 601)
  assert.equal(formatClock(run.start), '06:55:00')
  assert.equal(formatClock(run.end), '07:05:00')
  for (const o of run.observations) assert.ok(run.start <= o.collected && o.collected <= o.available && o.available <= run.end)
})

const SUMMARY = 'spec_id test-spec\ncapture_version 2\npacket test-packet\nend_time_ns 3000000000\n'
const CATALOGUE = 'gp-catalogue 1\nrow algebraic/test/one\nlens algebraic/test/one 1000000000\n'
const SECOND = 1_000_000_000n

function npy(rows: readonly (readonly bigint[])[], header = `{'descr': '<u8', 'fortran_order': False, 'shape': (${rows.length}, 6), }`) {
  const head = new Uint8Array(128).fill(0x20)
  head.set([0x93, 0x4e, 0x55, 0x4d, 0x50, 0x59, 0x01, 0x00])
  new DataView(head.buffer).setUint16(8, 118, true)
  head.set(new TextEncoder().encode(header), 10)
  head[127] = 0x0a
  const data = new DataView(new ArrayBuffer(rows.length * 48))
  rows.forEach((row, r) => row.forEach((word, c) => data.setBigUint64((r * 6 + c) * 8, word, true)))
  return new Uint8Array([...head, ...new Uint8Array(data.buffer)])
}

const bitsOf = (value: number) => {
  const view = new DataView(new ArrayBuffer(8))
  view.setFloat64(0, value, true)
  return view.getBigUint64(0, true)
}

const synthetic = (lens: Uint8Array, catalogue = CATALOGUE, summary = SUMMARY) => decodeRun({ lens, catalogue, summary })

test('a well-formed synthetic lens decodes', () => {
  const decoded = synthetic(npy([[0n, SECOND, SECOND, 1n, 0n, 0n], [0n, 2n * SECOND, 2n * SECOND, 0n, bitsOf(1.5), 0n]]))
  assert.equal(decoded.kind, 'decoded')
})

test('malformed lens, catalogue or summary input is refused at the boundary', () => {
  const row = (overrides: Partial<Record<number, bigint>>) => {
    const words = [0n, SECOND, SECOND, 0n, bitsOf(1.5), 0n]
    for (const [i, w] of Object.entries(overrides)) words[Number(i)] = w!
    return npy([words])
  }
  const cases: Record<string, () => ReturnType<typeof decodeRun>> = {
    'not NPY': () => synthetic(new Uint8Array(200)),
    'f64 rows': () => synthetic(npy([], `{'descr': '<f8', 'fortran_order': False, 'shape': (0, 6), }`)),
    'five columns': () => synthetic(npy([], `{'descr': '<u8', 'fortran_order': False, 'shape': (0, 5), }`)),
    'short data': () => synthetic(npy([[0n, SECOND, SECOND, 1n, 0n, 0n]]).subarray(0, 150)),
    'no rows': () => synthetic(npy([])),
    'unknown tag': () => synthetic(row({ 3: 4n })),
    'unknown channel': () => synthetic(row({ 0: 1n })),
    'absence with value bits': () => synthetic(row({ 3: 1n })),
    'infinite present value': () => synthetic(row({ 4: bitsOf(Infinity) })),
    'NaN present value': () => synthetic(row({ 4: bitsOf(NaN) })),
    'available before collected': () => synthetic(row({ 2: 0n })),
    'time past the safe range': () => synthetic(row({ 1: 2n ** 60n, 2: 2n ** 60n })),
    'available after the run ends': () => synthetic(row({ 2: 4n * SECOND })),
    'catalogue without lens lines': () => synthetic(row({}), 'gp-catalogue 1\nrow a\n'),
    'not a catalogue': () => synthetic(row({}), 'hello\nlens a 1\n'),
    'summary without packet': () => synthetic(row({}), CATALOGUE, 'spec_id s\ncapture_version 2\nend_time_ns 3000000000\n'),
    'summary of another capture version': () => synthetic(row({}), CATALOGUE, SUMMARY.replace('capture_version 2', 'capture_version 1')),
    'summary end not in nanoseconds': () => synthetic(row({}), CATALOGUE, SUMMARY.replace('3000000000', 'soon')),
  }
  for (const [name, decode] of Object.entries(cases)) assert.equal(decode().kind, 'refused', name)
})

test('the run gives out a reading once the observer has it: pending when collected, a value when available', () => {
  const decoded = synthetic(
    npy([
      [0n, 1n * SECOND, 1n * SECOND, 0n, bitsOf(1.5), 0n],
      [0n, 2n * SECOND, 4n * SECOND, 2n, 0n, 0n],
      [0n, 3n * SECOND, 3n * SECOND, 1n, 0n, 0n],
      [0n, 2n * SECOND, 4n * SECOND, 0n, bitsOf(2.5), 0n],
    ]),
    CATALOGUE,
    SUMMARY.replace('end_time_ns 3000000000', 'end_time_ns 5000000000'),
  )
  assert.equal(decoded.kind, 'decoded')
  const { run } = decoded
  const second = (n: number): PatientTime => {
    const time = n * 1e9
    assert.ok(isPatientTime(time))
    return time
  }
  const shown = (after: number | undefined, through: number) =>
    availableBetween(run, after === undefined ? undefined : second(after), second(through)).map(
      (o) => `${o.collected / 1e9}s ${o.value.kind === 'present' ? o.value.value : o.value.kind}`,
    )
  assert.deepEqual(shown(undefined, 1), ['1s 1.5'])
  assert.deepEqual(shown(1, 2), ['2s pending'])
  assert.deepEqual(shown(2, 3), ['3s not collected'])
  assert.deepEqual(shown(3, 4), ['2s 2.5'])
  assert.deepEqual(shown(undefined, 5).length, 4)
  assert.deepEqual(shown(4, 5), [])
})
