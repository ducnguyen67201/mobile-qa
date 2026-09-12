import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'
/** Merge conditional classes, resolving Tailwind conflicts so caller overrides work. */
export function cn(...inputs: ClassValue[]) { return twMerge(clsx(inputs)) }
