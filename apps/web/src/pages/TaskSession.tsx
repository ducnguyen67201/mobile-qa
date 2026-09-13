/** The standalone entry shares its task/phone workspace with saved test editing. */
import { Alert, Loader } from '@mantine/core'
import { useQuery } from '@tanstack/react-query'
import { useParams } from 'react-router'
import { useWorkspace } from '@/hooks/use-workspace'
import { appQuery } from '@/api/setup'
import { ErrorNotice } from '@/components/app/feedback'
import { PhoneWorkspace } from '@/components/task-session/phone-workspace'

export function TaskSession() {
  const { app_id = '' } = useParams()
  const { workspaceId } = useWorkspace()
  const app = useQuery(appQuery(app_id))
  if (app.isError) return <ErrorNotice error={app.error} retry={() => void app.refetch()} />
  if (!app.data) return <Loader aria-label="Loading app" />
  if (app.data.organization_id !== workspaceId)
    return <Alert>This app belongs to a different workspace.</Alert>
  return <PhoneWorkspace key={`${workspaceId}:${app_id}`} appId={app_id} />
}
