// Adapted from shadcn/ui button, MIT. See LICENSE-SHADCN.
import type { ComponentProps } from 'react'
import { cn } from '../../lib/utils'
export function Button({ className, type = 'button', ...props }: ComponentProps<'button'>) {
  return <button type={type} className={cn('inline-flex h-9 items-center justify-center rounded-md bg-foreground px-4 text-sm font-medium text-white transition-colors hover:opacity-90 disabled:pointer-events-none disabled:opacity-50', className)} {...props} />
}
