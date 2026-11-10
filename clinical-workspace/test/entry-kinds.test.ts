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
const wire = (fields: object = {}) => ({ writeId: `kinds-${++serial}`, patient, author: NURSE.id, care: null, kind: 'note', content: { text: 'Synthetic note alpha.' }, ...fields })
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

test('unknown kinds and content that does not fit its kind have named boundary refusals, and a misfit says why', () => {
  for (const parse of [parseNewEntry, parseAmendment]) {
    const unknown = parse(wire({ entry: 'synthetic-entry', replaces: 1, kind: 'unregistered', content: {} }))
    assert.deepEqual(unknown.ok ? undefined : unknown.refusal, { kind: 'unknown entry kind' })
    for (const [fields, reason] of [
      [{ kind: 'note', content: { value: 3 } }, 'A note has no field named value'],
      [{ kind: 'note', content: { text: '' } }, 'The note text is blank'],
      [{ kind: 'note', content: { text: ' \n\t' } }, 'The note text is blank'],
      [{ kind: 'note', content: { text: 'x'.repeat(10_001) } }, 'The note text is longer than 10,000 characters'],
      [{ kind: 'note', content: { text: 'Synthetic.', vital: 'HR' } }, 'A note has no field named vital'],
      [{ kind: 'note', content: 'Synthetic note as bare text.' }, 'The note content is not a set of named fields'],
      [{ kind: 'vital', content: { text: 'Synthetic.' } }, 'A vital has no field named text'],
      [{ kind: 'vital', content: { vital: 'HR' } }, 'The vital has no value'],
      [{ kind: 'vital', content: { vital: 'EtCO2', value: '1.25' } }, 'The vital is not one the chart lists'],
      [{ kind: 'vital', content: { vital: 'HR', value: 'twelve' } }, 'The value is not a number'],
      [{ kind: 'vital', content: { vital: 'HR', value: 'Infinity' } }, 'The value is not a number'],
    ] as const) {
      const invalid = parse(wire({ entry: 'synthetic-entry', replaces: 1, ...fields }))
      assert.deepEqual(invalid.ok ? undefined : invalid.refusal, { kind: 'content does not fit kind', reason }, JSON.stringify(fields))
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
      assert.deepEqual(first.content, { text: 'Synthetic note alpha.' })
      const second = taken(await chart.write(amend(first.entry, { author: PHYSICIAN.id, content: { text: 'Synthetic note beta.' }, care: at('06:55:15') }), at('06:57:00')))
      assert.deepEqual(second, { ...first, content: { text: 'Synthetic note beta.' }, number: 2, versions: 2, author: PHYSICIAN, care: at('06:55:15'), charted: at('06:57:00') })
      const model = await chart.readModel(at('06:57:00'))
      assert.deepEqual(model.entries, [second])
      assert.deepEqual(parseReadModel(JSON.parse(JSON.stringify(model))), { ok: true, value: model })
      assert.deepEqual((await chart.versions(first.entry))?.map((v) => [v.number, v.content, v.author]), [[1, first.content, NURSE], [2, second.content, PHYSICIAN]])
      assert.deepEqual(model.readings, [], 'an entry makes no device reading')
    }))

    test('note writes and amendments retry once and reject changed content, stale versions, another patient and future care', () => withChart(async (chart) => {
      const write = newWrite()
      const first = taken(await chart.write(write, at('06:56:00')))
      assert.deepEqual(taken(await chart.write(write, at('06:56:10'))), first)
      const amendment = amend(first.entry, { content: { text: 'Synthetic amended note.' } })
      const second = taken(await chart.write(amendment, at('06:56:20')))
      assert.deepEqual(taken(await chart.write(amendment, at('06:56:30'))), second)
      assert.deepEqual(taken(await chart.write(write, at('06:56:40'))), second)
      for (const [write, kind] of [
        [newWrite({ writeId: amendment.writeId }), 'write id reused with other content'],
        [amend(first.entry), 'not the current version'],
        [amend(first.entry, { replaces: 2, kind: 'vital', content: { vital: 'HR', value: '1.25' } }), 'entry kind differs'],
        [newWrite({ care: at('07:00:00') }), 'care time after now'],
        [amend(first.entry, { replaces: 2, care: at('07:00:00') }), 'care time after now'],
        [newWrite({ patient: 'another-synthetic-patient' }), 'another patient'],
      ] as const) assert.deepEqual(await chart.write(write, at('06:57:00')), { kind: 'refused', refusal: { kind } })
      assert.equal((await chart.versions(first.entry))?.length, 2)
    }))

    test('adding even an earlier note leaves every flowsheet cell and column unchanged', () => withChart(async (chart) => {
      taken(await chart.write(newWrite({ kind: 'vital', content: { vital: 'HR', value: '1.25' } }), at('06:56:00')))
      const before = flowsheet(await chart.readModel(at('06:57:00')))
      taken(await chart.write(newWrite({ care: at('05:00:00') }), at('06:57:00')))
      assert.deepEqual(flowsheet(await chart.readModel(at('06:57:00'))), before)
    }))

    test('SQL itself refuses a missing content, a wrong-kind content, a skipped version, future care and a changed content', () => withChart(async (chart, sql) => {
      const first = taken(await chart.write(newWrite(), at('06:56:00')))
      const next = (tx: Sql, number = 2, care = first.care) => tx.query(`
        INSERT INTO entry_version (entry, kind, number, care_ns, charted_ns, author, author_name, author_discipline, write_id, write_fingerprint)
        SELECT entry, kind, $2, $3, charted_ns, author, author_name, author_discipline, $4, write_fingerprint
        FROM entry_version WHERE entry = $1 AND number = 1`, [first.entry, number, care, `sql-${++serial}`])
      await assert.rejects(sql.transaction((tx) => next(tx)), /content/, 'commit refuses an empty version')
      await assert.rejects(sql.transaction(async (tx) => {
        await next(tx)
        await tx.query('INSERT INTO content_vital (entry, number, vital, value) VALUES ($1, 2, $2, $3)', [first.entry, 'HR', 1.25])
      }), /foreign key/, 'a vital content cannot belong to a note version')
      await assert.rejects(sql.transaction(async (tx) => {
        await next(tx, 3)
        await tx.query('INSERT INTO content_note (entry, number, text) VALUES ($1, 3, $2)', [first.entry, 'Synthetic skipped version.'])
      }), /foreign key/, 'version 3 cannot skip version 2')
      await assert.rejects(sql.transaction((tx) => next(tx, 2, at('06:57:00'))), /check constraint/, 'care must not be after charted time')
      for (const statement of ["UPDATE content_note SET text = 'Synthetic change.'", 'DELETE FROM content_note', 'TRUNCATE content_note']) {
        await assert.rejects(sql.query(statement), /append-only/, statement)
      }
      assert.deepEqual((await chart.readModel(at('06:57:00'))).entries, [first], 'failed SQL leaves no partial versions')
    }))

    test('SQL typed columns refuse invalid bodies and the spine refuses a kind it does not know', () => withChart(async (chart, sql) => {
      const note = taken(await chart.write(newWrite(), at('06:56:00')))
      const vital = taken(await chart.write(newWrite({ kind: 'vital', content: { vital: 'HR', value: '1.25' } }), at('06:56:00')))
      const invalidContent = (entry: string, kind: string, contentSql: string, params: unknown[]) => sql.transaction(async (tx) => {
        await tx.query(`INSERT INTO entry_version (entry, kind, number, care_ns, charted_ns, author, author_name, author_discipline, write_id, write_fingerprint)
          SELECT entry, kind, 2, care_ns, charted_ns, author, author_name, author_discipline, $2, write_fingerprint
          FROM entry_version WHERE entry = $1`, [entry, `sql-${++serial}`])
        await tx.query(`INSERT INTO content_${kind} (entry, number, ${kind === 'note' ? 'text' : 'vital, value'}) VALUES ($1, 2, ${contentSql})`, [entry, ...params])
      })
      for (const text of ['', ' \n\t', '\v', '\f\r']) await assert.rejects(invalidContent(note.entry, 'note', '$2', [text]), /check constraint/)
      for (const value of ['NaN', 'Infinity', '-Infinity']) await assert.rejects(invalidContent(vital.entry, 'vital', "'HR', $2", [value]), /check constraint/)
      await assert.rejects(invalidContent(vital.entry, 'vital', "'unknown', 1.25", []), /check constraint/)
      await assert.rejects(sql.query('INSERT INTO entry (patient, kind) VALUES ($1, $2)', [patient, 'unknown']), /check constraint/)
    }))

    test('note text preserves letters, Unicode, line breaks and literal markup without interpreting it', () => withChart(async (chart) => {
      for (const text of ['v', 'Synthetic Ω note.\nSecond line.', '<script>synthetic text only</script>']) {
        const entry = taken(await chart.write(newWrite({ content: { text } }), at('06:56:00')))
        assert.deepEqual(entry.content, { text })
      }
    }))
  })
}
