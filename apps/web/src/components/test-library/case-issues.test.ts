import { expect, it } from 'vitest'
import { libraryCase } from '@/test/library-fixtures'
import type { LibraryIssue } from '@/api/generated/types.gen'
import { caseIssueLocation, currentCaseIssues } from './case-issues'

const issue: LibraryIssue = {
  code: 'invalid_content',
  field: 'checks.resource_id',
  item_id: 'persisted',
  message: 'Pick a control',
}
it('keeps issue locations tied to identity after reordering or removing the owning action', () => {
  expect(caseIssueLocation(libraryCase, issue)).toBe('Action 2 → Check 1 → Control')
  expect(
    caseIssueLocation({ ...libraryCase, actions: [...libraryCase.actions].reverse() }, issue),
  ).toBe('Action 1 → Check 1 → Control')
  expect(caseIssueLocation({ ...libraryCase, actions: [] }, issue)).toBe('Check 1 → Control')
  expect(caseIssueLocation(libraryCase, { ...issue, item_id: 'missing' })).toBeNull()
})
it('clears only the edited server field, including the paired readiness target, until Save', () => {
  const baseline = {
    ...libraryCase,
    checks: [{ ...libraryCase.checks[0]!, ready_resource_id: '' }],
  }
  expect(currentCaseIssues([issue], baseline, { ...baseline, title: 'Renamed' })).toEqual([issue])
  expect(currentCaseIssues([issue], baseline, libraryCase)).toEqual([])
  expect(currentCaseIssues([issue], baseline, { ...baseline, checks: [] })).toEqual([])
  expect(currentCaseIssues([issue], baseline, baseline)).toEqual([issue])
})
