import { defineConfig } from '@hey-api/openapi-ts'
import { fileURLToPath } from 'node:url'

// The coordinator supplies local staging paths. No hosted schema/account or watcher.
export default defineConfig({
  input: process.env.MOBILE_QA_OPENAPI_INPUT ?? fileURLToPath(new URL('../../contracts/browser.openapi.json', import.meta.url)),
  output: process.env.MOBILE_QA_SDK_OUTPUT ?? fileURLToPath(new URL('./src/api/generated', import.meta.url)),
  plugins: [
    '@hey-api/typescript',
    { name: '@hey-api/client-fetch', baseUrl: false },
    { name: 'zod', definitions: true, requests: true, responses: true },
    // Query boundaries invoke generated response schemas once, including 204/empty
    // bodies which the bundled fetch responseValidator path otherwise skips.
    { name: '@hey-api/sdk', validator: { request: 'zod' } },
  ],
})
