import { Effect, Equal, Schema, Stream } from 'effect'
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

const Write = Schema.Union([
  Schema.Struct({ kind: Schema.Literal('new entry'), body: NewEntryBody }),
  Schema.Struct({ kind: Schema.Literal('amendment'), body: AmendmentBody }),
])
type Write = typeof Write.Type

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
  editing: Schema.NullOr(Schema.Struct({ entry: Schema.String, replaces: Schema.Number, value: Schema.String, care: Schema.String })),
  writing: Schema.NullOr(Write),
  unanswered: Schema.NullOr(Schema.Struct({ writeId: Schema.String, write: Write })),
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
  WriteUnanswered: { writeId: Schema.String },
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

const posting = (post: (writeId: string) => Promise<WriteResult | undefined>, writeId: string | null) =>
  Effect.promise(async () => {
    const id = writeId ?? crypto.randomUUID()
    const result = await post(id)
    return result === undefined ? Message.WriteUnanswered({ writeId: id }) : Message.Wrote({ result })
  })

export const PostEntry = Command.define('PostEntry', {
  args: { body: NewEntryBody, writeId: Schema.NullOr(Schema.String) },
  messages: [Message.Wrote, Message.WriteUnanswered],
  execute: ({ body, writeId }) => posting((id) => client.write('/api/entries', { ...body, writeId: id }), writeId),
})

export const PostAmendment = Command.define('PostAmendment', {
  args: { body: AmendmentBody, writeId: Schema.NullOr(Schema.String) },
  messages: [Message.Wrote, Message.WriteUnanswered],
  execute: ({ body, writeId }) => posting((id) => client.write('/api/amendments', { ...body, writeId: id }), writeId),
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
  'write id reused with other content': 'The chart holds another write under this write id; nothing was charted.',
  malformed: 'The chart service could not read the write; nothing was charted.',
}

const UNANSWERED: Readonly<Record<Write['kind'], string>> = {
  'new entry': 'The chart service did not answer, so the entry may already be in the chart. Charting it again unchanged cannot chart it twice.',
  amendment: 'The chart service did not answer, so the new version may already be in the chart. Saving it again unchanged cannot chart it twice.',
}

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
      writing: null,
      unanswered: null,
    },
  }
}

function careTime(text: string): { readonly ok: true; readonly care: number | null } | { readonly ok: false } {
  const trimmed = text.trim()
  if (trimmed === '') return { ok: true, care: null }
  const time = parseClock(trimmed)
  return time === undefined ? { ok: false } : { ok: true, care: time }
}

function send(model: Model, write: Write, notice: string): Return {
  const { unanswered } = model
  const writeId = unanswered !== null && Equal.equals(unanswered.write, write) ? unanswered.writeId : null
  const command = write.kind === 'new entry' ? PostEntry({ body: write.body, writeId }) : PostAmendment({ body: write.body, writeId })
  return { model: { ...model, notice, writing: write }, commands: [command] }
}

function wrote(model: Model, write: Write, result: WriteResult): Return {
  if (result.kind === 'refused') return { model: { ...model, notice: REFUSED[result.refusal.kind] } }
  const { entry } = result
  const unit = VITALS.find((row) => row.vital === entry.vital)?.unit ?? ''
  const did = entry.number > 1 ? 'Amended' : 'Charted'
  const answered = write.kind === 'new entry' ? { form: { ...model.form, value: '', care: '' } } : { editing: null }
  return {
    model: { ...model, ...answered, notice: `${did} ${entry.vital} ${entry.value} ${unit} as ${entry.author.name}.`, versions: null },
    commands: [FetchChart()],
  }
}

const DRAFTING: ReadonlySet<Message['_tag']> = new Set([
  'ChoseAuthor',
  'ChoseVital',
  'TypedValue',
  'TypedCare',
  'ClickedChart',
  'ClickedEdit',
  'TypedEditValue',
  'TypedEditCare',
  'ClickedSaveEdit',
])

export function update(model: Model, message: Message): Return {
  if (model.writing !== null && DRAFTING.has(message._tag)) return { model }
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
      return send(model, { kind: 'new entry', body: { patient: model.latest.run.patient, author, vital, value, care: care.care } }, 'Charting.')
    },
    Wrote: ({ result }) => (model.writing === null ? { model } : wrote({ ...model, writing: null, unanswered: null }, model.writing, result)),
    WriteUnanswered: ({ writeId }) =>
      model.writing === null
        ? { model }
        : { model: { ...model, writing: null, unanswered: { writeId, write: model.writing }, notice: UNANSWERED[model.writing.kind] } },
    ClickedEntry: ({ entry }) => ({ model: { ...model, selected: entry, versions: null, editing: null } }),
    ClickedVersions: () => (model.selected === null ? { model } : { model, commands: [FetchVersions({ entry: model.selected })] }),
    GotVersions: ({ entry, versions }) => ({ model: { ...model, versions: { entry, list: versions } } }),
    VersionsUnanswered: () => ({ model: { ...model, notice: 'The chart service did not answer; the earlier versions are not shown.' } }),
    ClickedEdit: () => {
      const current = selectedEntry(model)
      if (current === undefined) return { model }
      return { model: { ...model, editing: { entry: current.entry, replaces: current.number, value: String(current.value), care: formatClock(current.care) } } }
    },
    TypedEditValue: ({ text }) => (model.editing === null ? { model } : { model: { ...model, editing: { ...model.editing, value: text } } }),
    TypedEditCare: ({ text }) => (model.editing === null ? { model } : { model: { ...model, editing: { ...model.editing, care: text } } }),
    ClickedSaveEdit: () => {
      if (model.editing === null || model.latest === null) return { model }
      const care = careTime(model.editing.care)
      if (!care.ok) return { model: { ...model, notice: 'The care time is not HH:MM:SS; nothing was charted.' } }
      const { entry, replaces, value } = model.editing
      return send(model, { kind: 'amendment', body: { patient: model.latest.run.patient, author: model.form.author, entry, replaces, value, care: care.care } }, 'Amending.')
    },
    ClickedClose: () => ({ model: { ...model, screen: 'closed', latest: null, selected: null, versions: null, editing: null, notice: '' } }),
    ClickedReopen: () => ({ model: { ...model, screen: 'open', fetching: true }, commands: [FetchChart()] }),
  })
}

export const selectedEntry = (model: Model) => model.latest?.entries.find((e) => e.entry === model.selected)
