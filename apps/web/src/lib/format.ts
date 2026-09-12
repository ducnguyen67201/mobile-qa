export function formatBytes(value: number) {
  return value >= 1024 * 1024 ? `${(value / 1024 / 1024).toFixed(1)} MB` : `${(value / 1024).toFixed(1)} KB`
}
export function formatDate(value: string) {
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeStyle: 'short' }).format(
    new Date(value),
  )
}

/** Preserve short and mixed configured durations instead of rounding expiry windows. */
export function formatDuration(seconds: number) {
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  const remainder = seconds % 60
  const parts: string[] = []
  if (hours) parts.push(`${hours} ${hours === 1 ? 'hour' : 'hours'}`)
  if (minutes) parts.push(`${minutes} ${minutes === 1 ? 'minute' : 'minutes'}`)
  if (remainder || parts.length === 0) parts.push(`${remainder} ${remainder === 1 ? 'second' : 'seconds'}`)
  return parts.join(' ')
}
