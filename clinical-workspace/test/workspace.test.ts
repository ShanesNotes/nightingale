import assert from 'node:assert/strict'
import { test } from 'node:test'
import { Scene, Story } from 'foldkit'
import { FetchChart, FetchVersions, init, Message, type Model, PostAmendment, PostEntry, subscriptions, update } from '../src/workspace/program.ts'
import { makeView, view } from '../src/workspace/view.ts'
import { wallTime } from '../src/time.ts'
import { VITALS } from '../src/vitals.ts'
import { at, entry, reading, readModel, statement, THERAPIST, versions, written } from './fixtures.ts'

type Sim = Parameters<Extract<Parameters<typeof Story.story>[1], (s: never) => unknown>>[0]
const park = (sim: Sim) => ({ ...sim, commands: [] })

const loaded = (overrides: Readonly<Record<string, unknown>> = {}): Model =>
  update(init().model, Message.GotChart({ chart: readModel(overrides), at: wallTime(5_000_000) })).model

const notice = (expected: string | RegExp) => Story.model((m: Model) => assert.match(m.notice, typeof expected === 'string' ? new RegExp(expected) : expected))

test('a clinician of any discipline charts any vital: the page sends what was typed, then shows the chart as the service now has it', () => {
  const taken = written({ kind: 'taken', entry: { ...entry({ id: 'test-entry-1', vital: 'RR', value: 18, care: '06:56:10', charted: '06:57:30' }), author: THERAPIST } })
  Story.story(
    update,
    Story.given(loaded()),
    Story.message(Message.ChoseAuthor({ author: THERAPIST.id })),
    Story.message(Message.TypedField({ field: 'vital', text: 'RR' })),
    Story.message(Message.TypedField({ field: 'value', text: '18' })),
    Story.message(Message.TypedCare({ text: '06:56:10' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ content: { patient: 'test-patient', author: THERAPIST.id, kind: 'vital', content: { vital: 'RR', value: '18' }, care: at('06:56:10') }, writeId: null })),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: taken })),
    Story.Command.expectHas(FetchChart),
    notice('Charted RR 18 /min as Synthetic Respiratory Therapist C'),
    park,
  )
})

test('a blank care time is sent as now, for the chart service to read when it takes the write', () => {
  Story.story(
    update,
    Story.given(loaded()),
    Story.message(Message.TypedField({ field: 'value', text: '61' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ content: { patient: 'test-patient', author: 'synthetic-nurse', kind: 'vital', content: { vital: 'HR', value: '61' }, care: null }, writeId: null })),
    park,
  )
})

test('a refusal shows in words, and a write the service does not answer is never queued', () => {
  Story.story(
    update,
    Story.given(loaded()),
    Story.message(Message.TypedField({ field: 'value', text: '61' })),
    Story.message(Message.TypedCare({ text: '06:59:00' })),
    Story.message(Message.ClickedChart()),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'refused', refusal: { kind: 'care time after now' } }) })),
    notice('The care time is after now; nothing was charted.'),
    Story.message(Message.TypedField({ field: 'value', text: '' })),
    Story.message(Message.ClickedChart()),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'refused', refusal: { kind: 'content does not fit kind' } }) })),
    notice('Enter a finite value for a listed vital; nothing was charted.'),
    Story.message(Message.ClickedChart()),
    Story.Command.resolve(PostEntry, Message.WriteUnanswered({ writeId: 'test-write-1' })),
    notice('The chart service did not answer, so the entry may already be in the chart.'),
    Story.Command.expectNone(),
    Story.message(Message.TypedCare({ text: '6:59' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectNone(),
    notice('The care time is not HH:MM:SS; nothing was charted.'),
  )
})

test('a write the service does not answer may already be in the chart, so sending the same draft again sends the same write id', () => {
  const content = { patient: 'test-patient', author: 'synthetic-nurse', kind: 'vital' as const, content: { vital: 'HR', value: '61' }, care: null }
  Story.story(
    update,
    Story.given(loaded()),
    Story.message(Message.TypedField({ field: 'value', text: '61' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ content, writeId: null })),
    Story.Command.resolve(PostEntry, Message.WriteUnanswered({ writeId: 'test-write-1' })),
    notice('The chart service did not answer, so the entry may already be in the chart. Charting it again unchanged cannot chart it twice.'),
    Story.model((m: Model) => assert.equal(m.form.content.value, '61', 'the draft stays as typed')),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ content, writeId: 'test-write-1' })),
    Story.Command.resolve(PostEntry, Message.WriteUnanswered({ writeId: 'test-write-1' })),
    Story.message(Message.TypedField({ field: 'value', text: '' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ content: { ...content, content: { ...content.content, value: '' } }, writeId: null })),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'refused', refusal: { kind: 'content does not fit kind' } }) })),
    Story.message(Message.TypedField({ field: 'value', text: '61' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ content, writeId: null })),
    park,
  )
  const first = entry({ id: 'test-entry-1', vital: 'RR', value: 18, care: '06:56:30', charted: '06:57:15' })
  const amendment = { patient: 'test-patient', author: 'synthetic-nurse', entry: 'test-entry-1', replaces: 1, kind: 'vital' as const, content: { vital: 'RR', value: '19' }, care: at('06:56:30') }
  Story.story(
    update,
    Story.given(loaded({ entries: [first] })),
    Story.message(Message.ClickedEntry({ entry: 'test-entry-1' })),
    Story.message(Message.ClickedEdit()),
    Story.message(Message.TypedEditField({ field: 'value', text: '19' })),
    Story.message(Message.ClickedSaveEdit()),
    Story.Command.expectExact(PostAmendment({ content: amendment, writeId: null })),
    Story.Command.resolve(PostAmendment, Message.WriteUnanswered({ writeId: 'test-write-2' })),
    notice('The chart service did not answer, so the new version may already be in the chart. Saving it again unchanged cannot chart it twice.'),
    Story.message(Message.ClickedSaveEdit()),
    Story.Command.expectExact(PostAmendment({ content: amendment, writeId: 'test-write-2' })),
    park,
  )
})

const cell = (name: string) => Scene.selector(`td[data-cell="${name}"]`)

test('the page shows the flowsheet from the chart alone: monitor and charted rows apart, no source, missing and late as words', () => {
  const model = loaded({
    statements: [statement('06:55:00', ['HR', 'ABP mean'])],
    readings: [reading('HR', '06:55:00', 'not collected'), reading('HR', '06:55:01', 61.6), reading('HR', '06:55:02', 62.4, '06:55:40')],
    missing: [{ device: 'test-monitor', vital: 'HR', from: at('06:55:03'), through: at('06:55:05') }],
    entries: [entry({ id: 'test-entry-1', vital: 'RR', value: 18, care: '06:56:30', charted: '06:57:15', versions: 2 })],
  })
  Scene.scene(
    { update, view },
    Scene.given(model),
    Scene.expect(Scene.selector('#patient')).toContainText('test-patient'),
    Scene.expect(Scene.selector('#now')).toContainText('06:57:30'),
    Scene.expect(cell('HR monitor 06:55')).toHaveText('62 /min, 1 absent, 1 late, 3 missing'),
    Scene.expect(cell('HR monitor 06:56')).toHaveText('no reading'),
    Scene.expect(cell('SpO2 monitor 06:55')).toHaveText('no source'),
    Scene.expect(cell('RR monitor 06:56')).toHaveText('no source'),
    Scene.expect(cell('RR charted 06:56')).toContainText('18 /min'),
    Scene.expect(cell('RR charted 06:56')).toContainText('late by 45 s'),
    Scene.expect(cell('RR charted 06:56')).toContainText('edited'),
    Scene.expect(cell('RR charted 06:55')).toHaveText(''),
  )
})

test('before any device statement a monitor cell says no device has made one, which is not no source', () => {
  Scene.scene(
    { update, view },
    Scene.given(loaded({ statements: [statement('06:56:10', ['HR'])] })),
    Scene.expect(cell('HR monitor 06:55')).toHaveText('no device statement'),
    Scene.expect(cell('Temp monitor 06:55')).toHaveText('no device statement'),
    Scene.expect(cell('Temp monitor 06:56')).toHaveText('no source'),
  )
})

test('the charting form and the flowsheet follow the vitals table, so EtCO2 is one more option and two more rows', () => {
  const withEtCO2 = makeView([...VITALS, { vital: 'EtCO2', label: 'End-tidal CO2', unit: 'mmHg' }])
  Scene.scene(
    { update, view: withEtCO2 },
    Scene.given(loaded()),
    Scene.expectAll(Scene.all.selector('#vital option')).toHaveCount(VITALS.length + 1),
    Scene.expect(Scene.selector('#vital option[value="EtCO2"]')).toExist(),
    Scene.expect(cell('EtCO2 monitor 06:55')).toHaveText('no device statement'),
    Scene.expect(cell('EtCO2 charted 06:55')).toExist(),
  )
})

test('a charted value shows its details; edit amends the current version; earlier versions come from the service on request', () => {
  const first = entry({ id: 'test-entry-1', vital: 'RR', value: 18, care: '06:56:30', charted: '06:57:15' })
  const second = { ...first, number: 2, versions: 2, content: { ...first.content, value: 19 }, charted: at('06:57:40') }
  Scene.scene(
    { update, view },
    Scene.given(loaded({ entries: [first] })),
    Scene.expect(Scene.selector('#details')).toBeAbsent(),
    Scene.click(Scene.selector('td[data-cell="RR charted 06:56"] button')),
    Scene.expect(Scene.selector('#details')).toContainText('Synthetic Nurse A, nurse'),
    Scene.expect(Scene.selector('#details')).toContainText('Patient test-patient'),
    Scene.expect(Scene.selector('#details')).toContainText('Care time 06:56:30'),
    Scene.expect(Scene.selector('#details')).toContainText('Charted time 06:57:15'),
    Scene.expect(Scene.selector('#details')).toContainText('Late by 45 s'),
    Scene.expect(Scene.selector('#details')).toContainText('Version 1 of 1'),
    Scene.expect(Scene.selector('#versions')).toBeAbsent(),
    Scene.click('#edit'),
    Scene.expect(Scene.selector('#edit-value')).toHaveValue('18'),
    Scene.type('#edit-value', '19'),
    Scene.click('#save-edit'),
    Scene.Command.expectExact(
      PostAmendment({ content: { patient: 'test-patient', author: 'synthetic-nurse', entry: 'test-entry-1', replaces: 1, kind: 'vital' as const, content: { vital: 'RR', value: '19' }, care: at('06:56:30') }, writeId: null }),
    ),
    Scene.Command.resolve(PostAmendment, Message.Wrote({ result: written({ kind: 'taken', entry: second }) })),
    Scene.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ now: at('06:57:41'), entries: [second] }), at: wallTime(6_000_000) })),
    Scene.expect(cell('RR charted 06:56')).toContainText('19 /min'),
    Scene.expect(Scene.selector('#details')).toContainText('Version 2 of 2'),
    Scene.click('#versions-button'),
    Scene.Command.resolve(FetchVersions, Message.GotVersions({ entry: 'test-entry-1', versions: versions([{ ...first, number: 1 }, second]) })),
    Scene.expect(Scene.selector('#versions')).toContainText('Version 1: 18 /min'),
    Scene.expect(Scene.selector('#versions')).toContainText('Version 2: 19 /min'),
  )
})

test('an edit amends the version it began from, so a stale draft is refused when a poll brings a newer version', () => {
  const first = entry({ id: 'test-entry-1', vital: 'RR', value: 18, care: '06:56:30', charted: '06:57:15' })
  const newer = { ...first, number: 2, versions: 2, content: { ...first.content, value: 20 }, author: THERAPIST, charted: at('06:57:40') }
  Story.story(
    update,
    Story.given(loaded({ entries: [first] })),
    Story.message(Message.ClickedEntry({ entry: 'test-entry-1' })),
    Story.message(Message.ClickedEdit()),
    Story.message(Message.Polled({ at: wallTime(5_001_000) })),
    Story.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ now: at('06:57:41'), entries: [newer] }), at: wallTime(5_001_100) })),
    Story.message(Message.TypedEditField({ field: 'value', text: '19' })),
    Story.message(Message.ClickedSaveEdit()),
    Story.Command.expectExact(
      PostAmendment({ content: { patient: 'test-patient', author: 'synthetic-nurse', entry: 'test-entry-1', replaces: 1, kind: 'vital' as const, content: { vital: 'RR', value: '19' }, care: at('06:56:30') }, writeId: null }),
    ),
    Story.Command.resolve(PostAmendment, Message.Wrote({ result: written({ kind: 'refused', refusal: { kind: 'not the current version' } }) })),
    notice('Someone charted a newer version first; nothing was charted.'),
  )
})

test('a write in flight takes no input, and its answer clears only the draft it answered, so no draft is lost', () => {
  const first = entry({ id: 'test-entry-1', vital: 'RR', value: 18, care: '06:56:30', charted: '06:57:15' })
  const charted = entry({ id: 'test-entry-2', vital: 'HR', value: 61, care: '06:57:30', charted: '06:57:30' })
  const amended = { ...first, number: 2, versions: 2, content: { ...first.content, value: 19 }, charted: at('06:57:40') }
  Story.story(
    update,
    Story.given(loaded({ entries: [first] })),
    Story.message(Message.ClickedEntry({ entry: 'test-entry-1' })),
    Story.message(Message.ClickedEdit()),
    Story.message(Message.TypedEditField({ field: 'value', text: '19' })),
    Story.message(Message.TypedField({ field: 'value', text: '61' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ content: { patient: 'test-patient', author: 'synthetic-nurse', kind: 'vital', content: { vital: 'HR', value: '61' }, care: null }, writeId: null })),
    park,
    Story.message(Message.TypedField({ field: 'value', text: '62' })),
    Story.message(Message.TypedField({ field: 'vital', text: 'RR' })),
    Story.message(Message.TypedEditField({ field: 'value', text: '20' })),
    Story.message(Message.ClickedChart()),
    Story.message(Message.ClickedSaveEdit()),
    Story.Command.expectNone(),
    Story.model((m: Model) => assert.deepEqual([m.form.content.vital, m.form.content.value, m.editing?.content.value], ['HR', '61', '19'], 'no input while the write is in flight')),
    Story.message(Message.Wrote({ result: written({ kind: 'taken', entry: charted }) })),
    Story.model((m: Model) => assert.deepEqual([m.form.content.value, m.editing?.content.value], ['', '19'], 'the answer cleared the form it answered and kept the edit')),
    park,
    Story.message(Message.TypedField({ field: 'value', text: '64' })),
    Story.message(Message.ClickedSaveEdit()),
    park,
    Story.message(Message.Wrote({ result: written({ kind: 'taken', entry: amended }) })),
    Story.model((m: Model) => assert.deepEqual([m.editing, m.form.content.value], [null, '64'], 'the answer closed the edit it answered and kept the form')),
    park,
  )
  const whileCharting = ['#author', '#vital', '#value', '#care', '#chart', '#edit']
  const whileAmending = ['#author', '#vital', '#value', '#care', '#chart', '#edit-value', '#edit-care', '#save-edit']
  Scene.scene(
    { update, view },
    Scene.given(loaded({ entries: [first] })),
    Scene.click(Scene.selector('td[data-cell="RR charted 06:56"] button')),
    Scene.type('#value', '61'),
    Scene.click('#chart'),
    ...whileCharting.map((id) => Scene.expect(Scene.selector(id)).toBeDisabled()),
    Scene.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'taken', entry: charted }) })),
    Scene.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ now: at('06:57:31'), entries: [first, charted] }), at: wallTime(6_000_000) })),
    ...whileCharting.map((id) => Scene.expect(Scene.selector(id)).toBeEnabled()),
    Scene.click('#edit'),
    Scene.click('#save-edit'),
    ...whileAmending.map((id) => Scene.expect(Scene.selector(id)).toBeDisabled()),
    Scene.Command.resolve(PostAmendment, Message.Wrote({ result: written({ kind: 'taken', entry: amended }) })),
    Scene.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ now: at('06:57:41'), entries: [amended, charted] }), at: wallTime(6_001_000) })),
  )
})

test('the page asks for one chart at a time, so an older chart never replaces a newer one once the clock has stopped', () => {
  const stopped = at('07:05:00')
  const charted = entry({ id: 'test-entry-1', vital: 'HR', value: 61, care: '07:05:00', charted: '07:05:00' })
  const older = Message.GotChart({ chart: readModel({ now: stopped }), at: wallTime(5_001_200) })
  const newer = Message.GotChart({ chart: readModel({ now: stopped, entries: [charted] }), at: wallTime(5_001_300) })
  Story.story(
    update,
    Story.given(loaded({ now: stopped })),
    Story.message(Message.Polled({ at: wallTime(5_001_000) })),
    Story.Command.expectExact(FetchChart()),
    park,
    Story.message(Message.TypedField({ field: 'value', text: '61' })),
    Story.message(Message.ClickedChart()),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'taken', entry: charted }) })),
    Story.Command.expectNone(),
    Story.message(older),
    Story.Command.expectExact(FetchChart()),
    Story.Command.resolve(FetchChart, newer),
    Story.model((m: Model) => assert.deepEqual(m.latest?.entries.map((e) => e.entry), ['test-entry-1'], 'the chart read after the write is the one shown')),
    Story.message(Message.Polled({ at: wallTime(5_002_000) })),
    park,
    Story.message(Message.ClickedClose()),
    Story.message(Message.ClickedReopen()),
    Story.Command.expectNone(),
    Story.message(newer),
    Story.Command.expectExact(FetchChart()),
    Story.Command.resolve(FetchChart, newer),
  )
})

test('close hides the chart screen and stops asking the service; reopen asks at once for the chart as it is now', () => {
  const asking = (model: Model) => subscriptions.poll.modelToDependencies(model).open
  Story.story(
    update,
    Story.given(loaded({ entries: [entry({ id: 'test-entry-1', vital: 'RR', value: 18, care: '06:56:30', charted: '06:57:15' })] })),
    Story.model((m: Model) => assert.equal(asking(m), true)),
    Story.message(Message.ClickedClose()),
    Story.model((m: Model) => {
      assert.equal(asking(m), false)
      assert.equal(m.latest, null, 'a closed screen holds no chart of its own')
    }),
    Story.message(Message.ClickedReopen()),
    Story.Command.expectExact(FetchChart()),
    Story.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ now: at('06:59:00') }), at: wallTime(9_000_000) })),
    Story.model((m: Model) => {
      assert.equal(asking(m), true)
      assert.equal(m.latest?.now, at('06:59:00'))
    }),
  )
  Scene.scene(
    { update, view },
    Scene.given(loaded()),
    Scene.click('#close'),
    Scene.expect(Scene.selector('#flowsheet')).toBeAbsent(),
    Scene.expect(Scene.selector('#closed')).toContainText('The chart is closed.'),
    Scene.click('#reopen'),
    Scene.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({}), at: wallTime(9_000_000) })),
    Scene.expect(Scene.selector('#flowsheet')).toExist(),
  )
})

test('when the chart service stops answering, the page says how long ago its view was current', () => {
  Scene.scene(
    { update, view },
    Scene.given(loaded()),
    Scene.expect(Scene.selector('#service')).toHaveText('The chart service is answering.'),
    Scene.Subscription.emit(Message.Polled({ at: wallTime(5_001_000) })),
    Scene.Command.resolve(FetchChart, Message.ChartUnanswered({ at: wallTime(5_001_200) })),
    Scene.Subscription.emit(Message.Polled({ at: wallTime(5_012_000) })),
    Scene.Command.resolve(FetchChart, Message.ChartUnanswered({ at: wallTime(5_012_300) })),
    Scene.expect(Scene.selector('#service')).toHaveText('The chart service is not answering. This view was current 12 s ago, at patient time 06:57:30.'),
    Scene.Subscription.emit(Message.Polled({ at: wallTime(5_013_000) })),
    Scene.Command.resolve(FetchChart, Message.GotChart({ chart: readModel({ now: at('06:57:43') }), at: wallTime(5_013_100) })),
    Scene.expect(Scene.selector('#service')).toHaveText('The chart service is answering.'),
  )
})
