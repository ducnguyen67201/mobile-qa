import { Button, Card, Drawer, Stack, Text, Title } from '@mantine/core'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from 'react-router'
import { saveAuthoredTests, templatesQuery } from '@/api/test-authoring'
import type { SaveAuthoredTestsRequest } from '@/api/generated/types.gen'
import { ErrorNotice, LoadingPanel } from '@/components/app/feedback'
import { useWorkspace } from '@/hooks/use-workspace'
export function TemplatePicker({
  appId,
  opened,
  close,
}: {
  appId: string
  opened: boolean
  close: () => void
}) {
  const query = useQuery({ ...templatesQuery(appId), enabled: opened })
  const { href } = useWorkspace()
  const navigate = useNavigate()
  const client = useQueryClient()
  const save = useMutation({
    mutationFn: (body: SaveAuthoredTestsRequest) => saveAuthoredTests(appId, body),
    onSuccess: (r) => {
      void client.invalidateQueries({ queryKey: ['test-library'] })
      close()
      void navigate(href(`/tests/${appId}/${r.entry_ids[0]}`))
    },
  })
  return (
    <Drawer opened={opened} onClose={close} title="Choose a test template" size="lg">
      <Stack>
        <Text c="dimmed">Start with a named test, then pick your app’s controls. No AI calls.</Text>
        {query.isPending && <LoadingPanel label="Loading templates…" />}
        {query.isError && <ErrorNotice error={query.error} retry={() => void query.refetch()} />}
        {query.data?.items.map((t) => (
          <Card withBorder key={t.id}>
            <Stack gap="sm">
              <Title order={3} size="h4">
                {t.title}
              </Title>
              <Text size="sm">{t.description}</Text>
              <Button
                variant="light"
                loading={save.isPending}
                onClick={() =>
                  save.mutate({
                    mutation_id: crypto.randomUUID(),
                    source_task_id: null,
                    expectations_confirmed: false,
                    tests: [
                      {
                        template_id: t.id,
                        proposal_id: null,
                        title: t.title,
                        requirement: '',
                        sequence: { actions: t.definition.actions, checks: t.definition.checks },
                      },
                    ],
                  })
                }
              >
                Use template
              </Button>
            </Stack>
          </Card>
        ))}
        {save.isError && (
          <ErrorNotice
            error={save.error}
            retry={() => save.variables && save.mutate(save.variables)}
          />
        )}
      </Stack>
    </Drawer>
  )
}
