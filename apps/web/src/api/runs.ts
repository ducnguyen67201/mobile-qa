/** Rust-generated transport and Zod remain authoritative at every JSON boundary. */
import { queryOptions } from '@tanstack/react-query'
import * as sdk from './generated/sdk.gen'
import * as z from './generated/zod.gen'
import type { CreateRunRequest } from './generated/types.gen'
import { checked, headers, options } from './session-transport'
export const planQuery = (
  workspace: string,
  appId: string,
  buildId: string,
  planVersionId?: string,
) =>
  queryOptions({
    queryKey: ['execution-plan', workspace, appId, buildId, planVersionId],
    enabled: !!appId && !!buildId,
    queryFn: () =>
      checked(
        sdk.getExecutionPlan({
          ...options,
          path: { app_id: appId },
          query: { build_id: buildId, plan_version_id: planVersionId },
        }),
        z.zPlanPreviewResponse,
      ),
  })
export const runQuery = (workspace: string, runId: string) =>
  queryOptions({
    queryKey: ['run', workspace, runId],
    enabled: !!runId,
    queryFn: () => checked(sdk.getRun({ ...options, path: { run_id: runId } }), z.zRunResponse),
    refetchInterval: (query) =>
      query.state.data?.state === 'finished' && query.state.data.comparison
        ? false
        : document.hidden
          ? 15_000
          : 2_000,
    refetchIntervalInBackground: true,
  })
export const runsQuery = (workspace: string, appId: string, cursor?: string) =>
  queryOptions({
    queryKey: ['runs', workspace, appId, cursor],
    enabled: !!appId,
    queryFn: () =>
      checked(
        sdk.listRuns({ ...options, path: { app_id: appId }, query: { cursor } }),
        z.zRunListResponse,
      ),
  })
export function createRun(appId: string, body: CreateRunRequest, key: string) {
  return checked(
    sdk.createRun({
      ...options,
      path: { app_id: appId },
      headers: { ...headers(), 'Idempotency-Key': key },
      body: z.zCreateRunRequest.parse(body),
    }),
    z.zRunResponse,
    [200, 201],
  )
}
export function cancelRun(runId: string) {
  return checked(
    sdk.cancelRun({ ...options, path: { run_id: runId }, headers: headers() }),
    z.zRunResponse,
  )
}
