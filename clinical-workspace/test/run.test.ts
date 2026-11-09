import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { decodeRun } from '../src/run.ts'
import { formatClock } from '../src/time.ts'

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
