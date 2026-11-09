import {
  type BootId,
  type DeviceId,
  type DeviceValue,
  type Parsed,
  parseChannels,
  parseDeviceValue,
  type PatientId,
  type Sequence,
  type Source,
} from './device.ts'
import type { PatientTime, WallTime } from './time.ts'
import { isVital, type PerVital, type Vital } from './vitals.ts'

// The chart service's JSON contract with the workspace. Both ends parse what they receive with these parsers.
export type EntryId = string
export type ClinicianId = string
export type VersionNumber = number
export type Discipline = 'nurse' | 'physician' | 'respiratory therapist'

export type Clinician = { readonly id: ClinicianId; readonly name: string; readonly discipline: Discipline }

export const ROSTER = [
  { id: 'synthetic-nurse', name: 'Synthetic Nurse A', discipline: 'nurse' },
  { id: 'synthetic-physician', name: 'Synthetic Physician B', discipline: 'physician' },
  { id: 'synthetic-respiratory-therapist', name: 'Synthetic Respiratory Therapist C', discipline: 'respiratory therapist' },
] as const satisfies readonly Clinician[]

export type CareTime = { readonly kind: 'now' } | { readonly kind: 'at'; readonly at: PatientTime }

export type NewEntry = {
  readonly kind: 'new entry'
  readonly patient: PatientId
  readonly author: ClinicianId
  readonly vital: Vital
  readonly value: number
  readonly care: CareTime
}

export type Amendment = {
  readonly kind: 'amendment'
  readonly patient: PatientId
  readonly author: ClinicianId
  readonly entry: EntryId
  readonly replaces: VersionNumber
  readonly value: number
  readonly care: CareTime
}

export type EntryWrite = NewEntry | Amendment

// The value travels as the clinician typed it, so empty text reaches the chart as not a number, never as zero.
export type NewEntryBody = {
  readonly patient: string
  readonly author: string
  readonly vital: string
  readonly value: string
  readonly care: number | null
}

export type AmendmentBody = Omit<NewEntryBody, 'vital'> & { readonly entry: string; readonly replaces: number }

export type Refusal =
  | { readonly kind: 'another patient' }
  | { readonly kind: 'care time after now' }
  | { readonly kind: 'not a finite number' }
  | { readonly kind: 'no such entry' }
  | { readonly kind: 'not the current version' }
  | { readonly kind: 'malformed'; readonly reason: string }

export type EntryVersion = {
  readonly entry: EntryId
  readonly patient: PatientId
  readonly vital: Vital
  readonly number: VersionNumber
  readonly value: number
  readonly care: PatientTime
  readonly charted: PatientTime
  readonly author: Clinician
}

export type EntryView = EntryVersion & { readonly versions: number }

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

type Json = Readonly<Record<string, unknown>>

const DISCIPLINES: readonly unknown[] = ['nurse', 'physician', 'respiratory therapist'] satisfies Discipline[]
const REFUSALS: readonly unknown[] = [
  'another patient',
  'care time after now',
  'not a finite number',
  'no such entry',
  'not the current version',
] satisfies Refusal['kind'][]
const DECIMAL = /^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?$/

class Malformed extends Error {}

const fail = (reason: string): never => {
  throw new Malformed(reason)
}

const isObject = (value: unknown): value is Json => typeof value === 'object' && value !== null && !Array.isArray(value)
const object = (value: unknown, what: string): Json => (isObject(value) ? value : fail(`${what} is not an object`))
const list = (value: unknown, what: string): readonly unknown[] => (Array.isArray(value) ? value : fail(`${what} is not a list`))

function text(json: Json, key: string): string {
  const value = json[key]
  return typeof value === 'string' && value.length > 0 && value.length <= 200 ? value : fail(`${key} is not a short text`)
}

function whole(json: Json, key: string, least: number): number {
  const value = json[key]
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= least ? value : fail(`${key} is not a whole number from ${least}`)
}

function finite(json: Json, key: string): number {
  const value = json[key]
  return typeof value === 'number' && Number.isFinite(value) ? value : fail(`${key} is not a finite number`)
}

const unwrap = <T>(parsed: Parsed<T>): T => (parsed.ok ? parsed.value : fail(parsed.reason))

function vital(json: Json): Vital {
  const value = json['vital']
  return isVital(value) ? value : fail(`vital ${String(value)} is not in the vitals table`)
}

function author(json: Json): ClinicianId {
  const id = text(json, 'author')
  return ROSTER.some((c) => c.id === id) ? id : fail(`author ${id} is not on the roster`)
}

function typedValue(json: Json): number {
  const typed = json['value']
  if (typeof typed !== 'string') return fail('value is not the text the clinician typed')
  const trimmed = typed.trim()
  return DECIMAL.test(trimmed) ? Number(trimmed) : Number.NaN
}

function care(json: Json): CareTime {
  return json['care'] === null ? { kind: 'now' } : { kind: 'at', at: whole(json, 'care', 0) }
}

function clinician(value: unknown): Clinician {
  const json = object(value, 'author')
  const discipline = json['discipline']
  const known = (d: unknown): d is Discipline => DISCIPLINES.includes(d)
  return { id: text(json, 'id'), name: text(json, 'name'), discipline: known(discipline) ? discipline : fail('an unknown discipline') }
}

function entryVersion(value: unknown): EntryVersion {
  const json = object(value, 'an entry version')
  return {
    entry: text(json, 'entry'),
    patient: text(json, 'patient'),
    vital: vital(json),
    number: whole(json, 'number', 1),
    value: finite(json, 'value'),
    care: whole(json, 'care', 0),
    charted: whole(json, 'charted', 0),
    author: clinician(json['author']),
  }
}

function entryView(value: unknown): EntryView {
  return { ...entryVersion(value), versions: whole(object(value, 'an entry'), 'versions', 1) }
}

function refusal(value: unknown): Refusal {
  const json = object(value, 'a refusal')
  const kind = json['kind']
  if (kind === 'malformed') return { kind, reason: text(json, 'reason') }
  const known = (k: unknown): k is Exclude<Refusal['kind'], 'malformed'> => REFUSALS.includes(k)
  return known(kind) ? { kind } : fail(`refusal ${String(kind)} is not a kind the chart gives`)
}

function run(value: unknown): RunView {
  const json = object(value, 'the run')
  return {
    specId: text(json, 'specId'),
    patient: text(json, 'patient'),
    start: whole(json, 'start', 0),
    end: whole(json, 'end', 0),
    anchor: whole(json, 'anchor', 0),
  }
}

function reading(value: unknown): ReadingView {
  const json = object(value, 'a reading')
  return {
    device: text(json, 'device'),
    boot: text(json, 'boot'),
    sequence: whole(json, 'sequence', 1),
    vital: vital(json),
    collected: whole(json, 'collected', 0),
    available: whole(json, 'available', 0),
    arrived: whole(json, 'arrived', 0),
    value: unwrap(parseDeviceValue(json['value'])),
  }
}

function statement(value: unknown): StatementView {
  const json = object(value, 'a channel statement')
  return {
    device: text(json, 'device'),
    boot: text(json, 'boot'),
    sequence: whole(json, 'sequence', 1),
    at: whole(json, 'at', 0),
    arrived: whole(json, 'arrived', 0),
    channels: unwrap(parseChannels(json['channels'])),
  }
}

function missing(value: unknown): MissingSpan {
  const json = object(value, 'a missing span')
  const from = whole(json, 'from', 0)
  return { device: text(json, 'device'), vital: vital(json), from, through: whole(json, 'through', from) }
}

function parsing<T>(parse: () => T): Parsed<T> {
  try {
    return { ok: true, value: parse() }
  } catch (error) {
    if (error instanceof Malformed) return { ok: false, reason: error.message }
    throw error
  }
}

export const parseNewEntry = (body: unknown): Parsed<NewEntry> =>
  parsing(() => {
    const json = object(body, 'a new entry')
    return { kind: 'new entry', patient: text(json, 'patient'), author: author(json), vital: vital(json), value: typedValue(json), care: care(json) }
  })

export const parseAmendment = (body: unknown): Parsed<Amendment> =>
  parsing(() => {
    const json = object(body, 'an amendment')
    return {
      kind: 'amendment',
      patient: text(json, 'patient'),
      author: author(json),
      entry: text(json, 'entry'),
      replaces: whole(json, 'replaces', 1),
      value: typedValue(json),
      care: care(json),
    }
  })

export const parseWriteResult = (body: unknown): Parsed<WriteResult> =>
  parsing(() => {
    const json = object(body, 'a write result')
    if (json['kind'] === 'taken') return { kind: 'taken', entry: entryView(json['entry']) }
    if (json['kind'] === 'refused') return { kind: 'refused', refusal: refusal(json['refusal']) }
    return fail('a write result is taken or refused')
  })

export const parseVersions = (body: unknown): Parsed<readonly EntryVersion[]> => parsing(() => list(body, 'versions').map(entryVersion))

export const parseRun = (body: unknown): Parsed<RunView> => parsing(() => run(body))

export const parseReadModel = (body: unknown): Parsed<ReadModel> =>
  parsing(() => {
    const json = object(body, 'the read model')
    return {
      run: run(json['run']),
      now: whole(json, 'now', 0),
      entries: list(json['entries'], 'entries').map(entryView),
      readings: list(json['readings'], 'readings').map(reading),
      statements: list(json['statements'], 'statements').map(statement),
      missing: list(json['missing'], 'missing').map(missing),
    }
  })
