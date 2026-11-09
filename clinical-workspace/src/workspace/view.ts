import type { Html, HtmlBuilder } from 'foldkit/html'
import { ROSTER } from '../chart-api.ts'
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
  const parts = [`${entry.value} ${unit} by ${entry.author.name}`, `care ${formatClock(entry.care)}`, `charted ${formatClock(entry.charted)}`]
  if (value.lateBy > 0) parts.push(`late by ${formatDuration(value.lateBy)}`)
  if (value.edited) parts.push('edited')
  return parts.join(', ')
}

function form(model: Model, h: H, vitals: readonly VitalRow[]): Html {
  return h.div([h.Id('charting')], [
    h.label([h.For('author')], ['Clinician ']),
    h.select(
      [h.Id('author'), h.OnChange((author) => Message.ChoseAuthor({ author }))],
      ROSTER.map((c) => h.option([h.Value(c.id), h.Selected(c.id === model.form.author)], [`${c.name} (${c.discipline})`])),
    ),
    h.label([h.For('vital')], [' Vital ']),
    h.select(
      [h.Id('vital'), h.OnChange((vital) => Message.ChoseVital({ vital }))],
      vitals.map((row) => h.option([h.Value(row.vital), h.Selected(row.vital === model.form.vital)], [`${row.label} (${row.unit})`])),
    ),
    h.label([h.For('value')], [' Value ']),
    h.input([h.Id('value'), h.Value(model.form.value), h.OnInput((text) => Message.TypedValue({ text }))]),
    h.label([h.For('care')], [' Care time, HH:MM:SS or blank for now ']),
    h.input([h.Id('care'), h.Value(model.form.care), h.Placeholder('now'), h.OnInput((text) => Message.TypedCare({ text }))]),
    h.button([h.Id('chart'), h.Type('button'), h.OnClick(Message.ClickedChart())], ['Chart']),
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
  const unit = vitals.find((row) => row.vital === current.vital)?.unit ?? ''
  const late = current.charted > current.care ? `Late by ${formatDuration(since(current.charted, current.care))}` : 'Charted on time'
  const versions = model.versions?.entry === current.entry ? model.versions.list : undefined
  return h.section([h.Id('details')], [
    h.h2([], [`${current.vital} ${current.value} ${unit}`]),
    h.p([], [`By ${current.author.name}, ${current.author.discipline}`]),
    h.p([], [`Patient ${current.patient}`]),
    h.p([], [`Care time ${formatClock(current.care)}`]),
    h.p([], [`Charted time ${formatClock(current.charted)}`]),
    h.p([], [late]),
    h.p([], [`Version ${current.number} of ${current.versions}`]),
    model.editing === null
      ? h.button([h.Id('edit'), h.Type('button'), h.OnClick(Message.ClickedEdit())], ['Edit'])
      : h.div([h.Id('editing')], [
          h.label([h.For('edit-value')], ['New value ']),
          h.input([h.Id('edit-value'), h.Value(model.editing.value), h.OnInput((text) => Message.TypedEditValue({ text }))]),
          h.label([h.For('edit-care')], [' Care time ']),
          h.input([h.Id('edit-care'), h.Value(model.editing.care), h.OnInput((text) => Message.TypedEditCare({ text }))]),
          h.button([h.Id('save-edit'), h.Type('button'), h.OnClick(Message.ClickedSaveEdit())], ['Save as a new version']),
        ]),
    h.button([h.Id('versions-button'), h.Type('button'), h.OnClick(Message.ClickedVersions())], ['Show earlier versions']),
    versions === undefined
      ? h.empty
      : h.ul(
          [h.Id('versions')],
          versions.map((v) =>
            h.li([], [`Version ${v.number}: ${v.value} ${unit} by ${v.author.name}, care ${formatClock(v.care)}, charted ${formatClock(v.charted)}`]),
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
      sheet(model, h, vitals),
      details(model, h, vitals),
    ])
  }

export const view = makeView(VITALS)
