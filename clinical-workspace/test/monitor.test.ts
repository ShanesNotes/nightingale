import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { test } from 'node:test'
import { Scene, Story } from 'foldkit'
import { type BootId, isBootId, isPatientId, parseBatch } from '../src/device.ts'
import { Message, type Model, monitor } from '../src/monitor/monitor.ts'
import { availableBetween, decodeRun } from '../src/run.ts'
import { add, formatClock, type PatientTime, seconds } from '../src/time.ts'
import { chartDouble } from './link-double.ts'

const capture = new URL('../capture/307caf19/', import.meta.url)
const decoded = decodeRun({
  lens: readFileSync(new URL('lens.npy', capture)),
  catalogue: readFileSync(new URL('catalogue.txt', capture), 'utf8'),
  summary: readFileSync(new URL('summary.txt', capture), 'utf8'),
})
assert.ok(decoded.kind === 'decoded')
const { run } = decoded
const patient = run.packet
assert.ok(isPatientId(patient))
const feed = { start: run.start, end: run.end, channels: run.channels, between: (after: PatientTime | undefined, through: PatientTime) => availableBetween(run, after, through) }
const boot = (id: string): BootId => {
  assert.ok(isBootId(id))
  return id
}
const second = (n: number) => add(run.start, seconds(n))

// The observer's own value of a channel at a second, read from the decoded run, never from a previous run's screen.
const observed = (row: string, at: PatientTime) => {
  const channel = run.channels.indexOf(row)
  const o = run.observations.find((x) => x.channel === channel && x.collected === at)
  assert.ok(o !== undefined && o.value.kind === 'present', `${row} at ${formatClock(at)}`)
  return o.value.value
}

const tile = (vital: string) => Scene.selector(`td[data-tile="${vital}"]`)

test('fed the recorded run, the monitor shows heart rate and mean pressure with the observer\'s values, the rest no source, and never filtration', () => {
  const double = chartDouble(patient)
  const m = monitor({ patient, feed, link: double.link, boot: boot('test-boot-1') })
  const later = second(95)
  Scene.scene(
    { update: m.update, view: m.view },
    Scene.given(m.init().model),
    Scene.Subscription.emit(Message.SecondPassed({ now: run.start })),
    Scene.Command.resolve(m.SendBatch, Message.BatchUnanswered()),
    Scene.expect(tile('HR')).toHaveText('not collected'),
    Scene.expect(tile('ABP mean')).toHaveText('not collected'),
    ...['ABP sys', 'ABP dia', 'SpO2', 'RR', 'Temp'].map((vital) => Scene.expect(tile(vital)).toHaveText('no source')),
    Scene.expect(Scene.selector('#clock')).toContainText('06:55:00'),
    Scene.expect(Scene.selector('#feed')).toContainText('recorded synthetic run'),
    Scene.expect(Scene.selector('#feed')).toContainText(patient),
    Scene.expect(Scene.text(/filtration/i)).toBeAbsent(),
    Scene.Subscription.emit(Message.SecondPassed({ now: later })),
    Scene.Command.resolve(m.SendBatch, Message.BatchUnanswered()),
    Scene.expect(tile('HR')).toHaveText(`${Math.round(observed('algebraic/CV/rate/body', later) * 60)} /min`),
    Scene.expect(tile('ABP mean')).toHaveText(`${Math.round(observed('algebraic/CV/blood pressure/systemic arteries', later))} mmHg`),
    Scene.expect(Scene.selector('#feed')).not.toContainText('ended'),
    Scene.Subscription.emit(Message.SecondPassed({ now: run.end })),
    Scene.Command.resolve(m.SendBatch, Message.BatchUnanswered()),
    Scene.expect(Scene.selector('#feed')).toContainText('The recorded feed ended at 07:05:00.'),
  )
})

test('nothing under src/monitor imports anything but observation, device, vitals, time, foldkit and effect', () => {
  const dir = new URL('../src/monitor/', import.meta.url)
  const IMPORT = /^\s*(?:import|export)\b[^'"]*?\bfrom\s*['"]([^'"]+)['"]|\bimport\s*\(\s*['"]([^'"]+)['"]\s*\)|^\s*import\s*['"]([^'"]+)['"]/gm
  const specifiers = readdirSync(dir)
    .filter((file) => file.endsWith('.ts'))
    .flatMap((file) => [...readFileSync(new URL(file, dir), 'utf8').matchAll(IMPORT)].map((m) => `${file}: ${m[1] ?? m[2] ?? m[3]}`))
  assert.ok(specifiers.length > 0, 'the test read no import at all, so it proves nothing')
  const allowed = (specifier: string) =>
    ['../observation.ts', '../device.ts', '../vitals.ts', '../time.ts', 'effect', 'foldkit'].includes(specifier) ||
    /^\.\/[\w-]+\.ts$/.test(specifier) ||
    /^(effect|foldkit)\/[\w/-]+$/.test(specifier)
  assert.deepEqual(specifiers.filter((line) => !allowed(line.slice(line.indexOf(': ') + 2))), [])
})

type Program = ReturnType<typeof monitor>
type Double = ReturnType<typeof chartDouble>

// The chart service answers the batch in flight: the in-memory chart decides each answer.
function answerOf(double: Double, m: Program, commands: ReadonlyArray<{ readonly name: string; readonly args?: Record<string, unknown> }>) {
  const pending = commands.find((c) => c.name === 'SendBatch')
  assert.ok(pending?.args !== undefined, 'a batch is in flight')
  const batch = parseBatch(pending.args['batch'])
  assert.ok(batch.ok, batch.ok ? '' : batch.reason)
  return m.messageFor(double.answer(batch.value))
}

const serviceAnswers = (double: Double, m: Program) => (sim: Story.StorySimulation<Model, Message>) =>
  Story.Command.resolve(m.SendBatch, answerOf(double, m, sim.commands))(sim)

const sceneAnswers = (double: Double, m: Program) => (sim: Scene.SceneSimulation<Model, Message>) =>
  Scene.Command.resolve(m.SendBatch, answerOf(double, m, sim.commands))(sim)

test('on a healthy link the monitor states its channels at boot and delivers each second\'s readings in one batch, none late', () => {
  const double = chartDouble(patient)
  const m = monitor({ patient, feed, link: double.link, boot: boot('test-boot-1') })
  Story.story(
    m.update,
    Story.given(m.init().model),
    Story.message(Message.SecondPassed({ now: second(10) })),
    serviceAnswers(double, m),
    Story.model((model: Model) => assert.deepEqual(model.outbox, [], 'every answered message leaves the outbox')),
    Story.message(Message.SecondPassed({ now: second(11) })),
    serviceAnswers(double, m),
    Story.model((model: Model) => assert.deepEqual(model.outbox, [])),
  )
  const held = double.held()
  const [first, ...readings] = held
  assert.ok(first?.kind === 'statement', 'the boot statement comes first')
  assert.equal(first.at, second(10))
  assert.deepEqual(first.channels, { HR: 'sourced', 'ABP sys': 'no source', 'ABP dia': 'no source', 'ABP mean': 'sourced', SpO2: 'no source', RR: 'no source', Temp: 'no source' })
  assert.deepEqual(
    readings.map((r) => (r.kind === 'reading' ? `${r.sequence} ${r.vital} ${formatClock(r.collected)}` : r.kind)),
    ['2 HR 06:55:10', '3 ABP mean 06:55:10', '4 HR 06:55:11', '5 ABP mean 06:55:11'],
  )
  assert.ok(held.every((message) => message.device === held[0]?.device && message.boot === 'test-boot-1' && message.patient === patient))
  const hr = readings.find((r) => r.kind === 'reading' && r.vital === 'HR')
  assert.ok(hr?.kind === 'reading' && hr.value.kind === 'present')
  assert.equal(hr.value.value, observed('algebraic/CV/rate/body', second(10)) * 60, 'heart rate is sent per minute, the exact factor of 60')
  Scene.scene(
    { update: m.update, view: m.view },
    Scene.given(m.init().model),
    Scene.Subscription.emit(Message.SecondPassed({ now: second(12) })),
    sceneAnswers(chartDouble(patient), m),
    Scene.expect(Scene.selector('#link')).toHaveText('Everything delivered to the chart.'),
  )
})

const readingsHeld = (double: Double) =>
  double.held().flatMap((m) => (m.kind === 'reading' ? [`${m.boot} ${m.sequence} ${m.vital} ${formatClock(m.collected)}`] : []))

test('TLC path: a restart within the second of an undelivered reading takes that second\'s reading again', () => {
  const double = chartDouble(patient)
  const before = monitor({ patient, feed, link: double.link, boot: boot('test-boot-1') })
  double.set('down')
  Story.story(
    before.update,
    Story.given(before.init().model),
    Story.message(Message.SecondPassed({ now: second(20) })),
    serviceAnswers(double, before),
    Story.model((model: Model) => assert.equal(model.outbox.length, 3, 'the statement and both readings are held')),
  )
  double.set('up')
  const after = monitor({ patient, feed, link: double.link, boot: boot('test-boot-2') })
  Story.story(after.update, Story.given(after.init().model), Story.message(Message.SecondPassed({ now: second(20) })), serviceAnswers(double, after))
  assert.deepEqual(readingsHeld(double), ['test-boot-2 2 HR 06:55:20', 'test-boot-2 3 ABP mean 06:55:20'])
})

test('a lost answer leaves the batch held; the monitor sends the same messages again and the chart holds each once', () => {
  const double = chartDouble(patient)
  const m = monitor({ patient, feed, link: double.link, boot: boot('test-boot-1') })
  double.set('losing answers')
  Story.story(
    m.update,
    Story.given(m.init().model),
    Story.message(Message.SecondPassed({ now: second(30) })),
    serviceAnswers(double, m),
    Story.model((model: Model) => assert.equal(model.outbox.length, 3, 'nothing answered, nothing let go')),
    (sim: Story.StorySimulation<Model, Message>) => {
      double.set('up')
      return sim
    },
    Story.message(Message.SecondPassed({ now: second(31) })),
    Story.Command.expectHas(m.SendBatch),
    serviceAnswers(double, m),
    Story.model((model: Model) => assert.deepEqual(model.outbox, [])),
  )
  assert.deepEqual(readingsHeld(double), ['test-boot-1 2 HR 06:55:30', 'test-boot-1 3 ABP mean 06:55:30', 'test-boot-1 4 HR 06:55:31', 'test-boot-1 5 ABP mean 06:55:31'])
})
