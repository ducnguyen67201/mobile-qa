/** The editor keeps generated structured actions intact through save and run. */
import { useState } from 'react'
import {
  Alert,
  Box,
  Collapse,
  ActionIcon,
  Button,
  Card,
  Group,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Checkbox,
} from '@mantine/core'
import type {
  AutomationSequence,
  DirectCommand,
  DirectTarget,
  ExpectedCheck,
  TestAction,
} from '@/api/generated/types.gen'
import {
  MousePointer2,
  Type,
  Move,
  ArrowLeft,
  RotateCcw,
  Clock,
  Camera,
  Sparkles,
  Crosshair,
  ListChecks,
  ChevronDown,
} from 'lucide-react'
import { moveItem, Ordering } from '@/components/test-library/definition-fields'

export function newTaskStep(
  command: DirectCommand = { operation: 'tap', target: { by: 'resource_id', value: '' } },
): TestAction {
  const id = `step_${crypto.randomUUID()}`
  return { id, checkpoint_id: id, kind: 'direct', instruction: '', command }
}
export function commandTarget(command: DirectCommand | null | undefined): DirectTarget | undefined {
  return command && 'target' in command ? command.target : undefined
}
export function sequenceReady(value: AutomationSequence): boolean {
  return (
    value.actions.length > 0 &&
    value.actions.every((a) =>
      a.kind === 'navigate'
        ? !!a.instruction.trim()
        : a.kind !== 'direct' ||
          (!!a.command && (!commandTarget(a.command) || !!commandTarget(a.command)?.value.trim())),
    ) &&
    value.checks.every(
      (c) =>
        !!c.resource_id &&
        !!c.ready_resource_id &&
        !!c.description &&
        !!c.checkpoint_id &&
        value.actions.some((a) => a.checkpoint_id === c.checkpoint_id),
    )
  )
}
export function newCheck(checkpoint: string): ExpectedCheck {
  return {
    id: `check_${crypto.randomUUID()}`,
    checkpoint_id: checkpoint,
    description: 'Expected result',
    resource_id: '',
    ready_resource_id: '',
    method: 'ui_property_equals_v1',
    property: 'text',
    expected: '',
    text_filter: '',
    prerequisite_check_ids: [],
    required: true,
    observation_seconds: 10,
  }
}
export function TaskSteps({
  value,
  onChange,
  canPick = false,
  pickingId,
  onPick,
  disabled = false,
}: {
  value: AutomationSequence
  onChange: (value: AutomationSequence) => void
  canPick?: boolean
  pickingId?: string | null
  onPick?: (id: string) => void
  disabled?: boolean
}) {
  const [expandedId, setExpandedId] = useState<string | null>(value.actions[0]?.id ?? null)
  const activeId =
    expandedId === null
      ? null
      : value.actions.some((a) => a.id === expandedId)
        ? expandedId
        : value.actions[0]?.id
  const [checksId, setChecksId] = useState<string | null>(null)
  const update = (id: string, update: Partial<TestAction>) =>
    onChange({
      ...value,
      actions: value.actions.map((a) => (a.id === id ? { ...a, ...update } : a)),
    })
  const check = (id: string, update: Partial<ExpectedCheck>) =>
    onChange({ ...value, checks: value.checks.map((c) => (c.id === id ? { ...c, ...update } : c)) })
  const targetPicker = (id: string) => (
    <ActionIcon
      size="sm"
      title={pickingId === id ? 'Cancel picking' : 'Pick on phone'}
      variant="light"
      disabled={disabled || !canPick}
      aria-label={`Pick target for ${value.actions.some((a) => a.id === id) ? `action ${value.actions.findIndex((a) => a.id === id) + 1}` : `check ${value.checks.findIndex((c) => c.id === id) + 1}`} on phone`}
      aria-pressed={pickingId === id}
      onClick={() => onPick?.(id)}
    >
      <Crosshair size={16} />
    </ActionIcon>
  )
  const renderCheck = (c: ExpectedCheck) => {
    const index = value.checks.findIndex((item) => item.id === c.id)
    return (
      <Box key={c.id} p="sm" style={{ borderLeft: '3px solid var(--mantine-color-green-3)' }}>
        <Stack gap="sm">
          <>
            {!value.actions.some((a) => a.checkpoint_id === c.checkpoint_id) && (
              <Alert color="orange">Reconnect an expected result</Alert>
            )}
          </>
          <Group justify="space-between">
            <Text fw={600}>Check {index + 1}</Text>
            <Button
              size="xs"
              variant="subtle"
              color="red"
              disabled={disabled}
              onClick={() =>
                onChange({ ...value, checks: value.checks.filter((v) => v.id !== c.id) })
              }
            >
              Remove check
            </Button>
          </Group>
          <TextInput
            label={`Check ${index + 1} description`}
            value={c.description}
            disabled={disabled}
            onChange={(e) => check(c.id, { description: e.currentTarget.value })}
          />
          {!value.actions.some((a) => a.checkpoint_id === c.checkpoint_id) && (
            <Select
              label={`Check ${index + 1} after step`}
              disabled={disabled}
              value={c.checkpoint_id}
              data={value.actions.map((a, i) => ({
                value: a.checkpoint_id,
                label: `Action ${i + 1}`,
              }))}
              onChange={(v) => v && check(c.id, { checkpoint_id: v })}
            />
          )}
          <Group align="flex-end">
            <TextInput
              label={`Check ${index + 1} control`}
              value={c.resource_id}
              readOnly
              style={{ flex: 1 }}
            />
            {targetPicker(c.id)}
          </Group>
          <Select
            label={`Check ${index + 1} property`}
            disabled={disabled}
            value={c.property}
            data={[
              { value: 'text', label: 'Text' },
              { value: 'checked', label: 'Checked (true/false)' },
              { value: 'enabled', label: 'Enabled (true/false)' },
              { value: 'content_description', label: 'Accessibility description' },
            ]}
            onChange={(v) => {
              if (v === 'text' || v === 'checked' || v === 'enabled' || v === 'content_description')
                check(c.id, { property: v })
            }}
          />
          <TextInput
            label={`Check ${index + 1} expected value`}
            value={c.expected}
            disabled={disabled}
            onChange={(e) => check(c.id, { expected: e.currentTarget.value })}
          />
          <Checkbox
            label="Required result"
            checked={c.required}
            disabled={disabled}
            onChange={(e) => check(c.id, { required: e.currentTarget.checked })}
          />
        </Stack>
      </Box>
    )
  }
  return (
    <Stack gap="sm">
      <Text size="sm" c="dimmed">
        Add actions in order. Checks are optional when trying your actions.
      </Text>
      {value.actions.map((a, index) => {
        const operation =
          a.kind === 'navigate'
            ? 'ai'
            : a.kind === 'restart_app'
              ? 'restart'
              : a.kind === 'checkpoint'
                ? 'checkpoint'
                : (a.command?.operation ?? 'tap')
        const target = commandTarget(a.command)
        const Icon = {
          tap: MousePointer2,
          set_text: Type,
          swipe: Move,
          back: ArrowLeft,
          restart: RotateCcw,
          wait_for: Clock,
          checkpoint: Camera,
          ai: Sparkles,
        }[operation]

        return (
          <Card key={a.id} withBorder padding="xs" radius="sm">
            <Stack gap={4}>
              <Group gap={6} wrap="nowrap">
                <ActionIcon
                  size="sm"
                  variant="subtle"
                  aria-label={`Edit action ${index + 1}`}
                  aria-expanded={activeId === a.id}
                  title={`Expand or collapse action ${index + 1}`}
                  onClick={() => setExpandedId(activeId === a.id ? null : a.id)}
                >
                  <ChevronDown
                    size={14}
                    style={{ transform: activeId === a.id ? undefined : 'rotate(-90deg)' }}
                  />
                </ActionIcon>
                <Text size="xs" c="dimmed">
                  {index + 1}
                </Text>
                <Select
                  aria-label={`Action ${index + 1} action`}
                  size="xs"
                  variant="unstyled"
                  style={{ flex: 1, minWidth: 100 }}
                  leftSection={<Icon size={15} />}
                  value={operation}
                  disabled={disabled}
                  allowDeselect={false}
                  data={[
                    { value: 'tap', label: 'Tap' },
                    { value: 'set_text', label: 'Enter text' },
                    { value: 'swipe', label: 'Swipe' },
                    { value: 'back', label: 'Go back' },
                    { value: 'restart', label: 'Restart app' },
                    { value: 'wait_for', label: 'Wait for a control' },
                    { value: 'checkpoint', label: 'Capture checkpoint' },
                    { value: 'ai', label: 'Ask AI' },
                  ]}
                  onChange={(kind) => {
                    if (kind === 'ai' || kind === 'checkpoint')
                      update(a.id, {
                        kind: kind === 'ai' ? 'navigate' : 'checkpoint',
                        instruction: '',
                        command: null,
                      })
                    else if (kind === 'tap' || kind === 'set_text' || kind === 'wait_for')
                      update(a.id, {
                        kind: 'direct',
                        instruction: '',
                        command:
                          kind === 'set_text'
                            ? {
                                operation: kind,
                                target: target ?? { by: 'resource_id', value: '' },
                                text: '',
                              }
                            : {
                                operation: kind,
                                target: target ?? { by: 'resource_id', value: '' },
                              },
                      })
                    else if (kind === 'back' || kind === 'restart')
                      update(a.id, {
                        kind: 'direct',
                        instruction: '',
                        command: { operation: kind },
                      })
                    else if (kind === 'swipe')
                      update(a.id, {
                        kind: 'direct',
                        instruction: '',
                        command: { operation: 'swipe', direction: 'up' },
                      })
                  }}
                />
                <Button
                  variant="subtle"
                  size="xs"
                  title="Add an optional result check"
                  aria-label={`Add check to action ${index + 1}`}
                  leftSection={<ListChecks size={14} />}
                  disabled={disabled || value.checks.length >= 20}
                  onClick={() => {
                    onChange({ ...value, checks: [...value.checks, newCheck(a.checkpoint_id)] })
                    setExpandedId(a.id)
                    setChecksId(a.id)
                  }}
                >
                  + Check
                </Button>{' '}
                {value.checks.some((c) => c.checkpoint_id === a.checkpoint_id) && (
                  <ActionIcon
                    size="sm"
                    variant="light"
                    title="Show checks"
                    aria-label={`Show checks for action ${index + 1}`}
                    aria-expanded={checksId === a.id}
                    onClick={() => {
                      setExpandedId(a.id)
                      setChecksId(checksId === a.id ? null : a.id)
                    }}
                  >
                    <ListChecks size={14} />
                  </ActionIcon>
                )}
                <fieldset disabled={disabled} style={{ border: 0, padding: 0, margin: 0 }}>
                  <Ordering
                    index={index}
                    count={value.actions.length}
                    name={`action ${index + 1}`}
                    move={(d) => onChange({ ...value, actions: moveItem(value.actions, index, d) })}
                    remove={() =>
                      onChange({
                        actions: value.actions.filter((item) => item.id !== a.id),
                        checks: value.checks,
                      })
                    }
                  />
                </fieldset>
              </Group>
              <Collapse expanded={activeId === a.id}>
                <Stack gap={6}>
                  <Group gap="xs" align="center">
                    {target && (
                      <Group gap={6} wrap="nowrap" style={{ flex: 1, minWidth: 120 }}>
                        <TextInput
                          aria-label={`Action ${index + 1} target`}
                          size="xs"
                          title={target.value}
                          placeholder="Pick a control on the phone"
                          value={
                            target.by === 'resource_id'
                              ? (target.value.split(':id/').pop()?.replaceAll('_', ' ') ??
                                target.value)
                              : target.value
                          }
                          readOnly
                          style={{ flex: 1 }}
                        />
                        {targetPicker(a.id)}
                      </Group>
                    )}
                    {a.command?.operation === 'set_text' && (
                      <Textarea
                        aria-label={`Action ${index + 1} text`}
                        placeholder="Text to enter"
                        size="xs"
                        autosize
                        minRows={1}
                        maxRows={3}
                        style={{ flex: 1, minWidth: 120 }}
                        value={a.command.text}
                        maxLength={4000}
                        disabled={disabled}
                        onChange={(e) => {
                          if (a.command?.operation === 'set_text')
                            update(a.id, { command: { ...a.command, text: e.currentTarget.value } })
                        }}
                      />
                    )}
                    {a.command?.operation === 'swipe' && (
                      <Select
                        aria-label={`Action ${index + 1} direction`}
                        size="xs"
                        value={a.command.direction}
                        data={['up', 'down', 'left', 'right']}
                        disabled={disabled}
                        onChange={(direction) => {
                          if (
                            direction === 'up' ||
                            direction === 'down' ||
                            direction === 'left' ||
                            direction === 'right'
                          )
                            update(a.id, { command: { operation: 'swipe', direction } })
                        }}
                      />
                    )}
                    {a.kind === 'navigate' && (
                      <Textarea
                        aria-label={`Action ${index + 1} AI instruction`}
                        placeholder="Describe what AI should do"
                        title="Uses AI model calls"
                        size="xs"
                        autosize
                        minRows={1}
                        maxRows={3}
                        style={{ flex: 1 }}
                        value={a.instruction}
                        disabled={disabled}
                        maxLength={4000}
                        onChange={(e) => update(a.id, { instruction: e.currentTarget.value })}
                      />
                    )}
                  </Group>
                  <Collapse expanded={checksId === a.id}>
                    {value.checks
                      .filter((c) => c.checkpoint_id === a.checkpoint_id)
                      .map(renderCheck)}
                  </Collapse>
                </Stack>
              </Collapse>
            </Stack>
          </Card>
        )
      })}
      <Button
        variant="light"
        w="fit-content"
        disabled={disabled || value.actions.length >= 20}
        onClick={() => {
          const action = newTaskStep()
          onChange({ ...value, actions: [...value.actions, action] })
          setExpandedId(action.id)
        }}
      >
        Add action
      </Button>
      {value.checks
        .filter((c) => !value.actions.some((a) => a.checkpoint_id === c.checkpoint_id))
        .map(renderCheck)}
    </Stack>
  )
}
