import { Effect, Equal, Schema, Stream } from 'effect'
import { Command, Subscription, type Update } from 'foldkit'
import { defineMessageUnion } from 'foldkit/message'
import {
  type EntryVersion,
  type EntryView,
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
import { blankContent, type EntryKind, isEntryKind } from '../entry-kinds.ts'
import { contentText } from './content.ts'
import * as client from './client.ts'

const ReadModelSchema = Schema.declare((value: unknown): value is ReadModel => parseReadModel(value).ok)
const WriteResultSchema = Schema.declare((value: unknown): value is WriteResult => parseWriteResult(value).ok)
const VersionsSchema = Schema.declare((value: unknown): value is readonly EntryVersion[] => parseVersions(value).ok)
const WallTimeSchema = Schema.declare(isWallTime)
const KindSchema = Schema.declare(isEntryKind)
const Fields = Schema.Record(Schema.String, Schema.String)
const Draft = Schema.Struct({ content: Fields, care: Schema.String })

const NewEntryRequest = Schema.Struct({
  patient: Schema.String,
  author: Schema.String,
  kind: KindSchema,
  content: Fields,
  care: Schema.NullOr(Schema.Number),
})

const AmendmentRequest = Schema.Struct({
  patient: Schema.String,
  author: Schema.String,
  entry: Schema.String,
  replaces: Schema.Number,
  kind: KindSchema,
  content: Fields,
  care: Schema.NullOr(Schema.Number),
})

const Write = Schema.Union([
  Schema.Struct({ kind: Schema.Literal('new entry'), request: NewEntryRequest }),
  Schema.Struct({ kind: Schema.Literal('amendment'), request: AmendmentRequest }),
])
type Write = typeof Write.Type

export const Model = Schema.Struct({
  screen: Schema.Literals(['open', 'closed']),
  latest: Schema.NullOr(ReadModelSchema),
  answering: Schema.Boolean,
  heard: Schema.NullOr(WallTimeSchema),
  checked: Schema.NullOr(WallTimeSchema),
  read: Schema.Literals(['idle', 'in flight', 'in flight, another due']),
  author: Schema.String,
  drafts: Schema.Struct({ vital: Draft, note: Draft }),
  notice: Schema.Struct({ about: KindSchema, text: Schema.String }),
  selected: Schema.NullOr(Schema.String),
  versions: Schema.NullOr(Schema.Struct({ entry: Schema.String, list: VersionsSchema })),
  editing: Schema.NullOr(Schema.Struct({ entry: Schema.String, replaces: Schema.Number, kind: KindSchema, content: Fields, care: Schema.String })),
  writing: Schema.NullOr(Write),
  unanswered: Schema.NullOr(Schema.Struct({ writeId: Schema.String, write: Write })),
})
export type Model = typeof Model.Type

export const Message = defineMessageUnion({
  Polled: { at: WallTimeSchema },
  GotChart: { chart: ReadModelSchema, at: WallTimeSchema },
  ChartUnanswered: { at: WallTimeSchema },
  ChoseAuthor: { author: Schema.String },
  TypedField: { kind: KindSchema, field: Schema.String, text: Schema.String },
  TypedCare: { kind: KindSchema, text: Schema.String },
  ClickedChart: { kind: KindSchema },
  Wrote: { result: WriteResultSchema },
  WriteUnanswered: { writeId: Schema.String },
  ClickedEntry: { entry: Schema.String },
  ClickedVersions: { entry: Schema.String },
  GotVersions: { entry: Schema.String, versions: VersionsSchema },
  VersionsUnanswered: { entry: Schema.String },
  ClickedEdit: { entry: Schema.String },
  TypedEditField: { field: Schema.String, text: Schema.String },
  TypedEditCare: { text: Schema.String },
  ClickedSaveEdit: {},
  ClickedCancelEdit: {},
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
  args: { request: NewEntryRequest, writeId: Schema.NullOr(Schema.String) },
  messages: [Message.Wrote, Message.WriteUnanswered],
  execute: ({ request, writeId }) => posting((id) => client.write('/api/entries', { ...request, writeId: id }), writeId),
})

export const PostAmendment = Command.define('PostAmendment', {
  args: { request: AmendmentRequest, writeId: Schema.NullOr(Schema.String) },
  messages: [Message.Wrote, Message.WriteUnanswered],
  execute: ({ request, writeId }) => posting((id) => client.write('/api/amendments', { ...request, writeId: id }), writeId),
})

export const FetchVersions = Command.define('FetchVersions', {
  args: { entry: Schema.String },
  messages: [Message.GotVersions, Message.VersionsUnanswered],
  execute: ({ entry }) =>
    Effect.promise(async () => (isEntryId(entry) ? client.versions(entry) : undefined)).pipe(
      Effect.map((versions) => (versions === undefined ? Message.VersionsUnanswered({ entry }) : Message.GotVersions({ entry, versions }))),
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

function refusedWords(refusal: Refusal): string {
  switch (refusal.kind) {
    case 'unknown entry kind':
      return 'The chart does not know this kind of entry; nothing was charted.'
    case 'content does not fit kind':
      return `${refusal.reason}; nothing was charted.`
    case 'entry kind differs':
      return 'An amendment must keep the entry kind; nothing was charted.'
    case 'amendment changes a fixed field':
      return `An amendment cannot change the ${refusal.field} of the entry it amends; nothing was charted.`
    case 'another patient':
      return 'That write names another patient; nothing was charted.'
    case 'care time after now':
      return 'The care time is after now; nothing was charted.'
    case 'no such entry':
      return 'That entry does not exist; nothing was charted.'
    case 'not the current version':
      return 'Someone charted a newer version first; nothing was charted.'
    case 'write id reused with other content':
      return 'The chart holds another write under this write id; nothing was charted.'
    case 'malformed':
      return 'The chart service could not read the write; nothing was charted.'
  }
}

const UNANSWERED: Readonly<Record<Write['kind'], string>> = {
  'new entry': 'The chart service did not answer, so the entry may already be in the chart. Charting it again unchanged cannot chart it twice.',
  amendment: 'The chart service did not answer, so the new version may already be in the chart. Saving it again unchanged cannot chart it twice.',
}

const blankDraft = (kind: EntryKind) => ({ content: blankContent(kind), care: '' })

export function init(): Return {
  return {
    model: {
      screen: 'open',
      latest: null,
      answering: false,
      heard: null,
      checked: null,
      read: 'idle',
      author: ROSTER[0]?.id ?? '',
      drafts: { vital: blankDraft('vital'), note: blankDraft('note') },
      notice: { about: 'vital', text: '' },
      selected: null,
      versions: null,
      editing: null,
      writing: null,
      unanswered: null,
    },
  }
}

const told = (model: Model, about: EntryKind, text: string): Model => ({ ...model, notice: { about, text } })

const redrafted = (model: Model, kind: EntryKind, draft: Partial<Model['drafts'][EntryKind]>): Model => ({
  ...model,
  drafts: { ...model.drafts, [kind]: { ...model.drafts[kind], ...draft } },
})

function careTime(text: string): { readonly ok: true; readonly care: number | null } | { readonly ok: false } {
  const trimmed = text.trim()
  if (trimmed === '') return { ok: true, care: null }
  const time = parseClock(trimmed)
  return time === undefined ? { ok: false } : { ok: true, care: time }
}

function send(model: Model, write: Write, notice: string): Return {
  const { unanswered } = model
  const writeId = unanswered !== null && Equal.equals(unanswered.write, write) ? unanswered.writeId : null
  const command = write.kind === 'new entry' ? PostEntry({ request: write.request, writeId }) : PostAmendment({ request: write.request, writeId })
  return { model: { ...told(model, write.request.kind, notice), writing: write }, commands: [command] }
}

const reading = (model: Model): Return => ({ model: { ...model, read: 'in flight' }, commands: [FetchChart()] })

function readChart(model: Model): Return {
  if (model.screen === 'closed') return { model }
  return model.read === 'idle' ? reading(model) : { model: { ...model, read: 'in flight, another due' } }
}

function readAnswered(model: Model): Return {
  const idle: Model = { ...model, read: 'idle' }
  return model.read === 'in flight, another due' ? readChart(idle) : { model: idle }
}

function wrote(model: Model, write: Write, result: WriteResult): Return {
  const { kind } = write.request
  if (result.kind === 'refused') return { model: told(model, kind, refusedWords(result.refusal)) }
  const { entry } = result
  const did = entry.number > 1 ? 'Amended' : 'Charted'
  const answered = write.kind === 'new entry' ? redrafted(model, kind, blankDraft(kind)) : { ...model, editing: null }
  return readChart({ ...told(answered, kind, `${did} ${entry.kind === 'note' ? 'note' : contentText(entry)} as ${entry.author.name}.`), versions: null })
}

const entryOf = (model: Model, entry: string): EntryView | undefined => model.latest?.entries.find((e) => e.entry === entry)

const DRAFTING: ReadonlySet<Message['_tag']> = new Set([
  'ChoseAuthor',
  'TypedField',
  'TypedCare',
  'ClickedChart',
  'ClickedEdit',
  'TypedEditField',
  'TypedEditCare',
  'ClickedSaveEdit',
  'ClickedCancelEdit',
])

const UNREADABLE_CARE = 'The care time is not HH:MM:SS; nothing was charted.'

export function update(model: Model, message: Message): Return {
  if (model.writing !== null && DRAFTING.has(message._tag)) return { model }
  return Message.match<Return>(message, {
    Polled: ({ at }) => (model.read === 'idle' ? reading({ ...model, checked: at }) : { model: { ...model, checked: at } }),
    GotChart: ({ chart, at }) => readAnswered(model.screen === 'closed' ? model : { ...model, latest: chart, answering: true, heard: at, checked: at }),
    ChartUnanswered: ({ at }) => readAnswered({ ...model, answering: false, checked: at }),
    ChoseAuthor: ({ author }) => ({ model: { ...model, author } }),
    TypedField: ({ kind, field, text }) => ({ model: redrafted(model, kind, { content: { ...model.drafts[kind].content, [field]: text } }) }),
    TypedCare: ({ kind, text }) => ({ model: redrafted(model, kind, { care: text }) }),
    ClickedChart: ({ kind }) => {
      if (model.latest === null) return { model: told(model, kind, 'The chart has not loaded yet; nothing was charted.') }
      const { content, care } = model.drafts[kind]
      const when = careTime(care)
      if (!when.ok) return { model: told(model, kind, UNREADABLE_CARE) }
      return send(model, { kind: 'new entry', request: { patient: model.latest.run.patient, author: model.author, kind, content, care: when.care } }, 'Charting.')
    },
    Wrote: ({ result }) => (model.writing === null ? { model } : wrote({ ...model, writing: null, unanswered: null }, model.writing, result)),
    WriteUnanswered: ({ writeId }) =>
      model.writing === null
        ? { model }
        : { model: { ...told(model, model.writing.request.kind, UNANSWERED[model.writing.kind]), writing: null, unanswered: { writeId, write: model.writing } } },
    ClickedEntry: ({ entry }) => ({ model: { ...model, selected: entry, versions: null, editing: null } }),
    ClickedVersions: ({ entry }) => (model.versions?.entry === entry ? { model: { ...model, versions: null } } : { model, commands: [FetchVersions({ entry })] }),
    GotVersions: ({ entry, versions }) => ({ model: { ...model, versions: { entry, list: versions } } }),
    VersionsUnanswered: ({ entry }) => ({ model: told(model, entryOf(model, entry)?.kind ?? model.notice.about, 'The chart service did not answer; the earlier versions are not shown.') }),
    ClickedEdit: ({ entry }) => {
      const current = entryOf(model, entry)
      if (current === undefined) return { model }
      const content = Object.fromEntries(Object.entries(current.content).map(([field, value]) => [field, String(value)]))
      return { model: { ...model, editing: { entry: current.entry, replaces: current.number, kind: current.kind, content, care: formatClock(current.care) } } }
    },
    TypedEditField: ({ field, text }) => (model.editing === null ? { model } : { model: { ...model, editing: { ...model.editing, content: { ...model.editing.content, [field]: text } } } }),
    TypedEditCare: ({ text }) => (model.editing === null ? { model } : { model: { ...model, editing: { ...model.editing, care: text } } }),
    ClickedSaveEdit: () => {
      if (model.editing === null || model.latest === null) return { model }
      const { entry, replaces, kind, content, care } = model.editing
      const when = careTime(care)
      if (!when.ok) return { model: told(model, kind, UNREADABLE_CARE) }
      return send(model, { kind: 'amendment', request: { patient: model.latest.run.patient, author: model.author, entry, replaces, kind, content, care: when.care } }, 'Amending.')
    },
    ClickedCancelEdit: () => ({ model: { ...model, editing: null } }),
    ClickedClose: () => ({ model: { ...told(model, model.notice.about, ''), screen: 'closed', latest: null, selected: null, versions: null, editing: null } }),
    ClickedReopen: () => readChart({ ...model, screen: 'open' }),
  })
}

export const selectedEntry = (model: Model) => (model.selected === null ? undefined : entryOf(model, model.selected))
