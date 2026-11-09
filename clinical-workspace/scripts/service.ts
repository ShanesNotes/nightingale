import { readFileSync } from 'node:fs'
import { openChart, pgSql } from '../src/chart.ts'
import { isPatientId } from '../src/device.ts'
import { decodeRun } from '../src/run.ts'
import { startService } from '../src/service.ts'
import { patientTimeAt, wallTime } from '../src/time.ts'

// The chart service as a process: the recorded run names the patient and the clock, Postgres holds the chart.
// It prints one line, "chart service <url> anchor <ms>", once it answers.
const fail = (reason: string): never => {
  process.stderr.write(`chart service: ${reason}\n`)
  process.exit(1)
}

const capture = new URL('../capture/307caf19/', import.meta.url)
const url = process.env['CHART_DATABASE_URL'] ?? fail('CHART_DATABASE_URL names no database')
const port = Number(process.env['PORT'] ?? '0')
const decoded = decodeRun({
  lens: readFileSync(new URL('lens.npy', capture)),
  catalogue: readFileSync(new URL('catalogue.txt', capture), 'utf8'),
  summary: readFileSync(new URL('summary.txt', capture), 'utf8'),
})
if (decoded.kind === 'refused') fail(`the recorded run does not decode: ${decoded.reason}`)
const run = decoded.kind === 'decoded' ? decoded.run : fail('no run')
const patient = isPatientId(run.packet) ? run.packet : fail('the run names no patient')

const sql = await pgSql(url)
const chart = await openChart(sql, { specId: run.specId, patient, start: run.start, end: run.end }, wallTime(Date.now()))
const service = await startService({ chart, now: () => patientTimeAt(chart.run, wallTime(Date.now())), port })
process.stdout.write(`chart service ${service.url} anchor ${chart.run.anchor}\n`)

const stop = () => {
  service.close().then(() => sql.close()).then(
    () => process.exit(0),
    () => process.exit(1),
  )
}
process.once('SIGTERM', stop)
process.once('SIGINT', stop)
