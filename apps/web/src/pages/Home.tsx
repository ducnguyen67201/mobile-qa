// Retained foundation health view; "Ready" means only that the API answered.
import { Button, Card, Group, Text } from '@mantine/core'
import { useQuery } from '@tanstack/react-query'
import { healthQuery } from '../api/queries'
import { PageHeading } from '@/components/app/feedback'

export function Home() {
  const health = useQuery(healthQuery)
  return (
    <>
      <PageHeading
        eyebrow="Workspace"
        title="API connection"
        description="Service connectivity for the mobile QA workspace."
      />
      <Card maw={768}>
        <Group justify="space-between">
          <Text role="status" aria-live="polite" size="sm">
            API: {health.isFetching ? 'Connecting…' : health.isError ? 'Cannot connect' : 'Ready'}
          </Text>
          {health.isError && (
            <Button onClick={() => void health.refetch()} disabled={health.isFetching}>
              Retry
            </Button>
          )}
          {health.isSuccess && !health.isFetching && (
            <Text size="xs" c="dimmed">
              {health.data.service} · v{health.data.version}
            </Text>
          )}
        </Group>
        {health.isError && (
          <Text size="sm" c="dimmed" mt="md">
            The workspace service is unavailable. Try connecting again.
          </Text>
        )}
      </Card>
    </>
  )
}
