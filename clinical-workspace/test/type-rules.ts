import { entriesOfKind, type ClinicianId, type EntryId, type NewEntry, type ReadModel, type WriteId } from '../src/chart-api.ts'
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

export const signed: NewEntry = { operation: 'new entry', writeId, patient, author, kind: 'vital' as const, body: { vital: 'HR', value: 61 }, care: { kind: 'now' } }
// @ts-expect-error an entry cannot be written without an author
export const unsigned: NewEntry = { operation: 'new entry', writeId, patient, kind: 'vital' as const, body: { vital: 'HR', value: 61 }, care: { kind: 'now' } }
// @ts-expect-error an entry cannot be written without its write id
export const withoutWriteId: NewEntry = { operation: 'new entry', patient, author, kind: 'vital' as const, body: { vital: 'HR', value: 61 }, care: { kind: 'now' } }

export const heartRate: Reading = { ...reading, vital: 'HR' }
// @ts-expect-error a device message cannot carry a vital the table lacks
export const endTidal: Reading = { ...reading, vital: 'EtCO2' }

export const note: NewEntry = { operation: 'new entry', kind: 'note', body: { text: 'Synthetic note.' }, writeId, patient, author, care: { kind: 'now' } }
// @ts-expect-error a note cannot have a vital body
export const wrongBody: NewEntry = { ...note, body: { vital: 'HR', value: 1.25 } }
// @ts-expect-error an unregistered kind cannot be written
export const unknownKind: NewEntry = { ...note, kind: 'unregistered' }

declare const chart: ReadModel
export const noteText: string | undefined = entriesOfKind(chart, 'note')[0]?.body.text
// @ts-expect-error a typed note read has no numeric vital value
export const noteValue = entriesOfKind(chart, 'note')[0]?.body.value
