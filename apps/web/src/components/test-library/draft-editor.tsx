import { useEffect, useState } from 'react'
import { Alert, Badge, Button, Card, Group, Modal, Stack, Table, Text, Title } from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useBlocker, useNavigate } from 'react-router'
import {
  libraryDraftQuery,
  libraryErrorDetails,
  libraryKey,
  libraryOptionsQuery,
  saveLibraryDraft,
  submitLibraryDraft,
} from '@/api/test-library'
import type {
  LibraryDraftDefinition,
  LibraryDraftResponse,
  SaveLibraryDraftRequest,
  SubmitLibraryDraftRequest,
} from '@/api/generated/types.gen'
import { useWorkspace } from '@/hooks/use-workspace'
import { useMounted } from '@/hooks/use-mounted'
import { ErrorNotice, LoadingPanel } from '@/components/app/feedback'
import { PhoneWorkspace } from '@/components/task-session/phone-workspace'
import { PlanFields, SuiteFields } from './membership-fields'
import { Coverage, LibraryIssues } from './library-presentation'

/** Compare validated form values as individual fields, never use raw JSON as the editor. */
export function changedFields(local: LibraryDraftDefinition, remote: LibraryDraftDefinition) {
  const flatten = (value: unknown, prefix = '', result: Record<string, string> = {}) => {
    if (value !== null && typeof value === 'object') {
      for (const [key, item] of Object.entries(value))
        flatten(item, prefix ? `${prefix}.${key}` : key, result)
    } else result[prefix] = value == null ? 'Not selected' : String(value)
    return result
  }
  const mine = flatten(local)
  const saved = flatten(remote)
  return [...new Set([...Object.keys(mine), ...Object.keys(saved)])]
    .filter((key) => mine[key] !== saved[key])
    .map((key) => ({
      field: key.replace(/^content\./, '').replaceAll('_', ' '),
      local: mine[key] ?? 'Removed',
      saved: saved[key] ?? 'Removed',
    }))
}
export function DraftEditor({
  initial,
  available = true,
}: {
  initial: LibraryDraftResponse
  available?: boolean
}) {
  const { workspaceId = '', href } = useWorkspace()
  const appId = initial.entry.app_id
  const entryId = initial.entry.id
  const client = useQueryClient()
  const navigate = useNavigate()
  const mounted = useMounted()
  const [saved, setSaved] = useState(initial)
  const [definition, setDefinition] = useState(initial.definition)
  const [incoming, setIncoming] = useState<LibraryDraftResponse | null>(null)
  const [loadError, setLoadError] = useState<unknown>(null)
  const [loadingCurrent, setLoadingCurrent] = useState(false)
  const [submitted, setSubmitted] = useState(false)
  const dirty = JSON.stringify(definition) !== JSON.stringify(saved.definition)
  const options = useQuery(libraryOptionsQuery(workspaceId, appId))
  const blocker = useBlocker(dirty && !submitted)
  useEffect(() => {
    if (!dirty || submitted) return
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [dirty, submitted])
  const invalidate = () => {
    void client.invalidateQueries({ queryKey: libraryKey(workspaceId, appId) })
    void client.invalidateQueries({ queryKey: ['execution-plan', workspaceId, appId] })
  }
  const save = useMutation({
    mutationFn: (body: SaveLibraryDraftRequest) => saveLibraryDraft(appId, entryId, body),
    onSuccess: (response) => {
      if (!mounted.current) return
      setSaved(response)
      setDefinition(response.definition)
      setIncoming(null)
      invalidate()
    },
  })
  const submit = useMutation({
    mutationFn: (body: SubmitLibraryDraftRequest) => submitLibraryDraft(appId, entryId, body),
    onSuccess: (response) => {
      if (!mounted.current) return
      setSubmitted(true)
      invalidate()
      void navigate(href(`/tests/${appId}/${entryId}/versions/${response.version.id}`))
    },
  })
  const details = libraryErrorDetails(save.error ?? submit.error)
  const stale = details?.kind === 'stale_revision'
  const busy = save.isPending || submit.isPending || loadingCurrent
  const canEdit = available && initial.entry.capabilities.can_edit && !initial.entry.archived_at
  const reviewCurrent = async () => {
    setLoadingCurrent(true)
    setLoadError(null)
    try {
      const response = await client.fetchQuery({
        ...libraryDraftQuery(workspaceId, appId, entryId),
        staleTime: 0,
      })
      if (mounted.current) setIncoming(response)
    } catch (error) {
      if (mounted.current) setLoadError(error)
    } finally {
      if (mounted.current) setLoadingCurrent(false)
    }
  }
  return (
    <Stack gap="lg">
      <Card bg="var(--mantine-color-forest-0)">
        <Group justify="space-between">
          <Stack gap={4}>
            <Group gap="xs">
              <Badge color="gray">Draft v{definition.content.version}</Badge>
              <Text size="sm" role="status" aria-live="polite">
                {dirty ? 'Unsaved changes' : save.isSuccess ? 'Draft saved' : 'All changes saved'}
              </Text>
            </Group>
            <Text size="sm" c="dimmed">
              Save freely. Request review when the behavior and expectations are ready.
            </Text>
          </Stack>
          <Group>
            <Button
              variant="default"
              disabled={!canEdit || busy || stale}
              loading={save.isPending}
              onClick={() =>
                save.mutate({
                  mutation_id: crypto.randomUUID(),
                  expected_revision: saved.entry.revision,
                  definition,
                })
              }
            >
              Save draft
            </Button>
            <Button
              disabled={
                !canEdit ||
                dirty ||
                busy ||
                stale ||
                !!saved.issues.length ||
                !!saved.coverage.issues.length
              }
              loading={submit.isPending}
              onClick={() =>
                submit.mutate({
                  mutation_id: crypto.randomUUID(),
                  expected_revision: saved.entry.revision,
                })
              }
            >
              Request review
            </Button>
          </Group>
        </Group>
      </Card>
      {!canEdit && (
        <Alert color="gray" title="Read-only draft">
          {!available
            ? 'This draft has been submitted. Your local values are preserved for reference.'
            : initial.entry.archived_at
              ? 'This entry is archived. An operator can restore it before editing.'
              : 'You do not have permission to edit this draft.'}
        </Alert>
      )}
      {dirty && (
        <Text size="sm" c="dimmed">
          Save before requesting review. The readiness and coverage below describe the last saved
          draft.
        </Text>
      )}
      {stale ? (
        <Alert color="orange" title="A newer revision was saved">
          <Stack gap="sm">
            <Text size="sm">
              Your edits are still here. Review the current saved draft before deciding what to
              keep. Nothing has been overwritten.
            </Text>
            <Button
              variant="outline"
              w="fit-content"
              loading={loadingCurrent}
              onClick={() => void reviewCurrent()}
            >
              Compare with saved draft
            </Button>
          </Stack>
        </Alert>
      ) : (
        <>
          {save.isError && (
            <ErrorNotice
              error={save.error}
              retry={
                save.variables &&
                JSON.stringify(save.variables.definition) === JSON.stringify(definition)
                  ? () => {
                      if (save.variables) save.mutate(save.variables)
                    }
                  : undefined
              }
            />
          )}
          {submit.isError && (
            <ErrorNotice
              error={submit.error}
              retry={
                submit.variables
                  ? () => {
                      if (submit.variables) submit.mutate(submit.variables)
                    }
                  : undefined
              }
            />
          )}
        </>
      )}
      {loadError != null && <ErrorNotice error={loadError} retry={() => void reviewCurrent()} />}
      {details?.kind === 'validation' && (
        <LibraryIssues issues={details.issues} title="Review could not be requested" />
      )}
      <LibraryIssues issues={saved.issues} />
      {options.isError && (
        <ErrorNotice error={options.error} retry={() => void options.refetch()} />
      )}
      <fieldset
        disabled={!canEdit || busy}
        style={{ border: 0, margin: 0, padding: 0, minWidth: 0 }}
      >
        {definition.kind === 'case' ? (
          <PhoneWorkspace
            appId={appId}
            autoOpen={false}
            disabled={!canEdit || busy}
            value={definition.content}
            onChange={(content) => setDefinition({ kind: 'case', content })}
          />
        ) : options.isPending ? (
          <LoadingPanel label="Loading approved choices…" />
        ) : (
          options.data &&
          (definition.kind === 'suite' ? (
            <SuiteFields
              value={definition.content}
              options={options.data}
              onChange={(content) => setDefinition({ kind: 'suite', content })}
            />
          ) : (
            <PlanFields
              value={definition.content}
              options={options.data}
              onChange={(content) => setDefinition({ kind: 'plan', content })}
            />
          ))
        )}
      </fieldset>
      {definition.kind !== 'case' && <Coverage value={saved.coverage} saved={!dirty} />}
      <Modal
        opened={blocker.state === 'blocked'}
        onClose={() => {
          if (blocker.state === 'blocked') blocker.reset()
        }}
        title="Leave unsaved changes?"
        centered
      >
        <Stack>
          <Text size="sm">
            Your draft has changes that have not been saved. Stay here to save them, or discard them
            and leave.
          </Text>
          <Group justify="flex-end">
            <Button
              variant="default"
              onClick={() => {
                if (blocker.state === 'blocked') blocker.reset()
              }}
            >
              Keep editing
            </Button>
            <Button
              color="red"
              onClick={() => {
                if (blocker.state === 'blocked') blocker.proceed()
              }}
            >
              Discard changes and leave
            </Button>
          </Group>
        </Stack>
      </Modal>
      <Modal
        opened={incoming !== null}
        onClose={() => setIncoming(null)}
        title="Your changes and the saved draft"
        size="xl"
      >
        <Stack>
          <Text size="sm">
            Compare every changed field. You can keep your edits against the displayed revision and
            explicitly save them, or load the saved draft and discard your local edits. Another
            concurrent save will still be rejected.
          </Text>
          {incoming && (
            <>
              <Title order={3} size="h4">
                Saved revision {incoming.entry.revision}
              </Title>
              <Table.ScrollContainer minWidth={450}>
                <Table withTableBorder>
                  <Table.Thead>
                    <Table.Tr>
                      <Table.Th>Field</Table.Th>
                      <Table.Th>Your editor</Table.Th>
                      <Table.Th>Saved draft</Table.Th>
                    </Table.Tr>
                  </Table.Thead>
                  <Table.Tbody>
                    {changedFields(definition, incoming.definition).map((change) => (
                      <Table.Tr key={change.field}>
                        <Table.Td>
                          <Text size="xs">{change.field}</Text>
                        </Table.Td>
                        <Table.Td>
                          <Text size="sm">{change.local}</Text>
                        </Table.Td>
                        <Table.Td>
                          <Text size="sm">{change.saved}</Text>
                        </Table.Td>
                      </Table.Tr>
                    ))}
                  </Table.Tbody>
                </Table>
              </Table.ScrollContainer>
            </>
          )}
          <Group justify="flex-end">
            <Button variant="default" onClick={() => setIncoming(null)}>
              Keep my edits
            </Button>
            <Button
              variant="outline"
              onClick={() => {
                if (incoming) {
                  setSaved(incoming)
                  setIncoming(null)
                  save.reset()
                  submit.reset()
                }
              }}
            >
              Keep my edits against this revision
            </Button>
            <Button
              color="red"
              onClick={() => {
                if (incoming) {
                  setSaved(incoming)
                  setDefinition(incoming.definition)
                  setIncoming(null)
                  save.reset()
                  submit.reset()
                }
              }}
            >
              Discard my edits and load saved draft
            </Button>
          </Group>
        </Stack>
      </Modal>
    </Stack>
  )
}
