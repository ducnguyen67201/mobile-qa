import { Alert, Anchor, Button, Card, Group, Stack, Text } from '@mantine/core'
import { Link } from 'react-router'
import type { CommercialQuoteResponse } from '@/api/generated/types.gen'
import { useWorkspace } from '@/hooks/use-workspace'

export const dollars = (cents: number) =>
  new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(cents / 100)

export function RunAuthorization({
  appId,
  quote,
  pending,
  onConfirm,
  onRefresh,
}: {
  appId: string
  quote: CommercialQuoteResponse
  pending: boolean
  onConfirm: () => void
  onRefresh: () => void
}) {
  const { href } = useWorkspace()
  const expired = new Date(quote.expires_at).getTime() <= Date.now()
  return (
    <Card withBorder role="group" aria-label="Check authorization">
      <Stack gap="xs">
        <Text fw={700}>Review this check</Text>
        <Text size="sm">
          {quote.case_count} cases · build {quote.build_id.slice(0, 8)} · coverage version{' '}
          {quote.source_version_id.slice(0, 8)} · device {quote.profile_id.slice(0, 8)}
        </Text>
        <Group justify="space-between">
          <Text>{quote.maximum_credits != null ? 'Maximum hold' : 'This check'}</Text>
          <Text fw={700}>
            {quote.maximum_credits != null
              ? `${new Intl.NumberFormat('en-US').format(quote.maximum_credits)} credits`
              : quote.amount_cents === 0
                ? '$0 included in pilot'
                : dollars(quote.amount_cents)}
          </Text>
        </Group>
        <Text size="sm" c="dimmed">
          {quote.maximum_credits != null
            ? `${new Intl.NumberFormat('en-US').format(quote.credits_after_authorization ?? 0)} credits remain after this hold. Measured usage is settled after report review.`
            : `Authorization ${quote.checks_after_authorization} of ${quote.check_cap} this period. One build uses one check.`}{' '}
          Review expires {new Date(quote.expires_at).toLocaleTimeString()}.
        </Text>
        {expired && (
          <Alert color="orange">This quote expired. Refresh the price to continue.</Alert>
        )}
        <Group>
          <Button onClick={onConfirm} loading={pending} disabled={expired}>
            {quote.maximum_credits != null
              ? 'Authorize credits and run'
              : 'Authorize check and run'}
          </Button>
          <Button variant="subtle" onClick={onRefresh} disabled={pending}>
            Refresh price
          </Button>
        </Group>
        <Anchor component={Link} to={href(`/settings/commercial?app=${appId}`)} size="sm">
          View pricing and usage
        </Anchor>
      </Stack>
    </Card>
  )
}
