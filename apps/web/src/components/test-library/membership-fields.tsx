import {
  Accordion,
  Alert,
  Badge,
  Button,
  Card,
  Checkbox,
  Group,
  NumberInput,
  Select,
  Stack,
  Text,
  Textarea,
  TextInput,
  Title,
} from '@mantine/core'
import type {
  CaseSelection,
  LibraryOptionsResponse,
  LibraryVersionResponse,
  PlanDraftContent,
  SuiteDefinition,
} from '@/api/generated/types.gen'
import { BudgetFields, moveItem, Ordering } from './definition-fields'
const choiceLabel = (v: LibraryVersionResponse) =>
  `${v.version.definition.content.title} · v${v.version.definition.content.version}`
const countLabel = (count: number, singular: string) =>
  `${count} ${singular}${count === 1 ? '' : 's'}`
export function CaseMembership({
  selections,
  options,
  onChange,
  kind,
}: {
  selections: CaseSelection[]
  options: LibraryOptionsResponse
  onChange: (selections: CaseSelection[]) => void
  kind: 'suite' | 'plan'
}) {
  const cases = options.saved_versions.filter((v) => v.version.definition.kind === 'case')
  return (
    <Stack gap="sm">
      <Title order={3} size="h4">
        {kind === 'suite' ? 'Included test cases' : 'Direct cases'}
      </Title>
      <Text size="sm" c="dimmed">
        {kind === 'suite'
          ? 'Each case has its own action sequence and expected checks. Workers pick eligible cases when the device is free. Use these controls to organize or remove them.'
          : 'Add direct cases on the map above. Use these controls to organize or remove them.'}
      </Text>
      {!selections.length && (
        <Text size="sm" c="dimmed">
          Add a saved case directly from the coverage map.
        </Text>
      )}
      {selections.map((selection, i) => {
        const current = cases.find((v) => v.version.id === selection.case_version_id)
        const newer = current
          ? cases
              .filter(
                (v) =>
                  v.entry.id === current.entry.id &&
                  v.version.definition.content.version > current.version.definition.content.version,
              )
              .sort(
                (a, b) =>
                  b.version.definition.content.version - a.version.definition.content.version,
              )[0]
          : undefined
        return (
          <Card key={`${selection.case_version_id}:${i}`} padding="xs" radius="md">
            <Stack gap="xs">
              <Group justify="space-between">
                <Text size="sm" fw={500}>
                  Case {i + 1} ·{' '}
                  {current
                    ? choiceLabel(current)
                    : `Unavailable version · ${selection.case_version_id}`}
                </Text>
                <Ordering
                  index={i}
                  count={selections.length}
                  name={`case selection ${i + 1}`}
                  move={(d) => onChange(moveItem(selections, i, d))}
                  remove={() => onChange(selections.filter((_, index) => i !== index))}
                />
              </Group>
              {!current && (
                <Text size="xs" c="orange">
                  This version may be archived or no longer eligible. Choose a saved replacement or
                  remove it.
                </Text>
              )}
              <Checkbox
                label={`Required case ${i + 1}`}
                checked={selection.required}
                onChange={(e) =>
                  onChange(
                    selections.map((c, index) =>
                      index === i ? { ...c, required: e.currentTarget.checked } : c,
                    ),
                  )
                }
              />
              {newer && (
                <Button
                  variant="subtle"
                  size="xs"
                  w="fit-content"
                  onClick={() =>
                    onChange(
                      selections.map((c, index) =>
                        index === i ? { ...c, case_version_id: newer.version.id } : c,
                      ),
                    )
                  }
                >
                  Use newer saved v{newer.version.definition.content.version}
                </Button>
              )}
            </Stack>
          </Card>
        )
      })}
    </Stack>
  )
}
export function SuiteFields({
  value,
  options,
  onChange,
}: {
  value: SuiteDefinition
  options: LibraryOptionsResponse
  onChange: (value: SuiteDefinition) => void
}) {
  return (
    <Card>
      <Stack gap="lg">
        <TextInput
          label="Suite title"
          maxLength={200}
          placeholder="Core task behavior"
          value={value.title}
          onChange={(e) => onChange({ ...value, title: e.currentTarget.value })}
        />
        <CaseMembership
          selections={value.cases}
          options={options}
          onChange={(cases) => onChange({ ...value, cases })}
          kind="suite"
        />
      </Stack>
    </Card>
  )
}
export function PlanFields({
  value,
  options,
  onChange,
}: {
  value: PlanDraftContent
  options: LibraryOptionsResponse
  onChange: (value: PlanDraftContent) => void
}) {
  const suites = options.saved_versions.filter((v) => v.version.definition.kind === 'suite')
  const profile = options.profiles.find((p) => p.id === value.profile_id)
  return (
    <Stack gap="lg">
      <Card padding="md">
        <Stack>
          <Title order={2} size="h3">
            Your release check
          </Title>
          <TextInput
            label="Release plan title"
            maxLength={200}
            placeholder="Default regression check"
            value={value.title}
            onChange={(e) => onChange({ ...value, title: e.currentTarget.value })}
          />
          <Text size="sm" c="dimmed">
            Keep the selected scope stable across builds. Review this version before making it the
            app’s default.
          </Text>
        </Stack>
      </Card>
      <Card padding="md">
        <Stack gap="lg">
          <Stack gap="sm">
            <Title order={2} size="h3">
              Organize coverage
            </Title>
            <Text size="sm" c="dimmed">
              The map above shows release coverage. These controls change its display order.
            </Text>
            <Text size="sm" fw={600}>
              Included suites
            </Text>
            {value.suite_version_ids.map((id, i) => {
              const current = suites.find((v) => v.version.id === id)
              const newer = current
                ? suites
                    .filter(
                      (v) =>
                        v.entry.id === current.entry.id &&
                        v.version.definition.content.version >
                          current.version.definition.content.version,
                    )
                    .sort(
                      (a, b) =>
                        b.version.definition.content.version - a.version.definition.content.version,
                    )[0]
                : undefined
              return (
                <Stack key={`${id}:${i}`} gap="xs">
                  <Group justify="space-between">
                    <Text size="sm">
                      Suite {i + 1} · {current ? choiceLabel(current) : `Unavailable suite · ${id}`}
                    </Text>
                    <Ordering
                      index={i}
                      count={value.suite_version_ids.length}
                      name={`suite selection ${i + 1}`}
                      move={(d) =>
                        onChange({
                          ...value,
                          suite_version_ids: moveItem(value.suite_version_ids, i, d),
                        })
                      }
                      remove={() =>
                        onChange({
                          ...value,
                          suite_version_ids: value.suite_version_ids.filter(
                            (_, index) => index !== i,
                          ),
                        })
                      }
                    />
                  </Group>
                  {newer && (
                    <Button
                      variant="subtle"
                      size="xs"
                      w="fit-content"
                      onClick={() =>
                        onChange({
                          ...value,
                          suite_version_ids: value.suite_version_ids.map((item, index) =>
                            index === i ? newer.version.id : item,
                          ),
                        })
                      }
                    >
                      Use newer saved suite v{newer.version.definition.content.version}
                    </Button>
                  )}
                </Stack>
              )
            })}
            {!value.suite_version_ids.length && (
              <Text size="sm" c="dimmed">
                Add a saved suite directly from the release map.
              </Text>
            )}
          </Stack>
          <CaseMembership
            selections={value.cases}
            options={options}
            onChange={(cases) => onChange({ ...value, cases })}
            kind="plan"
          />
        </Stack>
      </Card>
      {!profile?.qualified && (
        <Alert color="yellow" title="Execution profile required">
          Open execution and boundaries below, then choose a qualified profile before saving a
          runnable plan.
        </Alert>
      )}
      <Card padding={0}>
        <Accordion variant="contained" defaultValue={profile?.qualified ? null : 'execution'}>
          <Accordion.Item value="execution">
            <Accordion.Control>
              <Group justify="space-between" gap="sm" wrap="wrap">
                <Text fw={600}>Execution & boundaries</Text>
                <Text size="xs" c="dimmed">
                  {profile?.name ?? 'No profile'} · {value.budget.duration_seconds}s ·{' '}
                  {countLabel(value.diagnostic_retries, 'retry')} ·{' '}
                  {countLabel(value.exclusions.length, 'exclusion')}
                </Text>
              </Group>
            </Accordion.Control>
            <Accordion.Panel>
              <Stack>
                <Select
                  label="Execution profile"
                  description="Profiles are configured and qualified by an operator."
                  placeholder="Choose a qualified profile"
                  data={options.profiles.map((p) => ({
                    value: p.id,
                    label: `${p.name} · ${p.driver === 'fake' ? 'Simulation' : p.resolved_model ? 'AI enabled' : p.model_available ? 'Direct only' : 'Setup needs attention'}${p.qualified ? '' : ' · not qualified'}`,
                    disabled: !p.qualified || !p.model_available,
                  }))}
                  value={value.profile_id ?? null}
                  onChange={(profile_id) => onChange({ ...value, profile_id })}
                />
                {profile?.driver === 'fake' && (
                  <Alert color="yellow" title="Simulation profile">
                    This checks the workflow with a fake worker. Its report will not represent a
                    real emulator test.
                  </Alert>
                )}
                {profile && (
                  <Group>
                    <Badge>{profile.adapter}</Badge>
                    <Badge variant="light">
                      {profile.resolved_model
                        ? 'AI enabled'
                        : profile.model_available
                          ? 'Direct only'
                          : 'Setup needs attention'}
                    </Badge>
                    <Text size="xs" c="dimmed">
                      {profile.package}
                    </Text>
                  </Group>
                )}
                <BudgetFields
                  value={value.budget}
                  onChange={(budget) => onChange({ ...value, budget })}
                />
                <NumberInput
                  label="Diagnostic retries"
                  description="Retries help diagnose a failure; they do not replace the original result."
                  min={0}
                  max={1}
                  allowDecimal={false}
                  value={value.diagnostic_retries}
                  onChange={(n) => onChange({ ...value, diagnostic_retries: Number(n) })}
                />
                <Textarea
                  label="Excluded coverage"
                  description="One explicit exclusion per line. Explain what this release check does not cover."
                  minRows={3}
                  value={value.exclusions.join('\n')}
                  onChange={(e) =>
                    onChange({
                      ...value,
                      exclusions: e.currentTarget.value ? e.currentTarget.value.split('\n') : [],
                    })
                  }
                />
              </Stack>
            </Accordion.Panel>
          </Accordion.Item>
        </Accordion>
      </Card>
    </Stack>
  )
}
