import { Alert, Button, Group, NativeSelect, Stack, Text } from '@mantine/core'
import { Link, Navigate, Outlet, useLocation } from 'react-router'
import { useSession } from './session'
import { useWorkspace, workspaceHref } from '@/hooks/use-workspace'

export function WorkspaceSwitcher() {
  const session = useSession()
  const { workspace, select } = useWorkspace()
  return (
    <Group gap="sm" wrap="nowrap">
      <NativeSelect
        aria-label="Current workspace"
        value={workspace?.organization_id ?? ''}
        onChange={(event) => {
          if (event.currentTarget.value) select(event.currentTarget.value)
        }}
        data={[
          { value: '', label: 'Choose workspace', disabled: true },
          ...session.memberships.map((m) => ({
            value: m.organization_id,
            label: m.name,
          })),
        ]}
        disabled={!session.memberships.length}
        maw={{ base: 180, sm: 260 }}
      />
      <Button component={Link} to="/workspaces" variant="subtle" size="xs">
        Workspaces
      </Button>
    </Group>
  )
}

export function WorkspaceGate() {
  const session = useSession()
  const { workspace, workspaceId } = useWorkspace()
  const location = useLocation()
  if (!workspaceId) {
    const first = session.memberships[0]
    if (first)
      return (
        <Navigate
          replace
          to={workspaceHref(location.pathname + location.search, first.organization_id)}
        />
      )
    return (
      <Navigate
        replace
        to={session.user.approval_status === 'approved' ? '/workspaces/new' : '/workspaces'}
      />
    )
  }
  if (!workspace)
    return (
      <Alert color="red" title="Workspace unavailable">
        <Stack gap="sm">
          <Text size="sm">This workspace does not exist or you no longer have access.</Text>
          <Button component={Link} to="/workspaces" variant="outline">
            Choose a workspace
          </Button>
        </Stack>
      </Alert>
    )
  // Remount local form/upload/pagination state when navigating between workspace URLs.
  return <Outlet key={workspace.organization_id} />
}

export function WorkspaceIndex() {
  const location = useLocation()
  return <Navigate to={'/apps' + location.search} replace />
}
