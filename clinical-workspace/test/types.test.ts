import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { test } from 'node:test'

const tsc = fileURLToPath(new URL('../node_modules/typescript/bin/tsc', import.meta.url))
const pkg = fileURLToPath(new URL('..', import.meta.url))

test('the type checker refuses each illegal use test/type-rules.ts names, and the package type-checks', () => {
  const result = spawnSync(process.execPath, [tsc, '-p', pkg], { encoding: 'utf8' })
  assert.equal(result.status, 0, result.stdout + result.stderr)
})
