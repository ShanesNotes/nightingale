import assert from 'node:assert/strict'
import { describe, test } from 'node:test'
import { isEntryId, parseAmendment, parseNewEntry, ROSTER, type WriteResult } from '../src/chart-api.ts'
import { type Chart, openChart, pgliteSql, pgSql, type Sql } from '../src/chart.ts'
import { isPatientId, parseBatch } from '../src/device.ts'
import { formatClock, parseClock, wallTime } from '../src/time.ts'

const branded = <T>(value: unknown, is: (v: unknown) => v is T): T => {
  assert.ok(is(value), String(value))
  return value
}

const at = (clock: string) => {
  const time = parseClock(clock)
  assert.ok(time !== undefined, clock)
  return time
}

const RUN = { specId: 'test-spec', patient: branded('test-patient', isPatientId), start: at('06:55:00'), end: at('07:05:00') }

const nurse = ROSTER[0]!

const newEntry = (body: Readonly<Record<string, unknown>>) => {
  const parsed = parseNewEntry({ patient: 'test-patient', author: nurse.id, vital: 'RR', value: '18', care: null, ...body })
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}

const amendment = (body: Readonly<Record<string, unknown>>) => {
  const parsed = parseAmendment({ patient: 'test-patient', author: nurse.id, replaces: 1, value: '18', care: null, ...body })
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}

const taken = (result: WriteResult) => {
  assert.ok(result.kind === 'taken', JSON.stringify(result))
  return result.entry
}

const DEVICE = { device: 'test-monitor', boot: 'test-boot-1', patient: 'test-patient' }
const SOURCED = { HR: 'sourced', 'ABP sys': 'no source', 'ABP dia': 'no source', 'ABP mean': 'sourced', SpO2: 'no source', RR: 'no source', Temp: 'no source' }

const deviceReading = (sequence: number, collected: string, value: number | 'not collected' = 1.5, header: object = DEVICE) => ({
  kind: 'reading',
  ...header,
  sequence,
  vital: 'HR',
  collected: at(collected),
  available: at(collected),
  value: value === 'not collected' ? { kind: 'absent', reason: value } : { kind: 'present', value },
})

const deviceStatement = (sequence: number, clock: string, channels: object = SOURCED, header: object = DEVICE) => ({
  kind: 'statement',
  ...header,
  sequence,
  at: at(clock),
  channels,
})

const batch = (...messages: object[]) => {
  const parsed = parseBatch(messages)
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}

const databases: [string, () => Promise<Sql>][] = [['PGlite', pgliteSql]]
const url = process.env['CHART_TEST_DATABASE_URL']
if (url) databases.push(['Postgres', () => pgSql(url, { fresh: true })])

for (const [name, connect] of databases) {
  const withChart = async (work: (chart: Chart, sql: Sql) => Promise<void>) => {
    const sql = await connect()
    try {
      await work(await openChart(sql, RUN, wallTime(1_000_000)), sql)
    } finally {
      await sql.close()
    }
  }

  describe(`the chart on ${name}`, () => {
    test('the chart keeps one run: made on its first opening and the same at every later one', async () => {
      const sql = await connect()
      try {
        const first = await openChart(sql, RUN, wallTime(1_000_000))
        const again = await openChart(sql, RUN, wallTime(9_000_000))
        assert.deepEqual(first.run, { ...RUN, anchor: 1_000_000 })
        assert.deepEqual(again.run, first.run)
        await assert.rejects(openChart(sql, { ...RUN, patient: branded('another-test-patient', isPatientId) }, wallTime(9_000_000)))
      } finally {
        await sql.close()
      }
    })

    test("a clinician's new entry is taken with who, which patient, care time and charted time", () =>
      withChart(async (chart) => {
        const typed = await chart.write(newEntry({ care: at('06:56:30') }), at('06:57:10'))
        assert.equal(typed.kind, 'taken')
        assert.deepEqual({ ...typed.entry, entry: 'any' }, {
          entry: 'any',
          patient: 'test-patient',
          vital: 'RR',
          number: 1,
          value: 18,
          care: at('06:56:30'),
          charted: at('06:57:10'),
          author: nurse,
          versions: 1,
        })
        const blank = await chart.write(newEntry({ vital: 'HR', value: '61' }), at('06:58:00'))
        assert.ok(blank.kind === 'taken')
        assert.equal(blank.entry.care, at('06:58:00'), 'a blank care time is now, read when the chart takes the write')
        const model = await chart.readModel(at('06:58:05'))
        assert.deepEqual(model.entries, [typed.entry, blank.entry])
        assert.equal(model.now, at('06:58:05'))
      }))

    test('the chart refuses an entry about another patient, a care time after now, or a value that is not a finite number', () =>
      withChart(async (chart) => {
        const now = at('06:57:00')
        const refused = (kind: string) => ({ kind: 'refused', refusal: { kind } })
        assert.deepEqual(await chart.write(newEntry({ patient: 'another-test-patient' }), now), refused('another patient'))
        assert.deepEqual(await chart.write(newEntry({ care: at('06:57:01') }), now), refused('care time after now'))
        for (const value of ['', '   ', 'twelve']) {
          assert.deepEqual(await chart.write(newEntry({ value }), now), refused('not a finite number'), JSON.stringify(value))
        }
        assert.deepEqual((await chart.readModel(now)).entries, [], 'a refused write charts nothing')
      }))

    test('an amendment adds a version naming its author; the read model shows the current one, and the earlier ones come on request', () =>
      withChart(async (chart) => {
        const physician = ROSTER[1]!
        const first = taken(await chart.write(newEntry({ value: '18', care: at('06:56:00') }), at('06:56:10')))
        const second = taken(
          await chart.write(amendment({ entry: first.entry, author: physician.id, value: '20', care: at('06:56:05') }), at('06:57:00')),
        )
        assert.deepEqual(second, { ...first, number: 2, versions: 2, value: 20, care: at('06:56:05'), charted: at('06:57:00'), author: physician })
        assert.deepEqual((await chart.readModel(at('06:57:05'))).entries, [second])
        const versions = await chart.versions(first.entry)
        assert.deepEqual(
          versions?.map((v) => [v.number, v.value, v.author.id, v.charted]),
          [
            [1, 18, nurse.id, at('06:56:10')],
            [2, 20, physician.id, at('06:57:00')],
          ],
        )
      }))

    test('an amendment of an entry that does not exist, or of a version that is not the current one, is refused', () =>
      withChart(async (chart) => {
        const now = at('06:58:00')
        const refused = (kind: string) => ({ kind: 'refused', refusal: { kind } })
        const first = taken(await chart.write(newEntry({}), at('06:56:10')))
        taken(await chart.write(amendment({ entry: first.entry, value: '19' }), at('06:57:00')))
        assert.deepEqual(await chart.write(amendment({ entry: first.entry, replaces: 1, value: '21' }), now), refused('not the current version'))
        assert.deepEqual(await chart.write(amendment({ entry: '00000000-0000-4000-8000-000000000000' }), now), refused('no such entry'))
        assert.deepEqual(await chart.write(amendment({ entry: first.entry, replaces: 2, care: at('06:58:01') }), now), refused('care time after now'))
        assert.deepEqual(await chart.versions(first.entry).then((v) => v?.length), 2, 'a refused amendment adds no version')
        assert.equal(await chart.versions(branded('00000000-0000-4000-8000-000000000000', isEntryId)), undefined)
      }))

    test('when another amendment takes the version first, the database refuses this one and the chart answers not the current version', async () => {
      const base = await connect()
      const racing: Sql = {
        ...base,
        transaction: (work) =>
          base.transaction((tx) =>
            work({
              ...tx,
              async query(text, params) {
                const rows = await tx.query(text, params)
                const [read] = rows
                if (read !== undefined && 'current' in read) {
                  await tx.query(
                    `INSERT INTO entry_version (entry, number, value, care_ns, charted_ns, author, author_name, author_discipline)
                     SELECT entry, number + 1, value, care_ns, charted_ns, author, author_name, author_discipline
                     FROM entry_version WHERE entry = $1 AND number = $2`,
                    [read['id'], read['current']],
                  )
                }
                return rows
              },
            }),
          ),
      }
      try {
        const chart = await openChart(racing, RUN, wallTime(1_000_000))
        const first = taken(await chart.write(newEntry({}), at('06:56:10')))
        assert.deepEqual(await chart.write(amendment({ entry: first.entry, value: '19' }), at('06:57:00')), {
          kind: 'refused',
          refusal: { kind: 'not the current version' },
        })
      } finally {
        await base.close()
      }
    })

    test('Postgres refuses any update, delete or truncate of chart rows, and a second version under one number', () =>
      withChart(async (chart, sql) => {
        const first = taken(await chart.write(newEntry({}), at('06:56:10')))
        for (const statement of [
          "UPDATE run SET patient = 'another-test-patient'",
          'DELETE FROM run',
          'TRUNCATE run',
          "UPDATE entry SET vital = 'HR'",
          'DELETE FROM entry',
          'TRUNCATE entry CASCADE',
          'UPDATE entry_version SET value = 99',
          'DELETE FROM entry_version',
          'TRUNCATE entry_version',
        ]) {
          await assert.rejects(sql.query(statement), /append-only/, statement)
        }
        await assert.rejects(
          sql.query(
            `INSERT INTO entry_version (entry, number, value, care_ns, charted_ns, author, author_name, author_discipline)
             SELECT entry, number, 99, care_ns, charted_ns, author, author_name, author_discipline FROM entry_version`,
          ),
          /duplicate key/,
        )
        assert.deepEqual((await chart.readModel(at('06:57:00'))).entries, [first])
      }))

    test('the chart takes a batch of device messages in one transaction and answers each message', () =>
      withChart(async (chart, sql) => {
        const answer = (sequence: number, answer: string, refusal?: string) => ({ boot: 'test-boot-1', sequence, answer, ...(refusal ? { refusal } : {}) })
        const first = batch(deviceStatement(1, '06:55:00'), deviceReading(2, '06:55:00', 'not collected'), deviceReading(3, '06:55:01', 1.5))
        assert.deepEqual(await chart.writeDeviceBatch(first, at('06:55:01')), [answer(1, 'taken'), answer(2, 'taken'), answer(3, 'taken')])
        const resent = batch(...first, deviceReading(4, '06:55:02', 2.5))
        assert.deepEqual(await chart.writeDeviceBatch(resent, at('06:55:03')), [
          answer(1, 'already held'),
          answer(2, 'already held'),
          answer(3, 'already held'),
          answer(4, 'taken'),
        ])
        const conflicting = batch(deviceReading(3, '06:55:01', 9.5), deviceReading(5, '06:55:03', 3.5, { ...DEVICE, patient: 'another-test-patient' }))
        assert.deepEqual(await chart.writeDeviceBatch(conflicting, at('06:55:04')), [
          answer(3, 'refused', 'sequence reused with other content'),
          answer(5, 'refused', 'another patient'),
        ])
        const model = await chart.readModel(at('06:55:05'))
        assert.deepEqual(
          model.readings.map((r) => [r.sequence, r.collected, r.arrived, r.value.kind === 'present' ? r.value.value : r.value.reason]),
          [
            [2, at('06:55:00'), at('06:55:01'), 'not collected'],
            [3, at('06:55:01'), at('06:55:01'), 1.5],
            [4, at('06:55:02'), at('06:55:03'), 2.5],
          ],
        )
        assert.deepEqual(model.statements.map((st) => [st.sequence, st.at, st.arrived, st.channels.HR, st.channels.SpO2]), [[1, at('06:55:00'), at('06:55:01'), 'sourced', 'no source']])
        for (const statement of ["UPDATE device_message SET arrived_ns = 0", 'DELETE FROM device_message', 'TRUNCATE device_message']) {
          await assert.rejects(sql.query(statement), /append-only/, statement)
        }
        await assert.rejects(sql.query('INSERT INTO device_message SELECT * FROM device_message'), /duplicate key/)
      }))

    test('missing is each second a sourced vital lacks its reading, up to the earlier of the run end and the second before now', () =>
      withChart(async (chart) => {
        const spans = async (now: string) =>
          (await chart.readModel(at(now))).missing.map((m) => `${m.vital} ${formatClock(m.from)}-${formatClock(m.through)}`)
        const abpMean = (sequence: number, collected: string) => ({ ...deviceReading(sequence, collected), vital: 'ABP mean' })
        assert.deepEqual(await spans('06:55:30'), [], 'no device has made a statement, so nothing is expected')
        await chart.writeDeviceBatch(batch(deviceStatement(1, '06:55:00'), deviceReading(2, '06:55:00', 'not collected'), deviceReading(3, '06:55:01')), at('06:55:01'))
        await chart.writeDeviceBatch(batch(deviceReading(4, '06:55:04'), abpMean(5, '06:55:04')), at('06:55:06'))
        assert.deepEqual(await spans('06:55:06'), ['HR 06:55:02-06:55:03', 'HR 06:55:05-06:55:05', 'ABP mean 06:55:00-06:55:03', 'ABP mean 06:55:05-06:55:05'])
        await chart.writeDeviceBatch(batch(deviceReading(6, '06:55:05')), at('06:55:07'))
        assert.deepEqual(await spans('06:55:07'), ['HR 06:55:02-06:55:03', 'HR 06:55:06-06:55:06', 'ABP mean 06:55:00-06:55:03', 'ABP mean 06:55:05-06:55:06'])
        const later = (await chart.readModel(at('06:55:07'))).readings.find((r) => r.sequence === 6)
        assert.ok(later !== undefined && later.arrived > later.available, 'the reading that filled a missing second arrived late')
        const restart = { ...DEVICE, boot: 'test-boot-2' }
        await chart.writeDeviceBatch(batch(deviceStatement(1, '06:55:08', { ...SOURCED, 'ABP mean': 'no source' }, restart)), at('06:55:08'))
        assert.deepEqual(await spans('06:55:10'), ['HR 06:55:02-06:55:03', 'HR 06:55:06-06:55:09', 'ABP mean 06:55:00-06:55:03', 'ABP mean 06:55:05-06:55:07'])
        const end = await spans('07:05:00')
        assert.deepEqual(end.filter((span) => span.startsWith('HR')).at(-1), 'HR 06:55:06-07:04:59', 'the run end caps what is expected')
      }))
  })
}
