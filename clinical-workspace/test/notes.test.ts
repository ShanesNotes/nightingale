import assert from 'node:assert/strict'
import { test } from 'node:test'
import { entriesOfKind } from '../src/chart-api.ts'
import { notes } from '../src/notes.ts'
import { at, NURSE, PHYSICIAN, readModel } from './fixtures.ts'

const note = (entry: string, care: string, charted: string, fields: object = {}) => ({
  entry,
  patient: 'test-patient',
  kind: 'note',
  content: { text: `Synthetic note ${entry}.` },
  number: 1,
  versions: 1,
  author: NURSE,
  care: at(care),
  charted: at(charted),
  ...fields,
})

test('the notes view lists typed notes newest care time first, whatever order the chart gives, with lateness and current versions only', () => {
  const middle = note('synthetic-note-b', '06:56:00', '06:56:00')
  const earliest = note('synthetic-note-a', '06:55:00', '06:56:00')
  const latest = note('synthetic-note-d', '06:57:00', '06:57:00', { number: 2, versions: 2, author: PHYSICIAN, content: { text: 'Synthetic amended note.' } })
  const sameCareChartedLater = note('synthetic-note-c', '06:56:00', '06:57:20')
  const vital = { ...earliest, entry: 'synthetic-vital', kind: 'vital', content: { vital: 'HR', value: 1.25 } }
  const model = readModel({ entries: [middle, vital, earliest, latest, sameCareChartedLater] })
  assert.deepEqual(entriesOfKind(model, 'note').map((entry) => entry.entry), [middle.entry, earliest.entry, latest.entry, sameCareChartedLater.entry])
  assert.deepEqual(notes(model).map(({ entry, lateBy, edited }) => [entry.entry, lateBy, edited]), [
    [latest.entry, 0, true],
    [sameCareChartedLater.entry, 80_000_000_000, false],
    [middle.entry, 0, false],
    [earliest.entry, 60_000_000_000, false],
  ])
  assert.deepEqual(notes(model)[0]?.entry.content, { text: 'Synthetic amended note.' })
  assert.deepEqual(model.entries.map((entry) => entry.entry), [middle.entry, vital.entry, earliest.entry, latest.entry, sameCareChartedLater.entry], 'the view does not sort the chart in place')
  assert.deepEqual(notes(readModel({})), [])
})
