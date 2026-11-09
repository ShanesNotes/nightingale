import { Effect, Schema, Stream } from 'effect'
import { Command, Runtime, Subscription, type Update } from 'foldkit'
import type { Html, HtmlBuilder } from 'foldkit/html'
import { defineMessageUnion } from 'foldkit/message'
import {
  type BootId,
  type ChannelStatement,
  type DeviceMessage,
  type DeviceValue,
  isDeviceId,
  isSequence,
  type MessageAnswer,
  type PatientId,
  parseAnswers,
  parseDeviceMessage,
} from '../device.ts'
import type { Observation } from '../observation.ts'
import {
  add,
  formatClock,
  isPatientTime,
  type PatientClock,
  type PatientTime,
  patientTimeAt,
  seconds,
  untilNextSecond,
  type WallTime,
  wallTime,
} from '../time.ts'
import { isVital, perVital, VITALS, type Vital, type VitalRow } from '../vitals.ts'
import type { Link, LinkResult } from './link.ts'

export type Feed = {
  readonly start: PatientTime
  readonly end: PatientTime
  readonly channels: readonly string[]
  readonly between: (after: PatientTime | undefined, through: PatientTime) => readonly Observation[]
}

export type MonitorOptions = {
  readonly patient: PatientId
  readonly feed: Feed
  readonly link: Link
  readonly boot: BootId
  readonly cap?: number
  readonly vitals?: readonly VitalRow[]
}

// A full outbox drops its oldest message: an hour of one message a second.
const CAP = 3_600

function branded<T>(value: unknown, is: (v: unknown) => v is T, what: string): T {
  if (!is(value)) throw new Error(`${String(value)} is not ${what}`)
  return value
}

const DEVICE = branded('synthetic-bedside-monitor', isDeviceId, 'a device id')

// The monitor's own channel table: the engine row each vital reads and the unit it shows and sends. Heart rate
// arrives per second; mean pressure is taken as mmHg, as native-contract 29 records.
const DEVICE_TABLE: Readonly<Partial<Record<Vital, { readonly row: string; readonly convert: (value: number) => number }>>> = {
  HR: { row: 'algebraic/CV/rate/body', convert: (perSecond) => perSecond * 60 },
  'ABP mean': { row: 'algebraic/CV/blood pressure/systemic arteries', convert: (mmHg) => mmHg },
}

const PatientTimeSchema = Schema.declare(isPatientTime)
const DeviceMessageSchema = Schema.declare((value: unknown): value is DeviceMessage => parseDeviceMessage(value).ok)
const AnswersSchema = Schema.declare((value: unknown): value is readonly MessageAnswer[] => parseAnswers({ answers: value }).ok)

export const Model = Schema.Struct({
  now: Schema.NullOr(PatientTimeSchema),
  startedAt: Schema.NullOr(PatientTimeSchema),
  taken: Schema.NullOr(PatientTimeSchema),
  sequence: Schema.Number,
  outbox: Schema.Array(DeviceMessageSchema),
  inFlight: Schema.Boolean,
  answeredAt: Schema.NullOr(PatientTimeSchema),
  refused: Schema.Number,
  dropped: Schema.Number,
})
export type Model = typeof Model.Type

export const Message = defineMessageUnion({
  SecondPassed: { now: PatientTimeSchema },
  BatchAnswered: { answers: AnswersSchema },
  BatchUnanswered: {},
})
export type Message = typeof Message.Type

type Return = Update.Return<Model, Message>

const whole = (value: number) => String(Math.round(value))

const counted = (n: number, what: string) => `${n} ${what}${n === 1 ? '' : 's'}`

function linkWords(model: Model): string {
  const since = model.answeredAt ?? model.startedAt
  const held =
    model.outbox.length > 0
      ? `Holding ${counted(model.outbox.length, 'message')}. The chart service has not answered since ${since === null ? 'the monitor started' : formatClock(since)}.`
      : model.refused > 0
        ? 'Every message has its answer from the chart.'
        : 'Everything delivered to the chart.'
  const refused = model.refused > 0 ? ` The chart refused ${counted(model.refused, 'message')}.` : ''
  const dropped = model.dropped > 0 ? ` The outbox dropped ${counted(model.dropped, 'message')} when it was full.` : ''
  return `${held}${refused}${dropped}`
}

export function monitor(options: MonitorOptions) {
  const { feed, link, boot, patient, cap = CAP, vitals = VITALS } = options
  const deviceRow = (vital: string) => (isVital(vital) ? DEVICE_TABLE[vital] : undefined)
  const sourced = (vital: string) => {
    const entry = deviceRow(vital)
    return entry !== undefined && feed.channels.includes(entry.row)
  }
  const vitalOf = (channel: number): Vital | undefined =>
    VITALS.map((row) => row.vital).find((vital) => DEVICE_TABLE[vital]?.row === feed.channels[channel] && sourced(vital))

  const messageFor = (result: LinkResult) =>
    result.kind === 'answered' ? Message.BatchAnswered({ answers: result.answers }) : Message.BatchUnanswered()

  const SendBatch = Command.define('SendBatch', {
    args: { batch: Schema.Array(DeviceMessageSchema) },
    messages: [Message.BatchAnswered, Message.BatchUnanswered],
    execute: ({ batch }) => Effect.promise(() => link.send(batch)).pipe(Effect.map(messageFor)),
  })

  const statementAt = (sequence: number, now: PatientTime): ChannelStatement => ({
    kind: 'statement',
    device: DEVICE,
    boot,
    patient,
    sequence: branded(sequence, isSequence, 'a sequence number'),
    at: now,
    channels: perVital((vital) => (sourced(vital) ? 'sourced' : 'no source')),
  })

  const init = (): Return => ({
    model: { now: null, startedAt: null, taken: null, sequence: 1, outbox: [], inFlight: false, answeredAt: null, refused: 0, dropped: 0 },
  })

  const valueOf = (vital: Vital, observation: Observation): DeviceValue => {
    const { value } = observation
    return value.kind === 'present'
      ? { kind: 'present', value: DEVICE_TABLE[vital]?.convert(value.value) ?? value.value }
      : { kind: 'absent', reason: value.kind }
  }

  const update = (model: Model, message: Message): Return =>
    Message.match<Return>(message, {
      SecondPassed: ({ now }) => {
        const after = model.taken ?? (now >= seconds(1) ? add(now, seconds(-1)) : undefined)
        const fresh = model.taken === null || now > model.taken ? feed.between(after, now) : []
        let sequence = model.sequence
        const readings: DeviceMessage[] = model.taken === null ? [statementAt(sequence++, now)] : []
        for (const observation of fresh) {
          const vital = vitalOf(observation.channel)
          if (vital === undefined) continue
          readings.push({
            kind: 'reading',
            device: DEVICE,
            boot,
            patient,
            sequence: branded(sequence, isSequence, 'a sequence number'),
            vital,
            collected: observation.collected,
            available: observation.available,
            value: valueOf(vital, observation),
          })
          sequence += 1
        }
        const held = [...model.outbox, ...readings]
        const drop = Math.max(0, held.length - cap)
        const outbox = held.slice(drop)
        const send = !model.inFlight && outbox.length > 0
        return {
          model: {
            ...model,
            now,
            startedAt: model.startedAt ?? now,
            taken: model.taken === null || now > model.taken ? now : model.taken,
            sequence,
            outbox,
            inFlight: model.inFlight || send,
            dropped: model.dropped + drop,
          },
          commands: send ? [SendBatch({ batch: outbox })] : [],
        }
      },
      BatchAnswered: ({ answers }) => {
        const answered = new Set(answers.map((a) => `${a.boot} ${a.sequence}`))
        return {
          model: {
            ...model,
            inFlight: false,
            outbox: model.outbox.filter((m) => !answered.has(`${m.boot} ${m.sequence}`)),
            answeredAt: model.now,
            refused: model.refused + answers.filter((a) => a.answer === 'refused').length,
          },
        }
      },
      BatchUnanswered: () => ({ model: { ...model, inFlight: false } }),
    })

  const tileText = (row: VitalRow, now: PatientTime): string => {
    const { vital } = row
    if (!isVital(vital) || !sourced(vital)) return 'no source'
    const channel = feed.channels.indexOf(DEVICE_TABLE[vital]?.row ?? '')
    const latest = feed.between(undefined, now).filter((o) => o.channel === channel).at(-1)
    if (latest === undefined) return 'no reading yet'
    const value = valueOf(vital, latest)
    return value.kind === 'present' ? `${whole(value.value)} ${row.unit}` : value.reason
  }

  const view = (model: Model, h: HtmlBuilder<Message>): Html =>
    h.div([h.Id('monitor')], [
      h.h1([], ['Bedside monitor']),
      h.p([h.Id('clock')], [model.now === null ? 'Starting.' : `Patient time ${formatClock(model.now)}`]),
      h.p([h.Id('feed')], [
        `Fed a recorded synthetic run, patient ${patient}.`,
        model.now !== null && model.now >= feed.end ? ` The recorded feed ended at ${formatClock(feed.end)}.` : '',
      ]),
      h.p([h.Id('link')], [linkWords(model)]),
      h.table([h.Id('tiles')], [
        h.tbody(
          [],
          vitals.map((row) =>
            h.tr([h.DataAttribute('vital', row.vital)], [
              h.th([], [`${row.label} (${row.unit})`]),
              h.td([h.DataAttribute('tile', row.vital)], [model.now === null ? '' : tileText(row, model.now)]),
            ]),
          ),
        ),
      ]),
    ])

  return { init, update, view, SendBatch, messageFor }
}

// A tick lands this long after each whole patient second, so the clock it reads is already in the new second.
const TICK_MARGIN_MS = 30

// With no anchor the patient clock starts at the page's load. The monitor never asks the chart service the time.
export function start(container: HTMLElement, options: MonitorOptions & { readonly anchor?: WallTime }): void {
  const program = monitor(options)
  const clock: PatientClock = { start: options.feed.start, end: options.feed.end, anchor: options.anchor ?? wallTime(Date.now()) }
  const tick = Effect.sync(() => Message.SecondPassed({ now: patientTimeAt(clock, wallTime(Date.now())) }))
  const next = Effect.suspend(() => Effect.sleep(untilNextSecond(clock, wallTime(Date.now())) + TICK_MARGIN_MS)).pipe(Effect.andThen(tick))
  const subscriptions = Subscription.make<Model, Message>()(() => ({
    clock: Subscription.persistentEntry(Stream.concat(Stream.fromEffect(tick), Stream.fromEffectRepeat(next))),
  }))
  Runtime.run(Runtime.makeElement({ Model, init: program.init, update: program.update, view: program.view, subscriptions, container }))
}
