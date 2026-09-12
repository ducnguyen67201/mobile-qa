import { expect, it } from 'vitest'
import { formatDuration } from './feedback'
it.each([
  [1800, '30 minutes'],
  [3600, '1 hour'],
  [3661, '1 hour 1 minute 1 second'],
  [0, '0 seconds'],
])('shows exact expiry duration %s', (seconds, expected) => {
  expect(formatDuration(Number(seconds))).toBe(expected)
})
