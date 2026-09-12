import {
  Alert,
  Badge,
  Box,
  Card,
  Divider,
  Group,
  SimpleGrid,
  Stack,
  Text,
  Title,
} from '@mantine/core'
import { Check, Circle, CircleAlert, Clock3, FileCheck2, Smartphone } from 'lucide-react'
import type { BuildResponse, ReadinessResponse, ValidationState } from '@/api/generated/types.gen'
import { formatBytes, formatDate } from './feedback'

const labels: Record<ValidationState, string> = {
  validating: 'Validating',
  validated: 'Validated',
  invalid: 'Invalid APK',
  unsupported: 'Unsupported',
  error: 'Validation unavailable',
}
const colors: Record<ValidationState, string> = {
  validating: 'blue',
  validated: 'forest',
  invalid: 'red',
  unsupported: 'orange',
  error: 'red',
}
const explanations: Record<ValidationState, string> = {
  validated:
    'Package identity, integrity, signature and intake compatibility have been checked. Device installation has not been checked.',
  error:
    'The validation service could not complete its checks. Retry validation using the same stored file.',
  unsupported: 'Upload a standalone APK compatible with the supported intake policy.',
  invalid: 'Correct the reported issue, then upload a new signed APK.',
  validating: 'You can leave this page. The saved status is available when you return.',
}

export function ValidationBadge({ state }: { state: ValidationState }) {
  const Icon = state === 'validated' ? Check : state === 'validating' ? Clock3 : CircleAlert
  return (
    <Badge color={colors[state]} leftSection={<Icon size={12} />}>
      {labels[state]}
    </Badge>
  )
}

export function BuildDetail({ build }: { build: BuildResponse }) {
  const metadata = build.metadata
  const message =
    build.validation.state === 'validated'
      ? 'APK intake checks passed.'
      : build.validation.state === 'validating'
        ? 'Reading and validating your APK…'
        : (build.validation.message ?? 'This build needs attention.')
  const fields = [
    ['Package', metadata?.package_name ?? 'Not available'],
    [
      'Version',
      metadata
        ? `${metadata.version_name ?? 'Unnamed'} (${metadata.version_code})`
        : 'Not available',
    ],
    ['File size', formatBytes(build.byte_size)],
    ['Uploaded', formatDate(build.created_at)],
    ['Minimum Android API', metadata?.min_sdk ?? 'Not available'],
    ['Target Android API', metadata?.target_sdk ?? 'Not specified'],
    [
      'Native ABIs',
      metadata ? metadata.native_abis.join(', ') || 'No native libraries' : 'Not available',
    ],
    [
      'Signature',
      metadata ? (metadata.signature_verified ? 'Verified' : 'Not verified') : 'Not available',
    ],
  ]

  return (
    <Card>
      <Group justify="space-between" gap="sm" mb="xs">
        <Group gap="xs">
          <FileCheck2 size={20} />
          <Title order={2} size="h5">
            Build identity
          </Title>
        </Group>
        <ValidationBadge state={build.validation.state} />
      </Group>
      <Text size="sm" c="dimmed" className="identifier">
        {build.original_filename}
      </Text>
      <Divider my="lg" />
      <Alert
        color={colors[build.validation.state]}
        title={message}
        role="status"
        aria-live="polite"
        mb="lg"
      >
        {explanations[build.validation.state]}
      </Alert>
      <SimpleGrid component="dl" cols={{ base: 1, xs: 2 }} spacing="lg" m={0}>
        {fields.map(([label, value]) => (
          <Box key={label} miw={0}>
            <Text component="dt" size="xs" c="dimmed">
              {label}
            </Text>
            <Text
              component="dd"
              size="sm"
              fw={500}
              m={0}
              mt={4}
              style={{ overflowWrap: 'anywhere' }}
            >
              {value}
            </Text>
          </Box>
        ))}
      </SimpleGrid>
      <Divider my="lg" />
      <Box component="dl" m={0}>
        <Text component="dt" size="xs" c="dimmed">
          SHA-256 · measured from stored bytes
        </Text>
        <Text component="dd" size="xs" m={0} mt="xs" className="identifier">
          {build.sha256}
        </Text>
      </Box>
      <Text size="xs" c="dimmed" mt="lg" className="identifier">
        Validator {build.validation.validator_version} · Policy{' '}
        {build.validation.intake_policy_version}
        {build.validation.reason_code ? ` · ${build.validation.reason_code}` : ''}
      </Text>
    </Card>
  )
}

export function ReadinessCard({ readiness }: { readiness: ReadinessResponse }) {
  const checkLabel = (state: string) =>
    state === 'operator_reported_ok'
      ? 'Operator reported OK'
      : state === 'operator_reported_blocked'
        ? 'Operator reported blocked'
        : 'Not checked'
  const rows = [
    ['Device installation', 'Not checked'],
    ['Backend access', checkLabel(readiness.backend)],
    ['Test account', checkLabel(readiness.account)],
    ['Reset procedure', checkLabel(readiness.reset)],
    ['Test cases', 'Not configured'],
  ]
  return (
    <Card>
      <Group gap="xs" mb="lg">
        <Smartphone size={16} />
        <Title order={2} size="h5">
          Next: execution setup
        </Title>
      </Group>
      <Stack gap="md">
        {rows.map(([name, state]) => (
          <Group key={name} align="flex-start" gap="xs" wrap="nowrap">
            <Circle size={12} aria-hidden="true" />
            <Group justify="space-between" gap="xs" flex={1}>
              <Text size="xs">{name}</Text>
              <Text size="xs" c="dimmed">
                {state}
              </Text>
            </Group>
          </Group>
        ))}
      </Stack>
      <Divider my="md" />
      <Text size="xs" c="dimmed">
        Device checks are not available yet. A validated APK is the first step; it is not
        confirmation that your app is ready to run.
      </Text>
    </Card>
  )
}
