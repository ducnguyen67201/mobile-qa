// Compiled with the app: generated endpoint inputs/output remain authoritative.
import type { GetHealthData, HealthResponse } from '../api/generated/types.gen'
export const healthy: HealthResponse = { status: 'ok', service: 'mobile-qa', version: '0.1.0' }
// @ts-expect-error Missing required response fields must fail compilation.
const staleResponse: HealthResponse = { status: 'ok' }
// @ts-expect-error This read-only operation does not accept a request body.
const unexpectedBody: GetHealthData['body'] = { anything: true }
void staleResponse
void unexpectedBody

// Build metadata and binary inputs must remain generated from Rust transport source.
import type {
  BuildResponse,
  UploadBuildContentData,
  CompleteBuildUploadResponses,
} from '../api/generated/types.gen'
import { build } from './fixtures'
export const persistedBuild: BuildResponse = build
export const binaryUpload: UploadBuildContentData['body'] = { file: new Blob(['synthetic']) }
export const completeStatuses: (keyof CompleteBuildUploadResponses)[] = [200, 202]
// @ts-expect-error A completed build cannot be fabricated from a success boolean.
const staleBuild: BuildResponse = { success: true }
void staleBuild

// Authoring and saved snapshots remain a discriminated Rust-owned pipeline.
import type {
  LibraryDraftDefinition,
  SaveLibraryDraftRequest,
  SetDefaultPlanRequest,
} from '../api/generated/types.gen'
import { libraryDraft } from './library-fixtures'
export const editableDefinition: LibraryDraftDefinition = libraryDraft.definition
// @ts-expect-error Saves require a revision and mutation identity, not only editable content.
const unversionedSave: SaveLibraryDraftRequest = { definition: editableDefinition }
const implicitDefault: SetDefaultPlanRequest = {
  mutation_id: 'id',
  expected_revision: 1,
  // @ts-expect-error Default selection requires an explicit immutable plan version.
  latest: true,
}
void unversionedSave
void implicitDefault
