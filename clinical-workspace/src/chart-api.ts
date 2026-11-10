import {
  type BootId,
  type DeviceId,
  type DeviceValue,
  isBootId,
  isDeviceId,
  isPatientId,
  isSequence,
  type Parsed,
  parseChannels,
  parseDeviceValue,
  type PatientId,
  type Sequence,
  type Source,
} from './device.ts'
import { isPatientTime, isWallTime, type PatientTime, type WallTime } from './time.ts'
import { isVital, type PerVital, type Vital } from './vitals.ts'
import { type EntryContent, type EntryKind, type KindRefusal, parseContent } from './entry-kinds.ts'

export type EntryId = string & { readonly __brand: 'EntryId' }
export type ClinicianId = string & { readonly __brand: 'ClinicianId' }
export type VersionNumber = number & { readonly __brand: 'VersionNumber' }
export type WriteId = string & { readonly __brand: 'WriteId' }

const isShortText = (value: unknown): value is string => typeof value === 'string' && value.length > 0 && value.length <= 200

export const isEntryId = (value: unknown): value is EntryId => isShortText(value)
export const isClinicianId = (value: unknown): value is ClinicianId => isShortText(value)
export const isWriteId = (value: unknown): value is WriteId => isShortText(value)
export const isVersionNumber = (value: unknown): value is VersionNumber =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 1
export type Discipline = 'nurse' | 'physician' | 'respiratory therapist'

export type Clinician = { readonly id: ClinicianId; readonly name: string; readonly discipline: Discipline }

function clinicianOf(id: string, name: string, discipline: Discipline): Clinician {
  if (!isClinicianId(id)) throw new Error(`${id} is not a clinician id`)
  return { id, name, discipline }
}

export const ROSTER: readonly Clinician[] = [
  clinicianOf('synthetic-nurse', 'Synthetic Nurse A', 'nurse'),
  clinicianOf('synthetic-physician', 'Synthetic Physician B', 'physician'),
  clinicianOf('synthetic-respiratory-therapist', 'Synthetic Respiratory Therapist C', 'respiratory therapist'),
]

export type CareTime = { readonly kind: 'now' } | { readonly kind: 'at'; readonly at: PatientTime }

export type NewEntry = EntryContent & {
  readonly operation: 'new entry'
  readonly writeId: WriteId
  readonly patient: PatientId
  readonly author: ClinicianId
  readonly care: CareTime
}

export type Amendment = EntryContent & {
  readonly operation: 'amendment'
  readonly writeId: WriteId
  readonly patient: PatientId
  readonly author: ClinicianId
  readonly entry: EntryId
  readonly replaces: VersionNumber
  readonly care: CareTime
}

export type EntryWrite = NewEntry | Amendment

export type NewEntryRequest = {
  readonly writeId: string
  readonly patient: string
  readonly author: string
  readonly kind: string
  readonly content: Readonly<Record<string, string>>
  readonly care: number | null
}

export type AmendmentRequest = NewEntryRequest & { readonly entry: string; readonly replaces: number }

export type Refusal =
  | KindRefusal
  | { readonly kind: 'entry kind differs' }
  | { readonly kind: 'amendment changes a fixed field'; readonly field: string }
  | { readonly kind: 'another patient' }
  | { readonly kind: 'care time after now' }
  | { readonly kind: 'no such entry' }
  | { readonly kind: 'not the current version' }
  | { readonly kind: 'write id reused with other content' }
  | { readonly kind: 'malformed'; readonly reason: string }

export type EntryVersion<K extends EntryKind = EntryKind> = EntryContent<K> & {
  readonly entry: EntryId
  readonly patient: PatientId
  readonly number: VersionNumber
  readonly care: PatientTime
  readonly charted: PatientTime
  readonly author: Clinician
}

export type EntryView<K extends EntryKind = EntryKind> = EntryVersion<K> & { readonly versions: number }

export type WriteResult = { readonly kind: 'taken'; readonly entry: EntryView } | { readonly kind: 'refused'; readonly refusal: Refusal }

export type RunView = {
  readonly specId: string
  readonly patient: PatientId
  readonly start: PatientTime
  readonly end: PatientTime
  readonly anchor: WallTime
}

export type ReadingView = {
  readonly device: DeviceId
  readonly boot: BootId
  readonly sequence: Sequence
  readonly vital: Vital
  readonly collected: PatientTime
  readonly available: PatientTime
  readonly arrived: PatientTime
  readonly value: DeviceValue
}

export type StatementView = {
  readonly device: DeviceId
  readonly boot: BootId
  readonly sequence: Sequence
  readonly at: PatientTime
  readonly arrived: PatientTime
  readonly channels: PerVital<Source>
}

export type MissingSpan = { readonly device: DeviceId; readonly vital: Vital; readonly from: PatientTime; readonly through: PatientTime }

export type ReadModel = {
  readonly run: RunView
  readonly now: PatientTime
  readonly entries: readonly EntryView[]
  readonly readings: readonly ReadingView[]
  readonly statements: readonly StatementView[]
  readonly missing: readonly MissingSpan[]
}

export const entriesOfKind = <K extends EntryKind>(model: ReadModel, kind: K): readonly Extract<EntryView, { kind: K }>[] =>
  model.entries.filter((entry): entry is Extract<EntryView, { kind: K }> => entry.kind === kind)

type Json = Readonly<Record<string, unknown>>

const DISCIPLINES: readonly unknown[] = ['nurse', 'physician', 'respiratory therapist'] satisfies Discipline[]
const REFUSALS: readonly unknown[] = [
  'unknown entry kind',
  'entry kind differs',
  'another patient',
  'care time after now',
  'no such entry',
  'not the current version',
  'write id reused with other content',
] satisfies Exclude<Refusal, { readonly reason: string } | { readonly field: string }>['kind'][]
class Malformed extends Error {
  readonly refusal: Refusal
  constructor(reason: string, refusal: Refusal = { kind: 'malformed', reason }) {
    super(reason)
    this.refusal = refusal
  }
}

const fail = (reason: string): never => {
  throw new Malformed(reason)
}

const isObject = (value: unknown): value is Json => typeof value === 'object' && value !== null && !Array.isArray(value)
const object = (value: unknown, what: string): Json => (isObject(value) ? value : fail(`${what} is not an object`))
const list = (value: unknown, what: string): readonly unknown[] => (Array.isArray(value) ? value : fail(`${what} is not a list`))

function text(json: Json, key: string): string {
  const value = json[key]
  return isShortText(value) ? value : fail(`${key} is not a short text`)
}

function branded<T>(json: Json, key: string, is: (value: unknown) => value is T, what: string): T {
  const value = json[key]
  return is(value) ? value : fail(`${key} is not ${what}`)
}

const time = (json: Json, key: string): PatientTime => branded(json, key, isPatientTime, 'a patient time')
const patientOf = (json: Json): PatientId => branded(json, 'patient', isPatientId, 'a patient id')
const entryOf = (json: Json, key: string): EntryId => branded(json, key, isEntryId, 'an entry id')
const versionOf = (json: Json, key: string): VersionNumber => branded(json, key, isVersionNumber, 'a version number')
const writeIdOf = (json: Json): WriteId => branded(json, 'writeId', isWriteId, 'a write id')

function count(json: Json, key: string): number {
  const value = json[key]
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 1 ? value : fail(`${key} is not a count from 1`)
}

const unwrap = <T>(parsed: Parsed<T>): T => (parsed.ok ? parsed.value : fail(parsed.reason))

function vital(json: Json): Vital {
  const value = json['vital']
  return isVital(value) ? value : fail(`vital ${String(value)} is not in the vitals table`)
}

function author(json: Json): ClinicianId {
  const id = branded(json, 'author', isClinicianId, 'a clinician id')
  return ROSTER.some((c) => c.id === id) ? id : fail(`author ${id} is not on the roster`)
}

function content(json: Json, from: 'write' | 'read'): EntryContent {
  const parsed = parseContent(json['kind'], json['content'], from)
  if (!parsed.ok) throw new Malformed(parsed.refusal.kind, parsed.refusal)
  return parsed.value
}

function care(json: Json): CareTime {
  return json['care'] === null ? { kind: 'now' } : { kind: 'at', at: time(json, 'care') }
}

function clinician(value: unknown): Clinician {
  const json = object(value, 'author')
  const discipline = json['discipline']
  const known = (d: unknown): d is Discipline => DISCIPLINES.includes(d)
  return {
    id: branded(json, 'id', isClinicianId, 'a clinician id'),
    name: text(json, 'name'),
    discipline: known(discipline) ? discipline : fail('an unknown discipline'),
  }
}

function entryVersion(value: unknown): EntryVersion {
  const json = object(value, 'an entry version')
  return {
    ...content(json, 'read'),
    entry: entryOf(json, 'entry'),
    patient: patientOf(json),
    number: versionOf(json, 'number'),
    care: time(json, 'care'),
    charted: time(json, 'charted'),
    author: clinician(json['author']),
  }
}

function entryView(value: unknown): EntryView {
  return { ...entryVersion(value), versions: count(object(value, 'an entry'), 'versions') }
}

function refusal(value: unknown): Refusal {
  const json = object(value, 'a refusal')
  const kind = json['kind']
  if (kind === 'malformed' || kind === 'content does not fit kind') return { kind, reason: text(json, 'reason') }
  if (kind === 'amendment changes a fixed field') return { kind, field: text(json, 'field') }
  const known = (k: unknown): k is Exclude<Refusal, { readonly reason: string } | { readonly field: string }>['kind'] => REFUSALS.includes(k)
  return known(kind) ? { kind } : fail(`refusal ${String(kind)} is not a kind the chart gives`)
}

function run(value: unknown): RunView {
  const json = object(value, 'the run')
  return {
    specId: text(json, 'specId'),
    patient: patientOf(json),
    start: time(json, 'start'),
    end: time(json, 'end'),
    anchor: branded(json, 'anchor', isWallTime, 'a wall-clock instant'),
  }
}

function reading(value: unknown): ReadingView {
  const json = object(value, 'a reading')
  return {
    device: branded(json, 'device', isDeviceId, 'a device id'),
    boot: branded(json, 'boot', isBootId, 'a boot id'),
    sequence: branded(json, 'sequence', isSequence, 'a sequence number'),
    vital: vital(json),
    collected: time(json, 'collected'),
    available: time(json, 'available'),
    arrived: time(json, 'arrived'),
    value: unwrap(parseDeviceValue(json['value'])),
  }
}

function statement(value: unknown): StatementView {
  const json = object(value, 'a channel statement')
  return {
    device: branded(json, 'device', isDeviceId, 'a device id'),
    boot: branded(json, 'boot', isBootId, 'a boot id'),
    sequence: branded(json, 'sequence', isSequence, 'a sequence number'),
    at: time(json, 'at'),
    arrived: time(json, 'arrived'),
    channels: unwrap(parseChannels(json['channels'])),
  }
}

function missing(value: unknown): MissingSpan {
  const json = object(value, 'a missing span')
  const from = time(json, 'from')
  const through = time(json, 'through')
  if (through < from) fail('a missing span ends before it starts')
  return { device: branded(json, 'device', isDeviceId, 'a device id'), vital: vital(json), from, through }
}

function parsing<T>(parse: () => T): Parsed<T> {
  try {
    return { ok: true, value: parse() }
  } catch (error) {
    if (error instanceof Malformed) return { ok: false, reason: error.message }
    throw error
  }
}

type ParsedWrite<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly reason: string; readonly refusal: Refusal }

function parsingWrite<T>(parse: () => T): ParsedWrite<T> {
  try {
    return { ok: true, value: parse() }
  } catch (error) {
    if (error instanceof Malformed) return { ok: false, reason: error.message, refusal: error.refusal }
    throw error
  }
}

export const parseNewEntry = (value: unknown): ParsedWrite<NewEntry> =>
  parsingWrite(() => {
    const json = object(value, 'a new entry')
    return { ...content(json, 'write'), operation: 'new entry', writeId: writeIdOf(json), patient: patientOf(json), author: author(json), care: care(json) }
  })

export const parseAmendment = (value: unknown): ParsedWrite<Amendment> =>
  parsingWrite(() => {
    const json = object(value, 'an amendment')
    return {
      ...content(json, 'write'),
      operation: 'amendment',
      writeId: writeIdOf(json),
      patient: patientOf(json),
      author: author(json),
      entry: entryOf(json, 'entry'),
      replaces: versionOf(json, 'replaces'),
      care: care(json),
    }
  })

export const parseWriteResult = (value: unknown): Parsed<WriteResult> =>
  parsing(() => {
    const json = object(value, 'a write result')
    if (json['kind'] === 'taken') return { kind: 'taken', entry: entryView(json['entry']) }
    if (json['kind'] === 'refused') return { kind: 'refused', refusal: refusal(json['refusal']) }
    return fail('a write result is taken or refused')
  })

export const parseEntries = (value: unknown): Parsed<readonly EntryView[]> => parsing(() => list(value, 'entries').map(entryView))

export const parseVersions = (value: unknown): Parsed<readonly EntryVersion[]> => parsing(() => list(value, 'versions').map(entryVersion))

export const parseRun = (value: unknown): Parsed<RunView> => parsing(() => run(value))

export const parseReadModel = (value: unknown): Parsed<ReadModel> =>
  parsing(() => {
    const json = object(value, 'the read model')
    return {
      run: run(json['run']),
      now: time(json, 'now'),
      entries: list(json['entries'], 'entries').map(entryView),
      readings: list(json['readings'], 'readings').map(reading),
      statements: list(json['statements'], 'statements').map(statement),
      missing: list(json['missing'], 'missing').map(missing),
    }
  })
