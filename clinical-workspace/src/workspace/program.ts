import { Effect, Schema, Stream } from 'effect'
import { Command, Subscription, type Update } from 'foldkit'
import { defineMessageUnion } from 'foldkit/message'
import {
  type EntryVersion,
  isEntryId,
  parseReadModel,
  parseVersions,
  parseWriteResult,
  type ReadModel,
  type Refusal,
  ROSTER,
  type WriteResult,
} from '../chart-api.ts'
import { formatClock, isWallTime, parseClock, wallTime } from '../time.ts'
import { VITALS } from '../vitals.ts'
import * as client from './client.ts'

const ReadModelSchema = Schema.declare((value: unknown): value is ReadModel => parseReadModel(value).ok)
const WriteResultSchema = Schema.declare((value: unknown): value is WriteResult => parseWriteResult(value).ok)
const VersionsSchema = Schema.declare((value: unknown): value is readonly EntryVersion[] => parseVersions(value).ok)
const WallTimeSchema = Schema.declare(isWallTime)

const NewEntryBody = Schema.Struct({
  patient: Schema.String,
  author: Schema.String,
  vital: Schema.String,
  value: Schema.String,
  care: Schema.NullOr(Schema.Number),
})

const AmendmentBody = Schema.Struct({
  patient: Schema.String,
  author: Schema.String,
  entry: Schema.String,
  replaces: Schema.Number,
  value: Schema.String,
  care: Schema.NullOr(Schema.Number),
})

export const Model = Schema.Struct({
  screen: Schema.Literals(['open', 'closed']),
  latest: Schema.NullOr(ReadModelSchema),
  answering: Schema.Boolean,
  heard: Schema.NullOr(WallTimeSchema),
  checked: Schema.NullOr(WallTimeSchema),
  fetching: Schema.Boolean,
  form: Schema.Struct({ author: Schema.String, vital: Schema.String, value: Schema.String, care: Schema.String }),
  notice: Schema.String,
  selected: Schema.NullOr(Schema.String),
  versions: Schema.NullOr(Schema.Struct({ entry: Schema.String, list: VersionsSchema })),
  editing: Schema.NullOr(Schema.Struct({ value: Schema.String, care: Schema.String })),
})
export type Model = typeof Model.Type

export const Message = defineMessageUnion({
  Polled: { at: WallTimeSchema },
  GotChart: { chart: ReadModelSchema, at: WallTimeSchema },
  ChartUnanswered: { at: WallTimeSchema },
  ChoseAuthor: { author: Schema.String },
  ChoseVital: { vital: Schema.String },
  TypedValue: { text: Schema.String },
  TypedCare: { text: Schema.String },
  ClickedChart: {},
  Wrote: { result: WriteResultSchema },
  WriteUnanswered: {},
  ClickedEntry: { entry: Schema.String },
  ClickedVersions: {},
  GotVersions: { entry: Schema.String, versions: VersionsSchema },
  VersionsUnanswered: {},
  ClickedEdit: {},
  TypedEditValue: { text: Schema.String },
  TypedEditCare: { text: Schema.String },
  ClickedSaveEdit: {},
  ClickedClose: {},
  ClickedReopen: {},
})
export type Message = typeof Message.Type

type Return = Update.Return<Model, Message>

export const FetchChart = Command.define('FetchChart', {
  messages: [Message.GotChart, Message.ChartUnanswered],
  execute: Effect.promise(async () => {
    const chart = await client.chart()
    const at = wallTime(Date.now())
    return chart === undefined ? Message.ChartUnanswered({ at }) : Message.GotChart({ chart, at })
  }),
})

export const PostEntry = Command.define('PostEntry', {
  args: { body: NewEntryBody },
  messages: [Message.Wrote, Message.WriteUnanswered],
  execute: ({ body }) =>
    Effect.promise(() => client.write('/api/entries', body)).pipe(
      Effect.map((result) => (result === undefined ? Message.WriteUnanswered() : Message.Wrote({ result }))),
    ),
})

export const PostAmendment = Command.define('PostAmendment', {
  args: { body: AmendmentBody },
  messages: [Message.Wrote, Message.WriteUnanswered],
  execute: ({ body }) =>
    Effect.promise(() => client.write('/api/amendments', body)).pipe(
      Effect.map((result) => (result === undefined ? Message.WriteUnanswered() : Message.Wrote({ result }))),
    ),
})

export const FetchVersions = Command.define('FetchVersions', {
  args: { entry: Schema.String },
  messages: [Message.GotVersions, Message.VersionsUnanswered],
  execute: ({ entry }) =>
    Effect.promise(async () => (isEntryId(entry) ? client.versions(entry) : undefined)).pipe(
      Effect.map((versions) => (versions === undefined ? Message.VersionsUnanswered() : Message.GotVersions({ entry, versions }))),
    ),
})

export const subscriptions = Subscription.make<Model, Message>()((entry) => ({
  poll: entry(
    { open: Schema.Boolean },
    {
      modelToDependencies: (model) => ({ open: model.screen === 'open' }),
      dependenciesToStream: ({ open }) =>
        open ? Stream.tick('1 second').pipe(Stream.map(() => Message.Polled({ at: wallTime(Date.now()) }))) : Stream.empty,
    },
  ),
}))

const REFUSED: Readonly<Record<Refusal['kind'], string>> = {
  'another patient': 'That write names another patient; nothing was charted.',
  'care time after now': 'The care time is after now; nothing was charted.',
  'not a finite number': 'The value is not a number; nothing was charted.',
  'no such entry': 'That entry does not exist; nothing was charted.',
  'not the current version': 'Someone charted a newer version first; nothing was charted.',
  malformed: 'The chart service could not read the write; nothing was charted.',
}

const UNANSWERED = 'The chart service did not answer; nothing was charted.'

export function init(): Return {
  return {
    model: {
      screen: 'open',
      latest: null,
      answering: false,
      heard: null,
      checked: null,
      fetching: false,
      form: { author: ROSTER[0]?.id ?? '', vital: VITALS[0].vital, value: '', care: '' },
      notice: '',
      selected: null,
      versions: null,
      editing: null,
    },
  }
}

function careTime(text: string): { readonly ok: true; readonly care: number | null } | { readonly ok: false } {
  const trimmed = text.trim()
  if (trimmed === '') return { ok: true, care: null }
  const time = parseClock(trimmed)
  return time === undefined ? { ok: false } : { ok: true, care: time }
}

function wrote(model: Model, result: WriteResult): Return {
  if (result.kind === 'refused') return { model: { ...model, notice: REFUSED[result.refusal.kind] } }
  const { entry } = result
  const unit = VITALS.find((row) => row.vital === entry.vital)?.unit ?? ''
  const did = entry.number > 1 ? 'Amended' : 'Charted'
  return {
    model: {
      ...model,
      notice: `${did} ${entry.vital} ${entry.value} ${unit} as ${entry.author.name}.`,
      form: { ...model.form, value: '', care: '' },
      editing: null,
      versions: null,
    },
    commands: [FetchChart()],
  }
}

export function update(model: Model, message: Message): Return {
  return Message.match<Return>(message, {
    Polled: ({ at }) => (model.fetching ? { model: { ...model, checked: at } } : { model: { ...model, checked: at, fetching: true }, commands: [FetchChart()] }),
    GotChart: ({ chart, at }) => {
      if (model.screen === 'closed') return { model: { ...model, fetching: false } }
      const stale = model.latest !== null && chart.now < model.latest.now
      return { model: { ...model, latest: stale ? model.latest : chart, answering: true, heard: at, checked: at, fetching: false } }
    },
    ChartUnanswered: ({ at }) => ({ model: { ...model, answering: false, checked: at, fetching: false } }),
    ChoseAuthor: ({ author }) => ({ model: { ...model, form: { ...model.form, author } } }),
    ChoseVital: ({ vital }) => ({ model: { ...model, form: { ...model.form, vital } } }),
    TypedValue: ({ text }) => ({ model: { ...model, form: { ...model.form, value: text } } }),
    TypedCare: ({ text }) => ({ model: { ...model, form: { ...model.form, care: text } } }),
    ClickedChart: () => {
      if (model.latest === null) return { model: { ...model, notice: 'The chart has not loaded yet; nothing was charted.' } }
      const care = careTime(model.form.care)
      if (!care.ok) return { model: { ...model, notice: 'The care time is not HH:MM:SS; nothing was charted.' } }
      const { author, vital, value } = model.form
      return {
        model: { ...model, notice: 'Charting.' },
        commands: [PostEntry({ body: { patient: model.latest.run.patient, author, vital, value, care: care.care } })],
      }
    },
    Wrote: ({ result }) => wrote(model, result),
    WriteUnanswered: () => ({ model: { ...model, notice: UNANSWERED } }),
    ClickedEntry: ({ entry }) => ({ model: { ...model, selected: entry, versions: null, editing: null } }),
    ClickedVersions: () => (model.selected === null ? { model } : { model, commands: [FetchVersions({ entry: model.selected })] }),
    GotVersions: ({ entry, versions }) => ({ model: { ...model, versions: { entry, list: versions } } }),
    VersionsUnanswered: () => ({ model: { ...model, notice: 'The chart service did not answer; the earlier versions are not shown.' } }),
    ClickedEdit: () => {
      const current = selectedEntry(model)
      return current === undefined ? { model } : { model: { ...model, editing: { value: String(current.value), care: formatClock(current.care) } } }
    },
    TypedEditValue: ({ text }) => (model.editing === null ? { model } : { model: { ...model, editing: { ...model.editing, value: text } } }),
    TypedEditCare: ({ text }) => (model.editing === null ? { model } : { model: { ...model, editing: { ...model.editing, care: text } } }),
    ClickedSaveEdit: () => {
      const current = selectedEntry(model)
      if (current === undefined || model.editing === null || model.latest === null) return { model }
      const care = careTime(model.editing.care)
      if (!care.ok) return { model: { ...model, notice: 'The care time is not HH:MM:SS; nothing was charted.' } }
      const body = {
        patient: model.latest.run.patient,
        author: model.form.author,
        entry: current.entry,
        replaces: current.number,
        value: model.editing.value,
        care: care.care,
      }
      return { model: { ...model, notice: 'Amending.' }, commands: [PostAmendment({ body })] }
    },
    ClickedClose: () => ({ model: { ...model, screen: 'closed', latest: null, selected: null, versions: null, editing: null, notice: '' } }),
    ClickedReopen: () => ({ model: { ...model, screen: 'open', fetching: true }, commands: [FetchChart()] }),
  })
}

export const selectedEntry = (model: Model) => model.latest?.entries.find((e) => e.entry === model.selected)
