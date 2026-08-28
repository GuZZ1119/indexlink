import path from 'node:path'
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

/** Keep browser-like frontend tests independent from production Vite configuration. */
export default defineConfig({
  plugins: [react()],
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  test: {
    environment: 'jsdom',
    coverage: {
      provider: 'v8',
      include: [
        'src/i18n/locales/en.ts',
        'src/i18n/locales/zh.ts',
        'src/pages/decisions/filters.ts',
      ],
      thresholds: { lines: 90, functions: 90, statements: 90, branches: 90 },
    },
  },
})
