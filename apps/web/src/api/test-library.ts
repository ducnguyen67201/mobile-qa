/** The library adds no independent wire models: Rust generates every request and response. */
import { queryOptions } from '@tanstack/react-query'
import * as sdk from './generated/sdk.gen'
import * as z from './generated/zod.gen'
import type {
  ArchiveLibraryEntryRequest,
  CreateLibraryEntryRequest,
  SaveLibraryDraftRequest,
  SetDefaultPlanRequest,
  ListTestLibraryData,
} from './generated/types.gen'
import { ApiClientError } from './runtime'
import { checked, headers, options } from './session-transport'
export const libraryKey = (workspace: string, appId: string) =>
  ['test-library', workspace, appId] as const
export const libraryQuery = (
  workspace: string,
  appId: string,
  query: ListTestLibraryData['query'] = {},
) =>
  queryOptions({
    queryKey: [...libraryKey(workspace, appId), 'list', query],
    enabled: !!appId,
    queryFn: () =>
      checked(
        sdk.listTestLibrary({ ...options, path: { app_id: appId }, query }),
        z.zLibraryListResponse,
      ),
  })
export const libraryOptionsQuery = (workspace: string, appId: string) =>
  queryOptions({
    queryKey: [...libraryKey(workspace, appId), 'options'],
    enabled: !!appId,
    queryFn: () =>
      checked(
        sdk.getTestLibraryOptions({ ...options, path: { app_id: appId } }),
        z.zLibraryOptionsResponse,
      ),
  })
export const libraryEntryQuery = (workspace: string, appId: string, entryId: string) =>
  queryOptions({
    queryKey: [...libraryKey(workspace, appId), entryId],
    enabled: !!appId && !!entryId,
    queryFn: () =>
      checked(
        sdk.getTestLibraryEntry({ ...options, path: { app_id: appId, entry_id: entryId } }),
        z.zLibraryEntryResponse,
      ),
  })
export const libraryDraftQuery = (workspace: string, appId: string, entryId: string) =>
  queryOptions({
    queryKey: [...libraryKey(workspace, appId), entryId, 'draft'],
    enabled: !!appId && !!entryId,
    queryFn: () =>
      checked(
        sdk.getTestLibraryDraft({ ...options, path: { app_id: appId, entry_id: entryId } }),
        z.zLibraryDraftResponse,
      ),
  })
export const libraryVersionQuery = (
  workspace: string,
  appId: string,
  entryId: string,
  versionId: string,
) =>
  queryOptions({
    queryKey: [...libraryKey(workspace, appId), entryId, 'version', versionId],
    enabled: !!appId && !!entryId && !!versionId,
    queryFn: () =>
      checked(
        sdk.getTestLibraryVersion({
          ...options,
          path: { app_id: appId, entry_id: entryId, version_id: versionId },
        }),
        z.zLibraryVersionResponse,
      ),
  })
export const libraryHistoryQuery = (
  workspace: string,
  appId: string,
  entryId: string,
  cursor?: string,
) =>
  queryOptions({
    queryKey: [...libraryKey(workspace, appId), entryId, 'history', cursor],
    enabled: !!appId && !!entryId,
    queryFn: () =>
      checked(
        sdk.listTestLibraryVersions({
          ...options,
          path: { app_id: appId, entry_id: entryId },
          query: { cursor },
        }),
        z.zLibraryVersionListResponse,
      ),
  })
export const defaultPlanQuery = (workspace: string, appId: string) =>
  queryOptions({
    queryKey: [...libraryKey(workspace, appId), 'default'],
    enabled: !!appId,
    queryFn: () =>
      checked(
        sdk.getDefaultTestPlan({ ...options, path: { app_id: appId } }),
        z.zDefaultPlanResponse,
      ),
  })
export function createLibraryEntry(appId: string, body: CreateLibraryEntryRequest) {
  return checked(
    sdk.createTestLibraryEntry({
      ...options,
      path: { app_id: appId },
      headers: headers(),
      body: z.zCreateLibraryEntryRequest.parse(body),
    }),
    z.zLibraryDraftResponse,
    [200, 201],
  )
}
export function saveLibraryDraft(appId: string, entryId: string, body: SaveLibraryDraftRequest) {
  return checked(
    sdk.saveTestLibraryDraft({
      ...options,
      path: { app_id: appId, entry_id: entryId },
      headers: headers(),
      body: z.zSaveLibraryDraftRequest.parse(body),
    }),
    z.zLibraryDraftResponse,
  )
}
export function archiveLibraryEntry(
  appId: string,
  entryId: string,
  body: ArchiveLibraryEntryRequest,
) {
  return checked(
    sdk.archiveTestLibraryEntry({
      ...options,
      path: { app_id: appId, entry_id: entryId },
      headers: headers(),
      body: z.zArchiveLibraryEntryRequest.parse(body),
    }),
    z.zLibraryEntryResponse,
  )
}
export function setDefaultPlan(appId: string, body: SetDefaultPlanRequest) {
  return checked(
    sdk.setDefaultTestPlan({
      ...options,
      path: { app_id: appId },
      headers: headers(),
      body: z.zSetDefaultPlanRequest.parse(body),
    }),
    z.zDefaultPlanResponse,
  )
}
/** ApiError.details is intentionally unknown. Only a generated feature union may be rendered. */
export function libraryErrorDetails(error: unknown) {
  if (!(error instanceof ApiClientError)) return undefined
  const result = z.zLibraryErrorDetails.safeParse(error.body?.details)
  return result.success ? result.data : undefined
}
