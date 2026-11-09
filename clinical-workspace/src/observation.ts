import type { PatientTime } from './time.ts'

// The engine's ng_observe::Observation: a value or why there is none, never a zero.
export type Absence = 'not collected' | 'pending' | 'invalid signal'

export type ObservedValue = { readonly kind: 'present'; readonly value: number } | { readonly kind: Absence }

export type Observation = {
  readonly channel: number
  readonly collected: PatientTime
  readonly available: PatientTime
  readonly value: ObservedValue
}
