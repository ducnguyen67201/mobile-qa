/** Health query used by the App page. Generated HTTP types alone do not validate JSON. */
import type { GetHealthResponses } from './generated/types.gen'
import { getHealth } from './generated/sdk.gen'
import { zHealthResponse } from './generated/zod.gen'
import { apiClient } from './runtime'

export const healthQuery = {
  queryKey: ['health'],
  queryFn: async () => {
    const result = await getHealth({ client: apiClient, throwOnError: true })
    // Bind the expected status to the generated operation: a plausible JSON body
    // on an undeclared 201/204 must not make the health indicator turn green.
    const expectedStatus = 200 satisfies keyof GetHealthResponses
    if (result.response.status !== expectedStatus) {
      throw new Error(`Unexpected health response status (${result.response.status})`)
    }
    // Generated validation, not a handwritten field parser. It also rejects an
    // unexpected 204 or empty body, and handles non-JSON success responses safely.
    return zHealthResponse.parse(result.data)
  },
  // Keep failures visible until the user invokes Retry; do not hide them with auto-retries.
  retry: false,
} as const
