import { Runtime } from 'foldkit'
import { init, Model, subscriptions, update } from './program.ts'
import { view } from './view.ts'

export function start(container: HTMLElement): void {
  Runtime.run(Runtime.makeElement({ Model, init, update, view, subscriptions, container }))
}
