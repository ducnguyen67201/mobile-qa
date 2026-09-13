/** Synthetic browser fixtures. They exercise generated contracts, not device execution. */
import type {
  CaseDefinition,
  LibraryCapabilities,
  LibraryCoveragePreview,
  LibraryDraftResponse,
  LibraryEntryResponse,
  LibraryOptionsResponse,
  LibraryVersionResponse,
} from '@/api/generated/types.gen'
import { appId, timestamp } from './fixtures'
export const entryId = '11111111-1111-4111-8111-111111111111'
export const versionId = '22222222-2222-4222-8222-222222222222'
export const mutationId = '33333333-3333-4333-8333-333333333333'
export const capabilities: LibraryCapabilities = {
  can_edit: true,
  can_review_business: true,
  can_review_executability: true,
  can_archive: true,
  can_set_default: true,
}
export const libraryCase: CaseDefinition = {
  key: 'task-survives-restart',
  version: 1,
  title: 'A saved task survives a restart',
  requirement: 'Saved work remains available after reopening the app.',
  provenance: 'user_authored',
  package: 'ai.mobileqa.demo',
  adapter: 'demo_persistence_v1',
  preconditions: [],
  actions: [
    {
      id: 'create_task',
      kind: 'navigate',
      instruction: 'Create and save ${task_title}.',
      checkpoint_id: 'created',
    },
    { id: 'restart', kind: 'restart_app', instruction: '', checkpoint_id: 'reopened' },
  ],
  checks: [
    {
      id: 'persisted',
      checkpoint_id: 'reopened',
      description: 'The saved task remains visible.',
      method: 'ui_element_presence_v1',
      resource_id: 'ai.mobileqa.demo:id/task_title',
      text_filter: '${task_title}',
      property: 'text',
      expected: 'true',
      ready_resource_id: 'ai.mobileqa.demo:id/task_list',
      prerequisite_check_ids: [],
      required: true,
      observation_seconds: 10,
    },
  ],
  budget: { duration_seconds: 600, max_steps: 30, artifact_bytes: 16777216 },
}
export const libraryEntry: LibraryEntryResponse = {
  id: entryId,
  app_id: appId,
  kind: 'case',
  key: libraryCase.key,
  title: libraryCase.title,
  revision: 1,
  archived_at: null,
  draft_version: 1,
  latest_version_id: null,
  latest_review_state: null,
  updated_at: timestamp,
  capabilities,
}
export const emptyCoverage: LibraryCoveragePreview = {
  cases: [],
  required_count: 0,
  exclusions: [],
  issues: [],
}
export const libraryDraft: LibraryDraftResponse = {
  entry: libraryEntry,
  definition: { kind: 'case', content: libraryCase },
  source_version_id: null,
  issues: [],
  coverage: emptyCoverage,
}
export const libraryVersion: LibraryVersionResponse = {
  entry: {
    ...libraryEntry,
    draft_version: null,
    latest_version_id: versionId,
    latest_review_state: 'in_review',
    revision: 2,
  },
  version: {
    id: versionId,
    app_id: appId,
    content_hash: 'a'.repeat(64),
    definition: { kind: 'case', content: libraryCase },
    approvals: [],
  },
  review_state: 'in_review',
  review_events: [],
  coverage: emptyCoverage,
  issues: [],
}
export const libraryOptions: LibraryOptionsResponse = {
  profiles: [],
  approved_versions: [],
  capabilities,
}
