import assert from 'node:assert/strict'
import { test } from 'node:test'
import { formatClock, parseClock, patientTimeAt, untilNextSecond, wallTime } from '../src/time.ts'

const at = (text: string) => {
  const time = parseClock(text)
  assert.ok(time !== undefined, text)
  return time
}

test('a patient time reads as HH:MM:SS and parses back', () => {
  for (const text of ['00:00:00', '06:55:00', '07:05:00', '23:59:59']) {
    const time = parseClock(text)
    assert.ok(time !== undefined, text)
    assert.equal(formatClock(time), text)
  }
})

test('clock text that is not HH:MM:SS on a 24-hour clock is refused', () => {
  for (const text of ['', '6:55:00', '06:55', '24:00:00', '06:60:00', '06:55:60', ' 06:55:00', '06:55:00.5', 'ab:cd:ef']) {
    assert.equal(parseClock(text), undefined, JSON.stringify(text))
  }
})

test('patient time is the run start plus the whole seconds since the anchor, capped at the run end', () => {
  const clock = { start: at('06:55:00'), end: at('07:05:00'), anchor: wallTime(1_000_000) }
  const shown = (ms: number) => formatClock(patientTimeAt(clock, wallTime(ms)))
  assert.equal(shown(999_000), '06:55:00')
  assert.equal(shown(1_000_000), '06:55:00')
  assert.equal(shown(1_000_999), '06:55:00')
  assert.equal(shown(1_001_000), '06:55:01')
  assert.equal(shown(1_125_400), '06:57:05')
  assert.equal(shown(1_600_000), '07:05:00')
  assert.equal(shown(4_600_000), '07:05:00')
})

test('the next tick of the patient clock falls on the next whole second since the anchor', () => {
  const clock = { start: at('06:55:00'), end: at('07:05:00'), anchor: wallTime(1_000_000) }
  const wait = (ms: number) => untilNextSecond(clock, wallTime(ms))
  assert.deepEqual([wait(1_000_000), wait(1_000_250), wait(1_000_999), wait(999_500), wait(1_002_400)], [1000, 750, 1, 500, 600])
})
