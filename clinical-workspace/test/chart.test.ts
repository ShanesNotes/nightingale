import assert from 'node:assert/strict'
import { describe, test } from 'node:test'
import { isEntryId, parseAmendment, parseNewEntry, ROSTER, type WriteResult } from '../src/chart-api.ts'
import { type Chart, openChart, pgliteSql, pgSql, type Sql } from '../src/chart.ts'
import { isPatientId, parseBatch } from '../src/device.ts'
import { type EntryContent, type EntryKind, isEntryKind } from '../src/entry-kinds.ts'
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
const physician = ROSTER[1]!

// A kind cannot join the catalog without three contents here, so every rule below runs on every kind the chart knows.
const SAMPLES: { readonly [K in EntryKind]: readonly [EntryContent<K>['content'], EntryContent<K>['content'], EntryContent<K>['content']] } = {
  vital: [{ vital: 'RR', value: 18 }, { vital: 'RR', value: 20 }, { vital: 'RR', value: 21.5 }],
  note: [{ text: 'Synthetic note, first version.' }, { text: 'Synthetic note, second version.' }, { text: 'Synthetic note, third version.' }],
}
const KINDS = Object.keys(SAMPLES).filter(isEntryKind)

let writes = 0
const writeId = () => `test-write-${(writes += 1)}`

const sent = (content: object) => Object.fromEntries(Object.entries(content).map(([field, value]) => [field, String(value)]))

const newEntry = (kind: EntryKind, content: object, fields: Readonly<Record<string, unknown>> = {}) => {
  const parsed = parseNewEntry({ writeId: writeId(), patient: 'test-patient', author: nurse.id, kind, content: sent(content), care: null, ...fields })
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}

const amendment = (kind: EntryKind, entry: string, content: object, fields: Readonly<Record<string, unknown>> = {}) => {
  const parsed = parseAmendment({ writeId: writeId(), patient: 'test-patient', author: nurse.id, entry, replaces: 1, kind, content: sent(content), care: null, ...fields })
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}

const taken = (result: WriteResult) => {
  assert.ok(result.kind === 'taken', JSON.stringify(result))
  return result.entry
}

const refused = (kind: string) => ({ kind: 'refused', refusal: { kind } })

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

  const racing = async (intercept: (tx: Sql, rows: readonly Readonly<Record<string, unknown>>[]) => Promise<readonly Readonly<Record<string, unknown>>[]>) => {
    const base = await connect()
    const sql: Sql = {
      ...base,
      transaction: (work) => base.transaction((tx) => work({ ...tx, query: async (text, params) => intercept(tx, await tx.query(text, params)) })),
    }
    return { base, chart: await openChart(sql, RUN, wallTime(1_000_000)) }
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

    for (const kind of KINDS) {
      const [first, second, third] = SAMPLES[kind]

      test(`${kind}: a clinician's new entry is taken with who, which patient, care time and charted time`, () =>
        withChart(async (chart) => {
          const typed = taken(await chart.write(newEntry(kind, first, { care: at('06:56:30') }), at('06:57:10')))
          assert.deepEqual({ ...typed, entry: 'any' }, {
            entry: 'any',
            patient: 'test-patient',
            kind,
            content: first,
            number: 1,
            care: at('06:56:30'),
            charted: at('06:57:10'),
            author: nurse,
            versions: 1,
          })
          const blank = taken(await chart.write(newEntry(kind, second), at('06:58:00')))
          assert.equal(blank.care, at('06:58:00'), 'a blank care time is now, read when the chart takes the write')
          const model = await chart.readModel(at('06:58:05'))
          assert.deepEqual(model.entries, [typed, blank])
          assert.equal(model.now, at('06:58:05'))
          assert.deepEqual(model.readings, [], 'an entry makes no device reading')
        }))

      test(`${kind}: the chart refuses an entry about another patient or a care time after now`, () =>
        withChart(async (chart) => {
          const now = at('06:57:00')
          assert.deepEqual(await chart.write(newEntry(kind, first, { patient: 'another-test-patient' }), now), refused('another patient'))
          assert.deepEqual(await chart.write(newEntry(kind, first, { care: at('06:57:01') }), now), refused('care time after now'))
          assert.deepEqual((await chart.readModel(now)).entries, [], 'a refused write charts nothing')
        }))

      test(`${kind}: an amendment adds a version naming its author; the read model shows the current one, and the earlier ones come on request`, () =>
        withChart(async (chart) => {
          const original = taken(await chart.write(newEntry(kind, first, { care: at('06:56:00') }), at('06:56:10')))
          const amended = taken(await chart.write(amendment(kind, original.entry, second, { author: physician.id, care: at('06:56:05') }), at('06:57:00')))
          assert.deepEqual(amended, { ...original, number: 2, versions: 2, content: second, care: at('06:56:05'), charted: at('06:57:00'), author: physician })
          assert.deepEqual((await chart.readModel(at('06:57:05'))).entries, [amended])
          assert.deepEqual(
            (await chart.versions(original.entry))?.map((v) => [v.number, v.kind, v.content, v.author.id, v.charted]),
            [
              [1, kind, first, nurse.id, at('06:56:10')],
              [2, kind, second, physician.id, at('06:57:00')],
            ],
          )
        }))

      test(`${kind}: an amendment of an entry that does not exist, of another kind, or of a version that is not the current one, is refused`, () =>
        withChart(async (chart) => {
          const now = at('06:58:00')
          const original = taken(await chart.write(newEntry(kind, first), at('06:56:10')))
          taken(await chart.write(amendment(kind, original.entry, second), at('06:57:00')))
          assert.deepEqual(await chart.write(amendment(kind, original.entry, third, { replaces: 1 }), now), refused('not the current version'))
          assert.deepEqual(await chart.write(amendment(kind, '00000000-0000-4000-8000-000000000000', third), now), refused('no such entry'))
          assert.deepEqual(await chart.write(amendment(kind, original.entry, third, { replaces: 2, care: at('06:58:01') }), now), refused('care time after now'))
          const other = KINDS.find((k) => k !== kind) ?? kind
          assert.deepEqual(await chart.write(amendment(other, original.entry, SAMPLES[other][0], { replaces: 2 }), now), refused('entry kind differs'))
          assert.equal((await chart.versions(original.entry))?.length, 2, 'a refused amendment adds no version')
          assert.equal(await chart.versions(branded('00000000-0000-4000-8000-000000000000', isEntryId)), undefined)
        }))

      test(`${kind}: when another amendment takes the version first, the database refuses this one and the chart answers not the current version`, async () => {
        const { base, chart } = await racing(async (tx, rows) => {
          const [read] = rows
          if (read !== undefined && 'current' in read) {
            await tx.query(
              `INSERT INTO entry_version (entry, kind, number, care_ns, charted_ns, author, author_name, author_discipline, write_id, write_fingerprint)
               SELECT entry, kind, number + 1, care_ns, charted_ns, author, author_name, author_discipline, 'test-write-racing', write_fingerprint
               FROM entry_version WHERE entry = $1 AND number = $2`,
              [read['id'], read['current']],
            )
          }
          return rows
        })
        try {
          const original = taken(await chart.write(newEntry(kind, first), at('06:56:10')))
          assert.deepEqual(await chart.write(amendment(kind, original.entry, second), at('06:57:00')), refused('not the current version'))
        } finally {
          await base.close()
        }
      })

      test(`${kind}: a write sent again under its write id charts nothing twice: the chart answers taken, with the entry as it now stands`, () =>
        withChart(async (chart) => {
          const write = newEntry(kind, first)
          const original = taken(await chart.write(write, at('06:56:10')))
          assert.deepEqual(await chart.write(write, at('06:56:20')), { kind: 'taken', entry: original }, 'a blank care time stays the now the chart first read')
          const amend = amendment(kind, original.entry, second)
          const amended = taken(await chart.write(amend, at('06:56:30')))
          assert.deepEqual(await chart.write(amend, at('06:56:40')), { kind: 'taken', entry: amended }, 'an amendment sent again is not refused as stale')
          assert.deepEqual(await chart.write(write, at('06:56:50')), { kind: 'taken', entry: amended }, 'the entry as it now stands')
          assert.deepEqual((await chart.readModel(at('06:57:00'))).entries, [amended])
          assert.equal((await chart.versions(original.entry))?.length, 2)
        }))

      test(`${kind}: a write id the chart already holds is refused when it comes with other content, and charts nothing`, () =>
        withChart(async (chart) => {
          const reused = refused('write id reused with other content')
          const original = taken(await chart.write(newEntry(kind, first, { writeId: `test-${kind}-reused` }), at('06:56:10')))
          assert.deepEqual(await chart.write(newEntry(kind, second, { writeId: `test-${kind}-reused` }), at('06:56:20')), reused)
          assert.deepEqual(await chart.write(amendment(kind, original.entry, second, { writeId: `test-${kind}-reused` }), at('06:56:30')), reused)
          assert.deepEqual((await chart.readModel(at('06:56:40'))).entries, [original])
          assert.equal((await chart.versions(original.entry))?.length, 1)
        }))

      test(`${kind}: when two copies of one write race, the copy that loses on the write id is answered taken, as the other was`, async () => {
        const { base, chart } = await racing(async (_tx, rows) => (rows.some((row) => 'same' in row) ? [] : rows))
        try {
          const write = newEntry(kind, first)
          const original = taken(await chart.write(write, at('06:56:10')))
          assert.deepEqual(await chart.write(write, at('06:56:20')), { kind: 'taken', entry: original })
          assert.deepEqual((await chart.readModel(at('06:56:30'))).entries, [original])
        } finally {
          await base.close()
        }
      })

      test(`${kind}: Postgres refuses any update, delete or truncate of its content, and a second version under one number or one write id`, () =>
        withChart(async (chart, sql) => {
          const original = taken(await chart.write(newEntry(kind, first), at('06:56:10')))
          for (const statement of [`UPDATE content_${kind} SET number = 99`, `DELETE FROM content_${kind}`, `TRUNCATE content_${kind}`]) {
            await assert.rejects(sql.query(statement), /append-only/, statement)
          }
          const again = (number: string, id: string) =>
            sql.query(
              `INSERT INTO entry_version (entry, kind, number, care_ns, charted_ns, author, author_name, author_discipline, write_id, write_fingerprint)
               SELECT entry, kind, ${number}, care_ns, charted_ns, author, author_name, author_discipline, ${id}, write_fingerprint FROM entry_version`,
            )
          await assert.rejects(again('number', "'test-write-again'"), /duplicate key/, 'a second version under one number')
          await assert.rejects(again('number + 1', 'write_id'), /duplicate key/, 'a second version under one write id')
          assert.deepEqual((await chart.readModel(at('06:57:00'))).entries, [original])
        }))
    }

    test('the chart will not open a database whose chart tables predate the write id, and says to run npm run reset', async () => {
      const sql = await connect()
      try {
        await sql.script(`CREATE TABLE entry_version (
          entry uuid NOT NULL, number integer NOT NULL CHECK (number >= 1), value double precision NOT NULL, care_ns bigint NOT NULL,
          charted_ns bigint NOT NULL, author text NOT NULL, author_name text NOT NULL, author_discipline text NOT NULL,
          inserted_at timestamptz NOT NULL DEFAULT now(), PRIMARY KEY (entry, number))`)
        await assert.rejects(openChart(sql, RUN, wallTime(1_000_000)), /npm run reset/)
      } finally {
        await sql.close()
      }
    })

    test('the chart will not open a database that lacks the content table of a kind it knows, and says to run npm run reset', async () => {
      const sql = await connect()
      try {
        await sql.script(`CREATE TABLE entry_version (
          entry uuid NOT NULL, kind text NOT NULL, number integer NOT NULL, write_fingerprint text NOT NULL, PRIMARY KEY (entry, number));
          CREATE TABLE entry_note (entry uuid NOT NULL, number integer NOT NULL, text text NOT NULL)`)
        await assert.rejects(openChart(sql, RUN, wallTime(1_000_000)), /npm run reset/)
      } finally {
        await sql.close()
      }
    })

    test('Postgres refuses any update, delete or truncate of the run and the entry spine', () =>
      withChart(async (chart, sql) => {
        const [kind] = KINDS
        assert.ok(kind !== undefined)
        const first = taken(await chart.write(newEntry(kind, SAMPLES[kind][0]), at('06:56:10')))
        for (const statement of [
          "UPDATE run SET patient = 'another-test-patient'",
          'DELETE FROM run',
          'TRUNCATE run CASCADE',
          "UPDATE entry SET kind = 'note'",
          'DELETE FROM entry',
          'TRUNCATE entry CASCADE',
          'UPDATE entry_version SET care_ns = 99',
          'DELETE FROM entry_version',
          'TRUNCATE entry_version CASCADE',
        ]) {
          await assert.rejects(sql.query(statement), /append-only/, statement)
        }
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
