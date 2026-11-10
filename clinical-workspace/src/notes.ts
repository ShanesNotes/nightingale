import { entriesOfKind, type EntryView, type ReadModel } from './chart-api.ts'
import { type Duration, since } from './time.ts'

export type NoteRow = { readonly entry: EntryView<'note'>; readonly lateBy: Duration; readonly edited: boolean }

export const notes = (model: ReadModel): readonly NoteRow[] => entriesOfKind(model, 'note')
  .toSorted((a, b) => b.care - a.care || b.charted - a.charted || a.entry.localeCompare(b.entry))
  .map((entry) => ({ entry, lateBy: since(entry.charted, entry.care), edited: entry.versions > 1 }))
