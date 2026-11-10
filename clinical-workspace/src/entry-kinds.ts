import { VITALS, type Vital } from './vitals.ts'

type Field<T = unknown> = {
  readonly label: string
  readonly input: 'line' | 'multiline' | readonly { readonly value: string; readonly label: string }[]
  parse(value: unknown, from: 'write' | 'read'): T | undefined
  column(name: string): string
}

export const sqlName = (name: string) => `"${name.replaceAll('"', '""')}"`
export const sqlText = (text: string) => `'${text.replaceAll("'", "''")}'`

const choices = <T extends string>(label: string, options: readonly { readonly value: T; readonly label: string }[]): Field<T> => ({
  label,
  input: options,
  parse: (value) => options.find((option) => option.value === value)?.value,
  column: (name) => `text NOT NULL CHECK (${name} IN (${options.map((option) => sqlText(option.value)).join(', ')}))`,
})

const DECIMAL = /^[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?$/
const numberField = (label: string): Field<number> => ({
  label,
  input: 'line',
  parse(value, from) {
    const number = from === 'write' && typeof value === 'string' && DECIMAL.test(value.trim()) ? Number(value) : from === 'read' ? value : undefined
    return typeof number === 'number' && Number.isFinite(number) ? number : undefined
  },
  column: (name) => `double precision NOT NULL CHECK (${name} NOT IN ('NaN', 'Infinity', '-Infinity'))`,
})

const textField = (label: string, limit: number): Field<string> => ({
  label,
  input: 'multiline',
  parse: (value) => typeof value === 'string' && /[^ \t\n\r\f\v]/.test(value) && !value.includes('\0') && [...value].length <= limit ? value : undefined,
  column: (name) => `text NOT NULL CHECK (length(${name}) BETWEEN 1 AND ${limit} AND ${name} ~ E'[^ \\t\\n\\r\\f\\v]')`,
})

export const ENTRY_KINDS = {
  vital: {
    label: 'Vital',
    fields: {
      vital: choices<Vital>('Vital', VITALS.map((row) => ({ value: row.vital, label: `${row.label} (${row.unit})` }))),
      value: numberField('Value'),
    },
  },
  note: { label: 'Note', fields: { text: textField('Note text', 10_000) } },
} as const

export type EntryKind = keyof typeof ENTRY_KINDS
type Content<K extends EntryKind> = { readonly [P in keyof typeof ENTRY_KINDS[K]['fields']]: typeof ENTRY_KINDS[K]['fields'][P] extends Field<infer T> ? T : never }
export type EntryContent<K extends EntryKind = EntryKind> = { [P in K]: { readonly kind: P; readonly content: Content<P> } }[K]
export type KindRefusal = { readonly kind: 'unknown entry kind' } | { readonly kind: 'content does not fit kind' }

export const isEntryKind = (value: unknown): value is EntryKind => typeof value === 'string' && Object.hasOwn(ENTRY_KINDS, value)
export const kindFields = (kind: EntryKind): Readonly<Record<string, Field>> => ENTRY_KINDS[kind].fields

export function parseContent(kind: unknown, content: unknown, from: 'write' | 'read'):
  { readonly ok: true; readonly value: EntryContent } | { readonly ok: false; readonly refusal: KindRefusal } {
  if (!isEntryKind(kind)) return { ok: false, refusal: { kind: 'unknown entry kind' } }
  const invalid = { ok: false, refusal: { kind: 'content does not fit kind' } } as const
  if (typeof content !== 'object' || content === null || Array.isArray(content)) return invalid
  const fields = kindFields(kind)
  if (Object.keys(content).length !== Object.keys(fields).length) return invalid
  const parsed: Record<string, unknown> = {}
  for (const [name, field] of Object.entries(fields)) {
    if (!Object.hasOwn(content, name)) return invalid
    const value = field.parse(Reflect.get(content, name), from)
    if (value === undefined) return invalid
    parsed[name] = value
  }
  return { ok: true, value: { kind, content: parsed } as EntryContent }
}

export const blankContent = (kind: EntryKind): Readonly<Record<string, string>> => Object.fromEntries(
  Object.entries(kindFields(kind)).map(([name, field]) => [name, typeof field.input === 'string' ? '' : field.input[0]?.value ?? '']),
)
