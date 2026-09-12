import { Container } from '@mantine/core'
import { createContext, useContext, useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Navigate, useLocation, useNavigate } from 'react-router'
import type { SessionResponse } from '@/api/generated/types.gen'
import { sessionQuery, forgetSession } from '@/api/setup'
import { ApiClientError } from '@/api/runtime'
import { ErrorNotice, LoadingPanel } from './feedback'
const SessionContext = createContext<SessionResponse | null>(null)
export function useSession() {
  const session = useContext(SessionContext)
  if (!session) throw new Error('Session provider required')
  return session
}
export function Protected({ children }: { children: React.ReactNode }) {
  const query = useQuery(sessionQuery)
  const client = useQueryClient()
  const location = useLocation()
  const navigate = useNavigate()
  useEffect(() => {
    const expire = () => {
      forgetSession()
      client.clear()
      void navigate('/sign-in', { replace: true })
    }
    window.addEventListener('mobile-qa:unauthorized', expire)
    return () => window.removeEventListener('mobile-qa:unauthorized', expire)
  }, [client, navigate])
  if (query.error instanceof ApiClientError && query.error.status === 401)
    return <Navigate to="/sign-in" state={{ returnTo: location.pathname + location.search }} replace />
  if (query.isError)
    return (
      <Container component="main" size="sm" p="xl">
        <ErrorNotice error={query.error} retry={() => void query.refetch()} />
      </Container>
    )
  if (!query.data)
    return (
      <Container component="main" size="md" p="xl">
        <LoadingPanel label="Checking your session…" />
      </Container>
    )
  return <SessionContext.Provider value={query.data}>{children}</SessionContext.Provider>
}
