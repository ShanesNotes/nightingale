export type PatientTime = number & { readonly __brand: 'PatientTime' }
export type Duration = number & { readonly __brand: 'Duration' }
export type WallTime = number & { readonly __brand: 'WallTime' }

export type PatientClock = {
  readonly start: PatientTime
  readonly end: PatientTime
  readonly anchor: WallTime
}

const NANOSECONDS_PER_SECOND = 1_000_000_000
const MILLISECONDS_PER_SECOND = 1_000
const CLOCK = /^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$/

export const isPatientTime = (value: unknown): value is PatientTime =>
  typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
export const isDuration = (value: unknown): value is Duration => typeof value === 'number' && Number.isSafeInteger(value)
export const isWallTime = (value: unknown): value is WallTime => typeof value === 'number' && Number.isSafeInteger(value)

function patientTime(nanoseconds: number): PatientTime {
  if (!isPatientTime(nanoseconds)) throw new RangeError(`${nanoseconds} ns is not a patient time`)
  return nanoseconds
}

function duration(nanoseconds: number): Duration {
  if (!isDuration(nanoseconds)) throw new RangeError(`${nanoseconds} ns is not a duration`)
  return nanoseconds
}

export function wallTime(milliseconds: number): WallTime {
  if (!isWallTime(milliseconds)) throw new RangeError(`${milliseconds} ms is not a wall-clock instant`)
  return milliseconds
}

export const seconds = (count: number): Duration => duration(count * NANOSECONDS_PER_SECOND)

export const ZERO = seconds(0)
export const SECOND = seconds(1)
export const MINUTE = seconds(60)

export const add = (time: PatientTime, by: Duration): PatientTime => patientTime(time + by)
export const since = (later: PatientTime, earlier: PatientTime): Duration => duration(later - earlier)
export const floorTo = (time: PatientTime, unit: Duration): PatientTime => patientTime(Math.floor(time / unit) * unit)
export const earlier = (a: PatientTime, b: PatientTime): PatientTime => (a <= b ? a : b)
export const later = (a: PatientTime, b: PatientTime): PatientTime => (a >= b ? a : b)

const twoDigits = (n: number) => String(n).padStart(2, '0')

export function parseClock(text: string): PatientTime | undefined {
  const match = CLOCK.exec(text)
  if (match === null) return undefined
  const [hours, minutes, seconds] = match.slice(1).map(Number)
  return patientTime(((hours! * 60 + minutes!) * 60 + seconds!) * NANOSECONDS_PER_SECOND)
}

export function formatClock(time: PatientTime): string {
  const seconds = Math.floor(time / NANOSECONDS_PER_SECOND)
  return `${twoDigits(Math.floor(seconds / 3600))}:${twoDigits(Math.floor(seconds / 60) % 60)}:${twoDigits(seconds % 60)}`
}

export function formatDuration(span: Duration): string {
  const total = Math.round(span / NANOSECONDS_PER_SECOND)
  const hours = Math.floor(total / 3600)
  const minutes = Math.floor(total / 60) % 60
  const parts = [hours > 0 ? `${hours} h` : '', hours > 0 || minutes > 0 ? `${minutes} min` : '', `${total % 60} s`]
  return parts.filter((part) => part !== '').join(' ')
}

export function patientTimeAt(clock: PatientClock, wall: WallTime): PatientTime {
  const elapsed = Math.max(0, Math.floor((wall - clock.anchor) / MILLISECONDS_PER_SECOND))
  return earlier(clock.end, add(clock.start, duration(elapsed * NANOSECONDS_PER_SECOND)))
}

export function msUntilNextSecond(clock: PatientClock, wall: WallTime): number {
  const into = (((wall - clock.anchor) % MILLISECONDS_PER_SECOND) + MILLISECONDS_PER_SECOND) % MILLISECONDS_PER_SECOND
  return MILLISECONDS_PER_SECOND - into
}
