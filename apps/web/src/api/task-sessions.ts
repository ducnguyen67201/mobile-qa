/** Rust owns the wire contracts; generated Zod validates every boundary. */
import { queryOptions } from '@tanstack/react-query'
import * as sdk from './generated/sdk.gen'
import * as z from './generated/zod.gen'
import type { OpenPhoneRequest } from './generated/types.gen'
import { checked, headers, options } from './session-transport'
export const phoneOptionsQuery = (appId: string) =>
  queryOptions({
    queryKey: ['phone-options', appId],
    enabled: !!appId,
    queryFn: () =>
      checked(sdk.getPhoneOptions({ ...options, path: { app_id: appId } }), z.zPhoneOptions),
  })
export const phoneQuery = (id: string) =>
  queryOptions({
    queryKey: ['phone', id],
    enabled: !!id,
    queryFn: () => checked(sdk.getPhone({ ...options, path: { session_id: id } }), z.zPhoneSession),
    refetchInterval: (query) =>
      ['closed', 'quarantined'].includes(query.state.data?.state ?? '') ? false : 2000,
  })
export const openPhone = (appId: string, body: OpenPhoneRequest) =>
  checked(
    sdk.openPhone({
      ...options,
      headers: headers(),
      path: { app_id: appId },
      body: z.zOpenPhoneRequest.parse(body),
    }),
    z.zPhoneSession,
  )
export const stopPhone = (id: string) =>
  checked(
    sdk.stopPhone({ ...options, headers: headers(), path: { session_id: id } }),
    z.zPhoneSession,
  )
