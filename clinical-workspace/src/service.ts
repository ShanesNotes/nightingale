import { createServer, type IncomingMessage, type ServerResponse } from 'node:http'
import { isEntryId, parseAmendment, parseNewEntry, type WriteResult } from './chart-api.ts'
import type { Chart } from './chart.ts'
import { parseBatch } from './device.ts'
import type { PatientTime } from './time.ts'

export type Service = { readonly url: string; close(): Promise<void> }

export type ServiceOptions = {
  readonly chart: Chart
  readonly now: () => PatientTime
  readonly port: number
  readonly host?: string
}

const BODY_LIMIT = 8 * 1024 * 1024
const VERSIONS = /^\/api\/entries\/([^/]+)\/versions$/

type Answer = { readonly status: number; readonly json: unknown }

const malformed = (reason: string): Answer => ({ status: 400, json: { kind: 'refused', refusal: { kind: 'malformed', reason } } })
const written = (result: WriteResult): Answer => ({ status: result.kind === 'taken' ? 200 : 422, json: result })

async function body(request: IncomingMessage): Promise<{ readonly ok: true; readonly json: unknown } | { readonly ok: false }> {
  let size = 0
  const chunks: Buffer[] = []
  for await (const chunk of request) {
    const buffer = Buffer.isBuffer(chunk) ? chunk : Buffer.from(String(chunk))
    size += buffer.length
    if (size > BODY_LIMIT) return { ok: false }
    chunks.push(buffer)
  }
  try {
    return { ok: true, json: JSON.parse(Buffer.concat(chunks).toString('utf8')) }
  } catch {
    return { ok: false }
  }
}

async function route(options: ServiceOptions, request: IncomingMessage): Promise<Answer> {
  const { chart, now } = options
  const path = new URL(request.url ?? '/', 'http://chart.invalid').pathname
  const post = async (parse: (json: unknown) => Answer | Promise<Answer>) => {
    const read = await body(request)
    return read.ok ? parse(read.json) : malformed('the body is not JSON')
  }
  if (request.method === 'GET' && path === '/api/run') return { status: 200, json: chart.run }
  if (request.method === 'GET' && path === '/api/chart') return { status: 200, json: await chart.readModel(now()) }
  const versions = VERSIONS.exec(path)
  if (request.method === 'GET' && versions !== null) {
    const id = decodeURIComponent(versions[1] ?? '')
    const held = isEntryId(id) ? await chart.versions(id) : undefined
    return held === undefined ? { status: 404, json: { error: 'no such entry' } } : { status: 200, json: held }
  }
  if (request.method === 'POST' && path === '/api/entries') {
    return post(async (json) => {
      const parsed = parseNewEntry(json)
      return parsed.ok ? written(await chart.write(parsed.value, now())) : { status: 400, json: { kind: 'refused', refusal: parsed.refusal } }
    })
  }
  if (request.method === 'POST' && path === '/api/amendments') {
    return post(async (json) => {
      const parsed = parseAmendment(json)
      return parsed.ok ? written(await chart.write(parsed.value, now())) : { status: 400, json: { kind: 'refused', refusal: parsed.refusal } }
    })
  }
  if (request.method === 'POST' && path === '/api/device/messages') {
    return post(async (json) => {
      const parsed = parseBatch(json)
      return parsed.ok ? { status: 200, json: { answers: await chart.writeDeviceBatch(parsed.value, now()) } } : malformed(parsed.reason)
    })
  }
  return { status: 404, json: { error: 'no such route' } }
}

function send(response: ServerResponse, answer: Answer) {
  response.writeHead(answer.status, { 'content-type': 'application/json', 'cache-control': 'no-store' })
  response.end(JSON.stringify(answer.json))
}

export function startService(options: ServiceOptions): Promise<Service> {
  const server = createServer((request, response) => {
    route(options, request).then(
      (answer) => send(response, answer),
      (error: unknown) => send(response, { status: 500, json: { error: error instanceof Error ? error.message : String(error) } }),
    )
  })
  return new Promise((resolve, reject) => {
    server.once('error', reject)
    server.listen(options.port, options.host ?? '127.0.0.1', () => {
      const bound = server.address()
      if (bound === null || typeof bound === 'string') return reject(new Error('the chart service bound no TCP port'))
      resolve({
        url: `http://${bound.address}:${bound.port}`,
        close: () => new Promise((done, fail) => server.close((error) => (error ? fail(error) : done()))),
      })
    })
  })
}
