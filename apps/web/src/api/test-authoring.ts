/** Generated contracts validate both requests and replies; there are no parallel wire models. */
import { queryOptions } from '@tanstack/react-query'
import * as sdk from './generated/sdk.gen'
import * as z from './generated/zod.gen'
import type {
  PhoneCommandRequest,
  GenerateTestsRequest,
  SaveAuthoredTestsRequest,
} from './generated/types.gen'
import { checked, headers, options } from './session-transport'
export const templatesQuery = (appId: string) =>
  queryOptions({
    queryKey: ['test-templates', appId],
    enabled: !!appId,
    queryFn: () =>
      checked(sdk.getTestTemplates({ ...options, path: { app_id: appId } }), z.zTestTemplates),
  })
export const runPhoneCommand = (id: string, body: PhoneCommandRequest) =>
  checked(
    sdk.runPhoneCommand({
      ...options,
      headers: headers(),
      path: { session_id: id },
      body: z.zPhoneCommandRequest.parse(body),
    }),
    z.zPhoneSession,
  )
export const generateTests = (appId: string, body: GenerateTestsRequest) =>
  checked(
    sdk.generateTests({
      ...options,
      headers: headers(),
      path: { app_id: appId },
      body: z.zGenerateTestsRequest.parse(body),
    }),
    z.zPhoneSession,
  )
export const saveAuthoredTests = (appId: string, body: SaveAuthoredTestsRequest) =>
  checked(
    sdk.saveAuthoredTests({
      ...options,
      headers: headers(),
      path: { app_id: appId },
      body: z.zSaveAuthoredTestsRequest.parse(body),
    }),
    z.zSavedAuthoredTests,
  )

export const cancelTestGeneration = (appId: string, jobId: string) =>
  checked(
    sdk.cancelTestGeneration({
      ...options,
      headers: headers(),
      path: { app_id: appId, job_id: jobId },
    }),
    z.zPhoneSession,
  )
