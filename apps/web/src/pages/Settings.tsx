import { Badge, Box, Card, Divider, Group, SimpleGrid, Stack, Text, Title } from '@mantine/core'
import { useQuery } from '@tanstack/react-query'
import { Database, ShieldCheck, SlidersHorizontal } from 'lucide-react'
import { settingsQuery } from '@/api/setup'
import { useSession } from '@/components/app/session'
import {
  ErrorNotice,
  LoadingPanel,
  PageHeading,
  formatBytes,
  formatDate,
  formatDuration,
} from '@/components/app/feedback'

export function Settings() {
  const settings = useQuery(settingsQuery)
  const session = useSession()
  return (
    <>
      <PageHeading
        eyebrow="Workspace settings"
        title="The essentials."
        description="Your access, upload limits and storage policy. A small set of settings with clear boundaries."
      />
      {settings.isError && <ErrorNotice error={settings.error} retry={() => void settings.refetch()} />}
      {settings.isPending ? (
        <LoadingPanel label="Loading settings…" />
      ) : (
        settings.data && (
          <Stack gap="lg">
            <SimpleGrid cols={{ base: 1, lg: 2 }} spacing="lg">
              <Card>
                <Group gap="xs" mb="lg">
                  <ShieldCheck size={20} />
                  <Title order={2} size="h5">
                    Your access
                  </Title>
                </Group>
                <Text fw={500}>{session.user.display_name}</Text>
                <Text size="sm" c="dimmed" className="identifier">
                  {session.user.email}
                </Text>
                {settings.data.memberships.map((m) => (
                  <Box key={m.organization_id} mt="lg">
                    <Divider mb="md" />
                    <Group justify="space-between">
                      <Text size="sm">{m.name}</Text>
                      <Badge tt="capitalize">{m.role}</Badge>
                    </Group>
                  </Box>
                ))}
                <Text size="xs" c="dimmed" mt="lg">
                  Session expires {formatDate(session.expires_at)}.
                </Text>
              </Card>
              <Card>
                <Group gap="xs" mb="lg">
                  <SlidersHorizontal size={20} />
                  <Title order={2} size="h5">
                    Upload limits
                  </Title>
                </Group>
                <Stack component="dl" gap="md" m={0}>
                  {[
                    ['Maximum APK size', formatBytes(settings.data.max_apk_bytes)],
                    ['Active uploads', settings.data.max_active_uploads],
                    ['Upload expiry', formatDuration(settings.data.upload_ttl_seconds)],
                    ['Session lifetime', formatDuration(settings.data.session_ttl_seconds)],
                  ].map(([key, value]) => (
                    <Group key={key} justify="space-between" align="flex-start">
                      <Text component="dt" size="sm" c="dimmed">
                        {key}
                      </Text>
                      <Text component="dd" size="sm" fw={500} m={0}>
                        {value}
                      </Text>
                    </Group>
                  ))}
                </Stack>
              </Card>
            </SimpleGrid>
            <Card>
              <Group gap="xs" mb="lg">
                <Database size={20} />
                <Title order={2} size="h5">
                  Artifact storage
                </Title>
              </Group>
              <Text size="sm" fw={500}>
                {settings.data.storage}
              </Text>
              <Text size="sm" c="dimmed" mt="sm" maw={680}>
                {settings.data.accepted_build_retention}
              </Text>
              <Text size="xs" c="dimmed" mt="sm">
                APK files stay private. Upload expiry applies to unfinished uploads; it does not expire
                accepted builds.
              </Text>
            </Card>
          </Stack>
        )
      )}
    </>
  )
}
