export type Parsed<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly reason: string }

export function parseDeviceMessage(json: unknown): Parsed<unknown> {
  return { ok: true, value: json }
}

export function parseBatch(json: unknown): Parsed<unknown> {
  return { ok: true, value: json }
}

export function parseAnswers(json: unknown): Parsed<unknown> {
  return typeof json === 'object' && json !== null && 'answers' in json
    ? { ok: true, value: json.answers }
    : { ok: false, reason: 'no answers' }
}
