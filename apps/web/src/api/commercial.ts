/** Commercial status and quotes use Rust-generated transport and runtime schemas. */
import { queryOptions } from '@tanstack/react-query'
import * as sdk from './generated/sdk.gen'
import * as z from './generated/zod.gen'
import type {
  CommercialQuoteRequest,
  CommercialPilotRequest,
  CreditCheckoutRequest,
  CreditPlanChangeRequest,
} from './generated/types.gen'
import { checked, headers, options } from './session-transport'

export const commercialAccessQuery = (workspace: string, appId: string) =>
  queryOptions({
    queryKey: ['commercial-access', workspace, appId],
    enabled: !!appId,
    queryFn: () =>
      checked(
        sdk.getCommercialAccess({ ...options, path: { app_id: appId } }),
        z.zCommercialAccessResponse,
      ),
  })

export function createCommercialCheckQuote(appId: string, body: CommercialQuoteRequest) {
  return checked(
    sdk.createCommercialCheckQuote({
      ...options,
      path: { app_id: appId },
      headers: headers(),
      body: z.zCommercialQuoteRequest.parse(body),
    }),
    z.zCommercialQuoteResponse,
    [201],
  )
}

export function requestCommercialPilot(appId: string, body: CommercialPilotRequest) {
  return checked(
    sdk.requestCommercialPilot({
      ...options,
      path: { app_id: appId },
      headers: headers(),
      body: z.zCommercialPilotRequest.parse(body),
    }),
    z.zCommercialPilotResponse,
    [201],
  )
}

export function createCreditCheckout(appId: string, body: CreditCheckoutRequest) {
  return checked(
    sdk.createCreditCheckout({
      ...options,
      path: { app_id: appId },
      headers: headers(),
      body: z.zCreditCheckoutRequest.parse(body),
    }),
    z.zCreditCheckoutResponse,
    [201],
  )
}

export function changeCreditPlan(appId: string, body: CreditPlanChangeRequest) {
  return checked(
    sdk.changeCreditPlan({
      ...options,
      path: { app_id: appId },
      headers: headers(),
      body: z.zCreditPlanChangeRequest.parse(body),
    }),
    z.zCommercialAccessResponse,
  )
}
