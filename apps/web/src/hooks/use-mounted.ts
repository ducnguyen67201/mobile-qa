import { useEffect, useRef } from 'react'

/** Late mutation responses may update caches, but must not navigate after leaving a workspace. */
export function useMounted() {
  const mounted = useRef(true)
  useEffect(() => {
    mounted.current = true
    return () => {
      mounted.current = false
    }
  }, [])
  return mounted
}
