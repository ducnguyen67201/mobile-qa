import { useForm } from '@mantine/form'
import { Button, Divider, Group, NativeSelect, Stack, Text, Textarea, TextInput } from '@mantine/core'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { ArrowRight } from 'lucide-react'
import { createApp } from '@/api/setup'
import { useSession } from './session'
import { ErrorNotice } from './feedback'

export function parseOrigins(value: string) {
  return value
    .split(/[\n,]/)
    .map((v) => v.trim())
    .filter(Boolean)
}
export function AppForm({ cancel }: { cancel: () => void }) {
  const session = useSession()
  const client = useQueryClient()
  const navigate = useNavigate()
  const form = useForm({
    initialValues: {
      organization: session.memberships[0]?.organization_id ?? '',
      name: '',
      packageName: '',
      environment: 'Staging',
      backend: '',
      login: '',
    },
    transformValues: (values) => ({
      organization_id: values.organization,
      name: values.name.trim(),
      android_package: values.packageName.trim(),
      environment_name: values.environment.trim(),
      backend_origins: parseOrigins(values.backend),
      login_origins: parseOrigins(values.login),
    }),
  })
  const mutation = useMutation({
    mutationFn: createApp,
    onSuccess: (app) => {
      void client.invalidateQueries({ queryKey: ['apps'] })
      void navigate(`/apps/${app.id}`)
    },
  })

  return (
    <form onSubmit={form.onSubmit((values) => mutation.mutate(values))}>
      <Stack gap="lg">
        <NativeSelect
          label="Organization"
          {...form.getInputProps('organization')}
          data={session.memberships.map((m) => ({ value: m.organization_id, label: m.name }))}
        />
        <TextInput
          label="App name"
          required
          withAsterisk={false}
          maxLength={100}
          {...form.getInputProps('name')}
          placeholder="Your Android app"
          data-autofocus
        />
        <TextInput
          label="Android package"
          required
          withAsterisk={false}
          {...form.getInputProps('packageName')}
          placeholder="com.company.app"
          pattern="[a-zA-Z][a-zA-Z0-9_]*(\.[a-zA-Z][a-zA-Z0-9_]*)+"
          description="Must match the package embedded in your APK."
        />
        <TextInput
          label="Environment name"
          required
          withAsterisk={false}
          {...form.getInputProps('environment')}
        />
        <Textarea
          label="Backend origins"
          required
          withAsterisk={false}
          {...form.getInputProps('backend')}
          placeholder="https://api.staging.example.com"
          aria-describedby="origins-help"
        />
        <Textarea
          label="Login origins (optional)"
          {...form.getInputProps('login')}
          placeholder="https://login.example.com"
          aria-describedby="origins-help"
        />
        <Text id="origins-help" size="xs" c="dimmed">
          Add at least one backend origin. One origin per line. Use scheme and host only, with no path,
          credentials, or tokens. These settings do not verify network access.
        </Text>
        {mutation.isError && <ErrorNotice focus error={mutation.error} />}
        <Divider />
        <Group justify="flex-end">
          <Button variant="subtle" onClick={cancel}>
            Cancel
          </Button>
          <Button
            type="submit"
            disabled={mutation.isPending || !form.values.organization}
            rightSection={<ArrowRight size={16} />}
          >
            {mutation.isPending ? 'Creating…' : 'Create app'}
          </Button>
        </Group>
      </Stack>
    </form>
  )
}
