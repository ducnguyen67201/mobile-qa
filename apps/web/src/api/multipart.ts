/** Control-plane operations use generated contracts; only part bytes go to storage. */
import * as sdk from './generated/sdk.gen'
import * as schemas from './generated/zod.gen'
import type { AuthorizeUploadPartRequest } from './generated/types.gen'
import { checked, headers, options } from './session-transport'

export function getMultipartUpload(appId: string, uploadId: string, signal: AbortSignal) {
  return checked(
    sdk.getMultipartUpload({
      ...options,
      path: { app_id: appId, upload_id: uploadId },
      headers: headers(),
      signal,
    }),
    schemas.zMultipartUpload,
  )
}

export function startMultipartUpload(appId: string, uploadId: string, signal: AbortSignal) {
  return checked(
    sdk.startMultipartUpload({
      ...options,
      path: { app_id: appId, upload_id: uploadId },
      headers: headers(),
      signal,
    }),
    schemas.zMultipartUpload,
  )
}

export function authorizeUploadPart(
  appId: string,
  uploadId: string,
  body: AuthorizeUploadPartRequest,
  signal: AbortSignal,
) {
  return checked(
    sdk.authorizeUploadPart({
      ...options,
      path: { app_id: appId, upload_id: uploadId },
      body: schemas.zAuthorizeUploadPartRequest.parse(body),
      headers: headers(),
      signal,
    }),
    schemas.zUploadPartAuthorization,
  )
}

export function confirmUploadPart(
  appId: string,
  uploadId: string,
  partNumber: number,
  etag: string,
  signal: AbortSignal,
) {
  return checked(
    sdk.confirmUploadPart({
      ...options,
      path: { app_id: appId, upload_id: uploadId, part_number: partNumber },
      body: { etag },
      headers: headers(),
      signal,
    }),
    schemas.zMultipartUpload,
  )
}

export function completeMultipartUpload(appId: string, uploadId: string, signal: AbortSignal) {
  return checked(
    sdk.completeMultipartUpload({
      ...options,
      path: { app_id: appId, upload_id: uploadId },
      headers: headers(),
      signal,
    }),
    schemas.zUploadResponse,
    [202],
  )
}

export function abortMultipartUpload(appId: string, uploadId: string, signal: AbortSignal) {
  return checked(
    sdk.abortMultipartUpload({
      ...options,
      path: { app_id: appId, upload_id: uploadId },
      headers: headers(),
      signal,
    }),
    schemas.zMultipartUpload,
  )
}
