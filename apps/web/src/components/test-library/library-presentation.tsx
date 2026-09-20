import { actionLabel } from '@/lib/action-label'
import { Accordion, Alert, Badge, Card, Group, List, Stack, Text, Title } from '@mantine/core'
import { Link } from 'react-router'
import { useWorkspace } from '@/hooks/use-workspace'
import type {
  LibraryCoveragePreview,
  LibraryDraftDefinition,
  LibraryIssue,
  LibraryOptionsResponse,
} from '@/api/generated/types.gen'
export function LibraryIssues({
  issues,
  title = 'Saved content needs setup',
}: {
  issues: LibraryIssue[]
  title?: string
}) {
  if (!issues.length) return null
  return (
    <Alert color="yellow" title={title}>
      <List size="sm">
        {issues.map((issue, i) => (
          <List.Item key={`${issue.field}:${issue.item_id}:${i}`}>
            {issue.message}
            <Text span size="xs" c="dimmed">
              {' '}
              · {issue.field.replaceAll('_', ' ')}
            </Text>
          </List.Item>
        ))}
      </List>
    </Alert>
  )
}
export function Coverage({
  value,
  saved = true,
}: {
  value: LibraryCoveragePreview
  saved?: boolean
}) {
  return (
    <Card>
      <Stack gap="sm">
        <Group justify="space-between">
          <Title order={2} size="h3">
            Coverage at a glance
          </Title>
          <Badge variant="outline">{saved ? 'Saved version' : 'Last saved draft'}</Badge>
        </Group>
        <Text size="sm">
          {value.cases.length} unique cases · {value.required_count} required
        </Text>
        {!saved && (
          <Text size="sm" c="dimmed">
            Save your changes to refresh resolved coverage and readiness.
          </Text>
        )}
        {value.cases.map((c) => (
          <Group key={c.definition_id} justify="space-between">
            <Text size="sm">
              {c.case.title} · v{c.case.version}
            </Text>
            <Badge color={c.required ? 'forest' : 'gray'}>
              {c.required ? 'Required' : 'Optional'}
            </Badge>
          </Group>
        ))}
        {!!value.exclusions.length && (
          <>
            <Text size="sm" fw={600}>
              Outside this release check
            </Text>
            <List size="sm">
              {value.exclusions.map((item, i) => (
                <List.Item key={i}>{item}</List.Item>
              ))}
            </List>
          </>
        )}
        <LibraryIssues issues={value.issues} title="Coverage needs attention" />
        <Text size="xs" c="dimmed">
          Duration and model cost: not yet measured.
        </Text>
      </Stack>
    </Card>
  )
}
export function DefinitionSummary({
  definition,
  options,
}: {
  definition: LibraryDraftDefinition
  options?: LibraryOptionsResponse
}) {
  const { href } = useWorkspace()
  const content = definition.content
  const reference = (id: string) => {
    const version = options?.saved_versions.find((v) => v.version.id === id)
    return version ? (
      <Text
        component={Link}
        to={href(`/tests/${version.entry.app_id}/${version.entry.id}/versions/${id}`)}
        size="sm"
      >
        {version.version.definition.content.title} · v{version.version.definition.content.version}
      </Text>
    ) : (
      <Text size="sm">Version {id}</Text>
    )
  }
  return (
    <Stack>
      {definition.kind === 'case' ? (
        <>
          <Card>
            <Stack gap="sm">
              <Title order={2} size="h3">
                Behavior protected
              </Title>
              <Text>{definition.content.requirement}</Text>
              {!!definition.content.preconditions.length && (
                <>
                  <Text size="sm" fw={600}>
                    Before the test
                  </Text>
                  <List size="sm">
                    {definition.content.preconditions.map((p, i) => (
                      <List.Item key={i}>{p}</List.Item>
                    ))}
                  </List>
                </>
              )}
            </Stack>
          </Card>
          <Card>
            <Stack>
              <Title order={2} size="h3">
                Actions & expected results
              </Title>
              {definition.content.actions.map((action, i) => (
                <Stack key={action.id} gap="xs">
                  <Group gap="sm">
                    <Badge circle>{i + 1}</Badge>
                    <Text fw={500}>{actionLabel(action)}</Text>
                  </Group>
                  {definition.content.checks
                    .filter((check) => check.checkpoint_id === action.checkpoint_id)
                    .map((check) => (
                      <Text key={check.id} size="sm" pl="xl">
                        Expected: {check.description} ·{' '}
                        {check.required ? 'Required' : 'Supporting observation'}
                      </Text>
                    ))}
                </Stack>
              ))}
            </Stack>
          </Card>
          <Accordion variant="separated">
            <Accordion.Item value="technical">
              <Accordion.Control>Technical verification details</Accordion.Control>
              <Accordion.Panel>
                <Stack gap="sm">
                  <Text size="sm">
                    {definition.content.package} · {definition.content.adapter}
                  </Text>
                  <Text size="xs">
                    Provenance: {definition.content.provenance.replaceAll('_', ' ')}
                  </Text>
                  {definition.content.actions.map((a, i) => (
                    <Text key={a.id} size="xs">
                      Action {i + 1} · {a.id} → checkpoint {a.checkpoint_id}
                    </Text>
                  ))}
                  {definition.content.checks.map((c) => (
                    <Stack key={c.id} gap={2}>
                      <Text size="sm" fw={600}>
                        {c.description}
                      </Text>
                      <Text size="xs">
                        {c.method.replaceAll('_', ' ')} · {c.resource_id || 'No element selected'} ·
                        expected{' '}
                        {c.expected ||
                          (c.method === 'ui_property_equals_v1' ? 'Empty string' : 'Not specified')}
                      </Text>
                      <Text size="xs">
                        Check ID: {c.id} · {c.required ? 'Required' : 'Supporting'}
                      </Text>
                      <Text size="xs">
                        Property: {c.property.replaceAll('_', ' ')} · text filter:{' '}
                        {c.text_filter || 'None'}
                      </Text>
                      <Text size="xs">Ready element: {c.ready_resource_id || 'None'}</Text>
                      <Text size="xs">
                        Prerequisite checks:{' '}
                        {c.prerequisite_check_ids.length
                          ? c.prerequisite_check_ids.join(', ')
                          : 'None'}
                      </Text>
                      <Text size="xs" c="dimmed">
                        Checkpoint {c.checkpoint_id} · timeout {c.observation_seconds}s
                      </Text>
                    </Stack>
                  ))}
                </Stack>
              </Accordion.Panel>
            </Accordion.Item>
          </Accordion>
        </>
      ) : (
        <Text size="sm" c="dimmed">
          Membership below is pinned to saved versions. New case edits do not change this{' '}
          {definition.kind === 'suite' ? 'suite' : 'release plan'}.
        </Text>
      )}
      {definition.kind !== 'case' && (
        <Card>
          <Stack gap="sm">
            <Title order={2} size="h3">
              Exact membership
            </Title>
            {definition.kind === 'plan' && (
              <>
                <Text size="sm" fw={600}>
                  Pinned suites
                </Text>
                {definition.content.suite_version_ids.length ? (
                  definition.content.suite_version_ids.map((id, i) => (
                    <Stack gap={2} key={`${id}:${i}`}>
                      {reference(id)}
                      <Text size="xs" c="dimmed">
                        Suite version {id}
                      </Text>
                    </Stack>
                  ))
                ) : (
                  <Text size="sm" c="dimmed">
                    No suite selections.
                  </Text>
                )}
              </>
            )}
            <Text size="sm" fw={600}>
              {definition.kind === 'plan' ? 'Direct case selections' : 'Pinned cases'}
            </Text>
            {definition.content.cases.length ? (
              definition.content.cases.map((c, i) => (
                <Stack key={`${c.case_version_id}:${i}`} gap={2}>
                  {reference(c.case_version_id)}
                  <Text size="xs" c="dimmed">
                    {c.required ? 'Required' : 'Optional'} · data variant {c.data_variant} · version{' '}
                    {c.case_version_id}
                  </Text>
                </Stack>
              ))
            ) : (
              <Text size="sm" c="dimmed">
                No direct case selections.
              </Text>
            )}
          </Stack>
        </Card>
      )}
      {definition.kind === 'plan' && (
        <Card>
          <Stack gap="xs">
            <Title order={2} size="h3">
              Execution profile
            </Title>
            <Text size="sm">
              {options?.profiles.find((p) => p.id === definition.content.profile_id)?.name ??
                'Pinned profile'}
            </Text>
            <Text size="xs" c="dimmed">
              Profile ID: {definition.content.profile_id ?? 'Not selected'}
            </Text>
            <Text size="sm">Diagnostic retries: {definition.content.diagnostic_retries}</Text>
            {options?.profiles.find((p) => p.id === definition.content.profile_id)?.driver ===
              'fake' && (
              <Alert color="yellow">
                Simulation profile — a report from this plan does not represent a real device test.
              </Alert>
            )}
          </Stack>
        </Card>
      )}
      {'budget' in content && (
        <Text size="sm" c="dimmed">
          Limits: {content.budget.duration_seconds}s · {content.budget.max_steps} actions ·{' '}
          {content.budget.artifact_bytes.toLocaleString()} evidence bytes
        </Text>
      )}
    </Stack>
  )
}
