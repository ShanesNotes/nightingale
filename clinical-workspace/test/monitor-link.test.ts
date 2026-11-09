import assert from 'node:assert/strict'
import { test } from 'node:test'
import { openChart, pgliteSql } from '../src/chart.ts'
import { isPatientId, parseBatch } from '../src/device.ts'
import { httpLink } from '../src/monitor/link.ts'
import type { Link } from '../src/monitor/link.ts'
import { startService } from '../src/service.ts'
import { wallTime } from '../src/time.ts'
import { at } from './fixtures.ts'
import { chartDouble } from './link-double.ts'

const patient = 'test-patient'
assert.ok(isPatientId(patient))

const batch = (...messages: object[]) => {
  const parsed = parseBatch(messages)
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}

const header = { device: 'test-monitor', boot: 'test-boot-1', patient }
const statement = {
  kind: 'statement',
  ...header,
  sequence: 1,
  at: at('06:56:00'),
  channels: { HR: 'sourced', 'ABP sys': 'no source', 'ABP dia': 'no source', 'ABP mean': 'sourced', SpO2: 'no source', RR: 'no source', Temp: 'no source' },
}
const reading = (sequence: number, value: number) => ({
  kind: 'reading',
  ...header,
  sequence,
  vital: 'HR',
  collected: at('06:56:00'),
  available: at('06:56:00'),
  value: { kind: 'present', value },
})

// The same exchange through either adapter: new messages, the same ones again, then one that reuses a sequence.
async function exchange(link: Link) {
  return [
    await link.send(batch(statement, reading(2, 61.5))),
    await link.send(batch(statement, reading(2, 61.5), reading(3, 62.5))),
    await link.send(batch(reading(3, 99.5), { ...reading(4, 63.5), patient: 'another-test-patient' })),
  ]
}

test('the HTTP link and the in-memory chart answer alike, and both leave a batch unanswered when the service is down', async () => {
  const sql = await pgliteSql()
  const chart = await openChart(sql, { specId: 'test-spec', patient, start: at('06:55:00'), end: at('07:05:00') }, wallTime(1_000_000))
  const service = await startService({ chart, now: () => at('06:56:01'), port: 0 })
  const http = httpLink(`${service.url}/api/device/messages`)
  const double = chartDouble(patient)
  try {
    const overHttp = await exchange(http)
    assert.deepEqual(overHttp, await exchange(double.link))
    assert.deepEqual(
      overHttp.map((result) => (result.kind === 'answered' ? result.answers.map((a) => (a.answer === 'refused' ? a.refusal : a.answer)) : result.kind)),
      [
        ['taken', 'taken'],
        ['already held', 'already held', 'taken'],
        ['sequence reused with other content', 'another patient'],
      ],
    )
    assert.deepEqual(await httpLink(`${service.url}/api/nothing`).send(batch(reading(5, 64.5))), { kind: 'unanswered' }, 'an error answer')
  } finally {
    await service.close()
    await sql.close()
  }
  assert.deepEqual(await http.send(batch(reading(5, 64.5))), { kind: 'unanswered' }, 'a refused connection')
  double.set('down')
  assert.deepEqual(await double.link.send(batch(reading(5, 64.5))), { kind: 'unanswered' })
})
