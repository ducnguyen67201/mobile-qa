import { actionLabel } from '@/lib/action-label'
import {
  Accordion,
  Alert,
  Badge,
  Button,
  Card,
  Group,
  List,
  Stack,
  Text,
  Title,
} from '@mantine/core'
import { useMutation, useQuery } from '@tanstack/react-query'
import { Link, useNavigate } from 'react-router'
import { useMounted } from '@/hooks/use-mounted'
import { createRun, planQuery } from '@/api/runs'
import { useWorkspace } from '@/hooks/use-workspace'
import { useSession } from './session'
import { ErrorNotice, LoadingPanel } from './feedback'

export function RunPreview({
  appId,
  buildId,
  readOnly = false,
  planVersionId,
}: {
  appId: string
  buildId: string
  readOnly?: boolean
  planVersionId?: string
}) {
  const { workspaceId = '', href } = useWorkspace()
  const mounted = useMounted()
  const session = useSession()
  const navigate = useNavigate()
  const query = useQuery(planQuery(workspaceId, appId, buildId, planVersionId))
  const start = useMutation({
    mutationFn: async () => {
      const manifest = query.data?.manifest
      if (!manifest || query.data?.blockers.length)
        throw new Error('Refresh the release check before running')
      const resolvedPlanVersionId =
        planVersionId ??
        (manifest.source?.kind === 'release_plan' ? manifest.source.plan_version_id : undefined) ??
        manifest.plan_version_id
      if (!resolvedPlanVersionId) throw new Error('Choose a saved release plan before running')
      const storageKey = `mobile-qa:run:${session.user.id}:${workspaceId}:${appId}:${buildId}:${resolvedPlanVersionId}:${manifest.environment_revision}`
      let key = sessionStorage.getItem(storageKey)
      if (!key) {
        key = crypto.randomUUID()
        sessionStorage.setItem(storageKey, key)
      }
      const run = await createRun(
        appId,
        {
          build_id: buildId,
          source: { kind: 'release_plan', plan_version_id: resolvedPlanVersionId },
          environment_revision: manifest.environment_revision,
          baseline_run_id: null,
        },
        key,
      )
      sessionStorage.removeItem(storageKey)
      return run
    },
    onSuccess: (run) => {
      if (mounted.current) void navigate(href(`/runs/${run.id}`))
    },
  })
  return (
    <Card withBorder>
      <Stack>
        <Group justify="space-between">
          <Title order={2}>Release check</Title>
          <Badge variant="light">Reviewed coverage</Badge>
        </Group>
        <Button
          component={Link}
          to={href(`/tests?app=${appId}&kind=plan`)}
          variant="subtle"
          w="fit-content"
        >
          Open test library & release plans
        </Button>
        {query.isPending && <LoadingPanel label="Loading saved tests…" />}
        {query.isError && <ErrorNotice error={query.error} retry={() => void query.refetch()} />}
        {query.data?.blockers.length ? (
          <Alert title="Before this build can run">
            <List>
              {query.data.blockers.map((b) => (
                <List.Item key={b}>{b}</List.Item>
              ))}
            </List>
          </Alert>
        ) : null}
        {query.data?.manifest && (
          <>
            <Text>
              {query.data.manifest.cases.length} cases · {query.data.manifest.profile.name}
            </Text>
            {query.data.manifest.profile.driver === 'fake' && (
              <Alert color="yellow">
                Simulated worker — this report will not represent a real device test.
              </Alert>
            )}
            <Accordion>
              {query.data.manifest.cases.map((c) => (
                <Accordion.Item key={c.definition_id} value={c.definition_id}>
                  <Accordion.Control>
                    {c.case.title} · v{c.case.version} · {c.required ? 'Required' : 'Optional'}
                  </Accordion.Control>
                  <Accordion.Panel>
                    <Stack gap="xs">
                      <Text>{c.case.requirement}</Text>
                      <Text fw={600}>Actions</Text>
                      <List type="ordered">
                        {c.case.actions.map((a) => (
                          <List.Item key={a.id}>
                            {actionLabel(a)} · {a.checkpoint_id}
                          </List.Item>
                        ))}
                      </List>
                      <Text fw={600}>Expected checks</Text>
                      <List>
                        {c.case.checks.map((check) => (
                          <List.Item key={check.id}>
                            {check.description} ·{' '}
                            {check.required ? 'Required' : 'Supporting observation'}
                          </List.Item>
                        ))}
                      </List>
                    </Stack>
                  </Accordion.Panel>
                </Accordion.Item>
              ))}
            </Accordion>
          </>
        )}
        {start.isError && <ErrorNotice error={start.error} retry={() => start.mutate()} />}
        {!readOnly && (
          <Button
            disabled={!query.data?.manifest || !!query.data.blockers.length || query.isError}
            loading={start.isPending}
            onClick={() => start.mutate()}
          >
            Run release check
          </Button>
        )}
      </Stack>
    </Card>
  )
}
