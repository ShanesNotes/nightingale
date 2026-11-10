import assert from 'node:assert/strict'
import { test } from 'node:test'
import { entriesOfKind } from '../src/chart-api.ts'
import { notes } from '../src/notes.ts'
import { at, NURSE, PHYSICIAN, readModel } from './fixtures.ts'

test('the notes view asks for typed notes, ordered by care time, with lateness and current versions only', () => {
  const first = { entry: 'synthetic-note-1', patient: 'test-patient', kind: 'note', body: { text: 'Synthetic first note.' }, number: 1, versions: 1, author: NURSE, care: at('06:55:00'), charted: at('06:56:00') }
  const latest = { ...first, entry: 'synthetic-note-2', body: { text: 'Synthetic amended note.' }, number: 2, versions: 2, author: PHYSICIAN, care: at('06:57:00'), charted: at('06:57:00') }
  const vital = { ...first, entry: 'synthetic-vital', kind: 'vital', body: { vital: 'HR', value: 1.25 } }
  const model = readModel({ entries: [first, vital, latest] })
  assert.deepEqual(entriesOfKind(model, 'note').map((entry) => entry.body.text), ['Synthetic first note.', 'Synthetic amended note.'])
  assert.deepEqual(notes(model), [
    { entry: latest, lateBy: 0, edited: true },
    { entry: first, lateBy: 60_000_000_000, edited: false },
  ])
  assert.deepEqual(model.entries.map((entry) => entry.entry), [first.entry, vital.entry, latest.entry], 'the view does not sort the chart in place')
  assert.deepEqual(notes(readModel({})), [])
})
