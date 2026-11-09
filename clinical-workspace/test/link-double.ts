import { type DeviceMessage, type MessageAnswer, parseDeviceMessage } from '../src/device.ts'
import type { Link, LinkResult } from '../src/monitor/link.ts'

export type ServiceState = 'up' | 'down' | 'losing answers'

export function chartDouble(patient: string) {
  const held = new Map<string, string>()
  let state: ServiceState = 'up'
  const answer = (batch: readonly DeviceMessage[]): LinkResult => {
    if (state === 'down') return { kind: 'unanswered' }
    const answers = batch.map((m): MessageAnswer => {
      const answered = { boot: m.boot, sequence: m.sequence }
      if (m.patient !== patient) return { ...answered, answer: 'refused', refusal: 'another patient' }
      const key = `${m.device} ${m.boot} ${m.sequence}`
      const content = JSON.stringify(m)
      const known = held.get(key)
      if (known === undefined) {
        held.set(key, content)
        return { ...answered, answer: 'taken' }
      }
      return known === content ? { ...answered, answer: 'already held' } : { ...answered, answer: 'refused', refusal: 'sequence reused with other content' }
    })
    return state === 'losing answers' ? { kind: 'unanswered' } : { kind: 'answered', answers }
  }
  const link: Link = { send: (batch) => Promise.resolve(answer(batch)) }
  return {
    link,
    answer,
    set: (next: ServiceState) => {
      state = next
    },
    held: (): readonly DeviceMessage[] =>
      [...held.values()].map((json) => {
        const parsed = parseDeviceMessage(JSON.parse(json))
        if (!parsed.ok) throw new Error(`the double holds a message the protocol refuses: ${parsed.reason}`)
        return parsed.value
      }),
  }
}
