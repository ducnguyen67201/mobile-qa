// Browser dev/build and DOM-test configuration. The proxy keeps API calls same-origin;
// it does not add authentication or make server secrets safe to expose to the browser.
import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'

// Google and the API use localhost as the local browser origin. Vite binds to
// loopback by IP, so redirect document visits before the sign-in challenge runs.
const canonicalDevOrigin = {
  name: 'canonical-dev-origin',
  configureServer(server: import('vite').ViteDevServer) {
    server.middlewares.use((request, response, next) => {
      if (
        request.method === 'GET' &&
        request.headers.host === '127.0.0.1:5173' &&
        request.headers.accept?.includes('text/html')
      ) {
        response.statusCode = 307
        response.setHeader('Location', `http://localhost:5173${request.url || '/'}`)
        response.end()
        return
      }
      next()
    })
  },
}

export default defineConfig({
  envDir: false, // Process environment only; no .env files.
  plugins: [react(), canonicalDevOrigin],
  resolve: { alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) } },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    proxy: { '/api': 'http://127.0.0.1:5150' },
  },
  test: { environment: 'jsdom', setupFiles: ['./src/test/setup.ts'], restoreMocks: true },
})
