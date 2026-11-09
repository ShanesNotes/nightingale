import { start } from '../workspace/start.ts'

const container = document.getElementById('workspace')
if (container === null) throw new Error('index.html has no #workspace element')
start(container)
