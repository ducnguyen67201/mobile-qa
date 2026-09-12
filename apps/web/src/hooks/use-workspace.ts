import { useLocation, useNavigate, useSearchParams } from 'react-router'
import { useSession } from '@/components/app/session'

export function workspaceHref(path: string, id?: string) {
  const [pathname, search = ''] = path.split('?')
  const params = new URLSearchParams(search)
  if (id) params.set('workspace', id)
  return pathname + (params.size ? `?${params}` : '')
}

/** Workspace selection lives in the URL, so tabs, reloads and browser history stay independent. */
export function useWorkspace() {
  const session = useSession()
  const [params] = useSearchParams()
  const navigate = useNavigate()
  const { pathname } = useLocation()
  const workspaceId = params.get('workspace') ?? undefined
  const workspace = session.memberships.find((m) => m.organization_id === workspaceId)
  const select = (id: string) => {
    // Detail/build/upload IDs belong to the previous workspace; switch to a safe landing page.
    const destination = ['/apps', '/tests', '/runs', '/settings'].includes(pathname)
      ? pathname
      : '/apps'
    void navigate(workspaceHref(destination, id))
  }
  return {
    workspace,
    workspaceId,
    select,
    href: (path: string) => workspaceHref(path, workspaceId),
  }
}
