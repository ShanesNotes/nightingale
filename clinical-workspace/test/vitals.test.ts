import assert from 'node:assert/strict'
import { test } from 'node:test'
import { isVital, VITALS } from '../src/vitals.ts'

test('the monitored set is one table of seven vitals in bedside order, each with its unit', () => {
  assert.deepEqual(
    VITALS.map((row) => `${row.vital} ${row.unit}`),
    ['HR /min', 'ABP sys mmHg', 'ABP dia mmHg', 'ABP mean mmHg', 'SpO2 %', 'RR /min', 'Temp °C'],
  )
})

test('only a vital the table names is a vital', () => {
  for (const row of VITALS) assert.ok(isVital(row.vital), row.vital)
  for (const other of ['EtCO2', 'hr', 'Filtration', '', 7, null]) assert.equal(isVital(other), false, String(other))
})
