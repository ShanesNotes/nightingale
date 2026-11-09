import assert from 'node:assert/strict'
import { test } from 'node:test'
import { Scene, Story } from 'foldkit'
import { FetchChart, init, Message, type Model, PostEntry, update } from '../src/workspace/program.ts'
import { makeView, view } from '../src/workspace/view.ts'
import { wallTime } from '../src/time.ts'
import { VITALS } from '../src/vitals.ts'
import { at, entry, reading, readModel, statement, THERAPIST, written } from './fixtures.ts'

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
    Story.message(Message.ChoseVital({ vital: 'RR' })),
    Story.message(Message.TypedValue({ text: '18' })),
    Story.message(Message.TypedCare({ text: '06:56:10' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ body: { patient: 'test-patient', author: THERAPIST.id, vital: 'RR', value: '18', care: at('06:56:10') } })),
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
    Story.message(Message.TypedValue({ text: '61' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectExact(PostEntry({ body: { patient: 'test-patient', author: 'synthetic-nurse', vital: 'HR', value: '61', care: null } })),
    park,
  )
})

test('a refusal shows in words, and a write the service does not answer is refused in words and never queued', () => {
  Story.story(
    update,
    Story.given(loaded()),
    Story.message(Message.TypedValue({ text: '61' })),
    Story.message(Message.TypedCare({ text: '06:59:00' })),
    Story.message(Message.ClickedChart()),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'refused', refusal: { kind: 'care time after now' } }) })),
    notice('The care time is after now; nothing was charted.'),
    Story.message(Message.TypedValue({ text: '' })),
    Story.message(Message.ClickedChart()),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: written({ kind: 'refused', refusal: { kind: 'not a finite number' } }) })),
    notice('The value is not a number; nothing was charted.'),
    Story.message(Message.ClickedChart()),
    Story.Command.resolve(PostEntry, Message.WriteUnanswered()),
    notice('The chart service did not answer; nothing was charted.'),
    Story.Command.expectNone(),
    Story.message(Message.TypedCare({ text: '6:59' })),
    Story.message(Message.ClickedChart()),
    Story.Command.expectNone(),
    notice('The care time is not HH:MM:SS; nothing was charted.'),
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
