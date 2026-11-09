import assert from 'node:assert/strict'
import { type EntryVersion, parseReadModel, parseVersions, parseWriteResult, ROSTER, type ReadModel, type WriteResult } from '../src/chart-api.ts'
import { parseClock, type PatientTime } from '../src/time.ts'
import { perVital } from '../src/vitals.ts'

export const at = (clock: string): PatientTime => {
  const time = parseClock(clock)
  assert.ok(time !== undefined, clock)
  return time
}

export const RUN = { specId: 'test-spec', patient: 'test-patient', start: at('06:55:00'), end: at('07:05:00'), anchor: 1_000_000 }
export const [NURSE, PHYSICIAN, THERAPIST] = [ROSTER[0]!, ROSTER[1]!, ROSTER[2]!]

export const statement = (clock: string, sourced: readonly string[], boot = 'test-boot') => ({
  device: 'test-monitor',
  boot,
  sequence: 1,
  at: at(clock),
  arrived: at(clock),
  channels: perVital((v) => (sourced.includes(v) ? 'sourced' : 'no source')),
})

export const reading = (vital: string, collected: string, value: number | 'not collected', arrived = collected) => ({
  device: 'test-monitor',
  boot: 'test-boot',
  sequence: 2,
  vital,
  collected: at(collected),
  available: at(collected),
  arrived: at(arrived),
  value: value === 'not collected' ? { kind: 'absent', reason: value } : { kind: 'present', value },
})

export const entry = (fields: { id: string; vital: string; value: number; care: string; charted: string; versions?: number; author?: object }) => ({
  entry: fields.id,
  patient: 'test-patient',
  vital: fields.vital,
  number: fields.versions ?? 1,
  value: fields.value,
  care: at(fields.care),
  charted: at(fields.charted),
  author: fields.author ?? NURSE,
  versions: fields.versions ?? 1,
})

export function readModel(overrides: Readonly<Record<string, unknown>>): ReadModel {
  const parsed = parseReadModel({ run: RUN, now: at('06:57:30'), entries: [], readings: [], statements: [], missing: [], ...overrides })
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}

export function written(json: unknown): WriteResult {
  const parsed = parseWriteResult(json)
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}

export function versions(json: readonly object[]): readonly EntryVersion[] {
  const parsed = parseVersions(json)
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}
