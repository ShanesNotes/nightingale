import { randomBytes } from 'node:crypto'
import { PGlite } from '@electric-sql/pglite'
import pg from 'pg'
import type { RunView } from './chart-api.ts'
import { isPatientId } from './device.ts'
import { isPatientTime, isWallTime, type WallTime } from './time.ts'

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
`

export type Chart = { readonly run: RunView }

const RUN_COLUMNS = 'spec_id, patient, start_ns, end_ns, anchor_ms'

// A row is read like any input from outside: checked once here, then trusted.
function runOf(row: Row | undefined): RunView {
  const { spec_id, patient, start_ns, end_ns, anchor_ms } = row ?? {}
  if (typeof spec_id !== 'string' || !isPatientId(patient)) throw new Error('the run row lacks its spec or patient')
  if (!isPatientTime(start_ns) || !isPatientTime(end_ns) || !isWallTime(anchor_ms)) throw new Error('the run row lacks its times')
  return { specId: spec_id, patient, start: start_ns, end: end_ns, anchor: anchor_ms }
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
  return { run: held }
}
