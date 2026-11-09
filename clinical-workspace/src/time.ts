export type PatientTime = number

const NANOSECONDS_PER_SECOND = 1_000_000_000
const CLOCK = /^([01]\d|2[0-3]):([0-5]\d):([0-5]\d)$/

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
