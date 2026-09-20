/** Locate server validation issues in the editable case without duplicating its validator. */
import type { CaseDefinition, LibraryIssue } from '@/api/generated/types.gen'

export type IssueFocus = { issue: LibraryIssue; request: number }

export function issueFieldId(prefix: string, field: string, itemId?: string | null) {
  return `${prefix}-${field}-${itemId ?? 'case'}`
}

export function caseIssueLocation(value: CaseDefinition, issue: LibraryIssue): string | null {
  if (issue.field === 'title') return 'Test name'
  if (issue.field === 'requirement') return 'Requirement and setup'
  if (issue.field === 'actions') return 'Actions'
  if (issue.field === 'checks') return 'Result checks'
  const checkIndex = value.checks.findIndex((c) => c.id === issue.item_id)
  const selectedCheck = value.checks[checkIndex]
  if (selectedCheck && issue.field.startsWith('checks.')) {
    const actionIndex = value.actions.findIndex(
      (a) => a.checkpoint_id === selectedCheck.checkpoint_id,
    )
    const field = {
      'checks.resource_id': 'Control',
      'checks.description': 'Description',
      'checks.checkpoint_id': 'After action',
    }[issue.field]
    if (!field) return null
    return `${actionIndex >= 0 ? `Action ${actionIndex + 1} → ` : ''}Check ${checkIndex + 1} → ${field}`
  }
  const actionIndex = value.actions.findIndex((a) => a.id === issue.item_id)
  if (actionIndex >= 0 && issue.field === 'actions.instruction')
    return `Action ${actionIndex + 1} → AI instruction`
  return null
}

export function caseIssueMessage(issue: LibraryIssue) {
  if (issue.field === 'checks.resource_id')
    return 'Pick the control to check on the phone. It must belong to this app.'
  if (issue.field === 'definition' && issue.message === 'invalid check')
    return 'A check is incomplete or invalid. Review your checks, then save again.'
  return issue.message
}

function fieldValue(value: CaseDefinition, issue: LibraryIssue): unknown {
  switch (issue.field) {
    case 'title':
      return value.title
    case 'requirement':
      return value.requirement
    case 'actions':
      return value.actions
    case 'checks':
      return value.checks
    case 'actions.instruction':
      return value.actions.find((a) => a.id === issue.item_id)?.instruction
    case 'checks.resource_id': {
      const check = value.checks.find((c) => c.id === issue.item_id)
      return check && [check.resource_id, check.ready_resource_id]
    }
    case 'checks.description':
      return value.checks.find((c) => c.id === issue.item_id)?.description
    case 'checks.checkpoint_id': {
      const checkpoint = value.checks.find((c) => c.id === issue.item_id)?.checkpoint_id
      return [checkpoint, value.actions.some((a) => a.checkpoint_id === checkpoint)]
    }
    default:
      return undefined
  }
}

/** A server error belongs to the submitted value. Edits are revalidated on Save. */
export function currentCaseIssues(
  issues: LibraryIssue[],
  saved: CaseDefinition,
  current: CaseDefinition,
) {
  return issues.filter(
    (issue) =>
      JSON.stringify(fieldValue(saved, issue)) === JSON.stringify(fieldValue(current, issue)),
  )
}

/** Wait for disclosure state to commit before focusing its previously hidden input. */
export function focusIssueField(id: string) {
  const frame = requestAnimationFrame(() => {
    const field = document.getElementById(id)
    field?.scrollIntoView?.({ block: 'center', behavior: 'instant' })
    field?.focus({ preventScroll: true })
  })
  return () => cancelAnimationFrame(frame)
}
