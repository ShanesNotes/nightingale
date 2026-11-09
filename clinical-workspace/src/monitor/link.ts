import type { DeviceMessage, MessageAnswer } from '../device.ts'

// The monitor's seam to the chart service. Its adapters are HTTP and, in the tests, an in-memory chart. A
// refused connection, an error answer and a timeout are all unanswered: the monitor removes nothing for them.
export type LinkResult = { readonly kind: 'answered'; readonly answers: readonly MessageAnswer[] } | { readonly kind: 'unanswered' }

export type Link = { readonly send: (batch: readonly DeviceMessage[]) => Promise<LinkResult> }
