import { useQuery } from '@tanstack/react-query'
import { healthQuery } from '../api/queries'
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '../components/ui/card'
import { Button } from '../components/ui/button'
export function Home() {
  const health = useQuery(healthQuery)
  return <>
    <p className="mb-3 text-xs font-semibold uppercase tracking-widest text-muted-foreground">Workspace</p>
    <h1 className="text-3xl font-semibold tracking-tight">App</h1>
    <p className="mt-3 text-muted-foreground">Your mobile QA workspace.</p>
    <Card className="mt-10 max-w-3xl">
      <CardHeader><CardTitle>Start with your app</CardTitle><CardDescription>App onboarding will be added in the next phase.</CardDescription></CardHeader>
      <CardContent><div className="flex flex-wrap items-center justify-between gap-4 rounded-lg border border-border bg-background px-4 py-3">
        <div role="status" aria-live="polite" className="text-sm">
          <span className="mr-2 text-muted-foreground">API:</span>
          {health.isFetching ? 'Connecting…' : health.isError ? 'Cannot connect' : 'Ready'}
        </div>
        {health.isError && <Button onClick={() => void health.refetch()} disabled={health.isFetching}>Retry</Button>}
        {health.isSuccess && !health.isFetching && <span className="text-xs text-muted-foreground">{health.data.service} · v{health.data.version}</span>}
      </div>
      {health.isError && <p className="mt-3 text-sm text-muted-foreground">The workspace service is unavailable. Try connecting again.</p>}
      </CardContent>
    </Card>
  </>
}
