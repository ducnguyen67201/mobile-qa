import { type ReactNode, useEffect, useState } from 'react'
import { Alert, Badge, Button, Card, Group, Modal, Stack, Table, Text, Title } from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useBlocker } from 'react-router'
import {
  libraryDraftQuery,
  libraryErrorDetails,
  libraryKey,
  libraryOptionsQuery,
  saveLibraryDraft,
} from '@/api/test-library'
import type {
  LibraryDraftDefinition,
  LibraryDraftResponse,
  SaveLibraryDraftRequest,
} from '@/api/generated/types.gen'
import { useWorkspace } from '@/hooks/use-workspace'
import { useMounted } from '@/hooks/use-mounted'
import { ErrorNotice, LoadingPanel } from '@/components/app/feedback'
import { PhoneWorkspace } from '@/components/task-session/phone-workspace'
import { PlanFields, SuiteFields } from './membership-fields'
import { Coverage, LibraryIssues } from './library-presentation'
import { SavedCaseRunControls } from './saved-case-run-controls'

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
  renderSaved,
}: {
  initial: LibraryDraftResponse
  available?: boolean
  renderSaved?: (saved: LibraryDraftResponse) => ReactNode
}) {
  const { workspaceId = '' } = useWorkspace()
  const appId = initial.entry.app_id
  const entryId = initial.entry.id
  const client = useQueryClient()
  const mounted = useMounted()
  const [saved, setSaved] = useState(initial)
  const [definition, setDefinition] = useState(initial.definition)
  const [incoming, setIncoming] = useState<LibraryDraftResponse | null>(null)
  const [loadError, setLoadError] = useState<unknown>(null)
  const [loadingCurrent, setLoadingCurrent] = useState(false)
  const dirty = JSON.stringify(definition) !== JSON.stringify(saved.definition)
  const options = useQuery(libraryOptionsQuery(workspaceId, appId))
  const blocker = useBlocker(dirty)
  useEffect(() => {
    if (!dirty) return
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [dirty])
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
  const saveRequest = (): SaveLibraryDraftRequest =>
    save.isError &&
    save.variables &&
    save.variables.expected_revision === saved.entry.revision &&
    JSON.stringify(save.variables.definition) === JSON.stringify(definition)
      ? save.variables
      : {
          mutation_id: crypto.randomUUID(),
          expected_revision: saved.entry.revision,
          definition,
        }
  const saveAndGetVersion = async () => {
    const response = await save.mutateAsync(saveRequest())
    if (!response.saved_version_id) {
      throw new Error('Complete the required test fields before running')
    }
    return response.saved_version_id
  }
  const details = libraryErrorDetails(save.error)
  const stale = details?.kind === 'stale_revision'
  const busy = save.isPending || loadingCurrent
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
              <Badge color="gray">v{definition.content.version}</Badge>
              <Text size="sm" role="status" aria-live="polite">
                {dirty ? 'Unsaved changes' : save.isSuccess ? 'Saved' : 'All changes saved'}
              </Text>
            </Group>
            <Text size="sm" c="dimmed">
              Save your changes. Previous versions and run results stay unchanged.
            </Text>
          </Stack>
          <Group>
            <Button
              variant="default"
              disabled={!canEdit || busy || stale}
              loading={save.isPending}
              onClick={() => save.mutate(saveRequest())}
            >
              Save
            </Button>
          </Group>
        </Group>
      </Card>
      {!canEdit && (
        <Alert color="gray" title="Read-only draft">
          {!available
            ? 'This entry is unavailable. Your local values are preserved.'
            : initial.entry.archived_at
              ? 'This entry is archived. An operator can restore it before editing.'
              : 'You do not have permission to edit this draft.'}
        </Alert>
      )}
      {dirty && (
        <Text size="sm" c="dimmed">
          Unsaved changes. Try actions uses the preview session; Save & run creates an immutable
          version and durable run.
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
        </>
      )}
      {loadError != null && <ErrorNotice error={loadError} retry={() => void reviewCurrent()} />}
      {details?.kind === 'validation' && (
        <LibraryIssues issues={details.issues} title="Could not save" />
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
          <LoadingPanel label="Loading saved choices…" />
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
      {definition.kind === 'case' && (
        <SavedCaseRunControls
          appId={appId}
          savedVersionId={saved.saved_version_id}
          dirty={dirty}
          disabled={!canEdit || busy}
          saveAndGetVersion={saveAndGetVersion}
        />
      )}
      {definition.kind !== 'case' && <Coverage value={saved.coverage} saved={!dirty} />}
      {!dirty && renderSaved?.(saved)}
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
                  setDefinition(
                    (local) =>
                      ({
                        ...local,
                        content: { ...local.content, version: incoming.definition.content.version },
                      }) as LibraryDraftDefinition,
                  )
                  setIncoming(null)
                  save.reset()
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
