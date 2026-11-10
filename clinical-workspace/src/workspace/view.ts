import type { Html, HtmlBuilder } from 'foldkit/html'
import { type EntryVersion, ROSTER } from '../chart-api.ts'
import { type EntryKind, kindFields } from '../entry-kinds.ts'
import { type ChartedValue, flowsheet, type MonitorCell } from '../flowsheet.ts'
import { type NoteRow, notes } from '../notes.ts'
import { type Duration, formatClock, formatDuration, type PatientTime, seconds, since } from '../time.ts'
import { VITALS, type VitalRow } from '../vitals.ts'
import { contentText } from './content.ts'
import { Message, type Model, selectedEntry } from './program.ts'

type H = HtmlBuilder<Message>

const minuteOf = (minute: PatientTime) => formatClock(minute).slice(0, 5)

const lateWords = (late: Duration) => (late > 0 ? `late by ${formatDuration(late)}` : 'on time')

function monitorText(cell: MonitorCell, unit: string): string {
  switch (cell.kind) {
    case 'no statement':
      return 'no device statement'
    case 'no source':
      return 'no source'
    case 'readings': {
      const parts = [cell.latest === undefined ? 'no reading' : `${Math.round(cell.latest.value)} ${unit}`]
      if (cell.absent > 0) parts.push(`${cell.absent} absent`)
      if (cell.late > 0) parts.push(`${cell.late} late`)
      if (cell.missing > 0) parts.push(`${cell.missing} missing`)
      return parts.join(', ')
    }
  }
}

function chartedText(value: ChartedValue, unit: string): string {
  const { entry } = value
  const parts = [`${entry.content.value} ${unit} by ${entry.author.name}`, `care ${formatClock(entry.care)}`, `charted ${formatClock(entry.charted)}`]
  if (value.lateBy > 0) parts.push(`late by ${formatDuration(value.lateBy)}`)
  if (value.edited) parts.push('edited')
  return parts.join(', ')
}

function fields(model: Model, h: H, kind: EntryKind, content: Readonly<Record<string, string>>, place: 'form' | 'editor', vitals: readonly VitalRow[]): Html[] {
  return kindFields(kind)
    .filter((field) => place === 'form' || !field.fixed)
    .flatMap((field) => {
      const id = place === 'editor' ? `edit-${field.name}` : kind === 'vital' ? field.name : `${kind}-${field.name}`
      const changed = (text: string) => (place === 'editor' ? Message.TypedEditField({ field: field.name, text }) : Message.TypedField({ kind, field: field.name, text }))
      const attributes = [h.Id(id), h.Disabled(model.writing !== null), h.Value(content[field.name] ?? '')]
      const options = kind === 'vital' && field.name === 'vital' ? vitals.map((row) => ({ value: row.vital, label: `${row.label} (${row.unit})` })) : field.input
      const control =
        typeof options !== 'string'
          ? h.select([...attributes, h.OnChange(changed)], options.map((option) => h.option([h.Value(option.value), h.Selected(option.value === content[field.name])], [option.label])))
          : options === 'multiline'
            ? h.textarea([...attributes, h.Rows(4), h.OnInput(changed)])
            : h.input([...attributes, h.OnInput(changed)])
      return [h.label([h.For(id)], [` ${field.label} `]), control]
    })
}

function clinician(model: Model, h: H): Html {
  return h.p([h.Id('clinician')], [
    h.label([h.For('author')], ['Charting as ']),
    h.select(
      [h.Id('author'), h.Disabled(model.writing !== null), h.OnChange((author) => Message.ChoseAuthor({ author }))],
      ROSTER.map((c) => h.option([h.Value(c.id), h.Selected(c.id === model.author)], [`${c.name} (${c.discipline})`])),
    ),
  ])
}

function careInput(model: Model, h: H, id: string, value: string, changed: (text: string) => Message, label: string): Html[] {
  return [
    h.label([h.For(id)], [label]),
    h.input([h.Id(id), h.Disabled(model.writing !== null), h.Value(value), h.Placeholder('now'), h.OnInput(changed)]),
  ]
}

const noticeAbout = (model: Model, kind: EntryKind) => (model.notice.about === kind ? model.notice.text : '')

function vitalForm(model: Model, h: H, vitals: readonly VitalRow[]): Html {
  const draft = model.drafts.vital
  return h.div([h.Id('charting')], [
    ...fields(model, h, 'vital', draft.content, 'form', vitals),
    ...careInput(model, h, 'care', draft.care, (text) => Message.TypedCare({ kind: 'vital', text }), ' Care time, HH:MM:SS or blank for now '),
    h.button([h.Id('chart'), h.Type('button'), h.Disabled(model.writing !== null), h.OnClick(Message.ClickedChart({ kind: 'vital' }))], ['Chart']),
  ])
}

function sheet(model: Model, h: H, vitals: readonly VitalRow[]): Html {
  if (model.latest === null) return h.p([h.Id('flowsheet')], ['The flowsheet shows once the chart service answers.'])
  const grid = flowsheet(model.latest, vitals)
  const minutes = grid.minutes.map(minuteOf)
  return h.table([h.Id('flowsheet')], [
    h.thead([], [h.tr([], [h.th([], ['Vital']), h.th([], ['Source']), ...minutes.map((m) => h.th([], [m]))])]),
    h.tbody(
      [],
      grid.rows.map((row) => {
        const name = (i: number) => `${row.vital.vital} ${row.source} ${minutes[i] ?? ''}`
        const cells =
          row.source === 'monitor'
            ? row.cells.map((cell, i) => h.td([h.DataAttribute('cell', name(i))], [monitorText(cell, row.vital.unit)]))
            : row.cells.map((cell, i) =>
                h.td(
                  [h.DataAttribute('cell', name(i))],
                  cell.entries.map((value) =>
                    h.button([h.Type('button'), h.OnClick(Message.ClickedEntry({ entry: value.entry.entry }))], [chartedText(value, row.vital.unit)]),
                  ),
                ),
              )
        return h.tr([h.DataAttribute('vital', row.vital.vital), h.DataAttribute('source', row.source)], [
          h.th([], [`${row.vital.label} (${row.vital.unit})`]),
          h.td([], [row.source === 'monitor' ? 'monitor' : 'charted']),
          ...cells,
        ])
      }),
    ),
  ])
}

function editor(model: Model, h: H, vitals: readonly VitalRow[]): Html {
  if (model.editing === null) return h.empty
  const writing = h.Disabled(model.writing !== null)
  return h.div([h.Id('editing')], [
    ...fields(model, h, model.editing.kind, model.editing.content, 'editor', vitals),
    ...careInput(model, h, 'edit-care', model.editing.care, (text) => Message.TypedEditCare({ text }), ' Care time '),
    h.button([h.Id('save-edit'), h.Type('button'), writing, h.OnClick(Message.ClickedSaveEdit())], ['Save as a new version']),
    h.button([h.Id('cancel-edit'), h.Type('button'), writing, h.OnClick(Message.ClickedCancelEdit())], ['Cancel']),
  ])
}

const versionsOf = (model: Model, entry: string) => (model.versions?.entry === entry ? model.versions.list : undefined)

function details(model: Model, h: H, vitals: readonly VitalRow[]): Html {
  const current = selectedEntry(model)
  if (current === undefined) return h.empty
  const late = current.charted > current.care ? `Late by ${formatDuration(since(current.charted, current.care))}` : 'Charted on time'
  const versions = versionsOf(model, current.entry)
  return h.section([h.Id('details')], [
    h.h2([], [contentText(current)]),
    h.p([], [`By ${current.author.name}, ${current.author.discipline}`]),
    h.p([], [`Patient ${current.patient}`]),
    h.p([], [`Care time ${formatClock(current.care)}`]),
    h.p([], [`Charted time ${formatClock(current.charted)}`]),
    h.p([], [late]),
    h.p([], [`Version ${current.number} of ${current.versions}`]),
    model.editing?.entry === current.entry
      ? editor(model, h, vitals)
      : h.button([h.Id('edit'), h.Type('button'), h.Disabled(model.writing !== null), h.OnClick(Message.ClickedEdit({ entry: current.entry }))], ['Edit']),
    h.button([h.Id('versions-button'), h.Type('button'), h.OnClick(Message.ClickedVersions({ entry: current.entry }))], [versions === undefined ? 'Show earlier versions' : 'Hide earlier versions']),
    versions === undefined
      ? h.empty
      : h.ul(
          [h.Id('versions')],
          versions.map((v) => h.li([], [`Version ${v.number}: ${contentText(v)} by ${v.author.name}, care ${formatClock(v.care)}, charted ${formatClock(v.charted)}`])),
        ),
  ])
}

function noteForm(model: Model, h: H, vitals: readonly VitalRow[]): Html {
  const draft = model.drafts.note
  return h.div([h.Id('note-form')], [
    ...fields(model, h, 'note', draft.content, 'form', vitals),
    h.div([], [
      ...careInput(model, h, 'note-care', draft.care, (text) => Message.TypedCare({ kind: 'note', text }), 'Care time, HH:MM:SS or blank for now '),
      h.button([h.Id('chart-note'), h.Type('button'), h.Disabled(model.writing !== null), h.OnClick(Message.ClickedChart({ kind: 'note' }))], ['Chart note']),
    ]),
  ])
}

function earlierVersions(h: H, versions: readonly EntryVersion[], current: number): Html {
  return h.ol(
    [h.Id('versions')],
    versions
      .filter((v) => v.number < current)
      .map((v) =>
        h.li([], [
          h.p([h.Class('note-head')], [`Version ${v.number} · ${v.author.name}, ${v.author.discipline} · care time ${formatClock(v.care)} · charted ${formatClock(v.charted)}`]),
          h.p([h.Class('note-text')], [v.kind === 'note' ? v.content.text : contentText(v)]),
        ]),
      ),
  )
}

function noteItem(model: Model, h: H, row: NoteRow, vitals: readonly VitalRow[]): Html {
  const { entry, lateBy, edited } = row
  const editing = model.editing?.entry === entry.entry
  const shown = versionsOf(model, entry.entry)
  const head = [`Care time ${formatClock(entry.care)}`, `${entry.author.name}, ${entry.author.discipline}`]
  if (edited) head.push(`edited, version ${entry.number} of ${entry.versions}`)
  return h.li([h.DataAttribute('entry', entry.entry), h.Class('note')], [
    h.p([h.Class('note-head')], [head.join(' · ')]),
    editing ? editor(model, h, vitals) : h.p([h.Class('note-text')], [entry.content.text]),
    h.p([h.Class('note-charted')], [`${edited ? `Version ${entry.number} charted` : 'Charted'} ${formatClock(entry.charted)}, ${lateWords(lateBy)}.`]),
    h.div([], [
      editing ? h.empty : h.button([h.Type('button'), h.DataAttribute('action', 'edit'), h.Disabled(model.writing !== null), h.OnClick(Message.ClickedEdit({ entry: entry.entry }))], ['Edit']),
      edited
        ? h.button([h.Type('button'), h.DataAttribute('action', 'versions'), h.OnClick(Message.ClickedVersions({ entry: entry.entry }))], [shown === undefined ? 'Show earlier versions' : 'Hide earlier versions'])
        : h.empty,
    ]),
    shown === undefined ? h.empty : earlierVersions(h, shown, entry.number),
  ])
}

function noteList(model: Model, h: H, vitals: readonly VitalRow[]): Html {
  if (model.latest === null) return h.p([h.Id('note-list')], ['The notes show once the chart service answers.'])
  const rows = notes(model.latest)
  if (rows.length === 0) return h.p([h.Id('note-list')], ['No notes charted yet.'])
  return h.ol([h.Id('note-list')], rows.map((row) => noteItem(model, h, row, vitals)))
}

function serviceWords(model: Model): string {
  if (model.latest === null) return 'The chart service has not answered yet.'
  if (model.answering) return 'The chart service is answering.'
  const ago = model.heard === null || model.checked === null ? 0 : Math.floor((model.checked - model.heard) / 1000)
  return `The chart service is not answering. This view was current ${formatDuration(seconds(ago))} ago, at patient time ${formatClock(model.latest.now)}.`
}

function closed(h: H): Html {
  return h.div([h.Id('workspace')], [
    h.h1([], ['Clinical workspace']),
    h.p([h.Id('closed')], ['The chart is closed.']),
    h.button([h.Id('reopen'), h.Type('button'), h.OnClick(Message.ClickedReopen())], ['Reopen chart']),
  ])
}

export const makeView =
  (vitals: readonly VitalRow[]) =>
  (model: Model, h: H): Html => {
    if (model.screen === 'closed') return closed(h)
    const { latest } = model
    return h.div([h.Id('workspace')], [
      h.h1([], ['Clinical workspace']),
      h.button([h.Id('close'), h.Type('button'), h.OnClick(Message.ClickedClose())], ['Close chart']),
      h.p([h.Id('patient')], [latest === null ? 'Waiting for the chart service.' : `Patient ${latest.run.patient}, synthetic, from a recorded run.`]),
      h.p([h.Id('now')], [latest === null ? '' : `Patient time ${formatClock(latest.now)}`]),
      h.p([h.Id('service'), h.AriaLive('polite')], [serviceWords(model)]),
      clinician(model, h),
      h.section([h.Id('vitals')], [
        h.h2([], ['Flowsheet']),
        vitalForm(model, h, vitals),
        h.p([h.Id('notice'), h.AriaLive('polite')], [noticeAbout(model, 'vital')]),
        sheet(model, h, vitals),
        details(model, h, vitals),
      ]),
      h.section([h.Id('notes')], [
        h.h2([], ['Notes']),
        noteForm(model, h, vitals),
        h.p([h.Id('note-notice'), h.AriaLive('polite')], [noticeAbout(model, 'note')]),
        noteList(model, h, vitals),
      ]),
    ])
  }

export const view = makeView(VITALS)
