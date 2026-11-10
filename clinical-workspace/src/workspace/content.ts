import { ENTRY_KINDS, type EntryContent } from '../entry-kinds.ts'
import { VITALS } from '../vitals.ts'

export function contentText(entry: EntryContent): string {
  if (entry.kind === 'vital') return `${entry.body.vital} ${entry.body.value} ${VITALS.find((row) => row.vital === entry.body.vital)?.unit ?? ''}`
  return `${ENTRY_KINDS[entry.kind].label}: ${Object.values(entry.body).join(' · ')}`
}
