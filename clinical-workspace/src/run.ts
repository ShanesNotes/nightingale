import type { Absence, Observation, ObservedValue } from './observation.ts'
import type { PatientTime } from './time.ts'

// A recorded run from `gp-driver run`: its lens rows, the catalogue that names their channels and the summary
// that names the run. The formats are tools/generative-physiology/driver/src/capture.rs and npy.rs.
export type Run = {
  readonly specId: string
  readonly packet: string
  readonly start: PatientTime
  readonly end: PatientTime
  readonly channels: readonly string[]
  readonly observations: readonly Observation[]
}

export type Decoded = { readonly kind: 'decoded'; readonly run: Run } | { readonly kind: 'refused'; readonly reason: string }

export type RunFiles = { readonly lens: Uint8Array; readonly catalogue: string; readonly summary: string }

const MAGIC = [0x93, 0x4e, 0x55, 0x4d, 0x50, 0x59, 0x01, 0x00]
const HEADER = /^\{'descr': '<u8', 'fortran_order': False, 'shape': \((\d+), 6\), \}$/
const COLUMNS = 6
const NANOSECONDS_PER_SECOND = 1_000_000_000
const ABSENCES: Readonly<Record<string, Absence>> = { '1': 'not collected', '2': 'pending', '3': 'invalid signal' }

class Malformed extends Error {}

const refuse = (reason: string): never => {
  throw new Malformed(reason)
}

const safe = (value: bigint, what: string): number =>
  value <= BigInt(Number.MAX_SAFE_INTEGER) ? Number(value) : refuse(`${what} ${value} is past the safe integer range`)

function float(bits: bigint): number {
  const view = new DataView(new ArrayBuffer(8))
  view.setBigUint64(0, bits, true)
  return view.getFloat64(0, true)
}

function channels(catalogue: string): string[] {
  const lines = catalogue.split('\n')
  if (lines[0] !== 'gp-catalogue 1') refuse('the catalogue does not start with gp-catalogue 1')
  const rows = lines.filter((line) => line.startsWith('lens ')).map((line) => {
    const match = /^lens (.+) (\d+)$/.exec(line)
    return match?.[1] ?? refuse(`the catalogue line "${line}" is not lens <row> <window_ns>`)
  })
  return rows.length > 0 ? rows : refuse('the catalogue names no lens channel')
}

function summary(text: string): Map<string, string> {
  const values = new Map<string, string>()
  for (const line of text.split('\n')) {
    const space = line.indexOf(' ')
    if (space > 0) values.set(line.slice(0, space), line.slice(space + 1))
  }
  if (values.get('capture_version') !== '2') refuse('the summary is not capture version 2')
  for (const key of ['spec_id', 'packet']) if (!values.get(key)) refuse(`the summary has no ${key}`)
  if (!/^\d+$/.test(values.get('end_time_ns') ?? '')) refuse('the summary has no end_time_ns in nanoseconds')
  return values
}

function value(tag: bigint, hi: bigint, lo: bigint): ObservedValue {
  if (tag === 0n) {
    const number = float(hi)
    return Number.isFinite(number) ? { kind: 'present', value: number } : refuse('a present value is not finite')
  }
  const absence = ABSENCES[String(tag)] ?? refuse(`tag ${tag} names no observed value`)
  return hi === 0n && lo === 0n ? { kind: absence } : refuse(`an absence (${absence}) carries value bits`)
}

function observations(lens: Uint8Array, channelCount: number): Observation[] {
  if (lens.length < 10 || MAGIC.some((byte, i) => lens[i] !== byte)) refuse('the lens is not an NPY 1.0 file')
  const view = new DataView(lens.buffer, lens.byteOffset, lens.byteLength)
  const headerLength = view.getUint16(8, true)
  const header = new TextDecoder('latin1').decode(lens.subarray(10, 10 + headerLength)).trimEnd()
  const shape = HEADER.exec(header) ?? refuse(`the lens header ${header} is not little-endian u64 rows of six`)
  const rows = Number(shape[1])
  const start = 10 + headerLength
  if (lens.length !== start + rows * COLUMNS * 8) refuse(`the lens holds ${lens.length - start} data bytes, not ${rows} rows`)
  const word = (row: number, column: number) => view.getBigUint64(start + (row * COLUMNS + column) * 8, true)
  return Array.from({ length: rows }, (_, row) => {
    const channel = safe(word(row, 0), 'a channel')
    if (channel >= channelCount) refuse(`row ${row} names channel ${channel} of ${channelCount}`)
    const collected = safe(word(row, 1), 'a collected time')
    const available = safe(word(row, 2), 'an available time')
    if (available < collected) refuse(`row ${row} is available before it is collected`)
    return { channel, collected, available, value: value(word(row, 3), word(row, 4), word(row, 5)) }
  })
}

export function decodeRun(files: RunFiles): Decoded {
  try {
    const names = channels(files.catalogue)
    const values = summary(files.summary)
    const read = observations(files.lens, names.length)
    if (read.length === 0) refuse('the lens holds no rows')
    const endTime = safe(BigInt(values.get('end_time_ns')!), 'the end time')
    const start = Math.min(...read.map((o) => o.collected))
    const end = Math.floor(endTime / NANOSECONDS_PER_SECOND) * NANOSECONDS_PER_SECOND
    if (read.some((o) => o.available > endTime)) refuse('a lens row is available after the run ends')
    if (end < start) refuse('the run ends before its first lens row')
    return {
      kind: 'decoded',
      run: { specId: values.get('spec_id')!, packet: values.get('packet')!, start, end, channels: names, observations: read },
    }
  } catch (error) {
    if (error instanceof Malformed) return { kind: 'refused', reason: error.message }
    throw error
  }
}
