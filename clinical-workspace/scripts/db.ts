import { execFileSync } from 'node:child_process'

// npm run db:stop stops the launcher's database container. npm run reset removes it and its volume, so the next
// npm run dev starts from an empty chart.
const CONTAINER = process.env['CW_DB_CONTAINER'] ?? 'clinical-workspace-pg'

const docker = (...args: string[]) => {
  try {
    execFileSync('docker', args, { stdio: ['ignore', 'ignore', 'pipe'] })
    return true
  } catch {
    return false
  }
}

const command = process.argv[2]
if (command === 'stop') {
  process.stdout.write(docker('stop', CONTAINER) ? `db: stopped ${CONTAINER}\n` : `db: ${CONTAINER} was not running\n`)
} else if (command === 'reset') {
  const removed = docker('inspect', CONTAINER) && docker('rm', '-f', CONTAINER)
  const dropped = docker('volume', 'inspect', CONTAINER) && docker('volume', 'rm', CONTAINER)
  process.stdout.write(`db: ${removed ? 'removed' : 'found no'} container ${CONTAINER}, ${dropped ? 'removed' : 'found no'} volume ${CONTAINER}\n`)
} else {
  process.stderr.write('usage: node scripts/db.ts stop|reset\n')
  process.exit(64)
}
