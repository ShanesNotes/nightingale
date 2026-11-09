import { randomBytes } from 'node:crypto'
import { PGlite } from '@electric-sql/pglite'
import pg from 'pg'
import {
  type EntryId,
  type EntryVersion,
  type EntryView,
  type EntryWrite,
  type MissingSpan,
  parseEntries,
  parseReadModel,
  parseVersions,
  type ReadModel,
  type Refusal,
  ROSTER,
  type RunView,
  type WriteResult,
} from './chart-api.ts'
import { type DeviceMessage, isPatientId, type MessageAnswer, type Parsed } from './device.ts'
import { add, earlier, floorTo, isPatientTime, isWallTime, type PatientTime, SECOND, seconds, type WallTime } from './time.ts'
import { VITALS } from './vitals.ts'

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
CREATE TABLE IF NOT EXISTS device_message (
  device text NOT NULL,
  boot text NOT NULL,
  sequence integer NOT NULL CHECK (sequence >= 1),
  patient text NOT NULL,
  message jsonb NOT NULL,
  arrived_ns bigint NOT NULL,
  inserted_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (device, boot, sequence)
);
CREATE OR REPLACE FUNCTION refuse_chart_change() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  RAISE EXCEPTION 'the chart is append-only: % on % is refused', TG_OP, TG_TABLE_NAME;
END
$$;
CREATE OR REPLACE TRIGGER run_append_only BEFORE UPDATE OR DELETE ON run FOR EACH ROW EXECUTE FUNCTION refuse_chart_change();
CREATE OR REPLACE TRIGGER run_kept BEFORE TRUNCATE ON run FOR EACH STATEMENT EXECUTE FUNCTION refuse_chart_change();
CREATE OR REPLACE TRIGGER entry_append_only BEFORE UPDATE OR DELETE ON entry FOR EACH ROW EXECUTE FUNCTION refuse_chart_change();
CREATE OR REPLACE TRIGGER entry_kept BEFORE TRUNCATE ON entry FOR EACH STATEMENT EXECUTE FUNCTION refuse_chart_change();
CREATE OR REPLACE TRIGGER entry_version_append_only BEFORE UPDATE OR DELETE ON entry_version
  FOR EACH ROW EXECUTE FUNCTION refuse_chart_change();
CREATE OR REPLACE TRIGGER entry_version_kept BEFORE TRUNCATE ON entry_version
  FOR EACH STATEMENT EXECUTE FUNCTION refuse_chart_change();
CREATE OR REPLACE TRIGGER device_message_append_only BEFORE UPDATE OR DELETE ON device_message
  FOR EACH ROW EXECUTE FUNCTION refuse_chart_change();
CREATE OR REPLACE TRIGGER device_message_kept BEFORE TRUNCATE ON device_message
  FOR EACH STATEMENT EXECUTE FUNCTION refuse_chart_change();
CREATE OR REPLACE VIEW current_entry AS
  SELECT e.id, e.patient, e.vital, v.number, v.value, v.care_ns, v.charted_ns, v.author, v.author_name,
    v.author_discipline, counts.versions, e.inserted_at
  FROM entry e
  JOIN (SELECT entry, max(number) AS current, count(*) AS versions FROM entry_version GROUP BY entry) counts
    ON counts.entry = e.id
  JOIN entry_version v ON v.entry = e.id AND v.number = counts.current;
`

const VERSION_FIELDS = `'entry', id, 'patient', patient, 'vital', vital, 'number', number, 'value', value, 'care', care_ns,
  'charted', charted_ns, 'author', json_build_object('id', author, 'name', author_name, 'discipline', author_discipline)`
const ENTRY_JSON = `json_build_object(${VERSION_FIELDS}, 'versions', versions)`
const VERSION_JSON = `json_build_object(${VERSION_FIELDS})`
const NUMBERED = `'device', device, 'boot', boot, 'sequence', sequence`
const CHART_JSON = `SELECT
  (SELECT coalesce(json_agg(${ENTRY_JSON} ORDER BY care_ns, inserted_at, id), '[]'::json) FROM current_entry) AS entries,
  (SELECT coalesce(json_agg(json_build_object(${NUMBERED}, 'vital', message->'vital', 'collected', message->'collected',
      'available', message->'available', 'arrived', arrived_ns, 'value', message->'value')
    ORDER BY (message->>'collected')::bigint, inserted_at, device, boot, sequence), '[]'::json)
    FROM device_message WHERE message->>'kind' = 'reading') AS readings,
  (SELECT coalesce(json_agg(json_build_object(${NUMBERED}, 'at', message->'at', 'arrived', arrived_ns, 'channels', message->'channels')
    ORDER BY (message->>'at')::bigint, inserted_at, device, boot, sequence), '[]'::json)
    FROM device_message WHERE message->>'kind' = 'statement') AS statements`

export type Chart = {
  readonly run: RunView
  write(write: EntryWrite, now: PatientTime): Promise<WriteResult>
  writeDeviceBatch(messages: readonly DeviceMessage[], now: PatientTime): Promise<readonly MessageAnswer[]>
  readModel(now: PatientTime): Promise<ReadModel>
  versions(entry: EntryId): Promise<readonly EntryVersion[] | undefined>
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

function runOf(row: Row | undefined): RunView {
  const { spec_id, patient, start_ns, end_ns, anchor_ms } = row ?? {}
  if (typeof spec_id !== 'string' || !isPatientId(patient)) throw new Error('the run row lacks its spec or patient')
  if (!isPatientTime(start_ns) || !isPatientTime(end_ns) || !isWallTime(anchor_ms)) throw new Error('the run row lacks its times')
  return { specId: spec_id, patient, start: start_ns, end: end_ns, anchor: anchor_ms }
}

function missingSpans(model: Omit<ReadModel, 'missing'>): MissingSpan[] {
  if (model.now <= model.run.start) return []
  const horizon = earlier(model.run.end, add(model.now, seconds(-1)))
  const key = (device: string, vital: string, second: PatientTime) => `${device} ${vital} ${second}`
  const charted = new Set(model.readings.map((r) => key(r.device, r.vital, floorTo(r.collected, SECOND))))
  const spans: MissingSpan[] = []
  for (const device of new Set(model.statements.map((s) => s.device))) {
    const statements = model.statements.filter((s) => s.device === device)
    const first = statements[0]
    if (first === undefined) continue
    for (const { vital } of VITALS) {
      let open: { from: PatientTime; through: PatientTime } | undefined
      for (let second = floorTo(first.at, SECOND); second <= horizon; second = add(second, SECOND)) {
        const latest = statements.filter((s) => s.at <= second).at(-1)
        if (latest?.channels[vital] === 'sourced' && !charted.has(key(device, vital, second))) {
          open = { from: open?.from ?? second, through: second }
        } else if (open !== undefined) {
          spans.push({ device, vital, ...open })
          open = undefined
        }
      }
      if (open !== undefined) spans.push({ device, vital, ...open })
    }
  }
  return spans
}

const refused = (refusal: Refusal): WriteResult => ({ kind: 'refused', refusal })

const lostTheVersionRace = (error: unknown) => typeof error === 'object' && error !== null && 'code' in error && error.code === '23505'

function refusalOf(write: { readonly patient: string; readonly value: number; readonly care: PatientTime }, run: RunView, now: PatientTime): Refusal | undefined {
  if (write.patient !== run.patient) return { kind: 'another patient' }
  if (write.care > now) return { kind: 'care time after now' }
  if (!Number.isFinite(write.value)) return { kind: 'not a finite number' }
  return undefined
}

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
    async write(write, now) {
      const author = ROSTER.find((c) => c.id === write.author)
      if (author === undefined) return refused({ kind: 'malformed', reason: 'the author is not on the roster' })
      const care = write.care.kind === 'now' ? now : write.care.at
      const refusal = refusalOf({ patient: write.patient, value: write.value, care }, held, now)
      if (refusal !== undefined) return refused(refusal)
      try {
        return await sql.transaction(async (tx) => {
          let id: unknown
          let number: number
          if (write.kind === 'new entry') {
            const [made] = await tx.query('INSERT INTO entry (patient, vital) VALUES ($1, $2) RETURNING id', [write.patient, write.vital])
            id = made?.['id']
            number = 1
          } else {
            const [current] = await tx.query(
              'SELECT e.id, max(v.number) AS current FROM entry e JOIN entry_version v ON v.entry = e.id WHERE e.id::text = $1 AND e.patient = $2 GROUP BY e.id',
              [write.entry, write.patient],
            )
            if (current === undefined) return refused({ kind: 'no such entry' })
            if (current['current'] !== write.replaces) return refused({ kind: 'not the current version' })
            id = current['id']
            number = write.replaces + 1
          }
          await tx.query(
            `INSERT INTO entry_version (entry, number, value, care_ns, charted_ns, author, author_name, author_discipline)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)`,
            [id, number, write.value, care, now, author.id, author.name, author.discipline],
          )
          const [taken] = await entries(tx, 'id = $1', [id])
          if (taken === undefined) throw new Error('the chart lost the entry it just took')
          return { kind: 'taken', entry: taken }
        })
      } catch (error) {
        if (lostTheVersionRace(error)) return refused({ kind: 'not the current version' })
        throw error
      }
    },
    async versions(entry) {
      const [row] = await sql.query(
        `SELECT coalesce(json_agg(${VERSION_JSON} ORDER BY number), '[]'::json) AS versions
         FROM entry_version v JOIN entry e ON e.id = v.entry WHERE e.id::text = $1`,
        [entry],
      )
      const versions = trusted(parseVersions(row?.['versions']))
      return versions.length > 0 ? versions : undefined
    },
    writeDeviceBatch(messages, now) {
      return sql.transaction(async (tx) => {
        const answers: MessageAnswer[] = []
        for (const message of messages) {
          const answered = { boot: message.boot, sequence: message.sequence }
          if (message.patient !== held.patient) {
            answers.push({ ...answered, answer: 'refused', refusal: 'another patient' })
            continue
          }
          const key = [message.device, message.boot, message.sequence, JSON.stringify(message)]
          const [taken] = await tx.query(
            `INSERT INTO device_message (device, boot, sequence, patient, message, arrived_ns) VALUES ($1, $2, $3, $5, $4::jsonb, $6)
             ON CONFLICT (device, boot, sequence) DO NOTHING RETURNING sequence`,
            [...key, message.patient, now],
          )
          if (taken !== undefined) {
            answers.push({ ...answered, answer: 'taken' })
            continue
          }
          const [stored] = await tx.query('SELECT message = $4::jsonb AS same FROM device_message WHERE device = $1 AND boot = $2 AND sequence = $3', key)
          answers.push(stored?.['same'] === true ? { ...answered, answer: 'already held' } : { ...answered, answer: 'refused', refusal: 'sequence reused with other content' })
        }
        return answers
      })
    },
    async readModel(now) {
      const [row] = await sql.query(CHART_JSON)
      const model = trusted(
        parseReadModel({ run: held, now, entries: row?.['entries'], readings: row?.['readings'], statements: row?.['statements'], missing: [] }),
      )
      return { ...model, missing: missingSpans(model) }
    },
  }
}
