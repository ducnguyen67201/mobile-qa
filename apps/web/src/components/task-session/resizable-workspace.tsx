import { useId, useRef, useState, type CSSProperties, type ReactNode } from 'react'
import classes from './phone-workspace.module.css'

/** Keep both panes usable; pointer capture keeps dragging attached outside the handle. */
export function ResizableWorkspace({ children }: { children: [ReactNode, ReactNode] }) {
  const root = useRef<HTMLDivElement>(null)
  const dragging = useRef(false)
  const [left, setLeft] = useState(50)
  const previewId = useId()
  const resize = (value: number) => setLeft(Math.min(70, Math.max(30, value)))
  const move = (clientX: number) => {
    const rect = root.current?.getBoundingClientRect()
    if (rect && rect.width > 0) resize(((clientX - rect.left) / rect.width) * 100)
  }
  return (
    <div
      ref={root}
      className={classes.workspace}
      style={
        { '--editor-width': `${left}fr`, '--preview-width': `${100 - left}fr` } as CSSProperties
      }
    >
      {children[0]}
      <div
        role="separator"
        aria-label="Resize phone preview"
        aria-orientation="vertical"
        aria-valuemin={30}
        aria-valuemax={70}
        aria-valuenow={100 - left}
        aria-valuetext={`Phone preview ${Math.round(100 - left)} percent`}
        aria-controls={previewId}
        tabIndex={0}
        className={classes.divider}
        onDoubleClick={() => setLeft(50)}
        onKeyDown={(event) => {
          if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return
          event.preventDefault()
          resize(
            event.key === 'Home'
              ? 70
              : event.key === 'End'
                ? 30
                : left + (event.key === 'ArrowLeft' ? -2 : 2),
          )
        }}
        onPointerDown={(event) => {
          if (event.button !== 0) return
          event.preventDefault()
          event.currentTarget.focus()
          event.currentTarget.setPointerCapture(event.pointerId)
          dragging.current = true
          move(event.clientX)
        }}
        onPointerMove={(event) => {
          if (dragging.current) move(event.clientX)
        }}
        onPointerUp={(event) => {
          dragging.current = false
          if (event.currentTarget.hasPointerCapture(event.pointerId))
            event.currentTarget.releasePointerCapture(event.pointerId)
        }}
        onPointerCancel={() => {
          dragging.current = false
        }}
        onLostPointerCapture={() => {
          dragging.current = false
        }}
      >
        <span aria-hidden="true" />
      </div>
      <div id={previewId} className={classes.previewColumn}>
        {children[1]}
      </div>
    </div>
  )
}
