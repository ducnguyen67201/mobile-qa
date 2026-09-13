import type { TestAction } from '@/api/generated/types.gen'

/** Read-only summaries never substitute for the structured commands sent to the worker. */
export function actionLabel(action: TestAction): string {
  if (action.kind === 'navigate') return action.instruction
  if (action.kind === 'restart_app') return 'Restart the app, preserving saved data'
  if (action.kind === 'checkpoint') return 'Capture checkpoint evidence'
  const command = action.command
  if (!command) return 'Choose a direct action'
  const target = 'target' in command ? command.target.value : ''
  switch (command.operation) {
    case 'tap':
      return `Tap ${target}`
    case 'set_text':
      return `Enter ${JSON.stringify(command.text)} into ${target}`
    case 'wait_for':
      return `Wait for ${target}`
    case 'swipe':
      return `Swipe ${command.direction}`
    case 'back':
      return 'Go back'
    case 'restart':
      return 'Restart the app, preserving saved data'
  }
}
