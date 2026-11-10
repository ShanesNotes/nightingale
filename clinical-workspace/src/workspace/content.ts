import { ENTRY_KINDS, type EntryContent } from '../entry-kinds.ts'
import { VITALS } from '../vitals.ts'

export function contentText(entry: EntryContent): string {
  if (entry.kind === 'vital') return `${entry.content.vital} ${entry.content.value} ${VITALS.find((row) => row.vital === entry.content.vital)?.unit ?? ''}`
  return `${ENTRY_KINDS[entry.kind].label}: ${Object.values(entry.content).join(' · ')}`
}
