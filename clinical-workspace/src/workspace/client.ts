import {
  type AmendmentBody,
  type EntryId,
  type EntryVersion,
  type NewEntryBody,
  parseReadModel,
  parseVersions,
  parseWriteResult,
  type ReadModel,
  type WriteResult,
} from '../chart-api.ts'

const TIMEOUT_MS = 3_000

async function call(path: string, init: RequestInit = {}): Promise<unknown> {
  try {
    const response = await fetch(path, { ...init, headers: { 'content-type': 'application/json' }, signal: AbortSignal.timeout(TIMEOUT_MS) })
    return response.status === 200 || response.status === 400 || response.status === 422 ? await response.json() : undefined
  } catch {
    return undefined
  }
}

const parsed = <T>(result: { readonly ok: true; readonly value: T } | { readonly ok: false }): T | undefined => (result.ok ? result.value : undefined)

export const chart = async (): Promise<ReadModel | undefined> => parsed(parseReadModel(await call('/api/chart')))

export const versions = async (entry: EntryId): Promise<readonly EntryVersion[] | undefined> =>
  parsed(parseVersions(await call(`/api/entries/${encodeURIComponent(entry)}/versions`)))

export const write = async (path: '/api/entries' | '/api/amendments', body: NewEntryBody | AmendmentBody): Promise<WriteResult | undefined> =>
  parsed(parseWriteResult(await call(path, { method: 'POST', body: JSON.stringify(body) })))
