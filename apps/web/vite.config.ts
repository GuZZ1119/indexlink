import path from 'node:path'
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      '/investment-plans': 'http://127.0.0.1:8080',
      '/signals': 'http://127.0.0.1:8080',
      '/decisions': 'http://127.0.0.1:8080',
      '/market-sentiment': 'http://127.0.0.1:8080',
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
})
