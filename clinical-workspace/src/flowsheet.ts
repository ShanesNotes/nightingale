import type { EntryView, ReadModel, ReadingView, StatementView } from './chart-api.ts'
import type { Source } from './device.ts'
import { add, type Duration, earlier, floorTo, later, MINUTE, type PatientTime, SECOND, seconds, since, ZERO } from './time.ts'
import { VITALS, type VitalRow } from './vitals.ts'

// A pure grid over the chart's read model: it computes and never stores.
export type MonitorCell =
  | { readonly kind: 'no statement' }
  | { readonly kind: 'no source' }
  | {
      readonly kind: 'readings'
      readonly latest: { readonly value: number; readonly collected: PatientTime } | undefined
      readonly absent: number
      readonly late: number
      readonly missing: number
    }

export type ChartedValue = { readonly entry: EntryView; readonly lateBy: Duration; readonly edited: boolean }

export type ChartedCell = { readonly kind: 'entries'; readonly entries: readonly ChartedValue[] }

export type FlowsheetRow =
  | { readonly source: 'monitor'; readonly vital: VitalRow; readonly cells: readonly MonitorCell[] }
  | { readonly source: 'charted'; readonly vital: VitalRow; readonly cells: readonly ChartedCell[] }

export type Flowsheet = { readonly minutes: readonly PatientTime[]; readonly rows: readonly FlowsheetRow[] }

const within = (time: PatientTime, minute: PatientTime) => minute <= time && time < add(minute, MINUTE)

const lateReading = (r: ReadingView) => floorTo(r.arrived, SECOND) > floorTo(r.available, SECOND)

function minutes(model: ReadModel): PatientTime[] {
  const first = model.entries.reduce((soonest, e) => earlier(soonest, e.care), model.run.start)
  const out: PatientTime[] = []
  for (let minute = floorTo(first, MINUTE); minute <= model.now; minute = add(minute, MINUTE)) out.push(minute)
  return out
}

function statementAt(statements: readonly StatementView[], time: PatientTime): StatementView | undefined {
  return statements.filter((s) => s.at <= time).reduce<StatementView | undefined>((latest, s) => (latest === undefined || s.at >= latest.at ? s : latest), undefined)
}

// A vital the device's table lacks, such as one added after it spoke, has no statement.
function sourceOf(statement: StatementView | undefined, vital: string): Source | undefined {
  const channels: Readonly<Record<string, Source>> | undefined = statement?.channels
  return channels?.[vital]
}

const LAST_SECOND = seconds(59)

function missingIn(model: ReadModel, vital: string, minute: PatientTime): number {
  const last = add(minute, LAST_SECOND)
  return model.missing
    .filter((span) => span.vital === vital)
    .reduce((count, span) => {
      const from = later(span.from, minute)
      const through = earlier(span.through, last)
      return through < from ? count : count + since(through, from) / SECOND + 1
    }, 0)
}

function monitorCell(model: ReadModel, vital: string, minute: PatientTime): MonitorCell {
  const readings = model.readings.filter((r) => r.vital === vital && within(r.collected, minute))
  const missing = missingIn(model, vital, minute)
  if (readings.length === 0 && missing === 0) {
    const source = sourceOf(statementAt(model.statements, earlier(add(minute, LAST_SECOND), model.now)), vital)
    if (source === undefined) return { kind: 'no statement' }
    if (source === 'no source') return { kind: 'no source' }
  }
  const latest = readings.reduce<{ value: number; collected: PatientTime } | undefined>(
    (best, r) => (r.value.kind === 'present' && (best === undefined || r.collected >= best.collected) ? { value: r.value.value, collected: r.collected } : best),
    undefined,
  )
  return {
    kind: 'readings',
    latest,
    absent: readings.filter((r) => r.value.kind === 'absent').length,
    late: readings.filter(lateReading).length,
    missing,
  }
}

function chartedCell(model: ReadModel, vital: string, minute: PatientTime): ChartedCell {
  const entries = model.entries.filter((e) => e.vital === vital && within(e.care, minute))
  const lateBy = (entry: EntryView) => (entry.charted > entry.care ? since(entry.charted, entry.care) : ZERO)
  return { kind: 'entries', entries: entries.map((entry) => ({ entry, lateBy: lateBy(entry), edited: entry.versions > 1 })) }
}

export function flowsheet(model: ReadModel, vitals: readonly VitalRow[] = VITALS): Flowsheet {
  const columns = minutes(model)
  return {
    minutes: columns,
    rows: vitals.flatMap((vital): FlowsheetRow[] => [
      { source: 'monitor', vital, cells: columns.map((minute) => monitorCell(model, vital.vital, minute)) },
      { source: 'charted', vital, cells: columns.map((minute) => chartedCell(model, vital.vital, minute)) },
    ]),
  }
}
