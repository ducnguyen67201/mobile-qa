import { ArrowUpFromLine, FileArchive, RotateCw, X } from 'lucide-react'
import {
  Badge,
  Button,
  Card,
  Group,
  Loader,
  Progress,
  Stack,
  Text,
  TextInput,
  Title,
} from '@mantine/core'
import { useApkUpload } from '@/hooks/use-apk-upload'
import { ErrorNotice, formatBytes } from './feedback'

export function ApkUpload({
  appId,
  maxBytes,
  multipartEnabled = false,
}: {
  appId: string
  maxBytes: number
  multipartEnabled?: boolean
}) {
  const {
    uploadId,
    upload,
    canResumeMultipart,
    phase,
    error,
    inputError,
    progress,
    selectFile,
    run,
    inspect,
    clear,
    stop,
    updateUrl,
  } = useApkUpload(appId, maxBytes, multipartEnabled)
  return (
    <Card component="section" className="upload-panel" aria-labelledby="upload-title">
      <Group justify="space-between" mb="lg">
        <Stack gap={4}>
          <Title id="upload-title" order={2} size="h5">
            Upload a build
          </Title>
          <Text size="xs" c="dimmed">
            Standalone APK · Up to {formatBytes(maxBytes)} · Private storage
          </Text>
        </Stack>
        <Badge>Android</Badge>
      </Group>
      <Stack gap="md">
        {(!upload ||
          upload.state === 'pending' ||
          (canResumeMultipart && upload.state === 'receiving')) && (
          <TextInput
            key={upload?.id ?? 'new-upload'}
            label="Choose APK file"
            type="file"
            accept=".apk,application/vnd.android.package-archive"
            disabled={!!phase}
            onChange={(event) => selectFile(event.currentTarget.files?.[0] ?? null)}
            description={
              multipartEnabled || canResumeMultipart
                ? 'Interrupted uploads can resume after you reselect the original file. Saved parts are checked against its contents.'
                : 'The file’s contents determine validation, not its name or extension.'
            }
          />
        )}
        {upload && (
          <Group gap="xs" wrap="nowrap">
            <FileArchive size={16} />
            <Text size="xs" className="identifier">
              {upload.original_filename} · {upload.state}
            </Text>
          </Group>
        )}
        {inputError && (
          <Text role="alert" size="sm" c="red">
            {inputError}
          </Text>
        )}
        {progress && (
          <Stack gap={4}>
            <Progress
              value={
                progress.totalBytes ? (progress.completedBytes / progress.totalBytes) * 100 : 0
              }
              aria-label="APK upload progress"
            />
            <Text size="xs" c="dimmed">
              {formatBytes(progress.completedBytes)} of {formatBytes(progress.totalBytes)}{' '}
              transferred
            </Text>
          </Stack>
        )}
        {error != null && (
          <>
            <ErrorNotice error={error} title="Upload needs attention" />
            <Text size="xs" c="dimmed">
              {uploadId
                ? 'Your upload reference is saved. Check server status before trying again.'
                : 'The upload request may have reached the server. No file transfer was started; an unused session will expire automatically.'}
            </Text>
          </>
        )}
        <Group gap="sm">
          {phase ? (
            <>
              <Group role="status" aria-live="polite" gap="xs">
                <Loader size="xs" />
                <Text size="xs">{phase}</Text>
              </Group>
              <Button variant="subtle" onClick={stop} leftSection={<X size={16} />}>
                Stop request
              </Button>
            </>
          ) : (
            <>
              <Button
                onClick={() => void run()}
                leftSection={<ArrowUpFromLine size={16} />}
                disabled={
                  upload?.state === 'expired' ||
                  (upload?.state === 'receiving' && !canResumeMultipart) ||
                  (upload?.state === 'finalized' && !error)
                }
              >
                {upload?.state === 'uploaded'
                  ? 'Validate stored APK'
                  : upload?.state === 'finalized'
                    ? 'Build saved'
                    : (multipartEnabled || canResumeMultipart) && uploadId
                      ? 'Resume and validate'
                      : 'Upload and validate'}
              </Button>
              {upload?.build_id && (
                <Button
                  variant="outline"
                  onClick={() => updateUrl(upload.id, upload.build_id ?? undefined)}
                >
                  View saved build
                </Button>
              )}
              {uploadId && (
                <Button
                  variant="outline"
                  onClick={() => void inspect()}
                  leftSection={<RotateCw size={16} />}
                >
                  Check status
                </Button>
              )}
              {uploadId && (
                <Button variant="subtle" onClick={() => void clear()}>
                  New upload
                </Button>
              )}
            </>
          )}
        </Group>
      </Stack>
    </Card>
  )
}
