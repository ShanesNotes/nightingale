export type VitalRow = {
  readonly vital: string
  readonly label: string
  readonly unit: string
}

export const VITALS = [
  { vital: 'HR', label: 'Heart rate', unit: '/min' },
  { vital: 'ABP sys', label: 'Systolic arterial pressure', unit: 'mmHg' },
  { vital: 'ABP dia', label: 'Diastolic arterial pressure', unit: 'mmHg' },
  { vital: 'ABP mean', label: 'Mean arterial pressure', unit: 'mmHg' },
  { vital: 'SpO2', label: 'SpO2', unit: '%' },
  { vital: 'RR', label: 'Respiratory rate', unit: '/min' },
  { vital: 'Temp', label: 'Temperature', unit: '°C' },
] as const satisfies readonly VitalRow[]

export type Vital = (typeof VITALS)[number]['vital']

const NAMES: ReadonlySet<unknown> = new Set(VITALS.map((row) => row.vital))

export function isVital(value: unknown): value is Vital {
  return NAMES.has(value)
}

export type PerVital<T> = { readonly [V in Vital]: T }

const covers = <T>(record: Readonly<Record<string, T>>): record is PerVital<T> => VITALS.every((row) => row.vital in record)

export function perVital<T>(make: (vital: Vital) => T): PerVital<T> {
  const record = Object.fromEntries(VITALS.map((row) => [row.vital, make(row.vital)]))
  if (!covers(record)) throw new Error('a record built from the vitals table lacks one of its vitals')
  return record
}
