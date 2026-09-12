import { useState } from 'react'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useLocation, useNavigate } from 'react-router'
import { sessionQuery, signIn, startGoogleSignIn } from '@/api/setup'
import { safeReturnTo } from '@/lib/navigation'

/** Only challenge metadata is cached; Google credentials never enter persistent storage. */
export function useGoogleSignIn() {
  const client = useQueryClient()
  const location = useLocation()
  const navigate = useNavigate()
  const [providerError, setProviderError] = useState<string | null>(null)
  const challenge = useQuery({
    queryKey: ['google-sign-in'],
    queryFn: startGoogleSignIn,
    retry: false,
    gcTime: 0,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  })
  const login = useMutation({
    mutationFn: signIn,
    gcTime: 0,
    onSuccess: (session) => {
      client.clear()
      client.setQueryData(sessionQuery.queryKey, session)
      void navigate(safeReturnTo(location.state?.returnTo), { replace: true })
    },
  })
  const acceptCredential = (credential?: string) => {
    if (!credential || !challenge.data) {
      setProviderError('Google did not return a sign-in credential. Please try again.')
      return
    }
    login.mutate({ credential, challenge_id: challenge.data.challenge_id })
  }
  const retry = () => {
    setProviderError(null)
    login.reset()
    void challenge.refetch()
  }
  return { challenge, login, providerError, setProviderError, acceptCredential, retry }
}
