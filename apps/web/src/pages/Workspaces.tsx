import { useMounted } from '@/hooks/use-mounted'
import { useState } from 'react'
import { Alert, Button, Card, Group, SimpleGrid, Stack, Text, TextInput } from '@mantine/core'
import { useForm } from '@mantine/form'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Link, Navigate, useNavigate } from 'react-router'
import { Plus, ArrowRight } from 'lucide-react'
import { createWorkspace, sessionQuery } from '@/api/setup'
import { useSession } from '@/components/app/session'
import { ErrorNotice, PageHeading } from '@/components/app/feedback'
import { workspaceHref } from '@/hooks/use-workspace'

export function Workspaces() {
  const session = useSession()
  const client = useQueryClient()
  const refresh = useMutation({
    mutationFn: () => client.fetchQuery({ ...sessionQuery, staleTime: 0 }),
  })
  const approved = session.user.approval_status === 'approved'
  return (
    <>
      <PageHeading
        eyebrow="Your account"
        title="Your workspaces."
        description="Keep each team's apps, builds and environments together."
        action={
          approved ? (
            <Button component={Link} to="/workspaces/new" leftSection={<Plus size={16} />}>
              Create workspace
            </Button>
          ) : undefined
        }
      />
      {!approved && (
        <Alert title="Awaiting approval" color="yellow" mb="xl">
          <Stack gap="sm">
            <Text size="sm">
              You're signed in. Your account is pending approval to create workspaces.
            </Text>
            <Button
              variant="outline"
              loading={refresh.isPending}
              onClick={() => refresh.mutate()}
              w="fit-content"
            >
              Check approval status
            </Button>
            {refresh.isError && <ErrorNotice error={refresh.error} />}
          </Stack>
        </Alert>
      )}
      {session.memberships.length ? (
        <SimpleGrid cols={{ base: 1, md: 2 }}>
          {session.memberships.map((workspace) => (
            <Card key={workspace.organization_id}>
              <Text fw={600}>{workspace.name}</Text>
              <Text size="sm" c="dimmed" mt="xs">
                {workspace.role === 'operator' ? 'Owner / operator' : 'Member'}
              </Text>
              <Button
                component={Link}
                to={workspaceHref('/apps', workspace.organization_id)}
                variant="light"
                mt="lg"
                rightSection={<ArrowRight size={16} />}
              >
                Open workspace
              </Button>
            </Card>
          ))}
        </SimpleGrid>
      ) : (
        approved && (
          <Card>
            <Text>You don't have a workspace yet. Create one to start adding apps.</Text>
          </Card>
        )
      )}
    </>
  )
}

export function CreateWorkspace() {
  const session = useSession()
  if (session.user.approval_status !== 'approved') return <Navigate to="/workspaces" replace />
  return <CreateWorkspaceForm />
}
function CreateWorkspaceForm() {
  const mounted = useMounted()
  const [id] = useState(() => crypto.randomUUID())
  const client = useQueryClient()
  const navigate = useNavigate()
  const form = useForm({
    initialValues: { name: '' },
    validate: {
      name: (value) => (value.trim() ? null : 'Enter a workspace name'),
    },
  })
  const create = useMutation({
    mutationFn: (name: string) => createWorkspace({ id, name }),
    onSuccess: async (workspace) => {
      // Refresh grants before routing, so a newly created workspace passes the membership gate.
      await client.fetchQuery({ ...sessionQuery, staleTime: 0 })
      if (mounted.current)
        void navigate(workspaceHref('/apps', workspace.organization_id), {
          replace: true,
        })
    },
    onError: () => {
      void client.invalidateQueries({ queryKey: sessionQuery.queryKey })
    },
  })
  return (
    <>
      <PageHeading
        eyebrow="Set up your workspace"
        title="A home for your apps."
        description="Create a workspace for a team or project. You can create more and switch between them anytime."
      />
      <Card maw={560}>
        <form onSubmit={form.onSubmit(({ name }) => create.mutate(name.trim()))}>
          <Stack gap="lg">
            <TextInput
              label="Workspace name"
              placeholder="Your team or project"
              required
              maxLength={100}
              autoFocus
              {...form.getInputProps('name')}
            />
            <Text size="sm" c="dimmed">
              You will own this workspace. Apps, environments and builds added here stay in this
              workspace.
            </Text>
            {create.isError && <ErrorNotice focus error={create.error} />}
            <Group justify="flex-end">
              <Button component={Link} to="/workspaces" variant="subtle">
                Cancel
              </Button>
              <Button type="submit" loading={create.isPending}>
                Create workspace
              </Button>
            </Group>
          </Stack>
        </form>
      </Card>
    </>
  )
}
