import { foldkit } from '@foldkit/vite-plugin'
import { defineConfig } from 'vite'

// Two pages, each its own Foldkit program. The launcher sets CHART_SERVICE_URL for the /api proxy and
// VITE_RUN_ANCHOR for the patient clock. WORKSPACE_OUT_DIR moves the build out of the package.
export default defineConfig({
  plugins: [foldkit()],
  publicDir: false,
  server: {
    host: '127.0.0.1',
    proxy: process.env['CHART_SERVICE_URL'] ? { '/api': process.env['CHART_SERVICE_URL'] } : {},
  },
  build: {
    outDir: process.env['WORKSPACE_OUT_DIR'] ?? 'dist',
    emptyOutDir: true,
    target: 'es2022',
    rolldownOptions: { input: { workspace: 'index.html', monitor: 'monitor.html' } },
  },
})
