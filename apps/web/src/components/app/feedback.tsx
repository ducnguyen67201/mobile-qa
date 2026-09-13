import { useEffect, useRef, type ReactNode } from 'react'
import { Alert, Button, Group, Loader, Skeleton, Stack, Text, Title } from '@mantine/core'
import { AlertCircle, ArrowRight } from 'lucide-react'
import { ApiClientError } from '@/api/runtime'

type ErrorNoticeProps = { error: unknown; retry?: () => void; title?: string; focus?: boolean }
export function ErrorNotice({
  error,
  retry,
  title = 'We couldn’t complete that request',
  focus = false,
}: ErrorNoticeProps) {
  const alert = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (focus) alert.current?.focus()
  }, [error, focus])
  const message =
    error instanceof ApiClientError && error.body
      ? error.message
      : 'The service could not confirm this request. Check your connection and reload the saved status.'

  return (
    <Alert
      ref={alert}
      tabIndex={focus ? -1 : undefined}
      color="red"
      title={title}
      icon={<AlertCircle size={18} />}
    >
      <Stack gap="sm">
        <Text size="sm">{message}</Text>
        {retry && (
          <Button
            variant="outline"
            color="red"
            onClick={retry}
            rightSection={<ArrowRight size={14} />}
            w="fit-content"
          >
            Retry
          </Button>
        )}
      </Stack>
    </Alert>
  )
}

export function LoadingPanel({ label = 'Loading your workspace…' }: { label?: string }) {
  return (
    <Stack gap="lg" py="xl" role="status" aria-live="polite">
      <Group gap="xs">
        <Loader size="xs" />
        <Text size="sm" c="dimmed">
          {label}
        </Text>
      </Group>
      <Skeleton height={36} width="min(224px, 100%)" />
      <Skeleton height={192} radius="lg" />
    </Stack>
  )
}

type PageHeadingProps = {
  eyebrow: string
  title: string
  description: string
  action?: ReactNode
  identifier?: boolean
}
export function PageHeading({
  eyebrow,
  title,
  description,
  action,
  identifier = false,
}: PageHeadingProps) {
  return (
    <Group justify="space-between" align="flex-end" gap="lg" mb={36}>
      <Stack gap="sm" maw={680} miw={0}>
        <Text size="xs" fw={600} tt="uppercase" c="dimmed" lts=".15em">
          {eyebrow}
        </Text>
        <Title order={1} fz={{ base: 30, sm: 38 }}>
          {title}
        </Title>
        <Text
          size={identifier ? 'xs' : 'sm'}
          c="dimmed"
          className={identifier ? 'identifier' : undefined}
        >
          {description}
        </Text>
      </Stack>
      {action}
    </Group>
  )
}

export { formatBytes, formatDate, formatDuration } from '@/lib/format'
