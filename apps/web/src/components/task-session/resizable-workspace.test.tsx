/** Synthetic geometry exercises resize behavior; it is not rendered layout acceptance. */
import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { expect, it, vi } from 'vitest'
import { ResizableWorkspace } from './resizable-workspace'

it('resizes by keyboard within bounds and resets without unmounting the editor', async () => {
  render(
    <ResizableWorkspace>
      <input aria-label="Draft" defaultValue="Keep my work" />
      <div>Phone</div>
    </ResizableWorkspace>,
  )
  const divider = screen.getByRole('separator', { name: 'Resize phone preview' })
  divider.focus()
  await userEvent.keyboard('{ArrowLeft}')
  expect(divider).toHaveAttribute('aria-valuenow', '52')
  await userEvent.keyboard('{End}{ArrowLeft}')
  expect(divider).toHaveAttribute('aria-valuenow', '70')
  await userEvent.keyboard('{Home}{ArrowRight}')
  expect(divider).toHaveAttribute('aria-valuenow', '30')
  fireEvent.doubleClick(divider)
  expect(divider).toHaveAttribute('aria-valuenow', '50')
  expect(screen.getByLabelText('Draft')).toHaveValue('Keep my work')
})

it('captures a drag, clamps it, and stops resizing after release', () => {
  render(
    <ResizableWorkspace>
      <div>Editor</div>
      <div>Phone</div>
    </ResizableWorkspace>,
  )
  const divider = screen.getByRole('separator')
  const capture = vi.fn()
  const release = vi.fn()
  divider.setPointerCapture = capture
  divider.hasPointerCapture = () => true
  divider.releasePointerCapture = release
  vi.spyOn(divider.parentElement!, 'getBoundingClientRect').mockReturnValue(
    new DOMRect(100, 0, 1000, 800),
  )
  fireEvent.pointerDown(divider, { button: 0, pointerId: 1, clientX: 600 })
  expect(capture).toHaveBeenCalledWith(1)
  fireEvent.pointerMove(divider, { pointerId: 1, clientX: 500 })
  expect(divider).toHaveAttribute('aria-valuenow', '60')
  fireEvent.pointerMove(divider, { pointerId: 1, clientX: 100 })
  expect(divider).toHaveAttribute('aria-valuenow', '70')
  fireEvent.pointerUp(divider, { pointerId: 1 })
  expect(release).toHaveBeenCalledWith(1)
  fireEvent.pointerMove(divider, { pointerId: 1, clientX: 800 })
  expect(divider).toHaveAttribute('aria-valuenow', '70')
})
