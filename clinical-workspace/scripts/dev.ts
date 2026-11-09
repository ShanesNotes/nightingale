import { type ChildProcess, execFileSync, spawn } from 'node:child_process'
import { mkdirSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { fileURLToPath } from 'node:url'
import pg from 'pg'

// npm run dev: a local Postgres 17 container (made once, kept between runs), the chart service, then Vite with
// /api proxied to the service and the run's anchor given to both pages. It records every pid in .dev/state.json
// and stops the service and Vite when it ends, leaving the database running.
const PACKAGE = fileURLToPath(new URL('..', import.meta.url))
const CONTAINER = process.env['CW_DB_CONTAINER'] ?? 'clinical-workspace-pg'
const IMAGE = 'postgres:17'
const STATE = `${PACKAGE}.dev/state.json`

const say = (line: string) => process.stdout.write(`${line}\n`)

const fail = (reason: string): never => {
  process.stderr.write(`dev: ${reason}\n`)
  process.exit(1)
}

const docker = (...args: string[]) => execFileSync('docker', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer()
    server.once('error', reject)
    server.listen(0, '127.0.0.1', () => {
      const bound = server.address()
      const port = bound !== null && typeof bound !== 'string' ? bound.port : 0
      server.close(() => (port > 0 ? resolve(port) : reject(new Error('no free port'))))
    })
  })
}

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms))

async function until<T>(what: string, seconds: number, attempt: () => Promise<T | undefined>): Promise<T> {
  for (let tries = 0; tries < seconds * 4; tries += 1) {
    const value = await attempt().catch(() => undefined)
    if (value !== undefined) return value
    await sleep(250)
  }
  return fail(`${what} not ready after ${seconds} s`)
}

// The container is made once with a free port and a named volume, and started again on later runs.
async function database(): Promise<string> {
  const running = (() => {
    try {
      return docker('inspect', '-f', '{{.State.Running}}', CONTAINER)
    } catch {
      return 'absent'
    }
  })()
  if (running === 'absent') {
    const port = await freePort()
    docker('run', '-d', '--name', CONTAINER, '-e', 'POSTGRES_HOST_AUTH_METHOD=trust', '-p', `127.0.0.1:${port}:5432`, '-v', `${CONTAINER}:/var/lib/postgresql/data`, IMAGE)
    say(`dev: made the database container ${CONTAINER} on 127.0.0.1:${port}`)
  } else if (running === 'false') {
    docker('start', CONTAINER)
    say(`dev: started the database container ${CONTAINER}`)
  }
  const mapped = docker('port', CONTAINER, '5432/tcp').split('\n')[0] ?? ''
  const url = `postgres://postgres@${mapped}/postgres`
  // The image's first start runs initdb on a socket-only server, so a TCP query answers only once that is done.
  await until('the database', 90, async () => {
    const client = new pg.Client({ connectionString: url })
    await client.connect()
    await client.query('SELECT 1')
    await client.end()
    return true
  })
  return url
}

function start(command: string, args: readonly string[], env: NodeJS.ProcessEnv): ChildProcess {
  const child = spawn(command, [...args], { cwd: PACKAGE, env, detached: true, stdio: ['ignore', 'pipe', 'inherit'] })
  child.stdout?.on('data', (chunk: Buffer) => process.stdout.write(chunk))
  return child
}

function firstLine(child: ChildProcess, pattern: RegExp, seconds: number): Promise<RegExpExecArray> {
  return new Promise((resolve, reject) => {
    let seen = ''
    const timer = setTimeout(() => reject(new Error(`no line like ${pattern} in ${seconds} s`)), seconds * 1000)
    child.stdout?.on('data', (chunk: Buffer) => {
      seen += chunk.toString('utf8')
      const match = seen.split('\n').map((line) => pattern.exec(line)).find((m) => m !== null)
      if (match) {
        clearTimeout(timer)
        resolve(match)
      }
    })
    child.once('exit', (code) => reject(new Error(`exited with ${code} before a line like ${pattern}`)))
  })
}

const stopGroup = (child: ChildProcess) => {
  if (child.pid === undefined) return
  try {
    process.kill(-child.pid, 'SIGTERM')
  } catch {
    // The group has already ended.
  }
}

const url = await database()
const service = start(process.execPath, ['scripts/service.ts'], { ...process.env, CHART_DATABASE_URL: url, PORT: '0' })
const [, serviceUrl, anchor] = await firstLine(service, /^chart service (\S+) anchor (\d+)$/, 60).catch((error: unknown) => fail(String(error)))
const vitePort = await freePort()
const vite = start(process.execPath, ['node_modules/vite/bin/vite.js', '--port', String(vitePort), '--strictPort', '--host', '127.0.0.1'], {
  ...process.env,
  CHART_SERVICE_URL: serviceUrl,
  VITE_RUN_ANCHOR: anchor,
})
const pages = { workspace: `http://127.0.0.1:${vitePort}/`, monitor: `http://127.0.0.1:${vitePort}/monitor.html` }
await until('Vite', 60, async () => ((await fetch(pages.workspace)).ok ? true : undefined))

mkdirSync(`${PACKAGE}.dev`, { recursive: true })
writeFileSync(
  STATE,
  `${JSON.stringify({ launcher: process.pid, database: { container: CONTAINER, url }, service: { pid: service.pid, url: serviceUrl }, vite: { pid: vite.pid, url: pages.workspace }, anchor: Number(anchor), pages }, null, 2)}\n`,
)
say(`dev: workspace ${pages.workspace}`)
say(`dev: monitor ${pages.monitor}`)
say(`dev: pids in ${STATE}; the database keeps running after this ends (npm run db:stop stops it)`)

const end = () => {
  stopGroup(vite)
  stopGroup(service)
  process.exit(0)
}
process.once('SIGINT', end)
process.once('SIGTERM', end)
service.once('exit', () => fail('the chart service exited'))
vite.once('exit', () => fail('Vite exited'))
