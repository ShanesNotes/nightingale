import assert from 'node:assert/strict'
import { test } from 'node:test'
import { Scene, Story } from 'foldkit'
import { FetchChart, FetchVersions, init, Message, type Model, PostAmendment, PostEntry, update } from '../src/workspace/program.ts'
import { view } from '../src/workspace/view.ts'
import { wallTime } from '../src/time.ts'
import { at, entry, NURSE, PHYSICIAN, readModel, versions, written } from './fixtures.ts'

const first = { entry: 'synthetic-note-1', patient: 'test-patient', kind: 'note', content: { text: 'Synthetic note alpha.' }, author: NURSE, care: at('06:55:00'), charted: at('06:56:00'), number: 1, versions: 1 }
const second = { ...first, content: { text: 'Synthetic note beta.' }, author: PHYSICIAN, number: 2, versions: 2, charted: at('06:57:00') }
const loaded = (entries: readonly object[] = []): Model => update(init().model, Message.GotChart({ chart: readModel({ entries }), at: wallTime(1_000_000) })).model
const note = (id: string) => `#note-list li[data-entry="${id}"]`

test('a note is charted from its own form, and its answer shows beside that form, not beside the vitals form', () => {
  Scene.scene({ update, view },
    Scene.given(loaded()),
    Scene.expect(Scene.selector('#note-list')).toHaveText('No notes charted yet.'),
    Scene.type('#note-text', 'Synthetic note alpha.'),
    Scene.type('#note-care', '06:55:00'),
    Scene.click('#chart-note'),
    Scene.Command.expectExact(PostEntry({ request: { patient: 'test-patient', author: NURSE.id, kind: 'note', content: { text: 'Synthetic note alpha.' }, care: at('06:55:00') }, writeId: null })),
    Scene.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'taken', entry: first }) })),
    Scene.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ entries: [first] }), at: wallTime(1_001_000) })),
    Scene.expect(Scene.selector('#note-notice')).toHaveText('Charted note as Synthetic Nurse A.'),
    Scene.expect(Scene.selector('#notice')).toHaveText(''),
    Scene.expect(Scene.selector('#note-text')).toHaveValue(''),
    Scene.expect(Scene.selector(note(first.entry))).toContainText('Synthetic note alpha.'),
    Scene.expect(Scene.selector(note(first.entry))).toContainText('Care time 06:55:00 · Synthetic Nurse A, nurse'),
    Scene.expect(Scene.selector(note(first.entry))).toContainText('Charted 06:56:00, late by 1 min 0 s.'),
    Scene.expect(Scene.selector(`${note(first.entry)} [data-action="versions"]`)).toBeAbsent(),
  )
})

test('a refused note keeps its draft and says why beside the note form', () => {
  Scene.scene({ update, view },
    Scene.given(loaded()),
    Scene.type('#note-text', '   '),
    Scene.click('#chart-note'),
    Scene.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'refused', refusal: { kind: 'content does not fit kind', reason: 'The note text is blank' } }) })),
    Scene.expect(Scene.selector('#note-notice')).toHaveText('The note text is blank; nothing was charted.'),
    Scene.expect(Scene.selector('#notice')).toHaveText(''),
    Scene.expect(Scene.selector('#note-text')).toHaveValue('   '),
  )
})

test('each kind keeps its own draft: charting a note leaves the vital draft as typed, and charting a vital leaves the note draft', () => {
  const vital = entry({ id: 'test-entry-1', vital: 'HR', value: 61, care: '06:57:30', charted: '06:57:30' })
  Story.story(update,
    Story.given(loaded()),
    Story.message(Message.TypedField({ kind: 'vital', field: 'value', text: '61' })),
    Story.message(Message.TypedField({ kind: 'note', field: 'text', text: 'Synthetic note alpha.' })),
    Story.message(Message.ClickedChart({ kind: 'note' })),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'taken', entry: first }) })),
    Story.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ entries: [first] }), at: wallTime(1_001_000) })),
    Story.model((m: Model) => assert.deepEqual([m.drafts.vital.content['value'], m.drafts.note.content['text']], ['61', ''])),
    Story.message(Message.TypedField({ kind: 'note', field: 'text', text: 'Synthetic draft kept.' })),
    Story.message(Message.ClickedChart({ kind: 'vital' })),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'taken', entry: vital }) })),
    Story.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ entries: [first, vital] }), at: wallTime(1_002_000) })),
    Story.model((m: Model) => assert.deepEqual([m.drafts.vital.content['value'], m.drafts.note.content['text'], m.notice], ['', 'Synthetic draft kept.', { about: 'vital', text: 'Charted HR 61 /min as Synthetic Nurse A.' }])),
  )
})

test('a note is edited in place, says which version is late, and shows its earlier versions inside it on request', () => {
  Scene.scene({ update, view },
    Scene.given(loaded([first])),
    Scene.click(`${note(first.entry)} [data-action="edit"]`),
    Scene.expect(Scene.selector(`${note(first.entry)} #edit-text`)).toHaveValue('Synthetic note alpha.'),
    Scene.expect(Scene.selector('#details')).toBeAbsent(),
    Scene.type('#edit-text', 'Synthetic note beta.'),
    Scene.click('#save-edit'),
    Scene.Command.expectExact(PostAmendment({ request: { patient: 'test-patient', author: NURSE.id, entry: first.entry, replaces: 1, kind: 'note', content: { text: 'Synthetic note beta.' }, care: first.care }, writeId: null })),
    Scene.Command.resolve(PostAmendment, Message.Wrote({ result: written({ kind: 'taken', entry: second }) })),
    Scene.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ entries: [second] }), at: wallTime(1_001_000) })),
    Scene.expect(Scene.selector('#note-notice')).toHaveText('Amended note as Synthetic Physician B.'),
    Scene.expect(Scene.selector(`${note(first.entry)} #edit-text`)).toBeAbsent(),
    Scene.expect(Scene.selector(note(first.entry))).toContainText('Synthetic note beta.'),
    Scene.expect(Scene.selector(note(first.entry))).toContainText('Synthetic Physician B, physician · edited, version 2 of 2'),
    Scene.expect(Scene.selector(note(first.entry))).toContainText('Version 2 charted 06:57:00, late by 2 min 0 s.'),
    Scene.expect(Scene.selector('#versions')).toBeAbsent(),
    Scene.click(`${note(first.entry)} [data-action="versions"]`),
    Scene.Command.resolve(FetchVersions, Message.GotVersions({ entry: first.entry, versions: versions([first, second]) })),
    Scene.expect(Scene.selector(`${note(first.entry)} #versions`)).toContainText('Version 1 · Synthetic Nurse A, nurse · care time 06:55:00 · charted 06:56:00'),
    Scene.expect(Scene.selector(`${note(first.entry)} #versions`)).toContainText('Synthetic note alpha.'),
    Scene.expect(Scene.selector(`${note(first.entry)} #versions`)).not.toContainText('Synthetic note beta.'),
    Scene.click(`${note(first.entry)} [data-action="versions"]`),
    Scene.expect(Scene.selector('#versions')).toBeAbsent(),
  )
})

test('earlier versions the service does not send say so beside the note form', () => {
  Story.story(update,
    Story.given(loaded([second])),
    Story.message(Message.ClickedVersions({ entry: second.entry })),
    Story.Command.resolve(FetchVersions, Message.VersionsUnanswered({ entry: second.entry })),
    Story.model((m: Model) => assert.deepEqual(m.notice, { about: 'note', text: 'The chart service did not answer; the earlier versions are not shown.' })),
  )
})
