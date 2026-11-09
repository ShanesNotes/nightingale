import { type DeviceMessage, type MessageAnswer, parseAnswers } from '../device.ts'

export type LinkResult = { readonly kind: 'answered'; readonly answers: readonly MessageAnswer[] } | { readonly kind: 'unanswered' }

export type Link = { readonly send: (batch: readonly DeviceMessage[]) => Promise<LinkResult> }

const UNANSWERED: LinkResult = { kind: 'unanswered' }

export function httpLink(url: string, timeoutMs = 3_000): Link {
  return {
    async send(batch) {
      try {
        const response = await fetch(url, {
          method: 'POST',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify(batch),
          signal: AbortSignal.timeout(timeoutMs),
        })
        if (response.status !== 200) return UNANSWERED
        const parsed = parseAnswers(await response.json())
        return parsed.ok ? { kind: 'answered', answers: parsed.value } : UNANSWERED
      } catch {
        return UNANSWERED
      }
    },
  }
}
