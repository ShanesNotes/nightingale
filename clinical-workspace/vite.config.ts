import { foldkit } from '@foldkit/vite-plugin'
import { defineConfig } from 'vite'

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
