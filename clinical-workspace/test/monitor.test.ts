import assert from 'node:assert/strict'
import { readdirSync, readFileSync } from 'node:fs'
import { test } from 'node:test'
import { Scene } from 'foldkit'
import { type BootId, isBootId, isPatientId } from '../src/device.ts'
import { Message, monitor } from '../src/monitor/monitor.ts'
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
