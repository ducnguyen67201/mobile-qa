/** Generated SDK calls are the only transport; every response is checked at runtime. */
import { queryOptions } from '@tanstack/react-query'
import * as sdk from './generated/sdk.gen'
import * as schemas from './generated/zod.gen'
import type {
  CreateAppRequest,
  UpdateEnvironmentRequest,
  LoginRequest,
} from './generated/types.gen'
import { checked, headers, options, setCsrfToken, forgetSession } from './session-transport'
export { forgetSession } from './session-transport'
export const sessionQuery = queryOptions({
  queryKey: ['session'],
  queryFn: async () => {
    const session = await checked(sdk.getSession(options), schemas.zSessionResponse)
    setCsrfToken(session.csrf_token)
    return session
  },
  retry: false,
  staleTime: 30_000,
})
export function startGoogleSignIn() {
  return checked(
    sdk.startGoogleSignIn({
      ...options,
      headers: { 'X-Mobile-QA-Request': '1' },
    }),
    schemas.zGoogleLoginChallenge,
  )
}
export async function signIn(body: LoginRequest) {
  const session = await checked(
    sdk.login({
      ...options,
      body: schemas.zLoginRequest.parse(body),
      headers: { 'X-Mobile-QA-Request': '1' },
    }),
    schemas.zSessionResponse,
  )
  setCsrfToken(session.csrf_token)
  return session
}
export async function signOut() {
  const result = await checked(
    sdk.logout({ ...options, headers: headers() }),
    schemas.zLogoutResponse,
  )
  forgetSession()
  return result
}
export const appsQuery = (workspaceId: string, cursor?: string) =>
  queryOptions({
    queryKey: ['apps', workspaceId, cursor],
    queryFn: () =>
      checked(
        sdk.listApps({
          ...options,
          query: { limit: 20, cursor, organization_id: workspaceId },
        }),
        schemas.zAppListResponse,
      ),
  })
export const appQuery = (appId: string) =>
  queryOptions({
    queryKey: ['app', appId],
    queryFn: () =>
      checked(sdk.getApp({ ...options, path: { app_id: appId } }), schemas.zAppResponse),
  })
export async function createApp(body: CreateAppRequest) {
  return checked(
    sdk.createApp({
      ...options,
      body: schemas.zCreateAppRequest.parse(body),
      headers: headers(),
    }),
    schemas.zAppResponse,
    [201],
  )
}
export async function saveEnvironment(appId: string, body: UpdateEnvironmentRequest) {
  return checked(
    sdk.updateEnvironment({
      ...options,
      path: { app_id: appId },
      body: schemas.zUpdateEnvironmentRequest.parse(body),
      headers: headers(),
    }),
    schemas.zEnvironmentResponse,
  )
}
export const settingsQuery = queryOptions({
  queryKey: ['settings'],
  queryFn: () => checked(sdk.getSettings(options), schemas.zSettingsResponse),
})
export const buildsQuery = (appId: string, cursor?: string) =>
  queryOptions({
    queryKey: ['builds', appId, cursor],
    queryFn: () =>
      checked(
        sdk.listBuilds({
          ...options,
          path: { app_id: appId },
          query: { cursor, limit: 20 },
        }),
        schemas.zBuildListResponse,
      ),
    refetchInterval: (query) =>
      query.state.data?.items.some((build) => build.validation.state === 'validating')
        ? 3000
        : false,
  })
export const buildQuery = (appId: string, buildId: string) =>
  queryOptions({
    queryKey: ['build', appId, buildId],
    queryFn: () =>
      checked(
        sdk.getBuild({
          ...options,
          path: { app_id: appId, build_id: buildId },
        }),
        schemas.zBuildResponse,
      ),
    refetchInterval: (query) =>
      query.state.data?.validation.state === 'validating' ? 3000 : false,
  })
export function createUpload(appId: string, file: File, signal?: AbortSignal) {
  return checked(
    sdk.createBuildUpload({
      ...options,
      signal,
      path: { app_id: appId },
      headers: headers(),
      body: schemas.zCreateBuildUploadRequest.parse({
        original_filename: file.name,
        expected_size: file.size,
      }),
    }),
    schemas.zUploadResponse,
    [201],
  )
}
export function getUpload(appId: string, uploadId: string, signal?: AbortSignal) {
  return checked(
    sdk.getBuildUpload({
      ...options,
      signal,
      path: { app_id: appId, upload_id: uploadId },
    }),
    schemas.zUploadResponse,
  )
}
export function transferUpload(appId: string, uploadId: string, file: File, signal?: AbortSignal) {
  return checked(
    sdk.uploadBuildContent({
      ...options,
      signal,
      path: { app_id: appId, upload_id: uploadId },
      headers: headers(),
      body: { file },
    }),
    schemas.zUploadResponse,
  )
}
export function completeUpload(appId: string, uploadId: string, signal?: AbortSignal) {
  return checked(
    sdk.completeBuildUpload({
      ...options,
      signal,
      path: { app_id: appId, upload_id: uploadId },
      headers: headers(),
    }),
    schemas.zBuildResponse,
    [200, 202],
  )
}

export async function createWorkspace(
  body: import('./generated/types.gen').CreateWorkspaceRequest,
) {
  return checked(
    sdk.createWorkspace({
      ...options,
      body: schemas.zCreateWorkspaceRequest.parse(body),
      headers: headers(),
    }),
    schemas.zOrganizationMembership,
    [201],
  )
}
