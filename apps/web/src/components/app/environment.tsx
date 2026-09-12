import { useDisclosure } from '@mantine/hooks'
import { useForm } from '@mantine/form'
import {
  Button,
  Card,
  Divider,
  Drawer,
  Group,
  NativeSelect,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from '@mantine/core'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Globe2, Pencil, ShieldCheck } from 'lucide-react'
import type {
  AppResponse,
  EnvironmentResponse,
  UpdateEnvironmentRequest,
} from '@/api/generated/types.gen'
import { saveEnvironment } from '@/api/setup'
import { ErrorNotice, formatDate } from './feedback'
import { parseOrigins } from './app-form'

export function EnvironmentCard({ app }: { app: AppResponse }) {
  const [editing, { open: openEditor, close: closeEditor }] = useDisclosure(false)
  return (
    <>
      <Card>
        <Group justify="space-between" mb="lg">
          <Group gap="xs">
            <Globe2 size={16} />
            <Title order={2} size="h5">
              Environment
            </Title>
          </Group>
          <Button
            size="xs"
            variant="subtle"
            onClick={openEditor}
            leftSection={<Pencil size={12} />}
          >
            Edit
          </Button>
        </Group>
        <Text size="sm" fw={500}>
          {app.environment.name}
        </Text>
        <Text size="xs" c="dimmed" mt="xs" className="identifier">
          {app.environment.backend_origins.join(', ') || 'No backend origins configured'}
        </Text>
        <Divider my="md" />
        <Group gap="xs">
          <ShieldCheck size={14} />
          <Text size="xs" fw={500}>
            Access references
          </Text>
        </Group>
        <Text size="xs" c="dimmed" mt="xs">
          Account: {app.readiness.account_configured ? 'Configured' : 'Not configured'} · Reset:{' '}
          {app.readiness.reset_configured ? 'Configured' : 'Not configured'}
        </Text>
        <Text size="xs" c="dimmed" mt="xs">
          A reference stores access configuration. It does not prove that sign-in or reset works.
        </Text>
        {app.environment.checks.map((check) => (
          <Stack key={check.kind} gap={4} mt="md">
            <Divider mb="xs" />
            <Text size="xs" tt="capitalize">
              {check.kind}: {check.state.replaceAll('_', ' ')}
            </Text>
            {check.checked_at && (
              <Text size="xs" c="dimmed">
                Recorded {formatDate(check.checked_at)} by operator {check.checked_by?.slice(0, 8)}
              </Text>
            )}
            {check.note && (
              <Text size="xs" c="dimmed">
                {check.note}
              </Text>
            )}
          </Stack>
        ))}
      </Card>
      <Drawer opened={editing} onClose={closeEditor} title="Edit environment">
        <Text size="sm" c="dimmed" mb="xl">
          Changes create a new revision and invalidate earlier observations.
        </Text>
        <EnvironmentForm
          key={app.environment.revision}
          appId={app.id}
          environment={app.environment}
          close={closeEditor}
        />
      </Drawer>
    </>
  )
}

function EnvironmentForm({
  appId,
  environment,
  close,
}: {
  appId: string
  environment: EnvironmentResponse
  close: () => void
}) {
  const client = useQueryClient()
  // The parent keys this form by environment revision, so saved revisions reset drafts.
  const form = useForm({
    initialValues: {
      name: environment.name,
      backend: environment.backend_origins.join('\n'),
      login: environment.login_origins.join('\n'),
      account: environment.account_secret_reference_id ?? 'none',
      reset: environment.reset_secret_reference_id ?? 'none',
    },
    transformValues: (values) => ({
      expected_revision: environment.revision,
      name: values.name.trim(),
      backend_origins: parseOrigins(values.backend),
      login_origins: parseOrigins(values.login),
      account_secret_reference_id: values.account === 'none' ? null : values.account,
      reset_secret_reference_id: values.reset === 'none' ? null : values.reset,
    }),
  })
  const mutation = useMutation({
    mutationFn: (body: UpdateEnvironmentRequest) => saveEnvironment(appId, body),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ['app', appId] })
      void client.invalidateQueries({ queryKey: ['build', appId] })
      void client.invalidateQueries({ queryKey: ['builds', appId] })
      close()
    },
  })

  return (
    <form onSubmit={form.onSubmit((values) => mutation.mutate(values))}>
      <Stack gap="lg">
        <TextInput
          label="Environment name"
          {...form.getInputProps('name')}
          required
          withAsterisk={false}
          data-autofocus
        />
        <Textarea
          label="Backend origins"
          required
          withAsterisk={false}
          {...form.getInputProps('backend')}
        />
        <Textarea label="Login origins" {...form.getInputProps('login')} />
        <Text size="xs" c="dimmed">
          At least one backend origin is required. One origin per line, without paths or
          credentials.
        </Text>
        {(
          [
            { kind: 'account', label: 'Account reference' },
            { kind: 'reset', label: 'Reset reference' },
          ] as const
        ).map((field) => (
          <NativeSelect
            key={field.kind}
            label={field.label}
            {...form.getInputProps(field.kind)}
            data={[
              { value: 'none', label: 'Not configured' },
              ...environment.secret_references
                .filter((ref) => ref.kind === field.kind)
                .map((ref) => ({ value: ref.id, label: ref.label })),
            ]}
          />
        ))}
        <Text size="xs" c="dimmed">
          Your operator manages private references. Only their masked labels are shown here.
        </Text>
        {mutation.isError && (
          <ErrorNotice
            focus
            error={mutation.error}
            retry={() => {
              void client.invalidateQueries({ queryKey: ['app', appId] })
            }}
            title="Environment was not saved"
          />
        )}
        <Divider />
        <Group justify="flex-end">
          <Button variant="subtle" onClick={close}>
            Cancel
          </Button>
          <Button type="submit" disabled={mutation.isPending}>
            {mutation.isPending ? 'Saving…' : 'Save environment'}
          </Button>
        </Group>
      </Stack>
    </form>
  )
}
