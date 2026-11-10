import assert from 'node:assert/strict'
import { test } from 'node:test'
import { Scene, Story } from 'foldkit'
import { FetchChart, FetchVersions, init, Message, PostAmendment, PostEntry, update } from '../src/workspace/program.ts'
import { view } from '../src/workspace/view.ts'
import { wallTime } from '../src/time.ts'
import { at, NURSE, readModel, versions, written } from './fixtures.ts'

const first = { entry: 'synthetic-note-1', patient: 'test-patient', kind: 'note', content: { text: 'Synthetic note alpha.' }, author: NURSE, care: at('06:55:00'), charted: at('06:56:00'), number: 1, versions: 1 }
const loaded = (entries: readonly object[] = []) => update(init().model, Message.GotChart({ chart: readModel({ entries }), at: wallTime(1_000_000) })).model

test('a clinician writes a note through the same chart command and gets useful confirmation', () => {
  Story.story(update,
    Story.given(loaded()),
    Story.message(Message.ChoseKind({ kind: 'note' })),
    Story.message(Message.TypedField({ field: 'text', text: 'Synthetic note alpha.' })),
    Story.message(Message.TypedCare({ text: '06:55:00' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ content: { patient: 'test-patient', author: NURSE.id, kind: 'note', content: { text: 'Synthetic note alpha.' }, care: at('06:55:00') }, writeId: null })),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'taken', entry: first }) })),
    Story.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ entries: [first] }), at: wallTime(1_001_000) })),
    Story.model((model) => assert.match(model.notice, /Charted note as Synthetic Nurse A/)),
  )
})

test('notes show author, care time and lateness; editing and earlier versions use the common entry controls', () => {
  const second = { ...first, content: { text: 'Synthetic note beta.' }, number: 2, versions: 2, charted: at('06:57:00') }
  Scene.scene({ update, view },
    Scene.given(loaded([first])),
    Scene.expect(Scene.selector('#notes')).toContainText('Synthetic note alpha.'),
    Scene.expect(Scene.selector('#notes')).toContainText('Synthetic Nurse A'),
    Scene.expect(Scene.selector('#notes')).toContainText('06:55:00'),
    Scene.expect(Scene.selector('#notes')).toContainText('late by 1 min 0 s'),
    Scene.expect(Scene.selector('#versions')).toBeAbsent(),
    Scene.click('[data-note="synthetic-note-1"] button'),
    Scene.click('#edit'),
    Scene.expect(Scene.selector('#edit-text')).toHaveValue('Synthetic note alpha.'),
    Scene.type('#edit-text', 'Synthetic note beta.'),
    Scene.click('#save-edit'),
    Scene.Command.expectExact(PostAmendment({ content: { patient: 'test-patient', author: NURSE.id, entry: first.entry, replaces: 1, kind: 'note', content: { text: 'Synthetic note beta.' }, care: first.care }, writeId: null })),
    Scene.Command.resolve(PostAmendment, Message.Wrote({ result: written({ kind: 'taken', entry: second }) })),
    Scene.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ entries: [second] }), at: wallTime(1_001_000) })),
    Scene.expect(Scene.selector('#notes')).toContainText('Synthetic note beta.'),
    Scene.expect(Scene.selector('#versions')).toBeAbsent(),
    Scene.click('#versions-button'),
    Scene.Command.resolve(FetchVersions, Message.GotVersions({ entry: first.entry, versions: versions([first, second]) })),
    Scene.expect(Scene.selector('#versions')).toContainText('Synthetic note alpha.'),
    Scene.expect(Scene.selector('#versions')).toContainText('Synthetic note beta.'),
    Scene.click('#versions-button'),
    Scene.expect(Scene.selector('#versions')).toBeAbsent(),
  )
})

test('a refused note keeps its draft and explains how to fix an empty content', () => {
  Scene.scene({ update, view },
    Scene.given(update(loaded(), Message.ChoseKind({ kind: 'note' })).model),
    Scene.expect(Scene.selector('#notes')).toContainText('No notes charted yet.'),
    Scene.type('#note-text', '   '),
    Scene.click('#chart'),
    Scene.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'refused', refusal: { kind: 'content does not fit kind' } }) })),
    Scene.expect(Scene.selector('#notice')).toContainText('Enter note text'),
    Scene.expect(Scene.selector('#note-text')).toHaveValue('   '),
  )
})
