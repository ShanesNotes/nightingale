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

export type NewEntry = {
  readonly kind: 'new entry'
  readonly writeId: WriteId
  readonly patient: PatientId
  readonly author: ClinicianId
  readonly vital: Vital
  readonly value: number
  readonly care: CareTime
}

export type Amendment = {
  readonly kind: 'amendment'
  readonly writeId: WriteId
  readonly patient: PatientId
  readonly author: ClinicianId
  readonly entry: EntryId
  readonly replaces: VersionNumber
  readonly value: number
  readonly care: CareTime
}

export type EntryWrite = NewEntry | Amendment

export type NewEntryBody = {
  readonly writeId: string
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
  | { readonly kind: 'write id reused with other content' }
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
  'write id reused with other content',
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
  const id = branded(json, 'author', isClinicianId, 'a clinician id')
  return ROSTER.some((c) => c.id === id) ? id : fail(`author ${id} is not on the roster`)
}

function typedValue(json: Json): number {
  const typed = json['value']
  if (typeof typed !== 'string') return fail('value is not the text the clinician typed')
  const trimmed = typed.trim()
  return DECIMAL.test(trimmed) ? Number(trimmed) : Number.NaN
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
    entry: entryOf(json, 'entry'),
    patient: patientOf(json),
    vital: vital(json),
    number: versionOf(json, 'number'),
    value: finite(json, 'value'),
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
  if (kind === 'malformed') return { kind, reason: text(json, 'reason') }
  const known = (k: unknown): k is Exclude<Refusal['kind'], 'malformed'> => REFUSALS.includes(k)
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

export const parseNewEntry = (body: unknown): Parsed<NewEntry> =>
  parsing(() => {
    const json = object(body, 'a new entry')
    return { kind: 'new entry', writeId: writeIdOf(json), patient: patientOf(json), author: author(json), vital: vital(json), value: typedValue(json), care: care(json) }
  })

export const parseAmendment = (body: unknown): Parsed<Amendment> =>
  parsing(() => {
    const json = object(body, 'an amendment')
    return {
      kind: 'amendment',
      writeId: writeIdOf(json),
      patient: patientOf(json),
      author: author(json),
      entry: entryOf(json, 'entry'),
      replaces: versionOf(json, 'replaces'),
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

export const parseEntries = (body: unknown): Parsed<readonly EntryView[]> => parsing(() => list(body, 'entries').map(entryView))

export const parseVersions = (body: unknown): Parsed<readonly EntryVersion[]> => parsing(() => list(body, 'versions').map(entryVersion))

export const parseRun = (body: unknown): Parsed<RunView> => parsing(() => run(body))

export const parseReadModel = (body: unknown): Parsed<ReadModel> =>
  parsing(() => {
    const json = object(body, 'the read model')
    return {
      run: run(json['run']),
      now: time(json, 'now'),
      entries: list(json['entries'], 'entries').map(entryView),
      readings: list(json['readings'], 'readings').map(reading),
      statements: list(json['statements'], 'statements').map(statement),
      missing: list(json['missing'], 'missing').map(missing),
    }
  })
