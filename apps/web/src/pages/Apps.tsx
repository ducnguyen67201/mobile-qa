import { useState } from 'react'
import { useDisclosure } from '@mantine/hooks'
import {
  Badge,
  Button,
  Card,
  Divider,
  Drawer,
  Group,
  Paper,
  SimpleGrid,
  Stack,
  Text,
  ThemeIcon,
  Title,
} from '@mantine/core'
import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import { AppWindow, ArrowRight, Plus, Smartphone } from 'lucide-react'
import { appsQuery } from '@/api/setup'
import { PageHeading, LoadingPanel, ErrorNotice } from '@/components/app/feedback'
import { AppForm } from '@/components/app/app-form'

export function Apps() {
  const [creating, { open: openCreate, close: closeCreate }] = useDisclosure(false)
  const [cursors, setCursors] = useState<(string | undefined)[]>([undefined])
  const apps = useQuery(appsQuery(cursors.at(-1)))

  return (
    <>
      <PageHeading
        eyebrow="App library"
        title="Your apps."
        description="A clear starting point for every Android release. Manage your apps, upload a build, and see exactly where things stand."
        action={
          <Button onClick={openCreate} leftSection={<Plus size={16} />}>
            Create app
          </Button>
        }
      />
      {apps.isError && <ErrorNotice error={apps.error} retry={() => void apps.refetch()} />}
      {apps.isPending ? (
        <LoadingPanel label="Loading apps…" />
      ) : (
        apps.data && (
          <>
            {apps.data.items.length === 0 ? (
              <Paper component="section" withBorder radius="lg" px="lg" py={64} className="workbench-grid">
                <Stack align="center" gap="md" ta="center">
                  <ThemeIcon variant="light" size={76} radius="lg">
                    <Smartphone size={36} strokeWidth={1.5} />
                  </ThemeIcon>
                  <Text size="xs" c="dimmed" tt="uppercase" lts=".15em">
                    01 / Set the foundation
                  </Text>
                  <Title order={2} size="h3">
                    Your first app belongs here.
                  </Title>
                  <Text size="sm" c="dimmed" maw={380}>
                    Add an Android app and its environment. Then upload an APK to validate its identity and
                    compatibility.
                  </Text>
                  <Button mt="sm" onClick={openCreate} rightSection={<ArrowRight size={16} />}>
                    Create your first app
                  </Button>
                  <Text size="xs" c="dimmed">
                    No device connection needed for app setup.
                  </Text>
                </Stack>
              </Paper>
            ) : (
              <SimpleGrid cols={{ base: 1, md: 2, xl: 3 }} spacing="lg">
                {apps.data.items.map((app) => (
                  <Card key={app.id} component={Link} to={`/apps/${app.id}`} className="app-card">
                    <Group justify="space-between" mb="xl">
                      <ThemeIcon variant="light" size={48} radius="lg">
                        <AppWindow size={24} strokeWidth={1.5} />
                      </ThemeIcon>
                      <Badge>Android</Badge>
                    </Group>
                    <Title order={2} size="h4">
                      {app.name}
                    </Title>
                    <Text size="xs" c="dimmed" mt="xs" className="identifier">
                      {app.android_package}
                    </Text>
                    <Divider mt="xl" mb="md" />
                    <Group justify="space-between" wrap="nowrap">
                      <Text size="xs" c="dimmed" lineClamp={2}>
                        {app.environment_name}
                      </Text>
                      <ArrowRight size={16} />
                    </Group>
                  </Card>
                ))}
              </SimpleGrid>
            )}
            <Group justify="flex-end" mt="lg">
              {cursors.length > 1 && (
                <Button variant="outline" onClick={() => setCursors((c) => c.slice(0, -1))}>
                  Previous
                </Button>
              )}
              {apps.data.next_cursor && (
                <Button
                  variant="outline"
                  onClick={() => setCursors((c) => [...c, apps.data.next_cursor ?? undefined])}
                >
                  Next apps
                </Button>
              )}
            </Group>
          </>
        )
      )}
      <Drawer opened={creating} onClose={closeCreate} title="Create an app">
        <Text size="sm" c="dimmed" mb="xl">
          Start with its identity and test environment.
        </Text>
        <AppForm cancel={closeCreate} />
      </Drawer>
    </>
  )
}
