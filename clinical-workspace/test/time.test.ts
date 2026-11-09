import assert from 'node:assert/strict'
import { test } from 'node:test'
import { formatClock, parseClock } from '../src/time.ts'

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
