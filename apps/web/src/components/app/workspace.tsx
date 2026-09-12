import { ActionIcon, Alert, Button, Menu, NativeSelect, Stack, Text, Tooltip } from '@mantine/core'
import { Building2, Check, Layers3 } from 'lucide-react'
import { Link, Navigate, Outlet, useLocation } from 'react-router'
import { useSession } from './session'
import { useWorkspace, workspaceHref } from '@/hooks/use-workspace'

export function WorkspaceSwitcher({
  minimized = false,
  close,
}: {
  minimized?: boolean
  close?: () => void
}) {
  const session = useSession()
  const { workspace, select } = useWorkspace()
  if (minimized)
    return (
      <Menu position="right-start" width={240}>
        <Menu.Target>
          <Tooltip label={workspace?.name ?? 'Workspaces'} position="right" withArrow>
            <ActionIcon
              aria-label="Switch workspace"
              className="nav-icon"
              variant="subtle"
              size={44}
              mx="auto"
            >
              <Building2 size={20} />
            </ActionIcon>
          </Tooltip>
        </Menu.Target>
        <Menu.Dropdown>
          <Menu.Label>Workspaces</Menu.Label>
          {session.memberships.map((m) => (
            <Menu.Item
              key={m.organization_id}
              onClick={() => select(m.organization_id)}
              rightSection={
                workspace?.organization_id === m.organization_id ? <Check size={14} /> : undefined
              }
            >
              {m.name}
            </Menu.Item>
          ))}
          <Menu.Divider />
          <Menu.Item component={Link} to="/workspaces" leftSection={<Layers3 size={16} />}>
            Manage workspaces
          </Menu.Item>
        </Menu.Dropdown>
      </Menu>
    )
  return (
    <Stack gap="xs">
      <NativeSelect
        aria-label="Current workspace"
        value={workspace?.organization_id ?? ''}
        onChange={(event) => {
          if (event.currentTarget.value) {
            select(event.currentTarget.value)
            close?.()
          }
        }}
        data={[
          { value: '', label: 'Choose workspace', disabled: true },
          ...session.memberships.map((m) => ({
            value: m.organization_id,
            label: m.name,
          })),
        ]}
        disabled={!session.memberships.length}
        classNames={{ input: 'nav-workspace-input' }}
      />
      <Button
        component={Link}
        to="/workspaces"
        onClick={close}
        variant="subtle"
        size="xs"
        c="var(--workspace-nav-muted)"
        leftSection={<Layers3 size={14} />}
      >
        Workspaces
      </Button>
    </Stack>
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
