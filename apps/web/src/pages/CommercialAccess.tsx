import {
  Alert,
  Badge,
  Box,
  Button,
  Card,
  Divider,
  Grid,
  Group,
  Progress,
  Select,
  Stack,
  Table,
  Text,
  Title,
} from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowRight, Check, CreditCard, RefreshCw, Sparkles } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'
import { changeCreditPlan, commercialAccessQuery, createCreditCheckout } from '@/api/commercial'
import type { CreditPlan, CreditPlanView } from '@/api/generated/types.gen'
import { appQuery, appsQuery } from '@/api/setup'
import { ErrorNotice, LoadingPanel, PageHeading } from '@/components/app/feedback'
import { dollars } from '@/components/commercial/RunAuthorization'
import { useWorkspace } from '@/hooks/use-workspace'

const number = (value: number) => new Intl.NumberFormat('en-US').format(value)
const planNames: Record<CreditPlan, string> = {
  starter: 'Starter',
  plus: 'Plus',
  business: 'Business',
}
const planDescriptions: Record<CreditPlan, string> = {
  starter: 'For the first release checks your team can trust.',
  plus: 'More room for every release cycle.',
  business: 'A larger allowance for a busy mobile team.',
}

function PlanCard({
  plan,
  current,
  scheduled,
  pending,
  onChoose,
}: {
  plan: CreditPlanView
  current: CreditPlan | null
  scheduled: CreditPlan | null
  pending: boolean
  onChoose: (plan: CreditPlan) => void
}) {
  const recommended = plan.plan === 'plus'
  const active = current === plan.plan
  const queued = scheduled === plan.plan
  return (
    <Card
      p="xl"
      radius="lg"
      style={{
        background: recommended ? 'linear-gradient(160deg,#252548,#191a2a 62%)' : '#191a21',
        border: recommended ? '1px solid #7779fb' : '1px solid #32343c',
        boxShadow: recommended ? '0 16px 44px rgba(79,72,229,.2)' : undefined,
        color: '#f9fafb',
        height: '100%',
      }}
    >
      <Stack h="100%" gap="lg">
        <Stack gap={8}>
          <Group justify="space-between">
            <Title order={3} fz={22} c="white">
              {planNames[plan.plan]}
            </Title>
            {recommended && (
              <Badge color="indigo" variant="filled">
                Recommended
              </Badge>
            )}
            {active && (
              <Badge color="teal" variant="filled">
                Current
              </Badge>
            )}
            {queued && <Badge color="violet">Next renewal</Badge>}
          </Group>
          <Text size="sm" c="#b6b7c2" mih={42}>
            {planDescriptions[plan.plan]}
          </Text>
        </Stack>
        <Divider color="#363841" />
        <Stack gap="sm" style={{ flex: 1 }}>
          {[
            `${number(plan.monthly_credits)} credits each month`,
            'One Android app',
            'Saved suites and evidence reports',
            'Measured usage for every run',
          ].map((feature) => (
            <Group key={feature} gap="sm" wrap="nowrap" align="flex-start">
              <Check size={17} color="#8587ff" style={{ flexShrink: 0, marginTop: 2 }} />
              <Text size="sm" c="#ededf2">
                {feature}
              </Text>
            </Group>
          ))}
        </Stack>
        <Divider color="#363841" />
        <Stack gap="md">
          <Group align="baseline" gap={5}>
            <Text fz={36} fw={700} c="white" lh={1}>
              {dollars(plan.monthly_cents).replace('.00', '')}
            </Text>
            <Text size="sm" c="#b6b7c2">
              / month
            </Text>
          </Group>
          <Button
            fullWidth
            size="md"
            color={recommended ? 'indigo' : 'gray'}
            variant={recommended ? 'filled' : 'light'}
            loading={pending}
            disabled={(active && !scheduled) || (!!scheduled && !active)}
            rightSection={!active && !queued && <ArrowRight size={17} />}
            onClick={() => onChoose(plan.plan)}
          >
            {queued
              ? `${planNames[plan.plan]} scheduled`
              : active
                ? scheduled
                  ? `Keep ${planNames[plan.plan]}`
                  : 'Your current plan'
                : current
                  ? `Schedule ${planNames[plan.plan]} next renewal`
                  : `Choose ${planNames[plan.plan]}`}
          </Button>
        </Stack>
      </Stack>
    </Card>
  )
}

export function CommercialAccess() {
  const queryClient = useQueryClient()
  const { workspaceId = '', href } = useWorkspace()
  const [params, setParams] = useSearchParams()
  const apps = useQuery(appsQuery(workspaceId))
  const appId = params.get('app') ?? apps.data?.items[0]?.id ?? ''
  const selectedApp = useQuery({ ...appQuery(appId), enabled: !!appId })
  const inWorkspace = selectedApp.data?.organization_id === workspaceId
  const access = useQuery({ ...commercialAccessQuery(workspaceId, appId), enabled: inWorkspace })
  const checkout = useMutation({
    mutationFn: (plan: CreditPlan) => createCreditCheckout(appId, { plan }),
    onSuccess: ({ url }) => window.location.assign(url),
  })
  const planChange = useMutation({
    mutationFn: (plan: CreditPlan) => changeCreditPlan(appId, { plan }),
    onSuccess: (updated) =>
      queryClient.setQueryData(commercialAccessQuery(workspaceId, appId).queryKey, updated),
  })
  const appChoices = apps.data?.items.map((app) => ({ value: app.id, label: app.name })) ?? []
  if (inWorkspace && selectedApp.data && !appChoices.some((app) => app.value === appId))
    appChoices.push({ value: appId, label: selectedApp.data.name })
  const credit = access.data?.credit
  const agreement = access.data?.agreement
  const checkoutReturn = params.get('checkout') === 'return'

  return (
    <Stack gap="xl">
      <PageHeading
        eyebrow="Plans & usage"
        title="Choose how much room your team needs."
        description="One monthly allowance. Each run draws credits from measured model tokens, device time, and stored evidence."
      />
      {apps.isPending && <LoadingPanel label="Loading apps…" />}
      {apps.isError && <ErrorNotice error={apps.error} retry={() => void apps.refetch()} />}
      {apps.data && !apps.data.items.length && !appId && (
        <Alert>Add an app to choose a plan.</Alert>
      )}
      {!!appChoices.length && (
        <Select
          label="App"
          value={appId}
          data={appChoices}
          maw={360}
          onChange={(value) => {
            if (value) {
              const next = new URLSearchParams(params)
              next.set('app', value)
              next.delete('checkout')
              setParams(next)
              checkout.reset()
              planChange.reset()
            }
          }}
        />
      )}
      {appId && selectedApp.isPending && <LoadingPanel label="Checking app workspace…" />}
      {selectedApp.isError && (
        <ErrorNotice error={selectedApp.error} retry={() => void selectedApp.refetch()} />
      )}
      {selectedApp.data && !inWorkspace && (
        <Alert color="orange">Choose an app in this workspace to view its plans.</Alert>
      )}
      {inWorkspace && access.isPending && <LoadingPanel label="Loading plans…" />}
      {access.isError && <ErrorNotice error={access.error} retry={() => void access.refetch()} />}
      {access.data && (
        <>
          {checkoutReturn && !credit && (
            <Alert color="indigo" title="Confirming payment">
              Your allowance appears after the payment provider confirms the invoice.{' '}
              <Button
                variant="subtle"
                size="compact-sm"
                leftSection={<RefreshCw size={14} />}
                onClick={() => void access.refetch()}
              >
                Refresh usage
              </Button>
            </Alert>
          )}
          {checkout.isError && <ErrorNotice error={checkout.error} />}
          {planChange.isError && <ErrorNotice error={planChange.error} />}
          <Grid gap="lg" align="stretch">
            <Grid.Col span={{ base: 12, lg: 4 }}>
              <Card p="xl" radius="lg" h="100%">
                <Stack gap="lg">
                  <Group justify="space-between">
                    <Text size="xs" fw={700} tt="uppercase" lts=".12em" c="dimmed">
                      Your usage
                    </Text>
                    <Badge color={credit ? 'green' : 'gray'} variant="light">
                      {credit ? 'Active' : 'No allowance'}
                    </Badge>
                  </Group>
                  <Title order={2} size="h3">
                    {selectedApp.data?.name ?? 'App'}
                  </Title>
                  {credit ? (
                    <>
                      <Stack gap={5}>
                        <Group align="baseline" gap="xs">
                          <Text fz={36} fw={700} lh={1}>
                            {number(credit.available_credits)}
                          </Text>
                          <Text c="dimmed">credits available</Text>
                        </Group>
                        <Progress
                          value={(credit.available_credits / credit.granted_credits) * 100}
                          color="indigo"
                          size="lg"
                          radius="xl"
                          aria-label={`${credit.available_credits} of ${credit.granted_credits} credits available`}
                        />
                        <Text size="sm" c="dimmed">
                          {number(credit.granted_credits)} granted · {number(credit.held_credits)}{' '}
                          held · {number(credit.charged_credits)} used
                        </Text>
                      </Stack>
                      <Divider />
                      <Text size="sm">
                        <strong>{planNames[credit.plan]}</strong> · resets{' '}
                        {new Date(credit.period_end).toLocaleDateString()}
                      </Text>
                      {credit.pending_plan && credit.pending_effective_at && (
                        <Alert color="indigo" title="Plan change scheduled">
                          {planNames[credit.pending_plan]} starts at your next paid renewal on{' '}
                          {new Date(credit.pending_effective_at).toLocaleDateString()}. Your current
                          allowance stays available until then. Select Keep {planNames[credit.plan]}{' '}
                          below to cancel the change.
                        </Alert>
                      )}
                      <Text size="xs" c="dimmed">
                        Unused credits expire at the end of this paid period. New runs pause when
                        the available balance cannot cover their maximum authorization.
                      </Text>
                      <Button
                        component={Link}
                        to={href(`/tests?app=${appId}&kind=suite`)}
                        rightSection={<ArrowRight size={16} />}
                      >
                        Start a release check
                      </Button>
                    </>
                  ) : (
                    <>
                      <Text fz={36} fw={700} lh={1}>
                        0
                      </Text>
                      <Text c="dimmed" size="sm">
                        Choose a plan to start a monthly allowance. Credits appear only after
                        payment is confirmed.
                      </Text>
                    </>
                  )}
                  {agreement && (
                    <Alert color="blue" title="Existing check agreement">
                      Your earlier {agreement.offer} agreement remains visible to the operator
                      through {new Date(agreement.ends_at).toLocaleDateString()}.
                    </Alert>
                  )}
                </Stack>
              </Card>
            </Grid.Col>
            <Grid.Col span={{ base: 12, lg: 8 }}>
              <Box
                p={{ base: 'xl', sm: 38 }}
                h="100%"
                style={{
                  borderRadius: 18,
                  background:
                    'radial-gradient(circle at 85% 0%,#353662 0%,#15151e 42%,#111117 100%)',
                  color: 'white',
                }}
              >
                <Stack gap="lg">
                  <Badge color="indigo" variant="outline" w="fit-content">
                    Simple monthly pricing
                  </Badge>
                  <Title order={2} fz={{ base: 30, sm: 40 }} c="white" lh={1.1} maw={620}>
                    Release with evidence. Pay for the work each run uses.
                  </Title>
                  <Text c="#c1c2ce" maw={570}>
                    Pick your allowance in one click. The run screen shows a maximum credit hold
                    before you authorize work, then records the measured result.
                  </Text>
                  <Group gap="lg">
                    <Group gap="xs">
                      <Sparkles size={18} color="#9193ff" />
                      <Text size="sm" c="#dedfeb">
                        No per-check fee
                      </Text>
                    </Group>
                    <Group gap="xs">
                      <CreditCard size={18} color="#9193ff" />
                      <Text size="sm" c="#dedfeb">
                        Secure hosted checkout
                      </Text>
                    </Group>
                  </Group>
                </Stack>
              </Box>
            </Grid.Col>
          </Grid>
          <Box>
            <Stack gap="md">
              <Group justify="space-between" align="end">
                <Box>
                  <Text size="xs" fw={700} tt="uppercase" lts=".12em" c="dimmed">
                    Plans and pricing
                  </Text>
                  <Title order={2}>Find your monthly fit.</Title>
                </Box>
                <Text size="sm" c="dimmed">
                  USD · credits reset monthly · no automatic overage
                </Text>
              </Group>
              {credit && !credit.pending_plan && (
                <Text size="sm" c="dimmed">
                  Plan changes start at your next paid renewal. Your current credits and price stay
                  in place until then.
                </Text>
              )}
              <Grid gap="lg" align="stretch">
                {access.data.plans.map((plan) => (
                  <Grid.Col span={{ base: 12, sm: 6, lg: 4 }} key={plan.plan}>
                    <PlanCard
                      plan={plan}
                      current={credit?.plan ?? null}
                      scheduled={credit?.pending_plan ?? null}
                      pending={
                        (checkout.isPending && checkout.variables === plan.plan) ||
                        (planChange.isPending && planChange.variables === plan.plan)
                      }
                      onChoose={(choice) =>
                        credit ? planChange.mutate(choice) : checkout.mutate(choice)
                      }
                    />
                  </Grid.Col>
                ))}
              </Grid>
            </Stack>
          </Box>
          <Card p="xl" radius="lg">
            <Stack gap="sm">
              <Title order={3}>How credits are used</Title>
              <Text size="sm" c="dimmed">
                We measure provider input and output tokens, device occupancy time, and sealed
                evidence storage. Current rate card, revision 1: 200 credits per million input
                tokens, 800 per million output tokens, 60 per device minute, and 100 per GiB of
                stored evidence.
              </Text>
              <Text size="sm" c="dimmed">
                A run reserves a maximum before it starts. After review, only measured usage is
                charged; unused held credits return to your available balance. If a usable report
                cannot be delivered, the operator releases the hold.
              </Text>
            </Stack>
          </Card>
          {credit && (
            <Card p="xl" radius="lg">
              <Stack gap="sm">
                <Title order={3}>Credit activity</Title>
                {credit.usage.length ? (
                  <Table.ScrollContainer minWidth={760}>
                    <Table striped>
                      <Table.Thead>
                        <Table.Tr>
                          <Table.Th>Run</Table.Th>
                          <Table.Th>Status</Table.Th>
                          <Table.Th>Authorized</Table.Th>
                          <Table.Th>Used</Table.Th>
                          <Table.Th>Measured from</Table.Th>
                        </Table.Tr>
                      </Table.Thead>
                      <Table.Tbody>
                        {credit.usage.map((item) => (
                          <Table.Tr key={item.run_id}>
                            <Table.Td>
                              <Button
                                component={Link}
                                to={href(`/runs/${item.run_id}`)}
                                variant="subtle"
                                size="compact-sm"
                              >
                                {item.run_id.slice(0, 8)}
                              </Button>
                            </Table.Td>
                            <Table.Td>{item.state}</Table.Td>
                            <Table.Td>{number(item.held_credits)}</Table.Td>
                            <Table.Td>
                              {item.state === 'held'
                                ? 'Pending review'
                                : number(item.charged_credits)}
                            </Table.Td>
                            <Table.Td>
                              {item.state === 'settled' ? (
                                <Text size="xs">
                                  {item.input_tokens ?? '0'} input · {item.output_tokens ?? '0'}{' '}
                                  output tokens
                                  <br />
                                  {item.device_seconds ?? 0}s device · {item.stored_bytes ?? '0'}{' '}
                                  evidence bytes
                                </Text>
                              ) : (
                                '—'
                              )}
                            </Table.Td>
                          </Table.Tr>
                        ))}
                      </Table.Tbody>
                    </Table>
                  </Table.ScrollContainer>
                ) : (
                  <Text c="dimmed">No runs have used this allowance yet.</Text>
                )}
              </Stack>
            </Card>
          )}
        </>
      )}
    </Stack>
  )
}
