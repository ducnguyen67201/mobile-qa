// Explicit unfinished-feature state. Replace it with persisted behavior as each spec lands.
import { Card, CardHeader, CardTitle, CardDescription } from '../components/ui/card'
export function Placeholder({ title, description }: { title: string; description: string }) {
  return <><h1 className="text-3xl font-semibold tracking-tight">{title}</h1><Card className="mt-10 max-w-3xl"><CardHeader><CardTitle>{description}</CardTitle><CardDescription>This area is planned for a later phase.</CardDescription></CardHeader></Card></>
}
