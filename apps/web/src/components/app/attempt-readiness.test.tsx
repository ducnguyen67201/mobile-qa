import { MantineProvider } from '@mantine/core'
import { render, screen } from '@testing-library/react'
import type { AttemptResponse, PreflightReceipt } from '@/api/generated/types.gen'
import { expect, it } from 'vitest'
import { AttemptReadiness } from './attempt-readiness'

const id = '33333333-3333-4333-8333-333333333333'
function attempt(): AttemptResponse {
  return {
    id,
    case_version_id: id,
    generation: 1,
    number: 1,
    state: 'finished',
    outcome: 'passed',
    cleanup: 'verified_clean',
    reason: null,
    checks: [],
    artifacts: [],
    usage: [],
    events: [],
  }
}
function receipt(): PreflightReceipt {
  return {
    attempt_id: id,
    instance_nonce: id,
    build_sha256: 'a'.repeat(64),
    started_at: '2026-09-15T00:00:00Z',
    ready_at: '2026-09-15T00:00:01Z',
    duration_ms: 1000,
    artifact_ids: [],
    context: {
      schema_version: 1,
      adapter_revision: 'android_direct_v1',
      verifier_revision: 'ui_v1',
      worker_runtime_revision: 'direct_v1',
      reset_policy_hash: 'b'.repeat(64),
      qualified_profile_id: id,
      package: 'com.example.test',
      launch_component: 'com.example.test/.MainActivity',
      image: 'system-images;android-35;google_apis;arm64-v8a',
      abi: 'arm64-v8a',
      width: 1080,
      height: 1920,
      density: 420,
      locale: 'en-US',
      timezone: 'Etc/UTC',
      state_scope: 'local_only',
      qualification_reference: 'fixture qualification',
      starting_checks: [],
      stages: { boot_seconds: 30, install_seconds: 30, start_seconds: 30, cleanup_seconds: 30 },
    },
  }
}
function show(value: AttemptResponse, requiresCleanStart = false, simulated = false) {
  return render(
    <MantineProvider env="test">
      <AttemptReadiness
        attempt={value}
        runId={id}
        requiresCleanStart={requiresCleanStart}
        simulated={simulated}
      />
    </MantineProvider>,
  )
}

it('does not infer a verified clean start from a legacy passed attempt', () => {
  show(attempt())
  expect(screen.getByText('Clean start not recorded')).toBeInTheDocument()
  expect(screen.queryByText('Clean start verified')).not.toBeInTheDocument()
  expect(screen.getByText('Start and cleanup details').closest('details')).not.toHaveAttribute(
    'open',
  )
})

it('distinguishes pending preparation from missing proof after completion', () => {
  const view = show({ ...attempt(), state: 'running', outcome: null, cleanup: 'pending' }, true)
  expect(screen.getByText('Clean start pending')).toBeInTheDocument()
  view.unmount()
  show(attempt(), true)
  expect(screen.getByText('Clean start missing')).toBeInTheDocument()
})

it('does not infer that startup never ran from cancellation without a receipt', () => {
  show({ ...attempt(), outcome: 'canceled' }, true)
  expect(screen.getByText('Clean start not recorded')).toBeInTheDocument()
})

it('keeps failed original cleanup visible after recovery releases the phone', () => {
  show({
    ...attempt(),
    outcome: 'failed',
    original_cleanup: {
      generation: 1,
      stopped: false,
      reset: 'quarantined',
      evidence_reference: 'Stop could not be verified',
      boot_id: 'fixture',
    },
    recovery_events: [
      { actor_id: id, created_at: '2026-09-15T00:00:00Z', evidence_reference: 'Phone discarded' },
    ],
  })
  expect(screen.getByText('Cleanup needs attention')).toBeInTheDocument()
  expect(screen.getByText('1 recovery record')).toBeInTheDocument()
  expect(screen.queryByText('Cleanup verified')).not.toBeInTheDocument()
  expect(screen.getByText(/The phone was released after recovery/)).toBeInTheDocument()
})

it('distinguishes verified receipts from simulated receipts', () => {
  const view = show({ ...attempt(), preflight: receipt() }, true)
  expect(screen.getByText('Clean start verified')).toBeInTheDocument()
  view.unmount()
  show({ ...attempt(), preflight: receipt() }, true, true)
  expect(screen.getByText('Simulated clean start')).toBeInTheDocument()
  expect(screen.getByText('Simulated cleanup')).toBeInTheDocument()
  expect(screen.queryByText('Clean start verified')).not.toBeInTheDocument()
})

it('keeps server quarantine visible even when local disposal succeeded', () => {
  show(
    {
      ...attempt(),
      state: 'recovery_required',
      outcome: 'inconclusive',
      cleanup: 'quarantined',
      original_cleanup: {
        generation: 1,
        stopped: true,
        reset: 'verified_clean',
        evidence_reference: 'Local disposal completed without a start acknowledgement',
        boot_id: 'fixture',
      },
    },
    true,
  )
  expect(screen.getByText('Clean start missing')).toBeInTheDocument()
  expect(screen.getByText('Cleanup needs attention')).toBeInTheDocument()
  expect(screen.queryByText('Cleanup verified')).not.toBeInTheDocument()
})
