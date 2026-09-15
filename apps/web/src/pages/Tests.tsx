import { useState } from 'react'
import {
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Drawer,
  Group,
  Select,
  Stack,
  Tabs,
  Text,
  Title,
} from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { ArrowRight, FileCheck2, Layers, Plus, Route } from 'lucide-react'
import { appsQuery } from '@/api/setup'
import {
  createLibraryEntry,
  defaultPlanQuery,
  libraryKey,
  libraryOptionsQuery,
  libraryQuery,
} from '@/api/test-library'
import type {
  CreateLibraryEntryRequest,
  DefinitionKind,
  LibraryReviewState,
} from '@/api/generated/types.gen'
import { useWorkspace } from '@/hooks/use-workspace'
import { useMounted } from '@/hooks/use-mounted'
import { ErrorNotice, LoadingPanel, PageHeading } from '@/components/app/feedback'
import { TemplatePicker } from '@/components/test-library/template-picker'
import { ReviewBadge } from '@/components/test-library/library-presentation'
const kinds: Record<DefinitionKind, { label: string; single: string; description: string }> = {
  case: {
    label: 'Cases',
    single: 'case',
    description:
      'Describe what a person does and what should happen. Save a draft before you have a build.',
  },
  suite: {
    label: 'Suites',
    single: 'suite',
    description:
      'Group reviewed case versions into reusable coverage. Each member stays pinned until you change it.',
  },
  plan: {
    label: 'Release plan',
    single: 'release plan',
    description:
      'Choose reviewed coverage and an execution profile. Set one approved version as the default for this app.',
  },
}
export function Tests() {
  const { workspaceId = '' } = useWorkspace()
  return <WorkspaceTests key={workspaceId} />
}
function WorkspaceTests() {
  const { workspaceId = '', href } = useWorkspace()
  const [params, setParams] = useSearchParams()
  const apps = useQuery(appsQuery(workspaceId))
  const appId = params.get('app') ?? apps.data?.items[0]?.id ?? ''
  const selectedTab = params.get('kind')
  const kind: DefinitionKind =
    selectedTab === 'suite' || selectedTab === 'plan' ? selectedTab : 'case'
  const [status, setStatus] = useState<LibraryReviewState | null>(null)
  const [archived, setArchived] = useState(false)
  const [cursor, setCursor] = useState<string | undefined>()
  const [creating, setCreating] = useState(false)
  const [templates, setTemplates] = useState(false)
  const updateParams = (field: string, value: string) => {
    const next = new URLSearchParams(params)
    next.set(field, value)
    setParams(next)
    setCursor(undefined)
  }
  const library = useQuery(
    libraryQuery(workspaceId, appId, { kind, status: status ?? undefined, archived, cursor }),
  )
  const options = useQuery(libraryOptionsQuery(workspaceId, appId))
  const defaultPlan = useQuery(defaultPlanQuery(workspaceId, appId))
  return (
    <Stack gap="lg">
      <PageHeading
        eyebrow="Build confidence, one behavior at a time"
        title="Tests"
        description="Write the expectation. Review the version. Run it on your next build."
      />
      {apps.isError && <ErrorNotice error={apps.error} retry={() => void apps.refetch()} />}
      {apps.isPending ? (
        <LoadingPanel label="Loading your apps…" />
      ) : !apps.data?.items.length ? (
        <Card>
          <Stack>
            <Title order={2} size="h3">
              A home for your test coverage
            </Title>
            <Text size="sm" c="dimmed">
              Create an app to start writing cases. You can add your first build later.
            </Text>
            <Button component={Link} to={href('/apps')} w="fit-content">
              Go to apps
            </Button>
          </Stack>
        </Card>
      ) : (
        <>
          <Select
            label="App"
            maw={440}
            data={apps.data.items.map((app) => ({ value: app.id, label: app.name }))}
            value={appId}
            allowDeselect={false}
            onChange={(id) => {
              if (id) updateParams('app', id)
            }}
          />
          <Card withBorder>
            <Stack gap="sm">
              <Title order={2} size="h3">
                How would you like to create a test?
              </Title>
              <Text size="sm" c="dimmed">
                Record direct steps, start from a template, or let AI suggest scenarios.
              </Text>
              <Group>
                <Button component={Link} to={href(`/apps/${appId}/try?generate=1`)}>
                  Generate with AI
                </Button>
                <Button variant="light" onClick={() => setTemplates(true)}>
                  Use a template
                </Button>
                <Button
                  variant="default"
                  onClick={() => {
                    updateParams('kind', 'case')
                    setCreating(true)
                  }}
                >
                  Create manually
                </Button>
              </Group>
            </Stack>
          </Card>
          <TemplatePicker
            key={appId}
            appId={appId}
            opened={templates}
            close={() => setTemplates(false)}
          />
          <Tabs
            value={kind}
            onChange={(value) => {
              if (value) updateParams('kind', value)
            }}
          >
            <Tabs.List>
              <Tabs.Tab value="case" leftSection={<FileCheck2 size={16} />}>
                Cases
              </Tabs.Tab>
              <Tabs.Tab value="suite" leftSection={<Layers size={16} />}>
                Suites
              </Tabs.Tab>
              <Tabs.Tab value="plan" leftSection={<Route size={16} />}>
                Release plan
              </Tabs.Tab>
            </Tabs.List>
          </Tabs>
          <Group justify="space-between" align="flex-start">
            <Stack gap={4} maw={650}>
              <Title order={2}>{kinds[kind].label}</Title>
              <Text size="sm" c="dimmed">
                {kinds[kind].description}
              </Text>
            </Stack>
            <Button
              leftSection={<Plus size={16} />}
              disabled={!options.data?.capabilities.can_edit}
              onClick={() => setCreating(true)}
            >
              New {kinds[kind].single}
            </Button>
          </Group>
          {options.isError && (
            <ErrorNotice error={options.error} retry={() => void options.refetch()} />
          )}
          {kind === 'plan' && defaultPlan.isError && (
            <ErrorNotice error={defaultPlan.error} retry={() => void defaultPlan.refetch()} />
          )}
          {kind === 'plan' && defaultPlan.data && (
            <Alert
              color={defaultPlan.data.plan_version_id ? 'forest' : 'gray'}
              title={
                defaultPlan.data.plan_version_id
                  ? 'An explicit default is selected'
                  : 'Choose your default release check'
              }
            >
              {defaultPlan.data.plan_version_id
                ? 'Approving another version will not change the default. Open an approved plan to change it explicitly.'
                : 'Create and review a release plan, then set it as the default. Each build can also preview a specific reviewed plan.'}
            </Alert>
          )}
          <Group align="flex-end">
            <Select
              label="Review status"
              clearable
              placeholder="All review states"
              value={status}
              data={[
                { value: 'in_review', label: 'In review' },
                { value: 'needs_input', label: 'Changes requested' },
                { value: 'rejected', label: 'Rejected' },
                { value: 'approved', label: 'Approved' },
              ]}
              onChange={(v) => {
                if (
                  v === 'in_review' ||
                  v === 'needs_input' ||
                  v === 'rejected' ||
                  v === 'approved' ||
                  v === null
                ) {
                  setStatus(v)
                  setCursor(undefined)
                }
              }}
            />
            <Checkbox
              label="Archived only"
              checked={archived}
              mb={8}
              onChange={(e) => {
                setArchived(e.currentTarget.checked)
                setCursor(undefined)
              }}
            />
          </Group>
          {library.isPending && <LoadingPanel label="Loading test library…" />}
          {library.isError && (
            <ErrorNotice error={library.error} retry={() => void library.refetch()} />
          )}
          {library.data && (
            <Stack gap="sm">
              {!library.data.items.length ? (
                <Card py="xl">
                  <Stack gap="sm">
                    <Title order={3}>
                      {status || archived
                        ? 'No tests match this view.'
                        : `Your first ${kinds[kind].single} starts here.`}
                    </Title>
                    <Text size="sm" c="dimmed">
                      {status || archived
                        ? 'Change the filters to see other test entries.'
                        : kind === 'case'
                          ? 'Start with one important behavior. Incomplete drafts stay editable until you request review.'
                          : kind === 'suite'
                            ? 'Approve a case first, then group the exact versions you want to reuse.'
                            : 'Choose approved cases or suites and a qualified profile. No build is needed to prepare coverage.'}
                    </Text>
                  </Stack>
                </Card>
              ) : (
                library.data.items.map((entry) => (
                  <Card
                    key={entry.id}
                    component={Link}
                    to={href(`/tests/${appId}/${entry.id}`)}
                    style={{ textDecoration: 'none', color: 'inherit' }}
                  >
                    <Group justify="space-between" wrap="nowrap">
                      <Stack gap={6} miw={0}>
                        <Group gap="xs">
                          <ReviewBadge state={entry.latest_review_state} />
                          {entry.ai_generated && (
                            <Badge color="grape" variant="light">
                              AI generated
                            </Badge>
                          )}
                          {entry.draft_version != null && (
                            <Badge color="gray">Draft v{entry.draft_version}</Badge>
                          )}
                          {entry.archived_at && <Badge color="orange">Archived</Badge>}
                          {entry.kind === 'plan' &&
                            entry.latest_version_id != null &&
                            entry.latest_version_id === defaultPlan.data?.plan_version_id && (
                              <Badge>Default</Badge>
                            )}
                        </Group>
                        <Text fw={600}>{entry.title || `Untitled ${kinds[kind].single}`}</Text>
                        <Text size="xs" c="dimmed">
                          {entry.key}
                        </Text>
                      </Stack>
                      <ArrowRight size={18} style={{ flexShrink: 0 }} />
                    </Group>
                  </Card>
                ))
              )}
              <Group justify="space-between">
                {cursor ? (
                  <Button variant="subtle" onClick={() => setCursor(undefined)}>
                    Back to first page
                  </Button>
                ) : (
                  <span />
                )}
                {library.data.next_cursor && (
                  <Button
                    variant="subtle"
                    onClick={() => setCursor(library.data.next_cursor ?? undefined)}
                  >
                    Next page
                  </Button>
                )}
              </Group>
            </Stack>
          )}
          <CreateEntry
            key={`${appId}:${kind}:${creating}`}
            appId={appId}
            kind={kind}
            opened={creating}
            close={() => setCreating(false)}
          />
        </>
      )}
    </Stack>
  )
}
function CreateEntry({
  appId,
  kind,
  opened,
  close,
}: {
  appId: string
  kind: DefinitionKind
  opened: boolean
  close: () => void
}) {
  const { workspaceId = '', href } = useWorkspace()
  const navigate = useNavigate()
  const client = useQueryClient()
  const mounted = useMounted()
  const [entryId] = useState(() => crypto.randomUUID())
  const create = useMutation({
    mutationFn: (body: CreateLibraryEntryRequest) => createLibraryEntry(appId, body),
    onSuccess: (draft) => {
      if (!mounted.current) return
      void client.invalidateQueries({ queryKey: libraryKey(workspaceId, appId) })
      close()
      void navigate(href(`/tests/${appId}/${draft.entry.id}`))
    },
  })
  return (
    <Drawer opened={opened} onClose={close} title={`New ${kinds[kind].single}`}>
      <Stack>
        <Text size="sm" c="dimmed">
          Start a draft, then give it a title and describe its content. You can save your progress
          before it is ready for review.
        </Text>
        {create.isError && (
          <ErrorNotice
            error={create.error}
            retry={create.variables ? () => create.mutate(create.variables!) : undefined}
          />
        )}
        <Button
          loading={create.isPending}
          onClick={() =>
            // Preserve the request identity when retrying an uncertain response.
            create.mutate(
              create.variables ?? {
                mutation_id: crypto.randomUUID(),
                entry_id: entryId,
                kind,
                key: `${kind}-${entryId}`,
                template_profile_id: null,
              },
            )
          }
        >
          Create draft
        </Button>
        <Text size="xs" c="dimmed">
          Creating a draft does not publish or approve it.
        </Text>
      </Stack>
    </Drawer>
  )
}
