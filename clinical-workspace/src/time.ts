export type PatientTime = number
export type Duration = number
export type WallTime = number

export type PatientClock = {
  readonly start: PatientTime
  readonly end: PatientTime
  readonly anchor: WallTime
}

const NANOSECONDS_PER_SECOND = 1_000_000_000
const MILLISECONDS_PER_SECOND = 1_000
const CLOCK = /^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$/

export const SECOND: Duration = NANOSECONDS_PER_SECOND
export const MINUTE: Duration = 60 * NANOSECONDS_PER_SECOND

export const add = (time: PatientTime, duration: Duration): PatientTime => time + duration
export const since = (later: PatientTime, earlier: PatientTime): Duration => later - earlier
export const floorTo = (time: PatientTime, unit: Duration): PatientTime => Math.floor(time / unit) * unit
export const earlier = (a: PatientTime, b: PatientTime): PatientTime => Math.min(a, b)

const twoDigits = (n: number) => String(n).padStart(2, '0')

export function parseClock(text: string): PatientTime | undefined {
  const match = CLOCK.exec(text)
  if (match === null) return undefined
  const [hours, minutes, seconds] = match.slice(1).map(Number)
  return ((hours! * 60 + minutes!) * 60 + seconds!) * NANOSECONDS_PER_SECOND
}

export function formatClock(time: PatientTime): string {
  const seconds = Math.floor(time / NANOSECONDS_PER_SECOND)
  return `${twoDigits(Math.floor(seconds / 3600))}:${twoDigits(Math.floor(seconds / 60) % 60)}:${twoDigits(seconds % 60)}`
}

export function wallTime(milliseconds: number): WallTime {
  if (!Number.isSafeInteger(milliseconds)) throw new RangeError(`${milliseconds} ms is not a wall-clock instant`)
  return milliseconds
}

export function patientTimeAt(clock: PatientClock, wall: WallTime): PatientTime {
  const elapsed = Math.max(0, Math.floor((wall - clock.anchor) / MILLISECONDS_PER_SECOND))
  return Math.min(clock.end, clock.start + elapsed * NANOSECONDS_PER_SECOND)
}
