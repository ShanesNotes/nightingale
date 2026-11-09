import { randomBytes } from 'node:crypto'
import { PGlite } from '@electric-sql/pglite'
import pg from 'pg'
import {
  type EntryView,
  type NewEntry,
  parseEntries,
  type ReadModel,
  type Refusal,
  ROSTER,
  type RunView,
  type WriteResult,
} from './chart-api.ts'
import { isPatientId, type Parsed } from './device.ts'
import { isPatientTime, isWallTime, type PatientTime, type WallTime } from './time.ts'

// The chart's one seam: a small SQL client with a pg adapter for a real Postgres and a PGlite adapter that runs
// Postgres in-process. Both give a bigint column as a safe integer, so callers never see which ran.
export type Row = Readonly<Record<string, unknown>>

export type Sql = {
  query(text: string, params?: readonly unknown[]): Promise<readonly Row[]>
  script(text: string): Promise<void>
  transaction<T>(work: (sql: Sql) => Promise<T>): Promise<T>
  close(): Promise<void>
}

const INT8 = 20

function int8(text: string): number {
  const value = Number(text)
  if (!Number.isSafeInteger(value)) throw new RangeError(`bigint ${text} is past the safe integer range`)
  return value
}

export async function pgliteSql(): Promise<Sql> {
  const db = await PGlite.create({ parsers: { [INT8]: int8 } })
  const over = (q: Pick<PGlite, 'query' | 'exec'>): Omit<Sql, 'transaction' | 'close'> => ({
    query: async (text, params = []) => (await q.query<Row>(text, [...params])).rows,
    script: async (text) => void (await q.exec(text)),
  })
  const inside = (q: Pick<PGlite, 'query' | 'exec'>): Sql => ({
    ...over(q),
    transaction: () => Promise.reject(new Error('the chart opens no transaction inside another')),
    close: () => Promise.reject(new Error('a transaction does not close the database')),
  })
  return {
    ...over(db),
    transaction: (work) => db.transaction((tx) => work(inside(tx))),
    close: () => db.close(),
  }
}

const types = { getTypeParser: (oid: number, format?: 'text' | 'binary') => (oid === INT8 ? int8 : pg.types.getTypeParser(oid, format)) }

// With `fresh`, the adapter makes a database of its own on that server and drops it when it closes.
export async function pgSql(url: string, options: { readonly fresh?: boolean } = {}): Promise<Sql> {
  let target = url
  let drop = async () => {}
  if (options.fresh) {
    const name = `chart_test_${randomBytes(6).toString('hex')}`
    const admin = new pg.Client({ connectionString: url })
    await admin.connect()
    await admin.query(`CREATE DATABASE ${name}`)
    await admin.end()
    const fresh = new URL(url)
    fresh.pathname = `/${name}`
    target = fresh.toString()
    drop = async () => {
      const again = new pg.Client({ connectionString: url })
      await again.connect()
      await again.query(`DROP DATABASE ${name}`)
      await again.end()
    }
  }
  const pool = new pg.Pool({ connectionString: target, types })
  const over = (q: pg.Pool | pg.PoolClient): Omit<Sql, 'transaction' | 'close'> => ({
    query: async (text, params = []) => (await q.query<Row>(text, [...params])).rows,
    script: async (text) => void (await q.query(text)),
  })
  return {
    ...over(pool),
    async transaction(work) {
      const client = await pool.connect()
      try {
        await client.query('BEGIN')
        const result = await work({
          ...over(client),
          transaction: () => Promise.reject(new Error('the chart opens no transaction inside another')),
          close: () => Promise.reject(new Error('a transaction does not close the database')),
        })
        await client.query('COMMIT')
        return result
      } catch (error) {
        await client.query('ROLLBACK')
        throw error
      } finally {
        client.release()
      }
    },
    async close() {
      await pool.end()
      await drop()
    },
  }
}

const SCHEMA = `
CREATE TABLE IF NOT EXISTS run (
  singleton boolean PRIMARY KEY DEFAULT true CHECK (singleton),
  spec_id text NOT NULL,
  patient text NOT NULL,
  start_ns bigint NOT NULL,
  end_ns bigint NOT NULL CHECK (end_ns >= start_ns),
  anchor_ms bigint NOT NULL,
  inserted_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS entry (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  patient text NOT NULL,
  vital text NOT NULL,
  inserted_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE IF NOT EXISTS entry_version (
  entry uuid NOT NULL REFERENCES entry (id),
  number integer NOT NULL CHECK (number >= 1),
  value double precision NOT NULL CHECK (value <> 'NaN' AND value <> 'Infinity' AND value <> '-Infinity'),
  care_ns bigint NOT NULL,
  charted_ns bigint NOT NULL,
  author text NOT NULL,
  author_name text NOT NULL,
  author_discipline text NOT NULL,
  inserted_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (entry, number)
);
CREATE OR REPLACE VIEW current_entry AS
  SELECT e.id, e.patient, e.vital, v.number, v.value, v.care_ns, v.charted_ns, v.author, v.author_name,
    v.author_discipline, counts.versions, e.inserted_at
  FROM entry e
  JOIN (SELECT entry, max(number) AS current, count(*) AS versions FROM entry_version GROUP BY entry) counts
    ON counts.entry = e.id
  JOIN entry_version v ON v.entry = e.id AND v.number = counts.current;
`

// Rows leave the database in the chart API's own shape, so the API's parser checks them on the way out.
const ENTRY_JSON = `json_build_object('entry', id, 'patient', patient, 'vital', vital, 'number', number, 'value', value,
  'care', care_ns, 'charted', charted_ns, 'author', json_build_object('id', author, 'name', author_name,
  'discipline', author_discipline), 'versions', versions)`

export type Chart = {
  readonly run: RunView
  write(entry: NewEntry, now: PatientTime): Promise<WriteResult>
  readModel(now: PatientTime): Promise<ReadModel>
}

const trusted = <T>(parsed: Parsed<T>): T => {
  if (!parsed.ok) throw new Error(`the chart's database gave a row the chart API refuses: ${parsed.reason}`)
  return parsed.value
}

async function entries(sql: Sql, where = 'true', params: readonly unknown[] = []): Promise<readonly EntryView[]> {
  const [row] = await sql.query(
    `SELECT coalesce(json_agg(${ENTRY_JSON} ORDER BY care_ns, inserted_at, id), '[]'::json) AS entries FROM current_entry WHERE ${where}`,
    params,
  )
  return trusted(parseEntries(row?.['entries']))
}

const RUN_COLUMNS = 'spec_id, patient, start_ns, end_ns, anchor_ms'

// A row is read like any input from outside: checked once here, then trusted.
function runOf(row: Row | undefined): RunView {
  const { spec_id, patient, start_ns, end_ns, anchor_ms } = row ?? {}
  if (typeof spec_id !== 'string' || !isPatientId(patient)) throw new Error('the run row lacks its spec or patient')
  if (!isPatientTime(start_ns) || !isPatientTime(end_ns) || !isWallTime(anchor_ms)) throw new Error('the run row lacks its times')
  return { specId: spec_id, patient, start: start_ns, end: end_ns, anchor: anchor_ms }
}

const refused = (refusal: Refusal): WriteResult => ({ kind: 'refused', refusal })

function refusalOf(write: { readonly patient: string; readonly value: number; readonly care: PatientTime }, run: RunView, now: PatientTime): Refusal | undefined {
  if (write.patient !== run.patient) return { kind: 'another patient' }
  if (write.care > now) return { kind: 'care time after now' }
  if (!Number.isFinite(write.value)) return { kind: 'not a finite number' }
  return undefined
}

// The schema is applied at every opening and changes nothing the second time. The run row is made once.
export async function openChart(sql: Sql, run: Omit<RunView, 'anchor'>, anchor: WallTime): Promise<Chart> {
  await sql.script(SCHEMA)
  await sql.query(`INSERT INTO run (${RUN_COLUMNS}) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (singleton) DO NOTHING`, [
    run.specId,
    run.patient,
    run.start,
    run.end,
    anchor,
  ])
  const held = runOf((await sql.query(`SELECT ${RUN_COLUMNS} FROM run`))[0])
  if (held.specId !== run.specId || held.patient !== run.patient || held.start !== run.start || held.end !== run.end) {
    throw new Error(`the chart's database holds run ${held.specId} for patient ${held.patient}, not this one`)
  }
  return {
    run: held,
    async write(entry, now) {
      const author = ROSTER.find((c) => c.id === entry.author)
      if (author === undefined) return refused({ kind: 'malformed', reason: 'the author is not on the roster' })
      const care = entry.care.kind === 'now' ? now : entry.care.at
      const refusal = refusalOf({ patient: entry.patient, value: entry.value, care }, held, now)
      if (refusal !== undefined) return refused(refusal)
      return sql.transaction(async (tx) => {
        const [made] = await tx.query('INSERT INTO entry (patient, vital) VALUES ($1, $2) RETURNING id', [entry.patient, entry.vital])
        const id = made?.['id']
        await tx.query(
          `INSERT INTO entry_version (entry, number, value, care_ns, charted_ns, author, author_name, author_discipline)
           VALUES ($1, 1, $2, $3, $4, $5, $6, $7)`,
          [id, entry.value, care, now, author.id, author.name, author.discipline],
        )
        const [taken] = await entries(tx, 'id = $1', [id])
        if (taken === undefined) throw new Error('the chart lost the entry it just took')
        return { kind: 'taken', entry: taken }
      })
    },
    async readModel(now) {
      return { run: held, now, entries: await entries(sql), readings: [], statements: [], missing: [] }
    },
  }
}
