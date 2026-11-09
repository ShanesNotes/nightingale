import assert from 'node:assert/strict'
import { test } from 'node:test'
import { Story } from 'foldkit'
import { FetchChart, init, Message, type Model, PostEntry, update } from '../src/workspace/program.ts'
import { wallTime } from '../src/time.ts'
import { at, entry, readModel, THERAPIST } from './fixtures.ts'

type Sim = Parameters<Extract<Parameters<typeof Story.story>[1], (s: never) => unknown>>[0]
const park = (sim: Sim) => ({ ...sim, commands: [] })

const loaded = (overrides: Readonly<Record<string, unknown>> = {}): Model =>
  update(init().model, Message.GotChart({ chart: readModel(overrides), at: wallTime(5_000_000) })).model

const notice = (expected: string | RegExp) => Story.model((m: Model) => assert.match(m.notice, typeof expected === 'string' ? new RegExp(expected) : expected))

test('a clinician of any discipline charts any vital: the page sends what was typed, then shows the chart as the service now has it', () => {
  const taken = { kind: 'taken', entry: { ...entry({ id: 'test-entry-1', vital: 'RR', value: 18, care: '06:56:10', charted: '06:57:30' }), author: THERAPIST } }
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
    Story.Command.resolve(PostEntry, Message.Wrote({ result: { kind: 'refused', refusal: { kind: 'care time after now' } } })),
    notice('The care time is after now; nothing was charted.'),
    Story.message(Message.TypedValue({ text: '' })),
    Story.message(Message.ClickedChart()),
    Story.Command.resolve(PostEntry, Message.Wrote({ result: { kind: 'refused', refusal: { kind: 'not a finite number' } } })),
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
