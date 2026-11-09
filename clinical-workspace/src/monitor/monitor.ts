import { Effect, Schema } from 'effect'
import { Command, type Update } from 'foldkit'
import type { Html, HtmlBuilder } from 'foldkit/html'
import { defineMessageUnion } from 'foldkit/message'
import {
  type BootId,
  type DeviceMessage,
  type DeviceValue,
  isDeviceId,
  isSequence,
  type PatientId,
  parseDeviceMessage,
  type Reading,
} from '../device.ts'
import type { Observation } from '../observation.ts'
import { add, formatClock, isPatientTime, type PatientTime, seconds } from '../time.ts'
import { VITALS, type Vital } from '../vitals.ts'
import type { Link } from './link.ts'

// The standalone bedside monitor: one tile per vital of the table, fed the observer's observations, and an outbox
// that delivers each reading to the chart service as a numbered device message.
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
}

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

export const Model = Schema.Struct({
  now: Schema.NullOr(PatientTimeSchema),
  taken: Schema.NullOr(PatientTimeSchema),
  sequence: Schema.Number,
  outbox: Schema.Array(DeviceMessageSchema),
  inFlight: Schema.Boolean,
})
export type Model = typeof Model.Type

export const Message = defineMessageUnion({
  SecondPassed: { now: PatientTimeSchema },
  BatchAnswered: {},
  BatchUnanswered: {},
})
export type Message = typeof Message.Type

type Return = Update.Return<Model, Message>

const whole = (value: number) => String(Math.round(value))

export function monitor(options: MonitorOptions) {
  const { feed, link, boot, patient } = options
  const sourced = (vital: Vital) => {
    const entry = DEVICE_TABLE[vital]
    return entry !== undefined && feed.channels.includes(entry.row)
  }
  const vitalOf = (channel: number): Vital | undefined =>
    VITALS.map((row) => row.vital).find((vital) => DEVICE_TABLE[vital]?.row === feed.channels[channel] && sourced(vital))

  const SendBatch = Command.define('SendBatch', {
    args: { batch: Schema.Array(DeviceMessageSchema) },
    messages: [Message.BatchAnswered, Message.BatchUnanswered],
    execute: ({ batch }) =>
      Effect.promise(() => link.send(batch)).pipe(Effect.map((result) => (result.kind === 'answered' ? Message.BatchAnswered() : Message.BatchUnanswered()))),
  })

  const init = (): Return => ({ model: { now: null, taken: null, sequence: 1, outbox: [], inFlight: false } })

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
        const readings: Reading[] = []
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
        const outbox = [...model.outbox, ...readings]
        const send = !model.inFlight && outbox.length > 0
        return {
          model: { ...model, now, taken: model.taken === null || now > model.taken ? now : model.taken, sequence, outbox, inFlight: model.inFlight || send },
          commands: send ? [SendBatch({ batch: outbox })] : [],
        }
      },
      BatchAnswered: () => ({ model: { ...model, inFlight: false } }),
      BatchUnanswered: () => ({ model: { ...model, inFlight: false } }),
    })

  const tileText = (vital: Vital, now: PatientTime): string => {
    if (!sourced(vital)) return 'no source'
    const channel = feed.channels.indexOf(DEVICE_TABLE[vital]?.row ?? '')
    const latest = feed.between(undefined, now).filter((o) => o.channel === channel).at(-1)
    if (latest === undefined) return 'no reading yet'
    const value = valueOf(vital, latest)
    const unit = VITALS.find((row) => row.vital === vital)?.unit ?? ''
    return value.kind === 'present' ? `${whole(value.value)} ${unit}` : value.reason
  }

  const view = (model: Model, h: HtmlBuilder<Message>): Html =>
    h.div([h.Id('monitor')], [
      h.h1([], ['Bedside monitor']),
      h.p([h.Id('clock')], [model.now === null ? 'Starting.' : `Patient time ${formatClock(model.now)}`]),
      h.p([h.Id('feed')], [
        `Fed a recorded synthetic run, patient ${patient}.`,
        model.now !== null && model.now >= feed.end ? ` The recorded feed ended at ${formatClock(feed.end)}.` : '',
      ]),
      h.table([h.Id('tiles')], [
        h.tbody(
          [],
          VITALS.map((row) =>
            h.tr([h.DataAttribute('vital', row.vital)], [
              h.th([], [row.label]),
              h.td([h.DataAttribute('tile', row.vital)], [model.now === null ? '' : tileText(row.vital, model.now)]),
            ]),
          ),
        ),
      ]),
    ])

  return { init, update, view, SendBatch }
}
