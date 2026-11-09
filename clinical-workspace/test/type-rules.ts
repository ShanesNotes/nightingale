import type { ClinicianId, EntryId, NewEntry, WriteId } from '../src/chart-api.ts'
import type { PatientId, Reading } from '../src/device.ts'
import type { PatientTime } from '../src/time.ts'

// Each rule pairs a legal line with an illegal one that differs from it in one way, so each @ts-expect-error can
// be met only by the rule it names.
declare const patient: PatientId
declare const entry: EntryId
declare const author: ClinicianId
declare const time: PatientTime
declare const reading: Reading
declare const writeId: WriteId

export const timeFromAClock: PatientTime = time
// @ts-expect-error a bare number is not a PatientTime
export const timeFromANumber: PatientTime = 25_500_000_000_000

export const patientFromAPatient: PatientId = patient
// @ts-expect-error an entry id is not a patient id
export const patientFromAnEntry: PatientId = entry

export const signed: NewEntry = { kind: 'new entry', writeId, patient, author, vital: 'HR', value: 61, care: { kind: 'now' } }
// @ts-expect-error an entry cannot be written without an author
export const unsigned: NewEntry = { kind: 'new entry', writeId, patient, vital: 'HR', value: 61, care: { kind: 'now' } }
// @ts-expect-error an entry cannot be written without its write id
export const withoutWriteId: NewEntry = { kind: 'new entry', patient, author, vital: 'HR', value: 61, care: { kind: 'now' } }

export const heartRate: Reading = { ...reading, vital: 'HR' }
// @ts-expect-error a device message cannot carry a vital the table lacks
export const endTidal: Reading = { ...reading, vital: 'EtCO2' }
