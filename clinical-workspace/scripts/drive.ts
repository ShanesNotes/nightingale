import { spawn } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'

// Drives headless Chromium over the DevTools protocol with Node's own WebSocket, one command per call, so a
// verification run can script the pages step by step. The session (pid, port, named tabs) lives in
// .dev/drive.json. Run `node scripts/drive.ts help` for the commands.
const PACKAGE = fileURLToPath(new URL('..', import.meta.url))
const SESSION = `${PACKAGE}.dev/drive.json`
const CHROMIUM = process.env['CHROMIUM'] ?? '/usr/local/bin/chromium'
const USAGE = `usage: node scripts/drive.ts <command>
  start                         launch headless Chromium and record its pid and port
  open <tab> <url>              open a named tab at the url
  close <tab>                   close a named tab, as a person closes a page
  wait <tab> <js> [seconds]     wait until the expression is truthy in the tab (default 20 s)
  text <tab> <selector>         print the element's text
  click <tab> <selector>        click the element
  fill <tab> <selector> <text>  type the text into an input, as a person would
  choose <tab> <selector> <v>   choose the option with value v in a select
  eval <tab> <js>               print the expression's value as JSON
  offline <tab> <seconds>       cut the tab off the network for that long, then reconnect it
  shot <tab> <file>             save a full-page screenshot as PNG
  stop                          end Chromium by its recorded pid and remove its profile`

type Session = { readonly pid: number; readonly port: number; readonly profile: string; readonly tabs: Readonly<Record<string, string>> }

const fail = (reason: string): never => {
  process.stderr.write(`drive: ${reason}\n`)
  process.exit(1)
}

const sleep = (ms: number) => new Promise((done) => setTimeout(done, ms))

// Reads a nested field of a DevTools answer, which arrives as untyped JSON.
const read = (value: unknown, ...path: readonly string[]): unknown =>
  path.reduce<unknown>((v, key) => (typeof v === 'object' && v !== null ? Reflect.get(v, key) : undefined), value)

function session(): Session {
  if (!existsSync(SESSION)) return fail('no session; run `node scripts/drive.ts start` first')
  const value: unknown = JSON.parse(readFileSync(SESSION, 'utf8'))
  const ok = (v: unknown): v is Session => typeof v === 'object' && v !== null && 'pid' in v && 'port' in v && 'profile' in v && 'tabs' in v
  return ok(value) ? value : fail(`${SESSION} is not a drive session`)
}

const save = (value: Session) => {
  mkdirSync(`${PACKAGE}.dev`, { recursive: true })
  writeFileSync(SESSION, `${JSON.stringify(value, null, 2)}\n`)
}

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

async function devtools(port: number, path: string, method = 'GET'): Promise<unknown> {
  const response = await fetch(`http://127.0.0.1:${port}${path}`, { method })
  if (!response.ok) return fail(`DevTools answered ${response.status} to ${path}`)
  return response.json()
}

type Target = { readonly id: string; readonly webSocketDebuggerUrl: string }
const isTarget = (v: unknown): v is Target =>
  typeof v === 'object' && v !== null && 'id' in v && typeof v.id === 'string' && 'webSocketDebuggerUrl' in v && typeof v.webSocketDebuggerUrl === 'string'

// One DevTools connection to a tab for the length of a command.
async function connect(tab: string) {
  const { port, tabs } = session()
  const id = tabs[tab] ?? fail(`no tab named ${tab}`)
  const list = await devtools(port, '/json/list')
  const target = (Array.isArray(list) ? list : []).filter(isTarget).find((t) => t.id === id) ?? fail(`tab ${tab} is gone`)
  const socket = new WebSocket(target.webSocketDebuggerUrl)
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true })
    socket.addEventListener('error', reject, { once: true })
  })
  let next = 0
  const waiting = new Map<number, (message: unknown) => void>()
  socket.addEventListener('message', (event) => {
    const message: unknown = JSON.parse(String(event.data))
    const id = read(message, 'id')
    if (typeof id === 'number') {
      waiting.get(id)?.(message)
      waiting.delete(id)
    }
  })
  const send = (method: string, params: Readonly<Record<string, unknown>> = {}) =>
    new Promise<unknown>((resolve) => {
      next += 1
      waiting.set(next, resolve)
      socket.send(JSON.stringify({ id: next, method, params }))
    })
  const evaluate = async (expression: string): Promise<unknown> => {
    const answer = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })
    const thrown = read(answer, 'result', 'exceptionDetails')
    if (thrown !== undefined) return fail(`the page threw: ${JSON.stringify(thrown)}`)
    return read(answer, 'result', 'result', 'value')
  }
  return { send, evaluate, close: () => socket.close() }
}

const quoted = (text: string) => JSON.stringify(text)

const element = (selector: string, then: string) =>
  `(() => { const el = document.querySelector(${quoted(selector)}); if (!el) throw new Error('no element matches ' + ${quoted(selector)}); ${then} })()`

async function command(args: readonly string[]): Promise<void> {
  const [name, tab = '', ...rest] = args
  switch (name) {
    case 'start': {
      const port = await freePort()
      const profile = mkdtempSync(join(tmpdir().length > 60 ? '/tmp' : tmpdir(), 'cw-drive-'))
      const chromium = spawn(
        CHROMIUM,
        [
          '--headless=new',
          `--remote-debugging-port=${port}`,
          `--user-data-dir=${profile}`,
          '--no-first-run',
          '--no-default-browser-check',
          '--hide-scrollbars',
          '--disable-background-timer-throttling',
          '--disable-renderer-backgrounding',
          '--disable-backgrounding-occluded-windows',
          '--window-size=1400,1000',
          'about:blank',
        ],
        { detached: true, stdio: 'ignore' },
      )
      chromium.unref()
      if (chromium.pid === undefined) return fail(`could not start ${CHROMIUM}`)
      for (let tries = 0; tries < 80; tries += 1) {
        const up = await fetch(`http://127.0.0.1:${port}/json/version`).then((r) => r.ok, () => false)
        if (up) {
          save({ pid: chromium.pid, port, profile, tabs: {} })
          process.stdout.write(`drive: Chromium pid ${chromium.pid} on port ${port}\n`)
          return
        }
        await sleep(250)
      }
      return fail('Chromium did not answer on its DevTools port')
    }
    case 'open': {
      const [url = fail('open needs a url')] = rest
      const current = session()
      const target = await devtools(current.port, `/json/new?${encodeURI(url)}`, 'PUT')
      if (!isTarget(target)) return fail('DevTools opened no tab')
      save({ ...current, tabs: { ...current.tabs, [tab]: target.id } })
      process.stdout.write(`drive: opened ${tab} at ${url}\n`)
      return
    }
    case 'close': {
      const current = session()
      const id = current.tabs[tab] ?? fail(`no tab named ${tab}`)
      await devtools(current.port, `/json/close/${id}`)
      const tabs = Object.fromEntries(Object.entries(current.tabs).filter(([key]) => key !== tab))
      save({ ...current, tabs })
      process.stdout.write(`drive: closed ${tab}\n`)
      return
    }
    case 'stop': {
      const current = session()
      try {
        process.kill(-current.pid, 'SIGTERM')
      } catch {
        try {
          process.kill(current.pid, 'SIGTERM')
        } catch {
          // Chromium has already ended.
        }
      }
      await sleep(500)
      rmSync(current.profile, { recursive: true, force: true })
      rmSync(SESSION, { force: true })
      process.stdout.write(`drive: stopped Chromium pid ${current.pid}\n`)
      return
    }
    case 'help':
      process.stdout.write(`${USAGE}\n`)
      return
  }
  const page = await connect(tab)
  try {
    switch (name) {
      case 'wait': {
        const [expression = fail('wait needs an expression'), seconds = '20'] = rest
        for (let waited = 0; waited < Number(seconds) * 1000; waited += 200) {
          if (await page.evaluate(`Boolean(${expression})`)) return
          await sleep(200)
        }
        return fail(`timed out after ${seconds} s waiting for ${expression}`)
      }
      case 'text': {
        const [selector = fail('text needs a selector')] = rest
        process.stdout.write(`${String(await page.evaluate(element(selector, 'return el.innerText')))}\n`)
        return
      }
      case 'click': {
        const [selector = fail('click needs a selector')] = rest
        await page.evaluate(element(selector, 'el.click(); return true'))
        return
      }
      case 'fill': {
        const [selector = fail('fill needs a selector'), text = ''] = rest
        await page.evaluate(
          element(selector, `el.focus(); el.value = ${quoted(text)}; el.dispatchEvent(new Event('input', { bubbles: true })); return true`),
        )
        return
      }
      case 'choose': {
        const [selector = fail('choose needs a selector'), value = fail('choose needs a value')] = rest
        await page.evaluate(element(selector, `el.value = ${quoted(value)}; el.dispatchEvent(new Event('change', { bubbles: true })); return true`))
        return
      }
      case 'eval': {
        const [expression = fail('eval needs an expression')] = rest
        process.stdout.write(`${JSON.stringify(await page.evaluate(expression))}\n`)
        return
      }
      case 'offline': {
        const [held = fail('offline needs seconds')] = rest
        const cut = (offline: boolean) =>
          page.send('Network.emulateNetworkConditions', { offline, latency: 0, downloadThroughput: -1, uploadThroughput: -1 })
        // The emulation lasts only while this connection is open, so the command holds it for the whole span.
        await page.send('Network.enable')
        await cut(true)
        process.stdout.write(`drive: ${tab} offline for ${held} s\n`)
        await sleep(Number(held) * 1000)
        await cut(false)
        process.stdout.write(`drive: ${tab} back online\n`)
        return
      }
      case 'shot': {
        const [file = fail('shot needs a file')] = rest
        const metrics = await page.send('Page.getLayoutMetrics')
        const measured = (key: string, fallback: number) => {
          const value = read(metrics, 'result', 'cssContentSize', key)
          return Math.ceil(typeof value === 'number' ? value : fallback)
        }
        const clip = { x: 0, y: 0, width: measured('width', 1400), height: measured('height', 1000), scale: 1 }
        const shot = await page.send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip })
        const data = read(shot, 'result', 'data')
        if (typeof data !== 'string') return fail('no screenshot data')
        writeFileSync(file, Buffer.from(data, 'base64'))
        process.stdout.write(`drive: ${file}\n`)
        return
      }
      default:
        return fail(`unknown command ${String(name)}\n${USAGE}`)
    }
  } finally {
    page.close()
  }
}

await command(process.argv.slice(2))
