import { isPatientTime, type PatientTime } from './time.ts'
import { isVital, type PerVital, perVital, VITALS, type Vital } from './vitals.ts'

// The device protocol between a bedside monitor and the chart service. Each message is numbered in sequence
// within one device boot, and the service answers each one.
export type DeviceId = string & { readonly __brand: 'DeviceId' }
export type BootId = string & { readonly __brand: 'BootId' }
export type PatientId = string & { readonly __brand: 'PatientId' }
export type Sequence = number & { readonly __brand: 'Sequence' }

const isShortText = (value: unknown): value is string => typeof value === 'string' && value.length > 0 && value.length <= 200

export const isDeviceId = (value: unknown): value is DeviceId => isShortText(value)
export const isBootId = (value: unknown): value is BootId => isShortText(value)
export const isPatientId = (value: unknown): value is PatientId => isShortText(value)
export const isSequence = (value: unknown): value is Sequence => typeof value === 'number' && Number.isSafeInteger(value) && value >= 1

export type DeviceAbsence = 'not collected' | 'pending' | 'invalid signal'
export type DeviceValue = { readonly kind: 'present'; readonly value: number } | { readonly kind: 'absent'; readonly reason: DeviceAbsence }
export type Source = 'sourced' | 'no source'

type Numbered = {
  readonly device: DeviceId
  readonly boot: BootId
  readonly patient: PatientId
  readonly sequence: Sequence
}

export type Reading = Numbered & {
  readonly kind: 'reading'
  readonly vital: Vital
  readonly collected: PatientTime
  readonly available: PatientTime
  readonly value: DeviceValue
}

export type ChannelStatement = Numbered & {
  readonly kind: 'statement'
  readonly at: PatientTime
  readonly channels: PerVital<Source>
}

export type DeviceMessage = Reading | ChannelStatement

export type DeviceRefusal = 'another patient' | 'sequence reused with other content'

type Answered = { readonly boot: BootId; readonly sequence: Sequence }
export type MessageAnswer =
  | (Answered & { readonly answer: 'taken' })
  | (Answered & { readonly answer: 'already held' })
  | (Answered & { readonly answer: 'refused'; readonly refusal: DeviceRefusal })

export type Parsed<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly reason: string }

type Json = Readonly<Record<string, unknown>>

const ABSENCES: readonly unknown[] = ['not collected', 'pending', 'invalid signal'] satisfies DeviceAbsence[]
const SOURCES: readonly unknown[] = ['sourced', 'no source'] satisfies Source[]
const REFUSALS: readonly unknown[] = ['another patient', 'sequence reused with other content'] satisfies DeviceRefusal[]

class Malformed extends Error {}

const fail = (reason: string): never => {
  throw new Malformed(reason)
}

const isObject = (value: unknown): value is Json => typeof value === 'object' && value !== null && !Array.isArray(value)

const object = (value: unknown, what: string): Json => (isObject(value) ? value : fail(`${what} is not an object`))

function branded<T>(json: Json, key: string, is: (value: unknown) => value is T, what: string): T {
  const value = json[key]
  return is(value) ? value : fail(`${key} is not ${what}`)
}

const time = (json: Json, key: string): PatientTime => branded(json, key, isPatientTime, 'a patient time')

const one = <T>(json: Json, key: string, words: readonly unknown[], parsed: (value: unknown) => value is T): T => {
  const value = json[key]
  return words.includes(value) && parsed(value) ? value : fail(`${key} is not one of ${words.join(', ')}`)
}

const isAbsence = (value: unknown): value is DeviceAbsence => ABSENCES.includes(value)
const isSource = (value: unknown): value is Source => SOURCES.includes(value)
const isRefusal = (value: unknown): value is DeviceRefusal => REFUSALS.includes(value)

function numbered(json: Json): Numbered {
  return {
    device: branded(json, 'device', isDeviceId, 'a device id'),
    boot: branded(json, 'boot', isBootId, 'a boot id'),
    patient: branded(json, 'patient', isPatientId, 'a patient id'),
    sequence: branded(json, 'sequence', isSequence, 'a sequence number'),
  }
}

function deviceValue(value: unknown): DeviceValue {
  const json = object(value, 'value')
  if (json['kind'] === 'present') {
    const number = json['value']
    return typeof number === 'number' && Number.isFinite(number) ? { kind: 'present', value: number } : fail('a present value is not a finite number')
  }
  if (json['kind'] === 'absent') return { kind: 'absent', reason: one(json, 'reason', ABSENCES, isAbsence) }
  return fail('value is neither present nor absent')
}

function vital(json: Json): Vital {
  const value = json['vital']
  return isVital(value) ? value : fail(`vital ${String(value)} is not in the vitals table`)
}

function channels(value: unknown): PerVital<Source> {
  const json = object(value, 'channels')
  const named = Object.keys(json)
  if (named.length !== VITALS.length || !named.every(isVital)) fail('channels must name each vital of the table once')
  return perVital((v) => one(json, v, SOURCES, isSource))
}

function message(value: unknown): DeviceMessage {
  const json = object(value, 'a device message')
  if (json['kind'] === 'reading') {
    const collected = time(json, 'collected')
    const available = time(json, 'available')
    if (available < collected) fail('a reading is available before it is collected')
    return { kind: 'reading', ...numbered(json), vital: vital(json), collected, available, value: deviceValue(json['value']) }
  }
  if (json['kind'] === 'statement') {
    return { kind: 'statement', ...numbered(json), at: time(json, 'at'), channels: channels(json['channels']) }
  }
  return fail('a device message is a reading or a channel statement')
}

function answer(value: unknown): MessageAnswer {
  const json = object(value, 'an answer')
  const answered = { boot: branded(json, 'boot', isBootId, 'a boot id'), sequence: branded(json, 'sequence', isSequence, 'a sequence number') }
  switch (json['answer']) {
    case 'taken':
      return { ...answered, answer: 'taken' }
    case 'already held':
      return { ...answered, answer: 'already held' }
    case 'refused':
      return { ...answered, answer: 'refused', refusal: one(json, 'refusal', REFUSALS, isRefusal) }
    default:
      return fail('an answer is taken, already held or refused')
  }
}

function parsing<T>(parse: () => T): Parsed<T> {
  try {
    return { ok: true, value: parse() }
  } catch (error) {
    if (error instanceof Malformed) return { ok: false, reason: error.message }
    throw error
  }
}

const list = (value: unknown, what: string): readonly unknown[] => (Array.isArray(value) ? value : fail(`${what} is not a list`))

export const parseDeviceMessage = (json: unknown): Parsed<DeviceMessage> => parsing(() => message(json))

export const parseDeviceValue = (json: unknown): Parsed<DeviceValue> => parsing(() => deviceValue(json))

export const parseChannels = (json: unknown): Parsed<PerVital<Source>> => parsing(() => channels(json))

export const parseBatch = (json: unknown): Parsed<readonly DeviceMessage[]> => parsing(() => list(json, 'a batch').map(message))

export const parseAnswers = (json: unknown): Parsed<readonly MessageAnswer[]> =>
  parsing(() => list(object(json, 'the answers')['answers'], 'answers').map(answer))
