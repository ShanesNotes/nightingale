import type { Html, HtmlBuilder } from 'foldkit/html'
import { ROSTER } from '../chart-api.ts'
import { ENTRY_KINDS, type EntryKind, isEntryKind, kindFields } from '../entry-kinds.ts'
import { notes } from '../notes.ts'
import { contentText } from './content.ts'
import { type ChartedValue, flowsheet, type MonitorCell } from '../flowsheet.ts'
import { formatClock, formatDuration, type PatientTime, seconds, since } from '../time.ts'
import { VITALS, type VitalRow } from '../vitals.ts'
import { Message, type Model, selectedEntry } from './program.ts'

type H = HtmlBuilder<Message>

const minuteOf = (minute: PatientTime) => formatClock(minute).slice(0, 5)

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

function fields(kind: EntryKind, content: Readonly<Record<string, string>>, editing: boolean, model: Model, h: H, vitals: readonly VitalRow[]): Html[] {
  return kindFields(kind).map((field) => {
    const { name } = field
    const id = editing ? `edit-${name}` : kind === 'note' ? `note-${name}` : name
    const attributes = [h.Id(id), h.Disabled(model.writing !== null), h.Value(content[name] ?? '')]
    const changed = (text: string) => editing ? Message.TypedEditField({ field: name, text }) : Message.TypedField({ field: name, text })
    const options = name === 'vital' && kind === 'vital'
      ? vitals.map((row) => ({ value: row.vital, label: `${row.label} (${row.unit})` })) : field.input
    const control = typeof options !== 'string'
      ? h.select([...attributes, h.OnChange(changed)], options.map((option) => h.option([h.Value(option.value), h.Selected(option.value === content[name])], [option.label])))
      : options === 'multiline'
        ? h.textarea([...attributes, h.Rows(4), h.OnInput(changed)])
        : h.input([...attributes, h.OnInput(changed)])
    return h.div([h.Class('field')], [h.label([h.For(id)], [field.label]), control])
  })
}

function form(model: Model, h: H, vitals: readonly VitalRow[]): Html {
  const writing = h.Disabled(model.writing !== null)
  return h.section([h.Id('charting')], [
    h.h2([], ['Chart an entry']),
    h.div([h.Class('entry-form')], [
      h.div([h.Class('field')], [
        h.label([h.For('author')], ['Clinician']),
        h.select([h.Id('author'), writing, h.OnChange((author) => Message.ChoseAuthor({ author }))],
          ROSTER.map((c) => h.option([h.Value(c.id), h.Selected(c.id === model.form.author)], [`${c.name} (${c.discipline})`]))),
      ]),
      h.div([h.Class('field')], [
        h.label([h.For('entry-kind')], ['Entry kind']),
        h.select([h.Id('entry-kind'), writing, h.OnChange((kind) => Message.ChoseKind({ kind: isEntryKind(kind) ? kind : model.form.kind }))],
          Object.entries(ENTRY_KINDS).map(([kind, spec]) => h.option([h.Value(kind), h.Selected(kind === model.form.kind)], [spec.label]))),
      ]),
      ...fields(model.form.kind, model.form.content, false, model, h, vitals),
      h.div([h.Class('field')], [
        h.label([h.For('care')], ['Care time (HH:MM:SS; blank for now)']),
        h.input([h.Id('care'), writing, h.Value(model.form.care), h.Placeholder('now'), h.OnInput((text) => Message.TypedCare({ text }))]),
      ]),
      h.button([h.Id('chart'), h.Type('button'), writing, h.OnClick(Message.ClickedChart())], ['Chart']),
    ]),
  ])
}

function noteList(model: Model, h: H): Html {
  const rows = model.latest === null ? [] : notes(model.latest)
  return h.section([h.Id('notes')], [
    h.h2([], ['Notes']),
    model.latest === null ? h.p([], ['Notes show once the chart service answers.'])
      : rows.length === 0 ? h.p([], ['No notes charted yet.'])
      : h.ol([], rows.map(({ entry, lateBy, edited }) => h.li([h.DataAttribute('note', entry.entry)], [
          h.p([h.Class('note-meta')], [`Care ${formatClock(entry.care)} · ${entry.author.name} · ${lateBy > 0 ? `late by ${formatDuration(lateBy)}` : 'on time'}${edited ? ' · edited' : ''}`]),
          h.p([h.Class('note-text')], [entry.content.text]),
          h.button([h.Type('button'), h.OnClick(Message.ClickedEntry({ entry: entry.entry }))], ['Details']),
        ]))),
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

function details(model: Model, h: H, vitals: readonly VitalRow[]): Html {
  const current = selectedEntry(model)
  if (current === undefined) return h.empty
  const unit = current.kind === 'vital' ? vitals.find((row) => row.vital === current.content.vital)?.unit ?? '' : ''
  const late = current.charted > current.care ? `Late by ${formatDuration(since(current.charted, current.care))}` : 'Charted on time'
  const versions = model.versions?.entry === current.entry ? model.versions.list : undefined
  const writing = h.Disabled(model.writing !== null)
  return h.section([h.Id('details')], [
    h.h2([], [current.kind === 'note' ? 'Note' : contentText(current)]),
    current.kind === 'note' ? h.p([h.Class('note-text')], [current.content.text]) : h.empty,
    h.p([], [`By ${current.author.name}, ${current.author.discipline}`]),
    h.p([], [`Patient ${current.patient}`]),
    h.p([], [`Care time ${formatClock(current.care)}`]),
    h.p([], [`Charted time ${formatClock(current.charted)}`]),
    h.p([], [late]),
    h.p([], [`Version ${current.number} of ${current.versions}`]),
    model.editing === null
      ? h.button([h.Id('edit'), h.Type('button'), writing, h.OnClick(Message.ClickedEdit())], ['Edit'])
      : h.div([h.Id('editing')], [
          ...fields(model.editing.kind, model.editing.content, true, model, h, vitals),
          h.label([h.For('edit-care')], [' Care time ']),
          h.input([h.Id('edit-care'), writing, h.Value(model.editing.care), h.OnInput((text) => Message.TypedEditCare({ text }))]),
          h.button([h.Id('save-edit'), h.Type('button'), writing, h.OnClick(Message.ClickedSaveEdit())], ['Save as a new version']),
          h.button([h.Id('cancel-edit'), h.Type('button'), writing, h.OnClick(Message.ClickedCancelEdit())], ['Cancel']),
        ]),
    h.button([h.Id('versions-button'), h.Type('button'), h.OnClick(Message.ClickedVersions())], [versions === undefined ? 'Show earlier versions' : 'Hide earlier versions']),
    versions === undefined
      ? h.empty
      : h.ul(
          [h.Id('versions')],
          versions.map((v) =>
            h.li([], [`Version ${v.number}: ${v.kind === 'vital' ? `${v.content.value} ${unit}` : contentText(v)} by ${v.author.name}, care ${formatClock(v.care)}, charted ${formatClock(v.charted)}`]),
          ),
        ),
  ])
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
      form(model, h, vitals),
      h.p([h.Id('notice'), h.AriaLive('polite')], [model.notice]),
      noteList(model, h),
      details(model, h, vitals),
      h.h2([], ['Vitals']),
      h.div([h.Class('sheet-scroll')], [sheet(model, h, vitals)]),
    ])
  }

export const view = makeView(VITALS)
