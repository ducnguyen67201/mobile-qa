import { expect, it } from 'vitest'
import { actionLabel } from './action-label'
it('shows the actual direct command and literal text in review and reports', () => {
  expect(
    actionLabel({
      id: 'a',
      checkpoint_id: 'a',
      kind: 'direct',
      instruction: '',
      command: {
        operation: 'set_text',
        target: { by: 'resource_id', value: 'ai.mobileqa.demo:id/task_input' },
        text: 'Xin chào',
      },
    }),
  ).toBe('Enter "Xin chào" into ai.mobileqa.demo:id/task_input')
})
