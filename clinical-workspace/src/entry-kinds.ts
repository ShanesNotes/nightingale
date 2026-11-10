import { VITALS, type Vital } from './vitals.ts'

export type Fit<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly reason: string }

export type Field<T> = {
  readonly name: string
  readonly label: string
  readonly input: 'line' | 'multiline' | readonly { readonly value: string; readonly label: string }[]
  parse(value: unknown, from: 'write' | 'read'): Fit<T>
  readonly column: string
}

export const sqlName = (name: string) => `"${name.replaceAll('"', '""')}"`
export const sqlText = (text: string) => `'${text.replaceAll("'", "''")}'`

const fits = <T>(value: T): Fit<T> => ({ ok: true, value })
const misfit = (reason: string): Fit<never> => ({ ok: false, reason })

const choices = <T extends string>(name: string, label: string, options: readonly { readonly value: T; readonly label: string }[]): Field<T> => ({
  name,
  label,
  input: options,
  parse(value) {
    const chosen = options.find((option) => option.value === value)
    return chosen === undefined ? misfit(`The ${label.toLowerCase()} is not one the chart lists`) : fits(chosen.value)
  },
  column: `text NOT NULL CHECK (${sqlName(name)} IN (${options.map((option) => sqlText(option.value)).join(', ')}))`,
})

const DECIMAL = /^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?$/
const numberField = (name: string, label: string): Field<number> => ({
  name,
  label,
  input: 'line',
  parse(value, from) {
    const number = from === 'write' ? (typeof value === 'string' && DECIMAL.test(value.trim()) ? Number(value) : undefined) : value
    return typeof number === 'number' && Number.isFinite(number) ? fits(number) : misfit(`The ${label.toLowerCase()} is not a number`)
  },
  column: `double precision NOT NULL CHECK (${sqlName(name)} NOT IN ('NaN', 'Infinity', '-Infinity'))`,
})

const textField = (name: string, label: string, limit: number): Field<string> => ({
  name,
  label,
  input: 'multiline',
  parse(value) {
    const what = label.toLowerCase()
    if (typeof value !== 'string' || !/[^ \t\n\r\f\v]/.test(value)) return misfit(`The ${what} is blank`)
    if (value.includes('\0')) return misfit(`The ${what} holds a null character`)
    if ([...value].length > limit) return misfit(`The ${what} is longer than ${limit.toLocaleString('en-US')} characters`)
    return fits(value)
  },
  column: `text NOT NULL CHECK (length(${sqlName(name)}) BETWEEN 1 AND ${limit} AND ${sqlName(name)} ~ E'[^ \\t\\n\\r\\f\\v]')`,
})

class Misfit extends Error {}

const kindOf = <K extends string, C>(kind: K, label: string, fields: readonly Field<unknown>[], read: (take: <T>(field: Field<T>) => T) => C) => ({
  kind,
  label,
  fields,
  parse(content: unknown, from: 'write' | 'read'): Fit<{ readonly kind: K; readonly content: Readonly<C> }> {
    const what = label.toLowerCase()
    if (typeof content !== 'object' || content === null || Array.isArray(content)) return misfit(`The ${what} content is not a set of named fields`)
    const stray = Object.keys(content).find((name) => !fields.some((field) => field.name === name))
    if (stray !== undefined) return misfit(`A ${what} has no field named ${stray}`)
    const take = <T>(field: Field<T>): T => {
      if (!Object.hasOwn(content, field.name)) throw new Misfit(`The ${what} has no ${field.label.toLowerCase()}`)
      const fit = field.parse(Reflect.get(content, field.name), from)
      if (!fit.ok) throw new Misfit(fit.reason)
      return fit.value
    }
    try {
      return fits({ kind, content: read(take) })
    } catch (error) {
      if (error instanceof Misfit) return misfit(error.message)
      throw error
    }
  },
})

const vital = choices<Vital>('vital', 'Vital', VITALS.map((row) => ({ value: row.vital, label: `${row.label} (${row.unit})` })))
const value = numberField('value', 'Value')
const text = textField('text', 'Note text', 10_000)

export const ENTRY_KINDS = {
  vital: kindOf('vital', 'Vital', [vital, value], (take) => ({ vital: take(vital), value: take(value) })),
  note: kindOf('note', 'Note', [text], (take) => ({ text: take(text) })),
} as const

export type EntryKind = keyof typeof ENTRY_KINDS
export type EntryContent<K extends EntryKind = EntryKind> = { [P in K]: Extract<ReturnType<(typeof ENTRY_KINDS)[P]['parse']>, { ok: true }>['value'] }[K]
export type KindRefusal = { readonly kind: 'unknown entry kind' } | { readonly kind: 'content does not fit kind'; readonly reason: string }

export const isEntryKind = (value: unknown): value is EntryKind => typeof value === 'string' && Object.hasOwn(ENTRY_KINDS, value)
export const kindFields = (kind: EntryKind): readonly Field<unknown>[] => ENTRY_KINDS[kind].fields

export function parseContent(kind: unknown, content: unknown, from: 'write' | 'read'):
  { readonly ok: true; readonly value: EntryContent } | { readonly ok: false; readonly refusal: KindRefusal } {
  if (!isEntryKind(kind)) return { ok: false, refusal: { kind: 'unknown entry kind' } }
  const fit = ENTRY_KINDS[kind].parse(content, from)
  return fit.ok ? fit : { ok: false, refusal: { kind: 'content does not fit kind', reason: fit.reason } }
}

export const blankContent = (kind: EntryKind): Readonly<Record<string, string>> =>
  Object.fromEntries(kindFields(kind).map((field) => [field.name, typeof field.input === 'string' ? '' : field.input[0]?.value ?? '']))
