import assert from 'node:assert/strict'
import { describe, test } from 'node:test'
import { parseAmendment, parseNewEntry, parseReadModel, type WriteResult } from '../src/chart-api.ts'
import { openChart, pgliteSql, pgSql, type Chart, type Sql } from '../src/chart.ts'
import { isPatientId } from '../src/device.ts'
import { flowsheet } from '../src/flowsheet.ts'
import { at, NURSE, PHYSICIAN } from './fixtures.ts'
import { wallTime } from '../src/time.ts'

const patient = 'synthetic-kinds-patient'
assert.ok(isPatientId(patient))
const run = { specId: 'synthetic-kinds-spec', patient, start: at('06:55:00'), end: at('07:05:00') }
let serial = 0
const wire = (fields: object = {}) => ({ writeId: `kinds-${++serial}`, patient, author: NURSE.id, care: null, kind: 'note', body: { text: 'Synthetic note alpha.' }, ...fields })
const newWrite = (fields: object = {}) => {
  const parsed = parseNewEntry(wire(fields))
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}
const amend = (entry: string, fields: object = {}) => {
  const parsed = parseAmendment(wire({ entry, replaces: 1, ...fields }))
  assert.ok(parsed.ok, parsed.ok ? '' : parsed.reason)
  return parsed.value
}
const taken = (result: WriteResult) => {
  assert.equal(result.kind, 'taken', JSON.stringify(result))
  return result.entry
}

test('unknown kinds and bodies that do not fit their kind have named boundary refusals', () => {
  for (const parse of [parseNewEntry, parseAmendment]) {
    const unknown = parse(wire({ entry: 'synthetic-entry', replaces: 1, kind: 'unregistered', body: {} }))
    assert.deepEqual(unknown.ok ? undefined : unknown.refusal, { kind: 'unknown entry kind' })
    for (const fields of [
      { kind: 'note', body: { value: 3 } },
      { kind: 'note', body: { text: '' } },
      { kind: 'note', body: { text: ' \n\t' } },
      { kind: 'note', body: { text: 'Synthetic.', vital: 'HR' } },
      { kind: 'vital', body: { text: 'Synthetic.' } },
      { kind: 'vital', body: { vital: 'HR', value: 'twelve' } },
      { kind: 'vital', body: { vital: 'HR', value: 'Infinity' } },
    ]) {
      const invalid = parse(wire({ entry: 'synthetic-entry', replaces: 1, ...fields }))
      assert.deepEqual(invalid.ok ? undefined : invalid.refusal, { kind: 'body does not fit kind' }, JSON.stringify(fields))
    }
  }
})

const databases: [string, () => Promise<Sql>][] = [['PGlite', pgliteSql]]
if (process.env['CHART_TEST_DATABASE_URL']) databases.push(['Postgres', () => pgSql(process.env['CHART_TEST_DATABASE_URL']!, { fresh: true })])
for (const [name, connect] of databases) {
  const withChart = async (work: (chart: Chart, sql: Sql) => Promise<void>) => {
    const sql = await connect()
    try {
      await work(await openChart(sql, run, wallTime(1_000_000)), sql)
    } finally {
      await sql.close()
    }
  }
  describe(`both entry kinds on ${name}`, () => {
    test('one write and read model keep a note, its amendment, author, care time and earlier versions', () => withChart(async (chart) => {
      const first = taken(await chart.write(newWrite({ care: at('06:55:10') }), at('06:56:00')))
      assert.equal(first.kind, 'note')
      assert.deepEqual(first.body, { text: 'Synthetic note alpha.' })
      const second = taken(await chart.write(amend(first.entry, { author: PHYSICIAN.id, body: { text: 'Synthetic note beta.' }, care: at('06:55:15') }), at('06:57:00')))
      assert.deepEqual(second, { ...first, body: { text: 'Synthetic note beta.' }, number: 2, versions: 2, author: PHYSICIAN, care: at('06:55:15'), charted: at('06:57:00') })
      const model = await chart.readModel(at('06:57:00'))
      assert.deepEqual(model.entries, [second])
      assert.deepEqual(parseReadModel(JSON.parse(JSON.stringify(model))), { ok: true, value: model })
      assert.deepEqual((await chart.versions(first.entry))?.map((v) => [v.number, v.body, v.author]), [[1, first.body, NURSE], [2, second.body, PHYSICIAN]])
      assert.deepEqual(model.readings, [], 'an entry makes no device reading')
    }))

    test('note writes and amendments retry once and reject changed content, stale versions, another patient and future care', () => withChart(async (chart) => {
      const write = newWrite()
      const first = taken(await chart.write(write, at('06:56:00')))
      assert.deepEqual(taken(await chart.write(write, at('06:56:10'))), first)
      const amendment = amend(first.entry, { body: { text: 'Synthetic amended note.' } })
      const second = taken(await chart.write(amendment, at('06:56:20')))
      assert.deepEqual(taken(await chart.write(amendment, at('06:56:30'))), second)
      assert.deepEqual(taken(await chart.write(write, at('06:56:40'))), second)
      for (const [write, kind] of [
        [newWrite({ writeId: amendment.writeId }), 'write id reused with other content'],
        [amend(first.entry), 'not the current version'],
        [amend(first.entry, { replaces: 2, kind: 'vital', body: { vital: 'HR', value: '1.25' } }), 'entry kind differs'],
        [newWrite({ care: at('07:00:00') }), 'care time after now'],
        [amend(first.entry, { replaces: 2, care: at('07:00:00') }), 'care time after now'],
        [newWrite({ patient: 'another-synthetic-patient' }), 'another patient'],
      ] as const) assert.deepEqual(await chart.write(write, at('06:57:00')), { kind: 'refused', refusal: { kind } })
      assert.equal((await chart.versions(first.entry))?.length, 2)
    }))

    test('adding even an earlier note leaves every flowsheet cell and column unchanged', () => withChart(async (chart) => {
      taken(await chart.write(newWrite({ kind: 'vital', body: { vital: 'HR', value: '1.25' } }), at('06:56:00')))
      const before = flowsheet(await chart.readModel(at('06:57:00')))
      taken(await chart.write(newWrite({ care: at('05:00:00') }), at('06:57:00')))
      assert.deepEqual(flowsheet(await chart.readModel(at('06:57:00'))), before)
    }))

    test('SQL itself refuses a missing body, a wrong-kind body, a skipped version, future care and a changed body', () => withChart(async (chart, sql) => {
      const first = taken(await chart.write(newWrite(), at('06:56:00')))
      const next = (tx: Sql, number = 2, care = first.care) => tx.query(`
        INSERT INTO entry_version (entry, kind, number, care_ns, charted_ns, author, author_name, author_discipline, write_id, write_fingerprint)
        SELECT entry, kind, $2, $3, charted_ns, author, author_name, author_discipline, $4, write_fingerprint
        FROM entry_version WHERE entry = $1 AND number = 1`, [first.entry, number, care, `sql-${++serial}`])
      await assert.rejects(sql.transaction((tx) => next(tx)), /body/, 'commit refuses an empty version')
      await assert.rejects(sql.transaction(async (tx) => {
        await next(tx)
        await tx.query('INSERT INTO entry_vital (entry, number, vital, value) VALUES ($1, 2, $2, $3)', [first.entry, 'HR', 1.25])
      }), /foreign key/, 'a vital body cannot belong to a note version')
      await assert.rejects(sql.transaction(async (tx) => {
        await next(tx, 3)
        await tx.query('INSERT INTO entry_note (entry, number, text) VALUES ($1, 3, $2)', [first.entry, 'Synthetic skipped version.'])
      }), /foreign key/, 'version 3 cannot skip version 2')
      await assert.rejects(sql.transaction((tx) => next(tx, 2, at('06:57:00'))), /check constraint/, 'care must not be after charted time')
      for (const statement of ["UPDATE entry_note SET text = 'Synthetic change.'", 'DELETE FROM entry_note', 'TRUNCATE entry_note']) {
        await assert.rejects(sql.query(statement), /append-only/, statement)
      }
      assert.deepEqual((await chart.readModel(at('06:57:00'))).entries, [first], 'failed SQL leaves no partial versions')
    }))

    test('SQL typed columns refuse invalid bodies and the spine refuses a kind it does not know', () => withChart(async (chart, sql) => {
      const note = taken(await chart.write(newWrite(), at('06:56:00')))
      const vital = taken(await chart.write(newWrite({ kind: 'vital', body: { vital: 'HR', value: '1.25' } }), at('06:56:00')))
      const invalidBody = (entry: string, kind: string, bodySql: string, params: unknown[]) => sql.transaction(async (tx) => {
        await tx.query(`INSERT INTO entry_version (entry, kind, number, care_ns, charted_ns, author, author_name, author_discipline, write_id, write_fingerprint)
          SELECT entry, kind, 2, care_ns, charted_ns, author, author_name, author_discipline, $2, write_fingerprint
          FROM entry_version WHERE entry = $1`, [entry, `sql-${++serial}`])
        await tx.query(`INSERT INTO entry_${kind} (entry, number, ${kind === 'note' ? 'text' : 'vital, value'}) VALUES ($1, 2, ${bodySql})`, [entry, ...params])
      })
      for (const text of ['', ' \n\t', '\v', '\f\r']) await assert.rejects(invalidBody(note.entry, 'note', '$2', [text]), /check constraint/)
      for (const value of ['NaN', 'Infinity', '-Infinity']) await assert.rejects(invalidBody(vital.entry, 'vital', "'HR', $2", [value]), /check constraint/)
      await assert.rejects(invalidBody(vital.entry, 'vital', "'unknown', 1.25", []), /check constraint/)
      await assert.rejects(sql.query('INSERT INTO entry (patient, kind) VALUES ($1, $2)', [patient, 'unknown']), /check constraint/)
    }))

    test('note text preserves letters, Unicode, line breaks and literal markup without interpreting it', () => withChart(async (chart) => {
      for (const text of ['v', 'Synthetic Ω note.\nSecond line.', '<script>synthetic text only</script>']) {
        const entry = taken(await chart.write(newWrite({ body: { text } }), at('06:56:00')))
        assert.deepEqual(entry.body, { text })
      }
    }))
  })
}
