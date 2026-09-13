/** Form state uses generated domain types; the server owns publishability and readiness. */
import {
  Accordion,
  ActionIcon,
  Alert,
  Button,
  Card,
  Checkbox,
  Group,
  MultiSelect,
  NumberInput,
  Select,
  SimpleGrid,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from '@mantine/core'
import { ArrowDown, ArrowUp, Plus, Trash2 } from 'lucide-react'
import type {
  CaseDefinition,
  ExecutionBudget,
  ExpectedCheck,
  TestAction,
} from '@/api/generated/types.gen'

export function moveItem<T>(items: readonly T[], index: number, direction: -1 | 1): T[] {
  const next = index + direction
  if (index < 0 || next < 0 || index >= items.length || next >= items.length) return [...items]
  const result = [...items]
  ;[result[index], result[next]] = [result[next]!, result[index]!]
  return result
}
const itemId = (prefix: string) => `${prefix}_${crypto.randomUUID()}`
export function Ordering({
  index,
  count,
  name,
  move,
  remove,
}: {
  index: number
  count: number
  name: string
  move: (direction: -1 | 1) => void
  remove: () => void
}) {
  return (
    <Group gap={4} wrap="nowrap">
      <ActionIcon
        variant="subtle"
        aria-label={`Move ${name} up`}
        disabled={index === 0}
        onClick={() => move(-1)}
      >
        <ArrowUp size={16} />
      </ActionIcon>
      <ActionIcon
        variant="subtle"
        aria-label={`Move ${name} down`}
        disabled={index === count - 1}
        onClick={() => move(1)}
      >
        <ArrowDown size={16} />
      </ActionIcon>
      <ActionIcon variant="subtle" color="red" aria-label={`Remove ${name}`} onClick={remove}>
        <Trash2 size={16} />
      </ActionIcon>
    </Group>
  )
}
export function BudgetFields({
  value,
  onChange,
}: {
  value: ExecutionBudget
  onChange: (value: ExecutionBudget) => void
}) {
  return (
    <SimpleGrid cols={{ base: 1, sm: 3 }}>
      <NumberInput
        label="Time limit (seconds)"
        min={0}
        max={1800}
        allowDecimal={false}
        value={value.duration_seconds}
        onChange={(n) => onChange({ ...value, duration_seconds: Number(n) })}
      />
      <NumberInput
        label="Maximum actions"
        min={0}
        max={400}
        allowDecimal={false}
        value={value.max_steps}
        onChange={(n) => onChange({ ...value, max_steps: Number(n) })}
      />
      <NumberInput
        label="Evidence limit (bytes)"
        min={0}
        max={262144000}
        allowDecimal={false}
        value={value.artifact_bytes}
        onChange={(n) => onChange({ ...value, artifact_bytes: Number(n) })}
      />
    </SimpleGrid>
  )
}
export function CaseFields({
  value,
  onChange,
}: {
  value: CaseDefinition
  onChange: (value: CaseDefinition) => void
}) {
  const action = (id: string, update: Partial<TestAction>) =>
    onChange({
      ...value,
      actions: value.actions.map((a) => (a.id === id ? { ...a, ...update } : a)),
    })
  const check = (id: string, update: Partial<ExpectedCheck>) =>
    onChange({ ...value, checks: value.checks.map((c) => (c.id === id ? { ...c, ...update } : c)) })
  const checkpointChoices = value.actions.map((a, i) => ({
    value: a.checkpoint_id,
    label: `After action ${i + 1} · ${a.kind === 'navigate' ? a.instruction.slice(0, 45) || 'Navigate' : a.kind === 'restart_app' ? 'Restart app' : 'Capture checkpoint'}`,
  }))
  const missingCheckpoint = value.checks.some(
    (c) => !value.actions.some((a) => a.checkpoint_id === c.checkpoint_id),
  )
  return (
    <Stack gap="xl">
      <Card>
        <Stack>
          <Title order={2} size="h3">
            What are we testing?
          </Title>
          <TextInput
            label="Case title"
            placeholder="A saved task survives an app restart"
            maxLength={200}
            value={value.title}
            onChange={(e) => onChange({ ...value, title: e.currentTarget.value })}
          />
          <Textarea
            label="Requirement or acceptance criterion"
            description="Describe the behavior this case protects. A ticket or criterion reference can go here."
            minRows={3}
            maxLength={4000}
            value={value.requirement}
            onChange={(e) => onChange({ ...value, requirement: e.currentTarget.value })}
          />
          <Textarea
            label="Before the test"
            description="One precondition per line. These describe setup; they do not execute commands."
            autosize
            minRows={2}
            maxRows={8}
            value={value.preconditions.join('\n')}
            onChange={(e) =>
              onChange({
                ...value,
                preconditions: e.currentTarget.value ? e.currentTarget.value.split('\n') : [],
              })
            }
          />
        </Stack>
      </Card>
      <Stack gap="sm">
        <Group justify="space-between">
          <div>
            <Title order={2} size="h3">
              Actions
            </Title>
            <Text size="sm" c="dimmed">
              Tell the runner what to do, in order.
            </Text>
          </div>
          <Text size="sm" c="dimmed">
            {value.actions.length} / 20
          </Text>
        </Group>
        {!value.actions.length && (
          <Text size="sm" c="dimmed">
            Start with the first action a person would take in your app.
          </Text>
        )}
        {value.actions.map((a, i) => (
          <Card key={a.id}>
            <Stack gap="sm">
              <Group justify="space-between">
                <Text fw={600}>Action {i + 1}</Text>
                <Ordering
                  index={i}
                  count={value.actions.length}
                  name={`action ${i + 1}`}
                  move={(d) => onChange({ ...value, actions: moveItem(value.actions, i, d) })}
                  remove={() =>
                    onChange({
                      ...value,
                      actions: value.actions.filter((item) => item.id !== a.id),
                    })
                  }
                />
              </Group>
              <Select
                label={`Action ${i + 1} type`}
                allowDeselect={false}
                data={[
                  { value: 'navigate', label: 'Navigate — describe an interaction' },
                  { value: 'restart_app', label: 'Restart app — keep saved data' },
                  { value: 'checkpoint', label: 'Capture checkpoint — collect evidence' },
                ]}
                value={a.kind}
                onChange={(kind) => {
                  if (kind === 'navigate' || kind === 'restart_app' || kind === 'checkpoint')
                    action(a.id, { kind, instruction: kind === 'navigate' ? a.instruction : '' })
                }}
              />
              {a.kind === 'navigate' ? (
                <Textarea
                  label={`Action ${i + 1} instruction`}
                  placeholder="Create a task named ${task_title} and save it."
                  minRows={2}
                  maxLength={4000}
                  value={a.instruction}
                  onChange={(e) => action(a.id, { instruction: e.currentTarget.value })}
                />
              ) : (
                <Text size="sm" c="dimmed">
                  {a.kind === 'restart_app'
                    ? 'Close and reopen the app. Existing app data is preserved.'
                    : 'Capture the current screen and UI evidence for the expected checks below.'}
                </Text>
              )}
            </Stack>
          </Card>
        ))}
        <Button
          variant="light"
          leftSection={<Plus size={16} />}
          w="fit-content"
          disabled={value.actions.length >= 20}
          onClick={() =>
            onChange({
              ...value,
              actions: [
                ...value.actions,
                {
                  id: itemId('action'),
                  checkpoint_id: itemId('checkpoint'),
                  kind: 'navigate',
                  instruction: '',
                },
              ],
            })
          }
        >
          Add action
        </Button>
      </Stack>
      <Stack gap="sm">
        <Group justify="space-between">
          <div>
            <Title order={2} size="h3">
              Expected results
            </Title>
            <Text size="sm" c="dimmed">
              Define the evidence that makes this case pass.
            </Text>
          </div>
          <Text size="sm" c="dimmed">
            {value.checks.length} / 20
          </Text>
        </Group>
        {missingCheckpoint && (
          <Alert color="yellow" title="Reconnect an expected result">
            An action was removed, or an expected result has no action yet. Choose when to check it.
            Existing expectations have been preserved.
          </Alert>
        )}
        {!value.checks.length && (
          <Text size="sm" c="dimmed">
            At least one required result is needed before review.
          </Text>
        )}
        {value.checks.map((c, i) => (
          <Card key={c.id}>
            <Stack gap="sm">
              <Group justify="space-between">
                <Text fw={600}>Expected result {i + 1}</Text>
                <Ordering
                  index={i}
                  count={value.checks.length}
                  name={`expected result ${i + 1}`}
                  move={(d) => onChange({ ...value, checks: moveItem(value.checks, i, d) })}
                  remove={() =>
                    onChange({ ...value, checks: value.checks.filter((item) => item.id !== c.id) })
                  }
                />
              </Group>
              <Textarea
                label={`Expected result ${i + 1}`}
                placeholder="The saved task is still visible after reopening the app."
                minRows={2}
                maxLength={1000}
                value={c.description}
                onChange={(e) => check(c.id, { description: e.currentTarget.value })}
              />
              <Select
                label={`When to check result ${i + 1}`}
                placeholder="Choose an action"
                data={checkpointChoices}
                value={c.checkpoint_id || null}
                onChange={(v) => check(c.id, { checkpoint_id: v ?? '' })}
              />
              <Checkbox
                label="Required for this case to pass"
                checked={c.required}
                onChange={(e) => check(c.id, { required: e.currentTarget.checked })}
              />
              <Accordion variant="separated">
                <Accordion.Item value="evidence">
                  <Accordion.Control>How this result is verified</Accordion.Control>
                  <Accordion.Panel>
                    <Stack gap="sm">
                      <Text size="sm" c="dimmed">
                        Technical evidence settings are checked during executability review. Ask
                        your operator if you are unsure which Android resource identifies the
                        result.
                      </Text>
                      <Select
                        label={`Verification method for result ${i + 1}`}
                        allowDeselect={false}
                        data={[
                          { value: 'ui_element_presence_v1', label: 'Element is present' },
                          { value: 'ui_property_equals_v1', label: 'Element property matches' },
                          { value: 'manual', label: 'Manual review (cannot run automatically)' },
                        ]}
                        value={c.method}
                        onChange={(method) => {
                          if (
                            method === 'ui_element_presence_v1' ||
                            method === 'ui_property_equals_v1' ||
                            method === 'manual'
                          )
                            check(c.id, { method })
                        }}
                      />
                      {c.method === 'manual' && (
                        <Alert color="yellow">
                          Manual checks cannot pass executability review for the current runner.
                        </Alert>
                      )}
                      <TextInput
                        label={`Android resource ID for result ${i + 1}`}
                        placeholder={`${value.package}:id/task_title`}
                        maxLength={255}
                        value={c.resource_id}
                        onChange={(e) => check(c.id, { resource_id: e.currentTarget.value })}
                      />
                      <TextInput
                        label={`Text filter for result ${i + 1}`}
                        maxLength={1000}
                        value={c.text_filter}
                        onChange={(e) => check(c.id, { text_filter: e.currentTarget.value })}
                      />
                      <Select
                        label={`Property for result ${i + 1}`}
                        allowDeselect={false}
                        data={[
                          { value: 'text', label: 'Text' },
                          { value: 'content_description', label: 'Accessibility description' },
                          { value: 'checked', label: 'Checked' },
                          { value: 'enabled', label: 'Enabled' },
                        ]}
                        value={c.property}
                        onChange={(property) => {
                          if (
                            property === 'text' ||
                            property === 'content_description' ||
                            property === 'checked' ||
                            property === 'enabled'
                          )
                            check(c.id, { property })
                        }}
                      />
                      {c.method === 'ui_element_presence_v1' ||
                      c.property === 'checked' ||
                      c.property === 'enabled' ? (
                        <Select
                          label={`Expected value for result ${i + 1}`}
                          placeholder="Choose the expected state"
                          allowDeselect={false}
                          data={[
                            {
                              value: 'true',
                              label: c.method === 'ui_element_presence_v1' ? 'Present' : 'True',
                            },
                            {
                              value: 'false',
                              label: c.method === 'ui_element_presence_v1' ? 'Absent' : 'False',
                            },
                          ]}
                          value={c.expected}
                          onChange={(expected) => {
                            if (expected) check(c.id, { expected })
                          }}
                        />
                      ) : (
                        <TextInput
                          label={`Expected value for result ${i + 1}`}
                          maxLength={1000}
                          value={c.expected}
                          onChange={(e) => check(c.id, { expected: e.currentTarget.value })}
                        />
                      )}
                      <TextInput
                        label={`Ready resource ID for result ${i + 1}`}
                        description="An element that confirms this screen has finished loading."
                        maxLength={255}
                        value={c.ready_resource_id}
                        onChange={(e) => check(c.id, { ready_resource_id: e.currentTarget.value })}
                      />
                      <NumberInput
                        label={`Observation timeout for result ${i + 1} (seconds)`}
                        min={0}
                        max={60}
                        allowDecimal={false}
                        value={c.observation_seconds}
                        onChange={(n) => check(c.id, { observation_seconds: Number(n) })}
                      />
                      <MultiSelect
                        label={`Prerequisite results for result ${i + 1}`}
                        description="Only earlier results can be prerequisites. Reordering may require updating these references."
                        data={value.checks.slice(0, i).map((before, index) => ({
                          value: before.id,
                          label: `Result ${index + 1} · ${before.description.slice(0, 50)}`,
                        }))}
                        value={c.prerequisite_check_ids}
                        onChange={(ids) => check(c.id, { prerequisite_check_ids: ids })}
                      />
                    </Stack>
                  </Accordion.Panel>
                </Accordion.Item>
              </Accordion>
            </Stack>
          </Card>
        ))}
        <Button
          variant="light"
          leftSection={<Plus size={16} />}
          w="fit-content"
          disabled={value.checks.length >= 20}
          onClick={() =>
            onChange({
              ...value,
              checks: [
                ...value.checks,
                {
                  id: itemId('check'),
                  checkpoint_id: value.actions.at(-1)?.checkpoint_id ?? '',
                  description: '',
                  method: 'ui_element_presence_v1',
                  resource_id: '',
                  text_filter: '',
                  property: 'text',
                  expected: 'true',
                  ready_resource_id: '',
                  prerequisite_check_ids: [],
                  required: true,
                  observation_seconds: 10,
                },
              ],
            })
          }
        >
          Add expected result
        </Button>
      </Stack>
      <Accordion variant="separated">
        <Accordion.Item value="runtime">
          <Accordion.Control>Runner compatibility and limits</Accordion.Control>
          <Accordion.Panel>
            <Stack>
              <TextInput label="Android package" value={value.package} readOnly />
              <TextInput label="Execution adapter" value={value.adapter} readOnly />
              <Text size="sm" c="dimmed">
                The current qualified runner supports the controlled demo adapter. Drafting and
                business review do not imply this app can run yet.
              </Text>
              <BudgetFields
                value={value.budget}
                onChange={(budget) => onChange({ ...value, budget })}
              />
            </Stack>
          </Accordion.Panel>
        </Accordion.Item>
      </Accordion>
    </Stack>
  )
}
