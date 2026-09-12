// Adapted from shadcn/ui card, MIT. See LICENSE-SHADCN.
import type { ComponentProps } from 'react'
import { cn } from '../../lib/utils'
export function Card({ className, ...props }: ComponentProps<'div'>) { return <div className={cn('rounded-xl border border-border bg-white shadow-sm', className)} {...props} /> }
export function CardHeader({ className, ...props }: ComponentProps<'div'>) { return <div className={cn('flex flex-col gap-2 p-6', className)} {...props} /> }
export function CardTitle({ className, ...props }: ComponentProps<'h2'>) { return <h2 className={cn('text-lg font-semibold tracking-tight', className)} {...props} /> }
export function CardDescription({ className, ...props }: ComponentProps<'p'>) { return <p className={cn('text-sm leading-relaxed text-muted-foreground', className)} {...props} /> }
export function CardContent({ className, ...props }: ComponentProps<'div'>) { return <div className={cn('px-6 pb-6', className)} {...props} /> }
