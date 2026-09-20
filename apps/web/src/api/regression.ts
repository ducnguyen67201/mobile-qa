/** Generated transport remains the only source of browser request/response shapes. */
import { queryOptions } from '@tanstack/react-query'
import * as sdk from './generated/sdk.gen'
import * as z from './generated/zod.gen'
import type { CaseRunRequest } from './generated/types.gen'
import { checked, headers, options } from './session-transport'
export const caseRunPreviewQuery = (appId: string, body: CaseRunRequest) =>
  queryOptions({
    queryKey: ['case-run-preview', appId, body],
    enabled: !!body.case_version_id && !!body.build_id && !!body.profile_id,
    queryFn: () =>
      checked(
        sdk.previewCaseRun({
          ...options,
          path: { app_id: appId },
          headers: headers(),
          body: z.zCaseRunRequest.parse(body),
        }),
        z.zCaseRunPreview,
      ),
  })
export const createCaseRun = (appId: string, body: CaseRunRequest, key: string) =>
  checked(
    sdk.createCaseRun({
      ...options,
      path: { app_id: appId },
      headers: { ...headers(), 'Idempotency-Key': key },
      body: z.zCaseRunRequest.parse(body),
    }),
    z.zRunResponse,
    [200, 201],
  )
export const historyQuery = (workspace: string, appId: string, source = 'all', cursor?: string) =>
  queryOptions({
    queryKey: ['run-history', workspace, appId, source, cursor],
    enabled: !!appId,
    queryFn: () =>
      checked(
        sdk.getRunHistory({ ...options, path: { app_id: appId }, query: { source, cursor } }),
        z.zRunHistory,
      ),
    refetchInterval: 10_000,
  })
