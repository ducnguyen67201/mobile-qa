// Generator configuration, not a live API connection. scripts/contracts.py supplies
// temporary output paths for drift checks; edit this file instead of generated code.
import { defineConfig } from '@hey-api/openapi-ts'
import { fileURLToPath } from 'node:url'

// The coordinator supplies local staging paths. No hosted schema/account or watcher.
export default defineConfig({
  input:
    process.env.MOBILE_QA_OPENAPI_INPUT ??
    fileURLToPath(new URL('../../contracts/browser.openapi.json', import.meta.url)),
  output:
    process.env.MOBILE_QA_SDK_OUTPUT ??
    fileURLToPath(new URL('./src/api/generated', import.meta.url)),
  plugins: [
    '@hey-api/typescript',
    { name: '@hey-api/client-fetch', baseUrl: false },
    {
      name: 'zod',
      definitions: true,
      requests: true,
      responses: true,
      // 0.99 maps OpenAPI binary to string in Zod, despite Blob|File in TS.
      // Generate the browser binary boundary from the schema's binary format.
      $resolvers: {
        string: (ctx) =>
          ctx.schema.format === 'binary'
            ? ctx.$(ctx.symbols.z).attr('instanceof').call(ctx.$('Blob'))
            : undefined,
      },
    },
    // Query boundaries invoke generated response schemas once, including 204/empty
    // bodies which the bundled fetch responseValidator path otherwise skips.
    { name: '@hey-api/sdk', validator: { request: 'zod' } },
  ],
})
